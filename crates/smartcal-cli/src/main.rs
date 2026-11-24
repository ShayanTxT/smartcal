use chrono::Utc;
use clap::Parser;
use smartcal_core::{
    CalibrationConfig, CalibrationMethod, CalibrationProfile, CalibrationType, LogLevel,
    ProfileStore, init_logging, record_log,
};
use smartcal_linux::{
    ProjectorGuard, RawTouchEvent, apply_calibration, collect_diagnostics, detect_displays,
    detect_input_devices, detect_session, format_diagnostics_text, install_permissions_udev_rule,
    reset_calibration, run_uinput_daemon, start_device_stream, virtual_desktop_size,
};
use smartcal_math::{
    RawSample, compute_calibration, evaluate_model_raw_to_screen, filter_touch_samples,
    generate_target_grid, is_quality_acceptable,
};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

#[derive(Debug, Parser)]
#[command(
    name = "smartcal",
    version,
    about = "Smart Board & Interactive Projector Calibration Suite for Linux"
)]
struct Cli {
    /// List detected displays (resolution, refresh rate, position)
    #[arg(long)]
    list_displays: bool,

    /// List detected input devices (touchscreens, styluses, mice, event paths)
    #[arg(long)]
    list_devices: bool,

    /// Run calibration (headless evdev wizard when --no-gui is set, or interactive GUI)
    #[arg(long)]
    calibrate: bool,

    /// Apply a saved calibration profile by name or JSON file path
    #[arg(long, value_name = "PROFILE")]
    apply: Option<String>,

    /// Automatically apply saved profiles marked for startup
    #[arg(long)]
    apply_startup: bool,

    /// Reset touch calibration back to factory identity matrix
    #[arg(long)]
    reset: bool,

    /// Restore the most recently backed-up calibration profile
    #[arg(long)]
    restore: bool,

    /// Enter live touch verification / test mode
    #[arg(long)]
    test: bool,

    /// Print full system & input diagnostics
    #[arg(long)]
    diagnostics: bool,

    /// Export diagnostics text report (e.g. calibration-diagnostics.txt)
    #[arg(long, value_name = "FILE")]
    export: Option<PathBuf>,

    /// Run userspace uinput projective/polynomial calibration daemon for a profile
    #[arg(long, value_name = "PROFILE")]
    daemon: Option<String>,

    /// Install non-root udev permission rule (/etc/udev/rules.d/99-smartcal-permissions.rules)
    #[arg(long)]
    install_udev_rule: bool,

    /// Target display name or ID (e.g. HDMI-1 or "Display 2")
    #[arg(long)]
    display: Option<String>,

    /// Target input device path, ID, or name (e.g. /dev/input/event5)
    #[arg(long)]
    device: Option<String>,

    /// Calibration point count (4, 9, 16, 25)
    #[arg(long, default_value_t = 9)]
    points: usize,

    /// Profile name when saving headless calibration
    #[arg(long, default_value = "classroom-projector")]
    profile: String,

    /// Run calibration/test in terminal mode without opening GUI window
    #[arg(long)]
    no_gui: bool,

    /// Output structured JSON for list/diagnostics commands
    #[arg(long)]
    json: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let store = ProfileStore::open_default();
    let settings = store.load_settings();
    init_logging(settings.log_level);

    if cli.list_displays {
        return cmd_list_displays(cli.json);
    }
    if cli.list_devices {
        return cmd_list_devices(cli.json);
    }
    if cli.diagnostics {
        return cmd_diagnostics(&store, cli.json, cli.export.as_deref());
    }
    if cli.install_udev_rule {
        return match install_permissions_udev_rule() {
            Ok(()) => {
                println!("Installed /etc/udev/rules.d/99-smartcal-permissions.rules successfully.");
                println!(
                    "Ensure classroom users belong to the 'input' group: sudo usermod -aG input $USER"
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}", e.user_message());
                ExitCode::from(1)
            }
        };
    }
    if let Some(profile_ref) = cli.apply.as_deref() {
        return cmd_apply(&store, profile_ref);
    }
    if cli.apply_startup {
        return cmd_apply_startup(&store);
    }
    if cli.reset {
        return cmd_reset(cli.device.as_deref());
    }
    if cli.restore {
        return cmd_restore(&store);
    }
    if let Some(profile_ref) = cli.daemon.as_deref() {
        return cmd_daemon(&store, profile_ref);
    }
    if cli.calibrate {
        if cli.no_gui {
            return cmd_calibrate_headless(&store, &cli);
        }
        return run_gui_entry();
    }
    if cli.test {
        if cli.no_gui {
            return cmd_test_headless(&store, &cli);
        }
        return run_gui_entry();
    }

