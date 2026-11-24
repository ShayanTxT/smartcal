use crate::fonts::configure_fonts;
use crate::icons::{IconKind, paint_brand_logo};
use crate::theme::{Palette, apply_theme, bidi_job, card_frame, status_badge};
use crate::widgets::{
    accuracy_vector_map, action_card, calibration_pattern_preview, wizard_stepper,
};
use chrono::Utc;
use eframe::egui::{
    self, Align, Color32, CursorIcon, Key, Layout, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};
use smartcal_core::{
    AppSettings, CalibrationConfig, CalibrationMethod, CalibrationProfile, CalibrationQuality,
    CalibrationType, ConnectionStatus, DeviceKind, DiagnosticsReport, DisplayInfo, InputDeviceInfo,
    Language, LogLevel, Point2D, ProfileStore, SmartCalError, ThemeMode, TouchCapability,
    recent_logs, record_log,
};
use smartcal_i18n::{TextKey, tr};
use smartcal_linux::{
    InputStreamHandle, ProjectorGuard, RawTouchEvent, UDEV_PERMISSIONS_RULE, apply_calibration,
    collect_diagnostics, format_diagnostics_text, install_permissions_udev_rule, reset_calibration,
    start_device_stream, start_hotplug_monitor, virtual_desktop_size,
};
use smartcal_math::{
    CalibrationOutput, RawSample, compute_calibration, evaluate_model_raw_to_screen,
    filter_touch_samples, generate_target_grid, is_quality_acceptable, map_screen_to_raw,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    WizardStep1,
    WizardStep2,
    WizardStep3,
    FullscreenCalibrating,
    Results,
    TestMode,
    Profiles,
    Settings,
    Diagnostics,
}

#[derive(Debug, Clone)]
struct TrailSample {
    norm_pos: Point2D,
    raw_pos: Point2D,
    error_px: f64,
}

#[derive(Debug, Clone)]
struct NotificationBanner {
    text: String,
    is_error: bool,
}

pub struct SmartCalApp {
    store: ProfileStore,
    settings: AppSettings,
    screen: Screen,

    // Hardware state
    displays: Vec<DisplayInfo>,
    devices: Vec<InputDeviceInfo>,
    selected_display_idx: usize,
    selected_device_idx: usize,
    hotplug_handle: Option<InputStreamHandle>,
    startup_match: Option<CalibrationProfile>,
    diagnostics: Option<DiagnosticsReport>,

    // Step 2 live probe state
    probe_stream: Option<InputStreamHandle>,
    probe_device_idx: Option<usize>,
    probe_last_raw: Option<(Point2D, Instant)>,

    // Wizard / Calibration session state
    cal_config: CalibrationConfig,
    targets: Vec<Point2D>,
    current_target_idx: usize,
    captured_points: Vec<smartcal_math::FilteredTouch>,
    active_samples: Vec<RawSample>,
    is_touch_held: bool,
    target_started_at: Option<Instant>,
    touch_error_banner: Option<String>,
    device_disconnected: bool,
    input_stream: Option<InputStreamHandle>,
    projector_guard: Option<ProjectorGuard>,
    using_pointer_fallback: bool,

    // Results & Profile state
    latest_output: Option<CalibrationOutput>,
    latest_profile: Option<CalibrationProfile>,
    profile_name_input: String,
    apply_on_startup: bool,
    profiles_list: Vec<CalibrationProfile>,

    // Test Mode state
    test_fullscreen: bool,
    test_grid_cols: usize,
    test_grid_rows: usize,
    test_trail: Vec<TrailSample>,
    test_last_raw: Option<Point2D>,
    test_last_screen: Option<Point2D>,
    test_last_error: Option<f64>,
    test_pointer_down: bool,

    // Diagnostics & Log Viewer
    log_filter: Option<LogLevel>,
    banner: Option<NotificationBanner>,
}

