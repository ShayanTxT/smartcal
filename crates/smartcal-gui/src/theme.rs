use eframe::egui::{
    self, Align, Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextFormat, Visuals,
    text::LayoutJob,
};
use smartcal_core::{Language, ThemeMode};
use smartcal_i18n::{bidi_visual_runs, contains_rtl_script, wrap_logical_lines};

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub card: Color32,
    pub card_alt: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub primary: Color32,
    pub primary_text: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
}

impl Palette {
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self {
                bg: Color32::from_rgb(244, 246, 250),
                card: Color32::from_rgb(255, 255, 255),
                card_alt: Color32::from_rgb(248, 250, 252),
                border: Color32::from_rgb(203, 213, 225),
                text: Color32::from_rgb(15, 23, 42),
                muted: Color32::from_rgb(71, 85, 105),
                primary: Color32::from_rgb(29, 78, 216),
                primary_text: Color32::WHITE,
                success: Color32::from_rgb(21, 128, 61),
                warning: Color32::from_rgb(180, 83, 9),
                danger: Color32::from_rgb(185, 28, 28),
            },
            ThemeMode::Dark => Self {
                bg: Color32::from_rgb(15, 23, 42),
                card: Color32::from_rgb(30, 41, 59),
                card_alt: Color32::from_rgb(24, 33, 49),
                border: Color32::from_rgb(51, 65, 85),
                text: Color32::from_rgb(241, 245, 249),
                muted: Color32::from_rgb(148, 163, 184),
                primary: Color32::from_rgb(59, 130, 246),
                primary_text: Color32::WHITE,
                success: Color32::from_rgb(34, 197, 94),
                warning: Color32::from_rgb(245, 158, 11),
                danger: Color32::from_rgb(239, 68, 68),
            },
        }
    }
}

pub fn apply_theme(ctx: &egui::Context, mode: ThemeMode) {
    let p = Palette::for_mode(mode);
    let mut visuals = match mode {
        ThemeMode::Light => Visuals::light(),
        ThemeMode::Dark => Visuals::dark(),
    };
    visuals.panel_fill = p.bg;
    visuals.window_fill = p.card;
    visuals.faint_bg_color = p.card_alt;
    visuals.widgets.noninteractive.bg_fill = p.card;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    visuals.widgets.inactive.bg_fill = p.card_alt;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);
    visuals.selection.bg_fill = p.primary;
    ctx.set_theme(match mode {
        ThemeMode::Light => egui::Theme::Light,
        ThemeMode::Dark => egui::Theme::Dark,
    });
    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.window_margin = Margin::same(14);
    });
}

/// Builds a BiDi-safe `LayoutJob` for egui 0.37 (`harfrust`) so:
/// - Pure Persian runs are shaped natively via OpenType without double-reversing
/// - Mixed Persian + English/numbers runs appear in exact Unicode BiDi visual order
/// - Multi-line text wraps logically top-to-bottom instead of reversing wrapped lines
pub fn bidi_job(
    lang: Language,
    text: &str,
    size: f32,
    color: Color32,
    wrap_chars: Option<usize>,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let format = TextFormat {
        font_id: FontId::new(size, FontFamily::Proportional),
        color,
        ..Default::default()
    };

    if lang != Language::Persian && !contains_rtl_script(text) {
        job.append(text, 0.0, format);
        return job;
    }

    let lines = match wrap_chars {
        Some(limit) => wrap_logical_lines(text, limit),
        None => text.split('\n').map(|s| s.to_string()).collect(),
    };

    if lines.len() > 1 {
        job.halign = Align::RIGHT;
    }

    for (line_idx, line) in lines.iter().enumerate() {
        if line_idx > 0 {
            job.append("\n", 0.0, format.clone());
        }
        for run in bidi_visual_runs(line) {
            job.append(&run.text, 0.0, format.clone());
        }
    }
    job
}

pub fn card_frame(p: Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(16))
}

pub fn status_badge(ui: &mut egui::Ui, lang: Language, label: &str, color: Color32) {
    let fill = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 26);
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, color))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.label(bidi_job(lang, label, 12.0, color, None));
        });
}