    run_gui_entry()
}

fn run_gui_entry() -> ExitCode {
    match smartcal_gui::launch_gui() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            record_log(LogLevel::Error, "gui", format!("GUI launch failed: {e}"));
            eprintln!("Failed to launch SmartCal GUI: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_list_displays(as_json: bool) -> ExitCode {
    let sess = detect_session();
    let displays = detect_displays(sess.display_server);
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&displays).unwrap_or_else(|_| "[]".into())
        );
    } else {
        for d in &displays {
            println!(
                "{}: {} — {} × {} @ {:.2} Hz (pos +{},+{}{})",
                d.id,
                d.name,
                d.width,
                d.height,
                d.refresh_hz,
                d.x,
                d.y,
                if d.primary { ", primary" } else { "" }
            );
        }
    }
    ExitCode::SUCCESS
}

fn cmd_list_devices(as_json: bool) -> ExitCode {
    let devices = detect_input_devices();
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&devices).unwrap_or_else(|_| "[]".into())
        );
    } else if devices.is_empty() {
        println!("No touch, stylus, or mouse input devices detected.");
    } else {
        for dev in &devices {
            println!(
                "{} | {} | {} | VID:PID {} | {} | Status: {}",
                dev.name,
                dev.event_path,
                dev.kind,
                dev.vid_pid_label(),
                dev.touch_capability,
                dev.status
            );
        }
    }
    ExitCode::SUCCESS
}

fn cmd_diagnostics(
    store: &ProfileStore,
    as_json: bool,
    export: Option<&std::path::Path>,
) -> ExitCode {
    let report = collect_diagnostics(store);
    if as_json {
        let json = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into());
        println!("{json}");
        if let Some(path) = export
            && let Err(e) = std::fs::write(path, json)
        {
            eprintln!("Failed to export diagnostics: {e}");
            return ExitCode::from(1);
        }
    } else {
        let txt = format_diagnostics_text(&report);
        println!("{txt}");
        if let Some(path) = export {
            if let Err(e) = std::fs::write(path, txt) {
                eprintln!("Failed to export diagnostics: {e}");
                return ExitCode::from(1);
            }
            println!("Exported diagnostics to {}", path.display());
        }
    }
    ExitCode::SUCCESS
}

fn cmd_apply(store: &ProfileStore, profile_ref: &str) -> ExitCode {
    let profile = match store.load_profile(profile_ref) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e.user_message());
            return ExitCode::from(1);
        }
    };

    let _ = store.create_backup(
        "Pre-CLI-apply backup",
        &profile.device_id,
        &profile.device_name,
        store.list_profiles().into_iter().next(),
        None,
    );

    match apply_calibration(&profile) {
        Ok(res) => {
            println!(
                "Applied profile '{}' to '{}' via {}",
                profile.profile_name, profile.device_name, res.backend_used
            );
            for w in res.warnings {
                println!("Warning: {w}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", e.user_message());
            ExitCode::from(1)
        }
    }
}

fn cmd_apply_startup(store: &ProfileStore) -> ExitCode {
    let sess = detect_session();
    let displays = detect_displays(sess.display_server);
    let devices = detect_input_devices();
    let mut applied = 0usize;

    for dev in &devices {
        for disp in &displays {
            if let Some(prof) = store.find_matching_profile(dev, Some(disp))
                && prof.auto_apply
            {
                match apply_calibration(&prof) {
                    Ok(res) => {
                        println!(
                            "Auto-applied '{}' for '{}' -> '{}' ({})",
                            prof.profile_name, dev.name, disp.name, res.backend_used
                        );
                        applied += 1;
                    }
                    Err(e) => eprintln!(
                        "Failed auto-apply for '{}': {}",
                        prof.profile_name,
                        e.user_message()
                    ),
                }
            }
        }
    }

    if applied == 0 {
        println!("No matching auto-apply calibration profiles active for connected devices.");
    }
    ExitCode::SUCCESS
}

