use crate::icons::{IconKind, paint_action_icon};
use crate::theme::{Palette, bidi_job, status_badge};
use eframe::egui::{
    self, Align, Color32, CornerRadius, CursorIcon, Layout, Pos2, Rect, Sense, Stroke, Vec2,
};
use smartcal_core::{CalibrationPointRecord, CalibrationType, Language};
use smartcal_math::generate_target_grid;

#[allow(clippy::too_many_arguments)]
pub fn action_card(
    ui: &mut egui::Ui,
    p: Palette,
    lang: Language,
    size: Vec2,
    icon: IconKind,
    title: &str,
    subtitle: &str,
    badge: Option<&str>,
    highlighted: bool,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hovered = resp.hovered();
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }

    let bg = if highlighted {
        if hovered {
            Color32::from_rgba_unmultiplied(p.primary.r(), p.primary.g(), p.primary.b(), 36)
        } else {
            Color32::from_rgba_unmultiplied(p.primary.r(), p.primary.g(), p.primary.b(), 22)
        }
    } else if hovered {
        p.card_alt
    } else {
        p.card
    };

    let border_col = if highlighted || hovered {
        p.primary
    } else {
        p.border
    };
    let border_w = if highlighted || hovered { 1.6 } else { 1.0 };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(12), bg);
    painter.rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(border_w, border_col),
        egui::StrokeKind::Inside,
    );

    let is_rtl = lang == Language::Persian;
    let icon_size = 42.0;
    let pad = 16.0;
    let icon_rect = if is_rtl {
        Rect::from_min_size(
            Pos2::new(rect.right() - pad - icon_size, rect.top() + pad),
            Vec2::splat(icon_size),
        )
    } else {
        Rect::from_min_size(
            Pos2::new(rect.left() + pad, rect.top() + pad),
            Vec2::splat(icon_size),
        )
    };

    let icon_bg = if highlighted {
        p.primary
    } else {
        Color32::from_rgba_unmultiplied(p.primary.r(), p.primary.g(), p.primary.b(), 26)
    };
    let icon_fg = if highlighted {
        Color32::WHITE
    } else {
        p.primary
    };
    painter.rect_filled(icon_rect, CornerRadius::same(10), icon_bg);
    paint_action_icon(&painter, icon_rect, icon, icon_fg);

    let text_rect = if is_rtl {
        Rect::from_min_max(
            Pos2::new(rect.left() + pad, rect.top() + pad),
            Pos2::new(icon_rect.left() - 12.0, rect.bottom() - pad),
        )
    } else {
        Rect::from_min_max(
            Pos2::new(icon_rect.right() + 12.0, rect.top() + pad),
            Pos2::new(rect.right() - pad, rect.bottom() - pad),
        )
    };

    let align = if is_rtl { Align::Max } else { Align::Min };
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(text_rect)
            .layout(Layout::top_down(align)),
    );
    child.horizontal_wrapped(|ui| {
        if is_rtl {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(bidi_job(lang, title, 15.5, p.text, None));
                if let Some(b) = badge {
                    status_badge(ui, lang, b, p.primary);
                }
            });
        } else {
            ui.label(bidi_job(lang, title, 15.5, p.text, None));
            if let Some(b) = badge {
                status_badge(ui, lang, b, p.primary);
            }
        }
    });
    child.add_space(4.0);
    child.label(bidi_job(lang, subtitle, 12.2, p.muted, Some(38)));

    resp.clicked()
}

pub fn wizard_stepper(
    ui: &mut egui::Ui,
    p: Palette,
    lang: Language,
    active_step: usize,
    labels: [&str; 3],
) {
    let total_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, 48.0), Sense::hover());
    let painter = ui.painter_at(rect);

    let step_w = (total_w / 3.0).max(110.0);
    let y = rect.center().y;

    for i in 0..2 {
        let x1 = rect.left() + step_w * (i as f32 + 0.5) + 18.0;
        let x2 = rect.left() + step_w * ((i + 1) as f32 + 0.5) - 18.0;
        if x2 > x1 {
            let done = active_step > i + 1;
            painter.line_segment(
                [Pos2::new(x1, y - 6.0), Pos2::new(x2, y - 6.0)],
                Stroke::new(2.0, if done { p.success } else { p.border }),
            );
        }
    }

    for (idx, lbl) in labels.iter().enumerate() {
        let step_num = idx + 1;
        let cx = rect.left() + step_w * (idx as f32 + 0.5);
        let center = Pos2::new(cx, y - 6.0);
        let is_done = active_step > step_num;
        let is_cur = active_step == step_num;

        let circle_col = if is_done {
            p.success
        } else if is_cur {
            p.primary
        } else {
            p.card_alt
        };
        let stroke_col = if is_done {
            p.success
        } else if is_cur {
            p.primary
        } else {
            p.border
        };

        painter.circle_filled(center, 12.0, circle_col);
        painter.circle_stroke(center, 12.0, Stroke::new(1.5, stroke_col));

        let num_txt = if is_done {
            "✓".to_string()
        } else {
            step_num.to_string()
        };
        let num_col = if is_done || is_cur {
            Color32::WHITE
        } else {
            p.muted
        };
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            num_txt,
            egui::FontId::proportional(12.0),
            num_col,
        );

        let job = bidi_job(lang, lbl, 12.0, if is_cur { p.text } else { p.muted }, None);
        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
        let tx = cx - galley.size().x * 0.5;
        let ty = y + 8.0;
        painter.galley(Pos2::new(tx, ty), galley, p.text);
    }
}