impl SmartCalApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        let store = ProfileStore::open_default();
        let settings = store.load_settings();
        apply_theme(&cc.egui_ctx, settings.theme);

        let mut app = Self {
            cal_config: settings.calibration.clone(),
            settings,
            store,
            screen: Screen::Home,
            displays: Vec::new(),
            devices: Vec::new(),
            selected_display_idx: 0,
            selected_device_idx: 0,
            hotplug_handle: Some(start_hotplug_monitor()),
            startup_match: None,
            diagnostics: None,
            probe_stream: None,
            probe_device_idx: None,
            probe_last_raw: None,
            targets: Vec::new(),
            current_target_idx: 0,
            captured_points: Vec::new(),
            active_samples: Vec::new(),
            is_touch_held: false,
            target_started_at: None,
            touch_error_banner: None,
            device_disconnected: false,
            input_stream: None,
            projector_guard: None,
            using_pointer_fallback: false,
            latest_output: None,
            latest_profile: None,
            profile_name_input: "classroom-projector".to_string(),
            apply_on_startup: ProfileStore::is_autostart_enabled(),
            profiles_list: Vec::new(),
            test_fullscreen: false,
            test_grid_cols: 6,
            test_grid_rows: 4,
            test_trail: Vec::new(),
            test_last_raw: None,
            test_last_screen: None,
            test_last_error: None,
            test_pointer_down: false,
            log_filter: None,
            banner: None,
        };

        app.refresh_hardware(true);
        record_log(
            LogLevel::Info,
            "gui",
            "SmartCal GUI initialized and hardware scan completed",
        );
        app
    }

    fn lang(&self) -> Language {
        self.settings.language
    }

    fn is_rtl(&self) -> bool {
        self.settings.language == Language::Persian
    }

    fn t(&self, key: TextKey) -> String {
        tr(self.lang(), key)
    }

    fn j(&self, text: &str, size: f32, color: Color32) -> egui::text::LayoutJob {
        bidi_job(self.lang(), text, size, color, None)
    }

    fn jt(&self, key: TextKey, size: f32, color: Color32) -> egui::text::LayoutJob {
        let s = self.t(key);
        bidi_job(self.lang(), &s, size, color, None)
    }

    fn j_wrap(
        &self,
        text: &str,
        size: f32,
        color: Color32,
        max_chars: usize,
    ) -> egui::text::LayoutJob {
        bidi_job(self.lang(), text, size, color, Some(max_chars))
    }

    fn kind_label(&self, kind: DeviceKind) -> String {
        match kind {
            DeviceKind::Touchscreen => self.t(TextKey::Touchscreen),
            DeviceKind::PenStylus => self.t(TextKey::PenStylus),
            DeviceKind::Mouse => self.t(TextKey::Mouse),
            DeviceKind::Touchpad => self.t(TextKey::Touchpad),
            DeviceKind::Other => "Other".into(),
        }
    }

    fn cap_label(&self, cap: TouchCapability) -> String {
        match cap {
            TouchCapability::SingleTouch => self.t(TextKey::SingleTouch),
            TouchCapability::MultiTouch { max_slots } => {
                if max_slots > 0 {
                    format!("{} ({max_slots})", self.t(TextKey::MultiTouch))
                } else {
                    self.t(TextKey::MultiTouch)
                }
            }
            TouchCapability::PointerOnly => format!("{cap}"),
        }
    }

    fn status_label(&self, st: ConnectionStatus) -> String {
        match st {
            ConnectionStatus::Connected => self.t(TextKey::Connected),
            ConnectionStatus::Disconnected => self.t(TextKey::Disconnected),
            ConnectionStatus::PermissionDenied => self.t(TextKey::PermissionDenied),
            ConnectionStatus::Busy => "Busy".into(),
        }
    }

    fn method_label(&self, m: CalibrationMethod) -> String {
        let key = match m {
            CalibrationMethod::Homography => TextKey::MethodHomography,
            CalibrationMethod::HomographyRansac => TextKey::MethodHomographyRansac,
            CalibrationMethod::Affine => TextKey::MethodAffine,
            CalibrationMethod::PolynomialDeg2 => TextKey::MethodPoly2,
            CalibrationMethod::PolynomialDeg3 => TextKey::MethodPoly3,
            CalibrationMethod::HybridTps => TextKey::MethodHybridTps,
        };
        self.t(key)
    }

    fn stop_probe(&mut self) {
        self.probe_stream = None;
        self.probe_device_idx = None;
        self.probe_last_raw = None;
    }

    fn sync_step2_probe(&mut self) {
        if self.screen != Screen::WizardStep2 {
            if self.probe_stream.is_some() {
                self.stop_probe();
            }
            return;
        }
        if self.probe_device_idx != Some(self.selected_device_idx) {
            self.probe_stream = self
                .devices
                .get(self.selected_device_idx)
                .filter(|d| d.readable)
                .map(start_device_stream);
            self.probe_device_idx = Some(self.selected_device_idx);
            self.probe_last_raw = None;
        }
        if let Some(stream) = &self.probe_stream {
            while let Ok(ev) = stream.rx.try_recv() {
                if let RawTouchEvent::Down(pt) | RawTouchEvent::Sample(pt) = ev {
                    self.probe_last_raw = Some((pt, Instant::now()));
                }
            }
        }
    }

    fn refresh_hardware(&mut self, check_startup_match: bool) {
        let diag = collect_diagnostics(&self.store);
        self.displays = diag.displays.clone();
        self.devices = diag.input_devices.clone();
        self.diagnostics = Some(diag);
        self.profiles_list = self.store.list_profiles();

        if let Some(proj_idx) = self
            .displays
            .iter()
            .position(|d| d.is_projector_candidate())
        {
            self.selected_display_idx = proj_idx;
        } else {
            self.selected_display_idx = 0;
        }

        if let Some(touch_idx) = self.devices.iter().position(|d| d.is_interactive_touch()) {
            self.selected_device_idx = touch_idx;
        } else {
            self.selected_device_idx = 0;
        }

        if check_startup_match
            && self.settings.auto_detect_on_startup
            && let Some(dev) = self.devices.get(self.selected_device_idx)
        {
            let disp = self.displays.get(self.selected_display_idx);
            self.startup_match = self.store.find_matching_profile(dev, disp);
        }
    }

    fn selected_display(&self) -> Option<&DisplayInfo> {
        self.displays.get(self.selected_display_idx)
    }

    fn selected_device(&self) -> Option<&InputDeviceInfo> {
        self.devices.get(self.selected_device_idx)
    }

    fn enter_fullscreen_on_selected_display(&self, ctx: &egui::Context) {
        if let Some(disp) = self.selected_display() {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(
                disp.x as f32,
                disp.y as f32,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                disp.width as f32,
                disp.height as f32,
            )));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
    }

    fn exit_fullscreen(&mut self, ctx: &egui::Context) {
        self.projector_guard = None;
        self.input_stream = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
    }

    fn start_calibration_session(&mut self, ctx: &egui::Context) {
        self.stop_probe();
        let Some(disp) = self.selected_display().cloned() else {
            return;
        };
        let Some(dev) = self.selected_device().cloned() else {
            return;
        };

        let sess = smartcal_linux::detect_session();
        self.projector_guard = Some(ProjectorGuard::enter(
            self.cal_config.projector_mode,
            sess.display_server,
        ));
        self.targets = generate_target_grid(
            disp.width,
            disp.height,
            self.cal_config.calibration_type,
            self.cal_config.margin_ratio,
        );
        self.current_target_idx = 0;
        self.captured_points.clear();
        self.active_samples.clear();
        self.is_touch_held = false;
        self.touch_error_banner = None;
        self.device_disconnected = false;
        self.target_started_at = Some(Instant::now());
        // On non-Linux hosts or when evdev lacks permissions, allow instant GUI click capture.
        self.using_pointer_fallback = !cfg!(target_os = "linux") || !dev.readable;
        self.input_stream = if cfg!(target_os = "linux") && dev.readable {
            Some(start_device_stream(&dev))
        } else {
            None
        };

        self.enter_fullscreen_on_selected_display(ctx);
        self.screen = Screen::FullscreenCalibrating;
        record_log(
            LogLevel::Info,
            "wizard",
            format!(
                "Started {} calibration on '{}' ({}x{}) using '{}'",
                self.cal_config.calibration_type.point_count(),
                disp.name,
                disp.width,
                disp.height,
                dev.name
            ),
        );
    }

    fn finish_calibration(&mut self, ctx: &egui::Context) {
        self.exit_fullscreen(ctx);

        let Some(disp) = self.selected_display().cloned() else {
            self.screen = Screen::Home;
            return;
        };
        let Some(dev) = self.selected_device().cloned() else {
            self.screen = Screen::Home;
            return;
        };

        let (virt_w, virt_h) = virtual_desktop_size(&self.displays);
        let raw_bounds = dev.abs_bounds.unwrap_or_default();

        match compute_calibration(
            &disp,
            virt_w,
            virt_h,
            &raw_bounds,
            &self.cal_config,
            &self.targets,
            &self.captured_points,
        ) {
            Ok(out) => {
                let sess = smartcal_linux::detect_session();
                let profile = CalibrationProfile {
                    profile_name: ProfileStore::sanitize_name(&self.profile_name_input),
                    device: dev.name.clone(),
                    device_id: dev.id.clone(),
                    vendor_id: dev.vendor_id.clone(),
                    product_id: dev.product_id.clone(),
                    device_name: dev.name.clone(),
                    device_path: dev.event_path.clone(),
                    display: disp.name.clone(),
                    display_id: disp.id.clone(),
                    resolution: [disp.width, disp.height],
                    display_position: [disp.x, disp.y],
                    virtual_desktop_size: [virt_w, virt_h],
                    calibration_type: self.cal_config.calibration_type,
                    method: self.cal_config.method,
                    matrix: out.matrix,
                    model: out.model.clone(),
                    raw_bounds,
                    points: out.points.clone(),
                    error: out.error.clone(),
                    timestamp: Utc::now(),
                    session_type: sess.session_type,
                    display_server: sess.display_server,
                    auto_apply: self.apply_on_startup,
                };
                self.latest_output = Some(out);
                self.latest_profile = Some(profile);
                self.screen = Screen::Results;
            }
            Err(err) => {
                self.banner = Some(NotificationBanner {
                    text: err.user_message(),
                    is_error: true,
                });
                self.screen = Screen::WizardStep3;
            }
        }
    }

    fn process_calibration_touch_samples(&mut self, ctx: &egui::Context) {
        let mut events = Vec::new();
        if let Some(stream) = &self.input_stream {
            while let Ok(ev) = stream.rx.try_recv() {
                events.push(ev);
            }
        }

        for ev in events {
            match ev {
                RawTouchEvent::Down(pt) => {
                    self.is_touch_held = true;
                    self.active_samples.clear();
                    self.touch_error_banner = None;
                    self.active_samples.push(RawSample {
                        raw: pt,
                        timestamp: Instant::now(),
                    });
                }
                RawTouchEvent::Sample(pt) => {
                    if self.is_touch_held {
                        self.active_samples.push(RawSample {
                            raw: pt,
                            timestamp: Instant::now(),
                        });
                        if self.active_samples.len()
                            >= self.cal_config.sample_count
                                + self.cal_config.initial_discard_samples
                        {
                            let elapsed = self
                                .active_samples
                                .first()
                                .map(|s| s.timestamp.elapsed().as_millis() as u64)
                                .unwrap_or(0);
                            if elapsed >= self.cal_config.min_touch_duration_ms {
                                self.commit_active_target(ctx, false);
                            }
                        }
                    }
                }
                RawTouchEvent::Up => {
                    if self.is_touch_held {
                        self.is_touch_held = false;
                        self.commit_active_target(ctx, false);
                    }
                }
                RawTouchEvent::Disconnected(_) => {
                    self.device_disconnected = true;
                    self.is_touch_held = false;
                    self.active_samples.clear();
                }
                RawTouchEvent::PermissionDenied(_) => {
                    self.using_pointer_fallback = true;
                }
                RawTouchEvent::HotplugChanged => {}
            }
        }

        if self.device_disconnected
            && let Some(hp) = &self.hotplug_handle
            && hp
                .rx
                .try_iter()
                .any(|e| matches!(e, RawTouchEvent::HotplugChanged))
        {
            self.refresh_hardware(false);
            if let Some(dev) = self.selected_device().cloned()
                && dev.readable
            {
                self.device_disconnected = false;
                self.input_stream = Some(start_device_stream(&dev));
            }
        }
    }

    fn commit_active_target(&mut self, ctx: &egui::Context, is_pointer_click: bool) {
        self.is_touch_held = false;
        let Some(dev) = self.selected_device() else {
            return;
        };
        let bounds = dev.abs_bounds.unwrap_or_default();
        let existing_norms: Vec<Point2D> = self
            .captured_points
            .iter()
            .map(|c| c.raw_normalized)
            .collect();

        let samples = std::mem::take(&mut self.active_samples);
        let mut effective_cfg = self.cal_config.clone();
        if is_pointer_click {
            // Fast mouse/touch clicks in pointer mode have short duration and few frames.
            effective_cfg.min_touch_duration_ms = 0;
            effective_cfg.initial_discard_samples = 0;
        }

        match filter_touch_samples(&samples, &bounds, &effective_cfg, &existing_norms) {
            Ok(filtered) => {
                self.touch_error_banner = None;
                self.captured_points.push(filtered);
                self.current_target_idx += 1;
                self.target_started_at = Some(Instant::now());
                if self.current_target_idx >= self.targets.len() {
                    self.finish_calibration(ctx);
                }
            }
            Err(_) => {
                self.touch_error_banner = Some(self.t(TextKey::TouchInaccurateError));
            }
        }
    }

    fn apply_profile_with_backup(&mut self, mut profile: CalibrationProfile) {
        let prev_profile = self.profiles_list.first().cloned();
        let prev_x11 = self.selected_device().and_then(|d| d.current_x11_matrix);
        let _ = self.store.create_backup(
            "Pre-apply automatic backup",
            &profile.device_id,
            &profile.device_name,
            prev_profile,
            prev_x11,
        );

        profile.auto_apply = self.apply_on_startup;
        let _ = self.store.save_profile(&profile);
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "smartcal".to_string());
        let _ = self
            .store
            .set_autostart_enabled(self.apply_on_startup, &exe);

        match apply_calibration(&profile) {
            Ok(res) => {
                self.latest_profile = Some(profile);
                self.profiles_list = self.store.list_profiles();
                self.banner = Some(NotificationBanner {
                    text: format!("{} ({})", self.t(TextKey::ApplySuccess), res.backend_used),
                    is_error: false,
                });
            }
            Err(SmartCalError::InsufficientPermissions(_)) => {
                self.banner = Some(NotificationBanner {
                    text: self.t(TextKey::PermissionsRequired),
                    is_error: true,
                });
            }
            Err(e) => {
                self.banner = Some(NotificationBanner {
                    text: e.user_message(),
                    is_error: true,
                });
            }
        }
    }

    fn enter_test_mode(&mut self, ctx: &egui::Context, fullscreen: bool) {
        self.stop_probe();
        self.test_trail.clear();
        self.test_last_raw = None;
        self.test_last_screen = None;
        self.test_last_error = None;
        self.test_fullscreen = fullscreen;
        self.device_disconnected = false;
        if let Some(dev) = self.selected_device().cloned() {
            self.using_pointer_fallback = !cfg!(target_os = "linux") || !dev.readable;
            self.input_stream = if cfg!(target_os = "linux") && dev.readable {
                Some(start_device_stream(&dev))
            } else {
                None
            };
        }
        if fullscreen {
            self.enter_fullscreen_on_selected_display(ctx);
        }
        self.screen = Screen::TestMode;
    }
}