fn cmd_reset(device_arg: Option<&str>) -> ExitCode {
    let devices = detect_input_devices();
    let target = device_arg.and_then(|q| {
        devices
            .iter()
            .find(|d| d.event_path == q || d.id == q || d.name.eq_ignore_ascii_case(q))
    });
    match reset_calibration(target) {
        Ok(()) => {
            println!("Calibration reset to identity matrix.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", e.user_message());
            ExitCode::from(1)
        }
    }
}

fn cmd_restore(store: &ProfileStore) -> ExitCode {
    let Some(bak) = store.latest_backup() else {
        eprintln!("No calibration backup found.");
        return ExitCode::from(1);
    };
    let Some(prev) = bak.previous_profile else {
        eprintln!(
            "Latest backup '{}' does not contain a restorable profile.",
            bak.backup_id
        );
        return ExitCode::from(1);
    };
    match apply_calibration(&prev) {
        Ok(res) => {
            println!(
                "Restored previous calibration '{}' ({})",
                prev.profile_name, res.backend_used
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", e.user_message());
            ExitCode::from(1)
        }
    }
}

fn cmd_daemon(store: &ProfileStore, profile_ref: &str) -> ExitCode {
    let profile = match store.load_profile(profile_ref) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e.user_message());
            return ExitCode::from(1);
        }
    };
    let stop = Arc::new(AtomicBool::new(false));
    println!(
        "Running SmartCal uinput daemon for '{}' -> '{}' (Ctrl+C to stop)...",
        profile.device_name, profile.display
    );
    match run_uinput_daemon(&profile, &stop) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", e.user_message());
            ExitCode::from(1)
        }
    }
}