pub fn calibration_pattern_preview(
    ui: &mut egui::Ui,
    p: Palette,
    cal_type: CalibrationType,
    margin_ratio: f64,
    high_contrast: bool,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(230.0, 140.0), Sense::hover());
    let painter = ui.painter_at(rect);

    let bg = if high_contrast {
        Color32::BLACK
    } else {
        p.card_alt
    };
    let dot_col = if high_contrast {
        Color32::from_rgb(250, 204, 21)
    } else {
        p.primary
    };
    painter.rect_filled(rect, CornerRadius::same(8), bg);
    painter.rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );

    let pts = generate_target_grid(1000, 600, cal_type, margin_ratio);
    for pt in pts {
        let px = rect.left() + (pt.x as f32 / 1000.0) * rect.width();
        let py = rect.top() + (pt.y as f32 / 600.0) * rect.height();
        painter.circle_stroke(
            Pos2::new(px, py),
            5.0,
            Stroke::new(1.2, dot_col.linear_multiply(0.55)),
        );
        painter.circle_filled(Pos2::new(px, py), 2.5, dot_col);
    }
}

/// Visualizes spatial calibration accuracy across the entire monitor with ×15 magnified error vectors.
pub fn accuracy_vector_map(
    ui: &mut egui::Ui,
    p: Palette,
    disp_w: u32,
    disp_h: u32,
    points: &[CalibrationPointRecord],
) {
    let w = ui.available_width().clamp(260.0, 560.0);
    let aspect = (disp_h.max(1) as f32 / disp_w.max(1) as f32).clamp(0.45, 0.85);
    let h = (w * aspect).clamp(160.0, 290.0);

    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, CornerRadius::same(10), p.card_alt);
    painter.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, p.border),
        egui::StrokeKind::Inside,
    );

    for i in 1..4 {
        let x = rect.left() + (i as f32 / 4.0) * rect.width();
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(0.8, p.border.linear_multiply(0.5)),
        );
    }
    for j in 1..3 {
        let y = rect.top() + (j as f32 / 3.0) * rect.height();
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.8, p.border.linear_multiply(0.5)),
        );
    }

    let scale_x = rect.width() / disp_w.max(1) as f32;
    let scale_y = rect.height() / disp_h.max(1) as f32;
    let mag = 15.0_f32;

    for pt in points {
        let exp = Pos2::new(
            rect.left() + pt.expected_screen[0] as f32 * scale_x,
            rect.top() + pt.expected_screen[1] as f32 * scale_y,
        );
        let vec_dx = (pt.predicted_screen[0] - pt.expected_screen[0]) as f32 * scale_x * mag;
        let vec_dy = (pt.predicted_screen[1] - pt.expected_screen[1]) as f32 * scale_y * mag;
        let tip = Pos2::new(
            (exp.x + vec_dx).clamp(rect.left() + 6.0, rect.right() - 6.0),
            (exp.y + vec_dy).clamp(rect.top() + 6.0, rect.bottom() - 6.0),
        );

        let col = if !pt.is_ransac_inlier || pt.pixel_error > 12.0 {
            p.danger
        } else if pt.pixel_error > 4.0 {
            p.warning
        } else {
            p.success
        };

        painter.circle_stroke(exp, 7.0, Stroke::new(1.4, col));
        painter.line_segment([exp, tip], Stroke::new(2.0, col));
        painter.circle_filled(tip, 3.0, col);
        painter.text(
            exp + Vec2::new(9.0, -9.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{:.1}px", pt.pixel_error),
            egui::FontId::monospace(10.0),
            p.muted,
        );
    }
}