impl eframe::App for SmartCalApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if !matches!(
            self.screen,
            Screen::FullscreenCalibrating | Screen::TestMode
        ) {
            let mut changed = false;
            if let Some(hp) = &self.hotplug_handle {
                while let Ok(ev) = hp.rx.try_recv() {
                    if matches!(ev, RawTouchEvent::HotplugChanged) {
                        changed = true;
                    }
                }
            }
            if changed {
                self.refresh_hardware(false);
            }
        }

        self.sync_step2_probe();
        if self.screen == Screen::WizardStep2 && self.probe_stream.is_some() {
            ctx.request_repaint_after(Duration::from_millis(60));
        }

        if self.screen == Screen::FullscreenCalibrating {
            ctx.request_repaint_after(Duration::from_millis(16));
            self.render_fullscreen_calibration(ui, &ctx);
            return;
        }

        if self.screen == Screen::TestMode {
            ctx.request_repaint_after(Duration::from_millis(16));
            self.render_test_mode(ui, &ctx);
            return;
        }

        let p = Palette::for_mode(self.settings.theme);

        egui::Panel::top("smartcal_topbar")
            .frame(
                egui::Frame::new()
                    .fill(p.card)
                    .stroke(Stroke::new(1.0, p.border))
                    .inner_margin(egui::Margin::symmetric(18, 10)),
            )
            .show(ui, |ui| {
                self.render_top_bar(ui, p, &ctx);
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ui, |ui| {
                self.render_banners(ui, p);
                egui::ScrollArea::vertical().show(ui, |ui| match self.screen {
                    Screen::Home => self.render_home(ui, p, &ctx),
                    Screen::WizardStep1 => self.render_wizard_step1(ui, p),
                    Screen::WizardStep2 => self.render_wizard_step2(ui, p),
                    Screen::WizardStep3 => self.render_wizard_step3(ui, p, &ctx),
                    Screen::Results => self.render_results(ui, p, &ctx),
                    Screen::Profiles => self.render_profiles(ui, p),
                    Screen::Settings => self.render_settings(ui, p, &ctx),
                    Screen::Diagnostics => self.render_diagnostics(ui, p),
                    Screen::FullscreenCalibrating | Screen::TestMode => {}
                });
            });
    }
}

impl SmartCalApp {
    fn render_top_bar(&mut self, ui: &mut egui::Ui, p: Palette, ctx: &egui::Context) {
        let lang = self.lang();
        ui.horizontal_wrapped(|ui| {
            let (logo_rect, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
            paint_brand_logo(
                &ui.painter_at(logo_rect),
                logo_rect,
                p.primary,
                Color32::from_rgb(250, 204, 21),
            );

            ui.label(self.jt(TextKey::AppTitle, 18.5, p.text));
            if let Some(diag) = &self.diagnostics {
                status_badge(
                    ui,
                    lang,
                    &format!("{}", diag.session.display_server),
                    p.primary,
                );
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Big 1-Click Quick Calibrate Button in the top-right corner!
                let quick_btn =
                    egui::Button::new(self.jt(TextKey::QuickCalibrate, 14.5, Color32::WHITE))
                        .fill(p.success)
                        .min_size(Vec2::new(190.0, 36.0));
                if ui.add(quick_btn).clicked() {
                    self.start_calibration_session(ctx);
                }

                let theme_lbl = match self.settings.theme {
                    ThemeMode::Light => self.t(TextKey::DarkMode),
                    ThemeMode::Dark => self.t(TextKey::LightMode),
                };
                if ui.button(self.j(&theme_lbl, 13.5, p.text)).clicked() {
                    self.settings.theme = match self.settings.theme {
                        ThemeMode::Light => ThemeMode::Dark,
                        ThemeMode::Dark => ThemeMode::Light,
                    };
                    apply_theme(ui.ctx(), self.settings.theme);
                    let _ = self.store.save_settings(&self.settings);
                }

                let lang_lbl = match self.settings.language {
                    Language::English => "فارسی",
                    Language::Persian => "English",
                };
                if ui.button(self.j(lang_lbl, 13.5, p.text)).clicked() {
                    self.settings.language = match self.settings.language {
                        Language::English => Language::Persian,
                        Language::Persian => Language::English,
                    };
                    let _ = self.store.save_settings(&self.settings);
                }

                if self.screen != Screen::Home
                    && ui.button(self.jt(TextKey::Home, 13.5, p.text)).clicked()
                {
                    self.screen = Screen::Home;
                }
            });
        });
    }