fn cmd_calibrate_headless(store: &ProfileStore, cli: &Cli) -> ExitCode {
    let sess = detect_session();
    let displays = detect_displays(sess.display_server);
    let devices = detect_input_devices();

    let Some(disp) = cli
        .display
        .as_deref()
        .and_then(|q| displays.iter().find(|d| d.name == q || d.id == q))
        .or_else(|| displays.iter().find(|d| d.is_projector_candidate()))
        .or_else(|| displays.first())
        .cloned()
    else {
        eprintln!("No display available.");
        return ExitCode::from(1);
    };

    let Some(dev) = cli
        .device
        .as_deref()
        .and_then(|q| {
            devices
                .iter()
                .find(|d| d.event_path == q || d.id == q || d.name.eq_ignore_ascii_case(q))
        })
        .or_else(|| devices.iter().find(|d| d.is_interactive_touch()))
        .or_else(|| devices.first())
        .cloned()
    else {
        eprintln!("No input device detected.");
        return ExitCode::from(1);
    };

    if !dev.readable {
        eprintln!("{}", smartcal_core::MSG_PERMISSIONS_REQUIRED);
        return ExitCode::from(1);
    }

    let cal_type = match cli.points {
        4 => CalibrationType::Points4,
        16 => CalibrationType::Points16,
        25 => CalibrationType::Points25,
        _ => CalibrationType::Points9,
    };
    let config = CalibrationConfig {
        calibration_type: cal_type,
        method: if cal_type == CalibrationType::Points4 {
            CalibrationMethod::Homography
        } else {
            CalibrationMethod::HomographyRansac
        },
        ..Default::default()
    };

    let _guard = ProjectorGuard::enter(config.projector_mode, sess.display_server);
    let targets = generate_target_grid(disp.width, disp.height, cal_type, config.margin_ratio);
    let bounds = dev.abs_bounds.unwrap_or_default();
    let stream = start_device_stream(&dev);

    println!(
        "Headless calibration on {} ({}x{}) using {} ({})",
        disp.name, disp.width, disp.height, dev.name, dev.event_path
    );

    let mut captured = Vec::new();
    let mut existing_norm = Vec::new();

    for (idx, target) in targets.iter().enumerate() {
        println!(
            "Target {}/{}: please touch screen coordinates ({:.0}, {:.0})...",
            idx + 1,
            targets.len(),
            target.x,
            target.y
        );

        loop {
            let mut samples = Vec::new();
            let mut down = false;
            let start_wait = Instant::now();
            while start_wait.elapsed() < Duration::from_secs(config.target_timeout_sec as u64) {
                match stream.rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(RawTouchEvent::Down(pt)) => {
                        down = true;
                        samples.clear();
                        samples.push(RawSample {
                            raw: pt,
                            timestamp: Instant::now(),
                        });
                    }
                    Ok(RawTouchEvent::Sample(pt)) if down => {
                        samples.push(RawSample {
                            raw: pt,
                            timestamp: Instant::now(),
                        });
                    }
                    Ok(RawTouchEvent::Up) if down => {
                        break;
                    }
                    Ok(RawTouchEvent::Disconnected(_)) => {
                        eprintln!("Input device disconnected.");
                        return ExitCode::from(1);
                    }
                    _ => {}
                }
            }

            match filter_touch_samples(&samples, &bounds, &config, &existing_norm) {
                Ok(ft) => {
                    existing_norm.push(ft.raw_normalized);
                    captured.push(ft);
                    break;
                }
                Err(_) => {
                    println!("{}", smartcal_core::MSG_TOUCH_INACCURATE);
                }
            }
        }
    }

    let (virt_w, virt_h) = virtual_desktop_size(&displays);
    match compute_calibration(&disp, virt_w, virt_h, &bounds, &config, &targets, &captured) {
        Ok(out) => {
            for pt in &out.points {
                println!("Point {} Error: {:.2} px", pt.index, pt.pixel_error);
            }
            println!(
                "Average Error: {:.2} px | Maximum Error: {:.2} px | Quality: {}",
                out.error.average, out.error.maximum, out.error.quality
            );
            if !is_quality_acceptable(out.error.quality) {
                println!("{}", smartcal_core::MSG_QUALITY_INSUFFICIENT);
            }
            let profile = CalibrationProfile {
                profile_name: ProfileStore::sanitize_name(&cli.profile),
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
                calibration_type: cal_type,
                method: config.method,
                matrix: out.matrix,
                model: out.model,
                raw_bounds: bounds,
                points: out.points,
                error: out.error,
                timestamp: Utc::now(),
                session_type: sess.session_type,
                display_server: sess.display_server,
                auto_apply: false,
            };
            let _ = store.save_profile(&profile);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", e.user_message());
            ExitCode::from(1)
        }
    }
}

fn cmd_test_headless(store: &ProfileStore, cli: &Cli) -> ExitCode {
    let devices = detect_input_devices();
    let Some(dev) = cli
        .device
        .as_deref()
        .and_then(|q| devices.iter().find(|d| d.event_path == q || d.name == q))
        .or_else(|| devices.iter().find(|d| d.is_interactive_touch()))
        .or_else(|| devices.first())
        .cloned()
    else {
        eprintln!("No input device detected.");
        return ExitCode::from(1);
    };
    if !dev.readable {
        eprintln!("{}", smartcal_core::MSG_PERMISSIONS_REQUIRED);
        return ExitCode::from(1);
    }

    let profile = store.load_profile(&cli.profile).ok();
    let stream = start_device_stream(&dev);
    println!(
        "Listening to {} ({}). Press Ctrl+C to stop.",
        dev.name, dev.event_path
    );
    loop {
        if let Ok(ev) = stream.rx.recv() {
            match ev {
                RawTouchEvent::Down(pt) | RawTouchEvent::Sample(pt) => {
                    if let Some(prof) = &profile
                        && let Some(scr) = evaluate_model_raw_to_screen(
                            &prof.model,
                            &prof.raw_bounds,
                            pt,
                            prof.method,
                        )
                    {
                        println!(
                            "Raw: {:.0}, {:.0} -> Screen: {:.1}, {:.1}",
                            pt.x, pt.y, scr.x, scr.y
                        );
                    } else {
                        println!("Raw: {:.0}, {:.0}", pt.x, pt.y);
                    }
                }
                RawTouchEvent::Disconnected(p) => {
                    println!("Disconnected: {p}");
                    break;
                }
                _ => {}
            }
        }
    }
    ExitCode::SUCCESS
}