    fn render_banners(&mut self, ui: &mut egui::Ui, p: Palette) {
        if let Some(matched) = self.startup_match.clone() {
            let msg = format!(
                "{} {}.",
                self.t(TextKey::PreviousCalibrationFound),
                matched.device_name
            );
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(
                    p.primary.r(),
                    p.primary.g(),
                    p.primary.b(),
                    20,
                ))
                .stroke(Stroke::new(1.0, p.primary))
                .corner_radius(10)
                .inner_margin(12)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(self.j(&msg, 14.0, p.text));
                        if ui.button(self.jt(TextKey::Apply, 13.5, p.text)).clicked() {
                            self.apply_profile_with_backup(matched);
                            self.startup_match = None;
                        }
                        if ui
                            .button(self.jt(TextKey::Recalibrate, 13.5, p.text))
                            .clicked()
                        {
                            self.startup_match = None;
                            self.screen = Screen::WizardStep1;
                        }
                        if ui.button(self.jt(TextKey::Dismiss, 13.5, p.text)).clicked() {
                            self.startup_match = None;
                        }
                    });
                });
            ui.add_space(10.0);
        }

        if let Some(b) = self.banner.clone() {
            let col = if b.is_error { p.danger } else { p.success };
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(
                    col.r(),
                    col.g(),
                    col.b(),
                    22,
                ))
                .stroke(Stroke::new(1.0, col))
                .corner_radius(10)
                .inner_margin(10)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(self.j(&b.text, 14.0, col));
                        if ui.button("×").clicked() {
                            self.banner = None;
                        }
                    });
                });
            ui.add_space(10.0);
        }
    }

    fn render_readiness_strip(&self, ui: &mut egui::Ui, p: Palette) {
        let lang = self.lang();
        let disp_val = self
            .selected_display()
            .map(|d| format!("{} ({}×{})", d.name, d.width, d.height))
            .unwrap_or_else(|| "—".into());
        let dev_val = self
            .selected_device()
            .map(|d| format!("{} ({})", d.name, d.event_path))
            .unwrap_or_else(|| "—".into());
        let sess_val = self
            .diagnostics
            .as_ref()
            .map(|d| format!("{}", d.session.display_server))
            .unwrap_or_else(|| "—".into());
        let perm_ok = self
            .diagnostics
            .as_ref()
            .is_some_and(|d| d.permissions.evdev_readable_count > 0 || d.permissions.is_root);

        let perm_lbl = if perm_ok {
            self.t(TextKey::ReadyBadge)
        } else {
            self.t(TextKey::SetupNeededBadge)
        };

        card_frame(p).show(ui, |ui| {
            ui.columns(4, |cols| {
                // Reverse column order in RTL so Target Display sits cleanly on the right.
                let (c_disp, c_dev, c_sess, c_perm) = if self.is_rtl() {
                    (3, 2, 1, 0)
                } else {
                    (0, 1, 2, 3)
                };
                let align = if self.is_rtl() {
                    Align::Max
                } else {
                    Align::Min
                };

                cols[c_disp].with_layout(Layout::top_down(align), |ui| {
                    ui.label(self.jt(TextKey::TargetDisplayLabel, 11.5, p.muted));
                    ui.label(self.j(&disp_val, 13.5, p.text));
                });

                cols[c_dev].with_layout(Layout::top_down(align), |ui| {
                    ui.label(self.jt(TextKey::TargetDeviceLabel, 11.5, p.muted));
                    ui.label(self.j(&dev_val, 13.0, p.text));
                });

                cols[c_sess].with_layout(Layout::top_down(align), |ui| {
                    ui.label(self.jt(TextKey::SessionLabel, 11.5, p.muted));
                    ui.label(self.j(&sess_val, 13.5, p.primary));
                });

                cols[c_perm].with_layout(Layout::top_down(align), |ui| {
                    ui.label(self.jt(TextKey::PermissionsLabel, 11.5, p.muted));
                    status_badge(
                        ui,
                        lang,
                        &perm_lbl,
                        if perm_ok { p.success } else { p.warning },
                    );
                });
            });
        });
    }

    fn render_home(&mut self, ui: &mut egui::Ui, p: Palette, ctx: &egui::Context) {
        let lang = self.lang();
        let is_rtl = self.is_rtl();
        let align = if is_rtl { Align::Max } else { Align::Min };

        // Hero Banner with Instant 1-Click Fullscreen Calibrate Button
        egui::Frame::new()
            .fill(Color32::from_rgba_unmultiplied(
                p.primary.r(),
                p.primary.g(),
                p.primary.b(),
                18,
            ))
            .stroke(Stroke::new(1.2, p.primary))
            .corner_radius(12)
            .inner_margin(16)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if is_rtl {
                        let btn = egui::Button::new(self.jt(
                            TextKey::QuickCalibrate,
                            17.0,
                            Color32::WHITE,
                        ))
                        .fill(p.primary)
                        .min_size(Vec2::new(260.0, 52.0));
                        if ui.add(btn).clicked() {
                            self.start_calibration_session(ctx);
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.vertical(|ui| {
                                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                    ui.label(self.jt(TextKey::Subtitle, 14.5, p.text));
                                    ui.label(self.jt(TextKey::QuickCalibrateHint, 12.5, p.muted));
                                });
                            });
                        });
                    } else {
                        ui.vertical(|ui| {
                            ui.label(self.jt(TextKey::Subtitle, 15.0, p.text));
                            ui.label(self.jt(TextKey::QuickCalibrateHint, 12.5, p.muted));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let btn = egui::Button::new(self.jt(
                                TextKey::QuickCalibrate,
                                16.5,
                                Color32::WHITE,
                            ))
                            .fill(p.primary)
                            .min_size(Vec2::new(260.0, 52.0));
                            if ui.add(btn).clicked() {
                                self.start_calibration_session(ctx);
                            }
                        });
                    }
                });
            });

        ui.add_space(12.0);
        self.render_readiness_strip(ui, p);
        ui.add_space(14.0);

        let avail = ui.available_width();
        let cols = if avail > 820.0 { 3 } else { 2 };
        let card_w = ((avail - (cols as f32 - 1.0) * 14.0) / cols as f32).max(220.0);
        let card_sz = Vec2::new(card_w, 98.0);

        egui::Grid::new("home_cards_grid")
            .num_columns(cols)
            .spacing([14.0, 14.0])
            .show(ui, |ui| {
                if action_card(
                    ui,
                    p,
                    lang,
                    card_sz,
                    IconKind::DetectHardware,
                    &self.t(TextKey::DetectHardware),
                    &self.t(TextKey::DetectHardwareDesc),
                    None,
                    false,
                ) {
                    self.refresh_hardware(false);
                }

                let rec = self.t(TextKey::RecommendedBadge);
                if action_card(
                    ui,
                    p,
                    lang,
                    card_sz,
                    IconKind::StartCalibration,
                    &self.t(TextKey::StartCalibration),
                    &self.t(TextKey::StartCalibrationDesc),
                    Some(&rec),
                    true,
                ) {
                    self.screen = Screen::WizardStep1;
                }

                if action_card(
                    ui,
                    p,
                    lang,
                    card_sz,
                    IconKind::TestTouch,
                    &self.t(TextKey::TestTouch),
                    &self.t(TextKey::TestTouchDesc),
                    None,
                    false,
                ) {
                    self.enter_test_mode(ctx, false);
                }

                if cols == 3 {
                    ui.end_row();
                }

                let prof_badge =
                    (!self.profiles_list.is_empty()).then(|| self.profiles_list.len().to_string());
                if action_card(
                    ui,
                    p,
                    lang,
                    card_sz,
                    IconKind::Profiles,
                    &self.t(TextKey::Profiles),
                    &self.t(TextKey::ProfilesDesc),
                    prof_badge.as_deref(),
                    false,
                ) {
                    self.profiles_list = self.store.list_profiles();
                    self.screen = Screen::Profiles;
                }

                if action_card(
                    ui,
                    p,
                    lang,
                    card_sz,
                    IconKind::Settings,
                    &self.t(TextKey::Settings),
                    &self.t(TextKey::SettingsDesc),
                    None,
                    false,
                ) {
                    self.screen = Screen::Settings;
                }

                if action_card(
                    ui,
                    p,
                    lang,
                    card_sz,
                    IconKind::Diagnostics,
                    &self.t(TextKey::Diagnostics),
                    &self.t(TextKey::DiagnosticsDesc),
                    None,
                    false,
                ) {
                    self.diagnostics = Some(collect_diagnostics(&self.store));
                    self.screen = Screen::Diagnostics;
                }
                ui.end_row();
            });

        ui.add_space(16.0);

        ui.columns(2, |cols| {
            card_frame(p).show(&mut cols[0], |ui| {
                ui.with_layout(Layout::top_down(align), |ui| {
                    ui.label(self.jt(TextKey::DisplaysHeading, 16.0, p.text));
                });
                ui.separator();
                for (i, d) in self.displays.iter().enumerate() {
                    let selected = i == self.selected_display_idx;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(format!("{}: {}", d.id, d.name))
                                .strong()
                                .color(if selected { p.primary } else { p.text }),
                        );
                        status_badge(
                            ui,
                            lang,
                            &format!("{} × {} @ {:.0} Hz", d.width, d.height, d.refresh_hz),
                            p.muted,
                        );
                        if d.is_projector_candidate() {
                            status_badge(ui, lang, &self.t(TextKey::ProjectorBadge), p.primary);
                        }
                        if d.primary {
                            status_badge(ui, lang, &self.t(TextKey::PrimaryBadge), p.success);
                        }
                    });
                    ui.add_space(4.0);
                }
            });

            card_frame(p).show(&mut cols[1], |ui| {
                ui.with_layout(Layout::top_down(align), |ui| {
                    ui.label(self.jt(TextKey::InputDevicesHeading, 16.0, p.text));
                });
                ui.separator();
                if self.devices.is_empty() {
                    ui.label(self.jt(TextKey::NoDevicesDetected, 13.0, p.muted));
                }
                for (i, dev) in self.devices.iter().enumerate() {
                    let selected = i == self.selected_device_idx;
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&dev.name).strong().color(if selected {
                            p.primary
                        } else {
                            p.text
                        }));
                        let st_col = match dev.status {
                            ConnectionStatus::Connected => p.success,
                            ConnectionStatus::PermissionDenied => p.warning,
                            _ => p.danger,
                        };
                        status_badge(ui, lang, &dev.event_path, st_col);
                    });
                    let meta = format!(
                        "{} | VID:PID {} | {}",
                        self.kind_label(dev.kind),
                        dev.vid_pid_label(),
                        self.cap_label(dev.touch_capability)
                    );
                    ui.label(self.j(&meta, 12.0, p.muted));
                    ui.add_space(4.0);
                }
            });
        });
    }

    fn render_wizard_step1(&mut self, ui: &mut egui::Ui, p: Palette) {
        let lang = self.lang();
        let s1 = self.t(TextKey::StepDisplayShort);
        let s2 = self.t(TextKey::StepDeviceShort);
        let s3 = self.t(TextKey::StepMethodShort);
        wizard_stepper(ui, p, lang, 1, [&s1, &s2, &s3]);
        ui.add_space(8.0);

        card_frame(p).show(ui, |ui| {
            ui.label(self.jt(TextKey::Step1Title, 19.0, p.text));
            ui.label(self.jt(TextKey::ClickMonitorHint, 12.5, p.muted));
            ui.separator();

            let (virt_w, virt_h) = virtual_desktop_size(&self.displays);
            let preview_h = 155.0;
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), preview_h),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 10.0, p.card_alt);
            painter.rect_stroke(
                rect,
                10.0,
                Stroke::new(1.0, p.border),
                egui::StrokeKind::Inside,
            );

            let scale = ((rect.width() - 32.0) / virt_w as f32)
                .min((rect.height() - 32.0) / virt_h as f32)
                .max(0.01);
            let total_px_w = virt_w as f32 * scale;
            let total_px_h = virt_h as f32 * scale;
            let origin = Pos2::new(
                rect.center().x - total_px_w * 0.5,
                rect.center().y - total_px_h * 0.5,
            );

            for (idx, d) in self.displays.clone().into_iter().enumerate() {
                let r = Rect::from_min_size(
                    origin + Vec2::new(d.x as f32 * scale, d.y as f32 * scale),
                    Vec2::new(d.width as f32 * scale, d.height as f32 * scale),
                );
                let click_resp =
                    ui.interact(r, ui.make_persistent_id(("mon_box", idx)), Sense::click());
                if click_resp.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if click_resp.clicked() {
                    self.selected_display_idx = idx;
                }

                let is_sel = idx == self.selected_display_idx;
                painter.rect_filled(
                    r,
                    6.0,
                    if is_sel {
                        Color32::from_rgba_unmultiplied(
                            p.primary.r(),
                            p.primary.g(),
                            p.primary.b(),
                            58,
                        )
                    } else if click_resp.hovered() {
                        p.card
                    } else {
                        p.bg
                    },
                );
                painter.rect_stroke(
                    r,
                    6.0,
                    Stroke::new(
                        if is_sel { 2.5 } else { 1.2 },
                        if is_sel { p.primary } else { p.border },
                    ),
                    egui::StrokeKind::Inside,
                );
                let tag = if d.is_projector_candidate() {
                    " [Projector]"
                } else if d.primary {
                    " [Primary]"
                } else {
                    ""
                };
                painter.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    format!(
                        "{}{tag}\n{}×{} @ {:.0}Hz",
                        d.name, d.width, d.height, d.refresh_hz
                    ),
                    egui::FontId::proportional(12.5),
                    p.text,
                );
            }

            ui.add_space(12.0);
            for (idx, d) in self.displays.clone().into_iter().enumerate() {
                let selected = idx == self.selected_display_idx;
                let label = format!(
                    "{} — {} ({} × {} @ {:.2} Hz, +{},+{})",
                    d.id, d.name, d.width, d.height, d.refresh_hz, d.x, d.y
                );
                ui.horizontal_wrapped(|ui| {
                    if ui.selectable_label(selected, label).clicked() {
                        self.selected_display_idx = idx;
                    }
                    if d.is_projector_candidate() {
                        status_badge(ui, lang, &self.t(TextKey::ProjectorBadge), p.primary);
                    }
                });
            }

            ui.add_space(10.0);
            let proj_lbl = self.jt(TextKey::ProjectorMode, 13.5, p.text);
            ui.checkbox(&mut self.cal_config.projector_mode, proj_lbl);
            ui.label(self.jt(TextKey::ProjectorModeHint, 12.0, p.muted));

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.button(self.jt(TextKey::Cancel, 13.5, p.text)).clicked() {
                    self.screen = Screen::Home;
                }
                if ui
                    .add(
                        egui::Button::new(self.jt(TextKey::Next, 13.5, p.primary_text))
                            .fill(p.primary),
                    )
                    .clicked()
                {
                    self.screen = Screen::WizardStep2;
                }
            });
        });
    }

    fn render_wizard_step2(&mut self, ui: &mut egui::Ui, p: Palette) {
        let lang = self.lang();
        let s1 = self.t(TextKey::StepDisplayShort);
        let s2 = self.t(TextKey::StepDeviceShort);
        let s3 = self.t(TextKey::StepMethodShort);
        wizard_stepper(ui, p, lang, 2, [&s1, &s2, &s3]);
        ui.add_space(8.0);

        card_frame(p).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.jt(TextKey::Step2Title, 19.0, p.text));
                if ui
                    .button(self.jt(TextKey::DetectHardware, 13.5, p.text))
                    .clicked()
                {
                    self.refresh_hardware(false);
                }
            });
            ui.separator();

            let recent_signal = self
                .probe_last_raw
                .filter(|(_, t)| t.elapsed() < Duration::from_secs(3));
            let probe_col = if recent_signal.is_some() {
                p.success
            } else {
                p.primary
            };
            egui::Frame::new()
                .fill(Color32::from_rgba_unmultiplied(
                    probe_col.r(),
                    probe_col.g(),
                    probe_col.b(),
                    18,
                ))
                .stroke(Stroke::new(1.0, probe_col))
                .corner_radius(8)
                .inner_margin(10)
                .show(ui, |ui| {
                    if let Some((pt, _)) = recent_signal {
                        let msg = format!(
                            "{}: ({:.0}, {:.0})",
                            self.t(TextKey::LiveTouchSignalReceived),
                            pt.x,
                            pt.y
                        );
                        ui.label(self.j(&msg, 13.5, p.success));
                    } else {
                        ui.label(self.jt(TextKey::LiveTouchSignalPrompt, 12.5, p.text));
                    }
                });
            ui.add_space(10.0);

            if self.devices.is_empty() {
                ui.label(self.jt(TextKey::NoDevicesDetected, 13.0, p.muted));
            }

            for (idx, dev) in self.devices.clone().into_iter().enumerate() {
                let selected = idx == self.selected_device_idx;
                let row_frame = egui::Frame::new()
                    .fill(if selected {
                        Color32::from_rgba_unmultiplied(
                            p.primary.r(),
                            p.primary.g(),
                            p.primary.b(),
                            20,
                        )
                    } else {
                        p.card_alt
                    })
                    .stroke(Stroke::new(
                        if selected { 1.6 } else { 1.0 },
                        if selected { p.primary } else { p.border },
                    ))
                    .corner_radius(8)
                    .inner_margin(10);

                row_frame.show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .radio(selected, RichText::new(&dev.name).strong())
                            .clicked()
                        {
                            self.selected_device_idx = idx;
                        }
                        status_badge(ui, lang, &self.kind_label(dev.kind), p.primary);
                        status_badge(ui, lang, &self.cap_label(dev.touch_capability), p.muted);
                        let st_col = match dev.status {
                            ConnectionStatus::Connected => p.success,
                            ConnectionStatus::PermissionDenied => p.warning,
                            _ => p.danger,
                        };
                        status_badge(ui, lang, &self.status_label(dev.status), st_col);
                    });
                    ui.label(
                        RichText::new(format!(
                            "{} | VID:PID {} | XInput ID: {:?}",
                            dev.event_path,
                            dev.vid_pid_label(),
                            dev.xinput_id
                        ))
                        .size(12.0)
                        .color(p.muted),
                    );
                });
                ui.add_space(5.0);
            }

            if let Some(dev) = self.selected_device()
                && !dev.readable
            {
                ui.add_space(6.0);
                ui.label(self.jt(TextKey::PermissionsRequired, 13.0, p.warning));
                ui.label(self.jt(TextKey::UdevInstallHint, 12.0, p.muted));
            }

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.button(self.jt(TextKey::Back, 13.5, p.text)).clicked() {
                    self.screen = Screen::WizardStep1;
                }
                if !self.devices.is_empty()
                    && ui
                        .add(
                            egui::Button::new(self.jt(TextKey::Next, 13.5, p.primary_text))
                                .fill(p.primary),
                        )
                        .clicked()
                {
                    self.screen = Screen::WizardStep3;
                }
            });
        });
    }

    fn render_wizard_step3(&mut self, ui: &mut egui::Ui, p: Palette, ctx: &egui::Context) {
        let lang = self.lang();
        let s1 = self.t(TextKey::StepDisplayShort);
        let s2 = self.t(TextKey::StepDeviceShort);
        let s3 = self.t(TextKey::StepMethodShort);
        wizard_stepper(ui, p, lang, 3, [&s1, &s2, &s3]);
        ui.add_space(8.0);

        card_frame(p).show(ui, |ui| {
            ui.label(self.jt(TextKey::Step3Title, 19.0, p.text));
            ui.separator();

            let cal_opts: Vec<(CalibrationType, egui::text::LayoutJob)> = [
                (CalibrationType::Points4, TextKey::Cal4Point),
                (CalibrationType::Points9, TextKey::Cal9Point),
                (CalibrationType::Points16, TextKey::Cal16Point),
                (CalibrationType::Points25, TextKey::Cal25Point),
                (CalibrationType::Advanced, TextKey::CalAdvanced),
            ]
            .into_iter()
            .map(|(ct, k)| (ct, self.jt(k, 14.0, p.text)))
            .collect();

            let method_items: Vec<(CalibrationMethod, egui::text::LayoutJob)> = [
                (CalibrationMethod::Homography, TextKey::MethodHomography),
                (
                    CalibrationMethod::HomographyRansac,
                    TextKey::MethodHomographyRansac,
                ),
                (CalibrationMethod::Affine, TextKey::MethodAffine),
                (CalibrationMethod::PolynomialDeg2, TextKey::MethodPoly2),
                (CalibrationMethod::PolynomialDeg3, TextKey::MethodPoly3),
                (CalibrationMethod::HybridTps, TextKey::MethodHybridTps),
            ]
            .into_iter()
            .map(|(m, k)| (m, self.jt(k, 13.5, p.text)))
            .collect();

            let l_method = self.jt(TextKey::MethodLabel, 13.5, p.text);
            let selected_method_job =
                self.j(&self.method_label(self.cal_config.method), 13.5, p.text);
            let l_preview = self.jt(TextKey::PatternPreviewTitle, 14.0, p.text);

            ui.columns(2, |cols| {
                for (ct, job) in cal_opts {
                    let checked = self.cal_config.calibration_type == ct;
                    if cols[0].radio(checked, job).clicked() {
                        self.cal_config.calibration_type = ct;
                        self.cal_config.method = ct.recommended_method();
                    }
                }

                cols[0].add_space(10.0);
                cols[0].horizontal_wrapped(|ui| {
                    ui.label(l_method);
                    egui::ComboBox::from_id_salt("cal_method_combo")
                        .selected_text(selected_method_job)
                        .show_ui(ui, |ui| {
                            for (m, lbl) in &method_items {
                                ui.selectable_value(&mut self.cal_config.method, *m, lbl.clone());
                            }
                        });
                });

                cols[1].label(l_preview);
                cols[1].add_space(6.0);
                calibration_pattern_preview(
                    &mut cols[1],
                    p,
                    self.cal_config.calibration_type,
                    self.cal_config.margin_ratio,
                    self.cal_config.high_contrast,
                );
            });

            if self.cal_config.calibration_type == CalibrationType::Advanced {
                ui.add_space(8.0);
                let l_samples = self.jt(TextKey::SettingsSampleCount, 13.5, p.text);
                let l_min_dur = self.jt(TextKey::SettingsMinDuration, 13.5, p.text);
                let l_max_dev = self.jt(TextKey::SettingsMaxDeviation, 13.5, p.text);
                let l_tgt_sz = self.jt(TextKey::SettingsTargetSize, 13.5, p.text);
                let l_hc = self.jt(TextKey::SettingsHighContrast, 13.5, p.text);
                card_frame(p).show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.cal_config.sample_count, 5..=40)
                            .text(l_samples),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.cal_config.min_touch_duration_ms, 80..=1000)
                            .text(l_min_dur),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.cal_config.max_deviation_norm, 0.01..=0.12)
                            .text(l_max_dev),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.cal_config.target_size_px, 20.0..=80.0)
                            .text(l_tgt_sz),
                    );
                    ui.checkbox(&mut self.cal_config.high_contrast, l_hc);
                });
            }

            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.button(self.jt(TextKey::Back, 13.5, p.text)).clicked() {
                    self.screen = Screen::WizardStep2;
                }
                if ui
                    .add(
                        egui::Button::new(self.jt(TextKey::StartCalibration, 14.5, p.primary_text))
                            .fill(p.primary),
                    )
                    .clicked()
                {
                    self.start_calibration_session(ctx);
                }
            });
        });
    }

    fn render_fullscreen_calibration(&mut self, root_ui: &mut egui::Ui, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.exit_fullscreen(ctx);
            self.screen = Screen::WizardStep3;
            return;
        }

        self.process_calibration_touch_samples(ctx);
        if self.screen != Screen::FullscreenCalibrating {
            return;
        }

        let (bg, fg, accent) = if self.cal_config.high_contrast {
            (
                Color32::BLACK,
                Color32::WHITE,
                Color32::from_rgb(250, 204, 21),
            )
        } else {
            match self.settings.theme {
                ThemeMode::Light => (
                    Color32::from_rgb(245, 247, 250),
                    Color32::from_rgb(15, 23, 42),
                    Color32::from_rgb(29, 78, 216),
                ),
                ThemeMode::Dark => (
                    Color32::from_rgb(12, 16, 24),
                    Color32::from_rgb(241, 245, 249),
                    Color32::from_rgb(56, 189, 248),
                ),
            }
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(bg))
            .show(root_ui, |ui| {
                let screen_rect = ui.max_rect();
                let painter = ui.painter_at(screen_rect);

                // Responsive click/tap capture in pointer fallback mode (Windows preview or unprivileged Linux)
                if self.using_pointer_fallback {
                    let _resp = ui.interact(
                        screen_rect,
                        ui.make_persistent_id("cal_pointer_fallback"),
                        egui::Sense::click_and_drag(),
                    );
                    let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
                    let pressed = ctx.input(|i| i.pointer.primary_pressed());
                    let down = ctx.input(|i| i.pointer.primary_down());
                    let released = ctx.input(|i| i.pointer.primary_released());

                    if pressed {
                        self.is_touch_held = true;
                        self.active_samples.clear();
                        self.touch_error_banner = None;
                    }
                    if (self.is_touch_held || down || released)
                        && let Some(pos) = pointer_pos
                    {
                        let bounds = self
                            .selected_device()
                            .and_then(|d| d.abs_bounds)
                            .unwrap_or_default();
                        let rx = bounds.x.min as f64
                            + ((pos.x - screen_rect.left()) / screen_rect.width().max(1.0)) as f64
                                * (bounds.x.max - bounds.x.min) as f64;
                        let ry = bounds.y.min as f64
                            + ((pos.y - screen_rect.top()) / screen_rect.height().max(1.0)) as f64
                                * (bounds.y.max - bounds.y.min) as f64;
                        self.active_samples.push(RawSample {
                            raw: Point2D::new(rx, ry),
                            timestamp: Instant::now(),
                        });
                    }
                    if released && !self.active_samples.is_empty() {
                        // Do not accept clicks inside the bottom control bar!
                        let ctrl_zone = Rect::from_center_size(
                            Pos2::new(screen_rect.center().x, screen_rect.bottom() - 42.0),
                            Vec2::new(460.0, 56.0),
                        );
                        let in_ctrl = pointer_pos.is_some_and(|p| ctrl_zone.contains(p));
                        if !in_ctrl {
                            self.commit_active_target(ctx, true);
                            if self.screen != Screen::FullscreenCalibrating {
                                return;
                            }
                        } else {
                            self.active_samples.clear();
                            self.is_touch_held = false;
                        }
                    }
                }

                if let Some(target_px) = self.targets.get(self.current_target_idx).copied() {
                    let disp = self.selected_display().cloned();
                    let sx = disp
                        .as_ref()
                        .map(|d| (target_px.x as f32 / d.width.max(1) as f32) * screen_rect.width())
                        .unwrap_or(target_px.x as f32);
                    let sy = disp
                        .as_ref()
                        .map(|d| {
                            (target_px.y as f32 / d.height.max(1) as f32) * screen_rect.height()
                        })
                        .unwrap_or(target_px.y as f32);
                    let center = Pos2::new(screen_rect.left() + sx, screen_rect.top() + sy);
                    let radius = self.cal_config.target_size_px * 0.5;

                    let prog = (self.active_samples.len() as f32
                        / (self.cal_config.sample_count + self.cal_config.initial_discard_samples)
                            .max(1) as f32)
                        .clamp(0.0, 1.0);
                    painter.circle_stroke(
                        center,
                        radius + 12.0,
                        Stroke::new(2.0, fg.linear_multiply(0.35)),
                    );
                    if prog > 0.0 {
                        painter.circle_stroke(
                            center,
                            radius + 12.0,
                            Stroke::new(4.0 * prog + 2.0, accent),
                        );
                    }

                    let cross = radius + 22.0;
                    painter.line_segment(
                        [
                            center - Vec2::new(cross, 0.0),
                            center + Vec2::new(cross, 0.0),
                        ],
                        Stroke::new(1.5, fg.linear_multiply(0.5)),
                    );
                    painter.line_segment(
                        [
                            center - Vec2::new(0.0, cross),
                            center + Vec2::new(0.0, cross),
                        ],
                        Stroke::new(1.5, fg.linear_multiply(0.5)),
                    );

                    painter.circle_filled(center, (radius * 0.42).max(6.0), accent);
                    painter.circle_filled(center, 3.5, fg);
                }

                let hud_rect = Rect::from_center_size(
                    Pos2::new(screen_rect.center().x, screen_rect.top() + 72.0),
                    Vec2::new(620.0, 120.0),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(hud_rect), |ui| {
                    ui.vertical_centered(|ui| {
                        let status = format!(
                            "{} {} / {}",
                            self.t(TextKey::PointProgress),
                            self.current_target_idx + 1,
                            self.targets.len()
                        );
                        ui.label(self.j(&status, 22.0, fg));
                        let prompt = if self.is_touch_held {
                            self.t(TextKey::CollectingSamples)
                        } else {
                            self.t(TextKey::TouchTargetPrompt)
                        };
                        ui.label(self.j(&prompt, 15.0, fg.linear_multiply(0.8)));

                        if self.device_disconnected {
                            ui.label(self.jt(
                                TextKey::DeviceDisconnectedMsg,
                                14.5,
                                Color32::from_rgb(239, 68, 68),
                            ));
                        } else if let Some(err) = &self.touch_error_banner {
                            ui.label(self.j(err, 14.5, Color32::from_rgb(239, 68, 68)));
                        }
                        ui.label(self.jt(TextKey::EscToExit, 12.0, fg.linear_multiply(0.6)));
                    });
                });

                let ctrl_rect = Rect::from_center_size(
                    Pos2::new(screen_rect.center().x, screen_rect.bottom() - 42.0),
                    Vec2::new(440.0, 44.0),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(ctrl_rect), |ui| {
                    ui.horizontal_centered(|ui| {
                        if self.current_target_idx > 0
                            && ui.button(self.jt(TextKey::UndoPoint, 13.5, fg)).clicked()
                        {
                            self.current_target_idx -= 1;
                            self.captured_points.pop();
                            self.active_samples.clear();
                            self.touch_error_banner = None;
                        }
                        if ui.button(self.jt(TextKey::RetryPoint, 13.5, fg)).clicked() {
                            self.active_samples.clear();
                            self.touch_error_banner = None;
                        }
                        if ui.button(self.jt(TextKey::Cancel, 13.5, fg)).clicked() {
                            self.exit_fullscreen(ctx);
                            self.screen = Screen::Home;
                        }
                    });
                });
            });
    }

    fn render_results(&mut self, ui: &mut egui::Ui, p: Palette, ctx: &egui::Context) {
        let lang = self.lang();
        let Some(profile) = self.latest_profile.clone() else {
            self.screen = Screen::Home;
            return;
        };

        card_frame(p).show(ui, |ui| {
            ui.label(self.jt(TextKey::CalibrationResults, 20.0, p.text));
            ui.separator();

            let q_color = match profile.error.quality {
                CalibrationQuality::Excellent | CalibrationQuality::Good => p.success,
                CalibrationQuality::Acceptable => p.warning,
                CalibrationQuality::Poor => p.danger,
            };
            let q_text = match profile.error.quality {
                CalibrationQuality::Excellent => self.t(TextKey::QualityExcellent),
                CalibrationQuality::Good => self.t(TextKey::QualityGood),
                CalibrationQuality::Acceptable => self.t(TextKey::QualityAcceptable),
                CalibrationQuality::Poor => self.t(TextKey::QualityPoor),
            };

            ui.horizontal_wrapped(|ui| {
                status_badge(
                    ui,
                    lang,
                    &format!(
                        "{}: {:.2} px",
                        self.t(TextKey::AverageError),
                        profile.error.average
                    ),
                    p.primary,
                );
                status_badge(
                    ui,
                    lang,
                    &format!(
                        "{}: {:.2} px",
                        self.t(TextKey::MaximumError),
                        profile.error.maximum
                    ),
                    p.muted,
                );
                status_badge(
                    ui,
                    lang,
                    &format!("{}: {:.2} px", self.t(TextKey::RmsError), profile.error.rms),
                    p.muted,
                );
                status_badge(
                    ui,
                    lang,
                    &format!(
                        "{}: {}/{}",
                        self.t(TextKey::InliersLabel),
                        profile.model.ransac_inliers,
                        profile.model.total_points
                    ),
                    p.primary,
                );
                status_badge(
                    ui,
                    lang,
                    &format!("{}: {}", self.t(TextKey::CalibrationQualityLabel), q_text),
                    q_color,
                );
            });

            if !is_quality_acceptable(profile.error.quality) {
                ui.add_space(10.0);
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(
                        p.danger.r(),
                        p.danger.g(),
                        p.danger.b(),
                        24,
                    ))
                    .stroke(Stroke::new(1.0, p.danger))
                    .corner_radius(8)
                    .inner_margin(10)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(self.jt(TextKey::QualityInsufficient, 14.0, p.danger));
                            if ui
                                .button(self.jt(TextKey::Recalibrate, 13.5, p.text))
                                .clicked()
                            {
                                self.start_calibration_session(ctx);
                            }
                        });
                    });
            }

            ui.add_space(12.0);
            let l_map_title = self.jt(TextKey::AccuracyVectorMapTitle, 14.0, p.text);
            let l_map_legend = self.jt(TextKey::AccuracyMapLegend, 11.5, p.muted);
            let pt_word = self.t(TextKey::PointRowLabel);
            let l_exp = self.jt(TextKey::ExpectedPos, 13.0, p.text);
            let l_act = self.jt(TextKey::ActualPos, 13.0, p.text);
            let l_err = self.jt(TextKey::PixelError, 13.0, p.text);

            ui.columns(2, |cols| {
                cols[0].label(l_map_title);
                cols[0].add_space(6.0);
                accuracy_vector_map(
                    &mut cols[0],
                    p,
                    profile.resolution[0],
                    profile.resolution[1],
                    &profile.points,
                );
                cols[0].label(l_map_legend);

                egui::ScrollArea::vertical()
                    .id_salt("results_table_scroll")
                    .max_height(260.0)
                    .show(&mut cols[1], |ui| {
                        egui::Grid::new("results_points_grid")
                            .striped(true)
                            .spacing([14.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("#").strong());
                                ui.label(l_exp);
                                ui.label(l_act);
                                ui.label(l_err);
                                ui.end_row();

                                for pt in &profile.points {
                                    ui.label(self.j(
                                        &format!("{pt_word} {}", pt.index),
                                        13.0,
                                        p.text,
                                    ));
                                    ui.label(format!(
                                        "{:.0}, {:.0}",
                                        pt.expected_screen[0], pt.expected_screen[1]
                                    ));
                                    ui.label(format!(
                                        "{:.0}, {:.0}",
                                        pt.predicted_screen[0], pt.predicted_screen[1]
                                    ));
                                    ui.label(format!("{:.2} px", pt.pixel_error));
                                    ui.end_row();
                                }
                            });
                    });
            });

            ui.add_space(14.0);
            let l_prof_name = self.jt(TextKey::ProfileNameLabel, 13.5, p.text);
            let l_apply_startup = self.jt(TextKey::ApplyOnStartup, 13.5, p.text);
            ui.horizontal_wrapped(|ui| {
                ui.label(l_prof_name);
                ui.text_edit_singleline(&mut self.profile_name_input);
                ui.checkbox(&mut self.apply_on_startup, l_apply_startup);
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(
                        egui::Button::new(self.jt(TextKey::ApplyCalibration, 14.0, p.primary_text))
                            .fill(p.primary),
                    )
                    .clicked()
                {
                    let mut updated = profile.clone();
                    updated.profile_name = ProfileStore::sanitize_name(&self.profile_name_input);
                    self.apply_profile_with_backup(updated);
                }
                if ui
                    .button(self.jt(TextKey::SaveProfile, 13.5, p.text))
                    .clicked()
                {
                    let mut updated = profile.clone();
                    updated.profile_name = ProfileStore::sanitize_name(&self.profile_name_input);
                    updated.auto_apply = self.apply_on_startup;
                    if let Err(e) = self.store.save_profile(&updated) {
                        self.banner = Some(NotificationBanner {
                            text: e.user_message(),
                            is_error: true,
                        });
                    } else {
                        self.latest_profile = Some(updated);
                        self.profiles_list = self.store.list_profiles();
                    }
                }
                if ui
                    .button(self.jt(TextKey::TestTouch, 13.5, p.text))
                    .clicked()
                {
                    self.enter_test_mode(ctx, false);
                }
                if ui
                    .button(self.jt(TextKey::Recalibrate, 13.5, p.text))
                    .clicked()
                {
                    self.start_calibration_session(ctx);
                }
            });
        });
    }

    fn render_test_mode(&mut self, root_ui: &mut egui::Ui, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.exit_fullscreen(ctx);
            self.screen = Screen::Home;
            return;
        }

        let mut evs = Vec::new();
        if let Some(stream) = &self.input_stream {
            while let Ok(ev) = stream.rx.try_recv() {
                evs.push(ev);
            }
        }
        let disp_w = self
            .selected_display()
            .map(|d| d.width.max(1))
            .unwrap_or(1920);
        let disp_h = self
            .selected_display()
            .map(|d| d.height.max(1))
            .unwrap_or(1080);

        for ev in evs {
            match ev {
                RawTouchEvent::Down(pt) | RawTouchEvent::Sample(pt) => {
                    self.test_last_raw = Some(pt);
                    let mapped = if let Some(prof) = &self.latest_profile {
                        evaluate_model_raw_to_screen(&prof.model, &prof.raw_bounds, pt, prof.method)
                    } else {
                        let bounds = self
                            .selected_device()
                            .and_then(|d| d.abs_bounds)
                            .unwrap_or_default();
                        let n = smartcal_math::normalize_raw_point(pt, &bounds);
                        Some(Point2D::new(n.x * disp_w as f64, n.y * disp_h as f64))
                    };
                    if let Some(scr) = mapped {
                        self.test_last_screen = Some(scr);
                        let err = self.nearest_grid_error(scr, disp_w, disp_h);
                        self.test_last_error = Some(err);
                        let norm = Point2D::new(scr.x / disp_w as f64, scr.y / disp_h as f64);
                        if self.test_trail.len() > 2500 {
                            self.test_trail.remove(0);
                        }
                        self.test_trail.push(TrailSample {
                            norm_pos: norm,
                            raw_pos: pt,
                            error_px: err,
                        });
                    }
                }
                RawTouchEvent::Disconnected(_) => self.device_disconnected = true,
                _ => {}
            }
        }

        let p = Palette::for_mode(self.settings.theme);
        egui::Panel::top("test_mode_bar").show(root_ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(self.jt(TextKey::TestModeHeading, 16.5, p.text));
                if ui
                    .button(self.jt(TextKey::ClearPath, 13.5, p.text))
                    .clicked()
                {
                    self.test_trail.clear();
                }
                ui.label(self.jt(TextKey::GridDensityLabel, 13.5, p.muted));
                for (c, r, lbl) in [(4, 3, "4×3"), (6, 4, "6×4"), (10, 6, "10×6")] {
                    if ui.selectable_label(self.test_grid_cols == c, lbl).clicked() {
                        self.test_grid_cols = c;
                        self.test_grid_rows = r;
                    }
                }
                let fs_key = if self.test_fullscreen {
                    TextKey::WindowedLabel
                } else {
                    TextKey::FullscreenLabel
                };
                if ui.button(self.jt(fs_key, 13.5, p.text)).clicked() {
                    self.test_fullscreen = !self.test_fullscreen;
                    if self.test_fullscreen {
                        self.enter_fullscreen_on_selected_display(ctx);
                    } else {
                        self.exit_fullscreen(ctx);
                        self.screen = Screen::TestMode;
                        if let Some(dev) = self.selected_device().cloned()
                            && dev.readable
                        {
                            self.input_stream = Some(start_device_stream(&dev));
                        }
                    }
                }
                if ui
                    .button(self.jt(TextKey::ExitTestMode, 13.5, p.text))
                    .clicked()
                {
                    self.exit_fullscreen(ctx);
                    self.screen = Screen::Home;
                }
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg))
            .show(root_ui, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter_at(rect);

                let cols = self.test_grid_cols;
                let rows = self.test_grid_rows;
                for c in 0..=cols {
                    let x = rect.left() + (c as f32 / cols as f32) * rect.width();
                    painter.line_segment(
                        [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                        Stroke::new(1.0, p.border),
                    );
                }
                for r in 0..=rows {
                    let y = rect.top() + (r as f32 / rows as f32) * rect.height();
                    painter.line_segment(
                        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                        Stroke::new(1.0, p.border),
                    );
                }
                for c in 1..cols {
                    for r in 1..rows {
                        let gx = rect.left() + (c as f32 / cols as f32) * rect.width();
                        let gy = rect.top() + (r as f32 / rows as f32) * rect.height();
                        painter.circle_stroke(Pos2::new(gx, gy), 7.0, Stroke::new(1.5, p.primary));
                        painter.circle_filled(Pos2::new(gx, gy), 2.5, p.primary);
                    }
                }

                let resp = ui.interact(
                    rect,
                    ui.make_persistent_id("test_canvas"),
                    egui::Sense::click_and_drag(),
                );
                if resp.drag_started() || ctx.input(|i| i.pointer.primary_pressed()) {
                    self.test_pointer_down = true;
                }
                if ctx.input(|i| i.pointer.primary_released()) {
                    self.test_pointer_down = false;
                }
                if self.test_pointer_down
                    && let Some(pos) = resp.interact_pointer_pos()
                {
                    let nx = ((pos.x - rect.left()) / rect.width().max(1.0)) as f64;
                    let ny = ((pos.y - rect.top()) / rect.height().max(1.0)) as f64;
                    let screen_pt = Point2D::new(nx * disp_w as f64, ny * disp_h as f64);
                    let raw_pt = if let Some(prof) = &self.latest_profile {
                        map_screen_to_raw(&prof.model, &prof.raw_bounds, screen_pt)
                            .unwrap_or(screen_pt)
                    } else {
                        screen_pt
                    };
                    let err_px = self.nearest_grid_error(screen_pt, disp_w, disp_h);
                    self.test_last_raw = Some(raw_pt);
                    self.test_last_screen = Some(screen_pt);
                    self.test_last_error = Some(err_px);
                    let norm_pt = Point2D::new(nx, ny);
                    let dist_ok = self.test_trail.last().is_none_or(|last| {
                        let dx = (last.norm_pos.x - nx) as f32 * rect.width();
                        let dy = (last.norm_pos.y - ny) as f32 * rect.height();
                        (dx * dx + dy * dy).sqrt() > 2.0
                    });
                    if dist_ok {
                        if self.test_trail.len() > 2500 {
                            self.test_trail.remove(0);
                        }
                        self.test_trail.push(TrailSample {
                            norm_pos: norm_pt,
                            raw_pos: raw_pt,
                            error_px: err_px,
                        });
                    }
                }

                let to_pos = |n: Point2D| -> Pos2 {
                    Pos2::new(
                        rect.left() + n.x as f32 * rect.width(),
                        rect.top() + n.y as f32 * rect.height(),
                    )
                };
                for win in self.test_trail.windows(2) {
                    painter.line_segment(
                        [to_pos(win[0].norm_pos), to_pos(win[1].norm_pos)],
                        Stroke::new(3.0, p.primary),
                    );
                }
                if let Some(last) = self.test_trail.last() {
                    let pos = to_pos(last.norm_pos);
                    painter.circle_filled(pos, 6.0, p.success);
                    painter.text(
                        pos + Vec2::new(10.0, -10.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!(
                            "({:.0},{:.0}) {:.1}px",
                            last.raw_pos.x, last.raw_pos.y, last.error_px
                        ),
                        egui::FontId::monospace(11.0),
                        p.text,
                    );
                }

                let hud_rect =
                    Rect::from_min_size(rect.min + Vec2::new(16.0, 16.0), Vec2::new(280.0, 145.0));
                ui.scope_builder(egui::UiBuilder::new().max_rect(hud_rect), |ui| {
                    card_frame(p).show(ui, |ui| {
                        let raw_txt = self
                            .test_last_raw
                            .map(|r| format!("{:.0}, {:.0}", r.x, r.y))
                            .unwrap_or_else(|| "—, —".into());
                        let scr_txt = self
                            .test_last_screen
                            .map(|s| format!("{:.0}, {:.0}", s.x, s.y))
                            .unwrap_or_else(|| "—, —".into());
                        let err_txt = self
                            .test_last_error
                            .map(|e| format!("{e:.1} px"))
                            .unwrap_or_else(|| "—".into());

                        ui.label(self.j(
                            &format!("{}: {raw_txt}", self.t(TextKey::RawCoords)),
                            13.5,
                            p.text,
                        ));
                        ui.label(self.j(
                            &format!("{}: {scr_txt}", self.t(TextKey::ScreenCoords)),
                            13.5,
                            p.text,
                        ));
                        ui.label(self.j(
                            &format!("{}: {err_txt}", self.t(TextKey::ErrorCoords)),
                            14.0,
                            p.primary,
                        ));
                    });
                });
            });
    }

    fn nearest_grid_error(&self, pt: Point2D, disp_w: u32, disp_h: u32) -> f64 {
        let mut best = f64::INFINITY;
        for c in 1..self.test_grid_cols {
            for r in 1..self.test_grid_rows {
                let gx = (c as f64 / self.test_grid_cols as f64) * disp_w as f64;
                let gy = (r as f64 / self.test_grid_rows as f64) * disp_h as f64;
                best = best.min(pt.distance_to(&Point2D::new(gx, gy)));
            }
        }
        if best.is_finite() { best } else { 0.0 }
    }

    fn render_profiles(&mut self, ui: &mut egui::Ui, p: Palette) {
        let lang = self.lang();
        card_frame(p).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(self.jt(TextKey::Profiles, 20.0, p.text));
                if ui
                    .button(self.jt(TextKey::ResetCalibration, 13.5, p.text))
                    .clicked()
                {
                    match reset_calibration(self.selected_device()) {
                        Ok(()) => {
                            self.banner = Some(NotificationBanner {
                                text: self.t(TextKey::ResetSuccess),
                                is_error: false,
                            });
                        }
                        Err(e) => {
                            self.banner = Some(NotificationBanner {
                                text: e.user_message(),
                                is_error: true,
                            });
                        }
                    }
                }
                if ui
                    .button(self.jt(TextKey::RestorePreviousCalibration, 13.5, p.text))
                    .clicked()
                {
                    if let Some(bak) = self.store.latest_backup()
                        && let Some(prev) = bak.previous_profile
                    {
                        self.apply_profile_with_backup(prev);
                        self.banner = Some(NotificationBanner {
                            text: self.t(TextKey::RestoreSuccess),
                            is_error: false,
                        });
                    } else {
                        self.banner = Some(NotificationBanner {
                            text: self.t(TextKey::NoBackupFound),
                            is_error: true,
                        });
                    }
                }
            });
            ui.separator();

            if self.profiles_list.is_empty() {
                ui.label(self.jt(TextKey::NoProfiles, 13.5, p.muted));
            }

            for prof in self.profiles_list.clone() {
                egui::Frame::new()
                    .fill(p.card_alt)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(8)
                    .inner_margin(12)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(&prof.profile_name).strong().size(15.0));
                            status_badge(
                                ui,
                                lang,
                                &format!(
                                    "{} ({}×{})",
                                    prof.display, prof.resolution[0], prof.resolution[1]
                                ),
                                p.primary,
                            );
                            status_badge(
                                ui,
                                lang,
                                &format!("{:.2} px ({})", prof.error.average, prof.error.quality),
                                p.success,
                            );
                        });
                        let meta = format!(
                            "{}: {} | {}: {} | {}: {} | {}: {}",
                            self.t(TextKey::DeviceLabel),
                            prof.device_name,
                            self.t(TextKey::MethodLabelShort),
                            prof.method,
                            self.t(TextKey::SessionLabel),
                            prof.display_server,
                            self.t(TextKey::SavedLabel),
                            prof.timestamp.format("%Y-%m-%d %H:%M UTC")
                        );
                        ui.label(self.j_wrap(&meta, 12.0, p.muted, 90));
                        ui.horizontal(|ui| {
                            if ui.button(self.jt(TextKey::Apply, 13.5, p.text)).clicked() {
                                self.apply_profile_with_backup(prof.clone());
                            }
                            if ui.button(self.jt(TextKey::Delete, 13.5, p.text)).clicked() {
                                let _ = self.store.delete_profile(&prof.profile_name);
                                self.profiles_list = self.store.list_profiles();
                            }
                        });
                    });
                ui.add_space(6.0);
            }
        });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui, p: Palette, ctx: &egui::Context) {
        card_frame(p).show(ui, |ui| {
            ui.label(self.jt(TextKey::Settings, 20.0, p.text));
            ui.separator();

            ui.horizontal(|ui| {
                ui.label(self.jt(TextKey::SettingsLanguage, 13.5, p.text));
                if ui
                    .selectable_label(self.settings.language == Language::English, "English")
                    .clicked()
                {
                    self.settings.language = Language::English;
                }
                if ui
                    .selectable_label(
                        self.settings.language == Language::Persian,
                        self.j("فارسی", 13.5, p.text),
                    )
                    .clicked()
                {
                    self.settings.language = Language::Persian;
                }
            });

            ui.horizontal(|ui| {
                ui.label(self.jt(TextKey::SettingsTheme, 13.5, p.text));
                if ui
                    .selectable_label(
                        self.settings.theme == ThemeMode::Light,
                        self.jt(TextKey::LightMode, 13.5, p.text),
                    )
                    .clicked()
                {
                    self.settings.theme = ThemeMode::Light;
                    apply_theme(ctx, ThemeMode::Light);
                }
                if ui
                    .selectable_label(
                        self.settings.theme == ThemeMode::Dark,
                        self.jt(TextKey::DarkMode, 13.5, p.text),
                    )
                    .clicked()
                {
                    self.settings.theme = ThemeMode::Dark;
                    apply_theme(ctx, ThemeMode::Dark);
                }
            });

            ui.separator();
            let l_tgt_sz = self.jt(TextKey::SettingsTargetSize, 13.5, p.text);
            let l_hc = self.jt(TextKey::SettingsHighContrast, 13.5, p.text);
            let l_dwell = self.jt(TextKey::SettingsTargetDwell, 13.5, p.text);
            let l_samples = self.jt(TextKey::SettingsSampleCount, 13.5, p.text);
            let l_min_dur = self.jt(TextKey::SettingsMinDuration, 13.5, p.text);
            let l_discard = self.jt(TextKey::SettingsDiscardInitial, 13.5, p.text);
            let l_max_dev = self.jt(TextKey::SettingsMaxDeviation, 13.5, p.text);
            let l_sens = self.jt(TextKey::SettingsTouchSensitivity, 13.5, p.text);

            ui.add(
                egui::Slider::new(&mut self.cal_config.target_size_px, 20.0..=80.0).text(l_tgt_sz),
            );
            ui.checkbox(&mut self.cal_config.high_contrast, l_hc);
            ui.add(
                egui::Slider::new(&mut self.cal_config.target_timeout_sec, 10..=90).text(l_dwell),
            );
            ui.add(egui::Slider::new(&mut self.cal_config.sample_count, 5..=40).text(l_samples));
            ui.add(
                egui::Slider::new(&mut self.cal_config.min_touch_duration_ms, 80..=1000)
                    .text(l_min_dur),
            );
            ui.add(
                egui::Slider::new(&mut self.cal_config.initial_discard_samples, 0..=8)
                    .text(l_discard),
            );
            ui.add(
                egui::Slider::new(&mut self.cal_config.max_deviation_norm, 0.01..=0.12)
                    .text(l_max_dev),
            );
            ui.add(
                egui::Slider::new(&mut self.cal_config.touch_sensitivity, 0.1..=1.0).text(l_sens),
            );

            self.settings.calibration = self.cal_config.clone();
            let _ = self.store.save_settings(&self.settings);

            ui.add_space(12.0);
            ui.separator();
            ui.label(self.jt(TextKey::UdevSectionTitle, 14.5, p.text));
            ui.label(
                RichText::new(UDEV_PERMISSIONS_RULE.trim())
                    .monospace()
                    .size(11.0)
                    .color(p.muted),
            );
            ui.horizontal(|ui| {
                if ui
                    .button(self.jt(TextKey::InstallUdevRule, 13.5, p.text))
                    .clicked()
                {
                    match install_permissions_udev_rule() {
                        Ok(()) => {
                            self.banner = Some(NotificationBanner {
                                text: self.t(TextKey::UdevInstallSuccess),
                                is_error: false,
                            });
                        }
                        Err(e) => {
                            self.banner = Some(NotificationBanner {
                                text: format!(
                                    "{} — {}",
                                    e.user_message(),
                                    self.t(TextKey::UdevInstallHint)
                                ),
                                is_error: true,
                            });
                        }
                    }
                }
                if ui
                    .button(self.jt(TextKey::OpenLogs, 13.5, p.text))
                    .clicked()
                {
                    let dir = smartcal_core::log_dir();
                    open_path_in_desktop(&dir);
                }
            });
        });
    }

    fn render_diagnostics(&mut self, ui: &mut egui::Ui, p: Palette) {
        if self.diagnostics.is_none() {
            self.diagnostics = Some(collect_diagnostics(&self.store));
        }

        card_frame(p).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(self.jt(TextKey::Diagnostics, 20.0, p.text));
                if ui
                    .button(self.jt(TextKey::DetectHardware, 13.5, p.text))
                    .clicked()
                {
                    self.diagnostics = Some(collect_diagnostics(&self.store));
                }
                if ui
                    .button(self.jt(TextKey::ExportDiagnostics, 13.5, p.text))
                    .clicked()
                    && let Some(diag) = &self.diagnostics
                {
                    let report_text = format_diagnostics_text(diag);
                    let dest = rfd::FileDialog::new()
                        .set_file_name("calibration-diagnostics.txt")
                        .save_file()
                        .unwrap_or_else(|| PathBuf::from("calibration-diagnostics.txt"));
                    if let Err(e) = std::fs::write(&dest, report_text) {
                        self.banner = Some(NotificationBanner {
                            text: e.to_string(),
                            is_error: true,
                        });
                    } else {
                        self.banner = Some(NotificationBanner {
                            text: format!("Diagnostics exported to {}", dest.display()),
                            is_error: false,
                        });
                    }
                }
                if ui
                    .button(self.jt(TextKey::OpenLogs, 13.5, p.text))
                    .clicked()
                {
                    open_path_in_desktop(&smartcal_core::log_dir());
                }
            });
            ui.separator();

            if let Some(diag) = &self.diagnostics {
                let formatted = format_diagnostics_text(diag);
                egui::ScrollArea::vertical()
                    .id_salt("diag_scroll")
                    .max_height(280.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(formatted).monospace().size(12.0));
                    });
            }

            ui.add_space(10.0);
            let l_all = self.jt(TextKey::AllLogsFilter, 13.0, p.text);
            ui.horizontal_wrapped(|ui| {
                ui.label(self.jt(TextKey::LogViewer, 14.0, p.text));
                ui.selectable_value(&mut self.log_filter, None, l_all);
                ui.selectable_value(&mut self.log_filter, Some(LogLevel::Debug), "DEBUG");
                ui.selectable_value(&mut self.log_filter, Some(LogLevel::Info), "INFO");
                ui.selectable_value(&mut self.log_filter, Some(LogLevel::Warning), "WARNING");
                ui.selectable_value(&mut self.log_filter, Some(LogLevel::Error), "ERROR");
            });

            let logs = recent_logs(self.log_filter);
            egui::ScrollArea::vertical()
                .id_salt("logs_scroll")
                .max_height(180.0)
                .show(ui, |ui| {
                    for entry in logs.iter().rev() {
                        ui.label(
                            RichText::new(format!(
                                "[{}] [{}] {}: {}",
                                entry.timestamp.format("%H:%M:%S"),
                                entry.level,
                                entry.target,
                                entry.message
                            ))
                            .monospace()
                            .size(11.5),
                        );
                    }
                });
        });
    }
}

fn open_path_in_desktop(path: &std::path::Path) {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
}

pub fn launch_gui() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Smart Board Calibration — SmartCal")
            .with_inner_size([1120.0, 760.0])
            .with_min_inner_size([780.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SmartCal",
        options,
        Box::new(|cc| Ok(Box::new(SmartCalApp::new(cc)))),
    )
}
