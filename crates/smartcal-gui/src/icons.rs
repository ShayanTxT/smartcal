use eframe::egui::{Color32, CornerRadius, Painter, Pos2, Rect, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    DetectHardware,
    StartCalibration,
    TestTouch,
    Profiles,
    Settings,
    Diagnostics,
}

/// Draws the SmartCal brand emblem inside a compact square rect.
pub fn paint_brand_logo(painter: &Painter, rect: Rect, primary: Color32, accent: Color32) {
    painter.rect_filled(rect, CornerRadius::same(7), primary);
    let inner = rect.shrink(5.0);
    painter.rect_stroke(
        inner,
        CornerRadius::same(3),
        Stroke::new(1.5, Color32::WHITE.linear_multiply(0.85)),
        eframe::egui::StrokeKind::Inside,
    );
    let c = rect.center();
    painter.circle_stroke(c, 5.0, Stroke::new(1.5, accent));
    painter.circle_filled(c, 2.0, Color32::WHITE);
}

/// Draws sharp, DPI-independent vector icons for the 6 dashboard cards.
pub fn paint_action_icon(painter: &Painter, rect: Rect, kind: IconKind, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.36;
    let stroke = Stroke::new(2.0, color);

    match kind {
        IconKind::DetectHardware => {
            // Concentric radar / scan arcs + center dot
            painter.circle_stroke(c, r, Stroke::new(1.8, color.linear_multiply(0.45)));
            painter.circle_stroke(c, r * 0.62, stroke);
            painter.circle_filled(c, 2.6, color);
            painter.line_segment([c, c + Vec2::new(r * 0.72, -r * 0.72)], stroke);
        }
        IconKind::StartCalibration => {
            // Precision calibration target & crosshair
            painter.circle_stroke(c, r * 0.85, stroke);
            painter.circle_filled(c, 3.2, color);
            let ext = r * 1.15;
            painter.line_segment(
                [c - Vec2::new(ext, 0.0), c - Vec2::new(r * 0.45, 0.0)],
                stroke,
            );
            painter.line_segment(
                [c + Vec2::new(r * 0.45, 0.0), c + Vec2::new(ext, 0.0)],
                stroke,
            );
            painter.line_segment(
                [c - Vec2::new(0.0, ext), c - Vec2::new(0.0, r * 0.45)],
                stroke,
            );
            painter.line_segment(
                [c + Vec2::new(0.0, r * 0.45), c + Vec2::new(0.0, ext)],
                stroke,
            );
        }
        IconKind::TestTouch => {
            // 3x3 precision grid + diagonal stylus stroke
            let g = Rect::from_center_size(c, Vec2::splat(r * 1.9));
            painter.rect_stroke(
                g,
                CornerRadius::same(2),
                Stroke::new(1.5, color.linear_multiply(0.55)),
                eframe::egui::StrokeKind::Inside,
            );
            for f in [0.33, 0.67] {
                let x = g.left() + g.width() * f;
                let y = g.top() + g.height() * f;
                painter.line_segment(
                    [Pos2::new(x, g.top()), Pos2::new(x, g.bottom())],
                    Stroke::new(1.0, color.linear_multiply(0.4)),
                );
                painter.line_segment(
                    [Pos2::new(g.left(), y), Pos2::new(g.right(), y)],
                    Stroke::new(1.0, color.linear_multiply(0.4)),
                );
            }
            painter.line_segment(
                [
                    g.left_bottom() + Vec2::new(3.0, -3.0),
                    g.right_top() + Vec2::new(-3.0, 3.0),
                ],
                Stroke::new(2.2, color),
            );
            painter.circle_filled(g.right_top() + Vec2::new(-3.0, 3.0), 2.5, color);
        }
        IconKind::Profiles => {
            // Folder / profile layers
            let box_r =
                Rect::from_center_size(c + Vec2::new(0.0, 1.5), Vec2::new(r * 1.9, r * 1.35));
            painter.rect_stroke(
                box_r,
                CornerRadius::same(3),
                stroke,
                eframe::egui::StrokeKind::Inside,
            );
            let tab = Rect::from_min_size(
                box_r.left_top() - Vec2::new(0.0, 4.0),
                Vec2::new(r * 0.9, 5.0),
            );
            painter.rect_filled(tab, CornerRadius::same(2), color);
            painter.line_segment(
                [
                    box_r.left_center() + Vec2::new(4.0, 0.0),
                    box_r.right_center() - Vec2::new(4.0, 0.0),
                ],
                Stroke::new(1.6, color.linear_multiply(0.75)),
            );
        }
        IconKind::Settings => {
            // Three horizontal sliders with knobs
            for (idx, knob_t) in [-0.6, 0.4, -0.15].iter().enumerate() {
                let y = c.y + (idx as f32 - 1.0) * (r * 0.68);
                let lx = c.x - r * 0.95;
                let rx = c.x + r * 0.95;
                painter.line_segment(
                    [Pos2::new(lx, y), Pos2::new(rx, y)],
                    Stroke::new(1.8, color.linear_multiply(0.55)),
                );
                let kx = c.x + knob_t * r * 0.75;
                painter.circle_filled(Pos2::new(kx, y), 3.2, color);
            }
        }
        IconKind::Diagnostics => {
            // Activity waveform pulse inside frame
            let box_r = Rect::from_center_size(c, Vec2::splat(r * 1.9));
            painter.rect_stroke(
                box_r,
                CornerRadius::same(3),
                Stroke::new(1.5, color.linear_multiply(0.45)),
                eframe::egui::StrokeKind::Inside,
            );
            let pts = [
                Pos2::new(box_r.left() + 2.5, c.y + 1.0),
                Pos2::new(c.x - r * 0.45, c.y + 1.0),
                Pos2::new(c.x - r * 0.2, c.y - r * 0.55),
                Pos2::new(c.x + r * 0.15, c.y + r * 0.55),
                Pos2::new(c.x + r * 0.4, c.y - r * 0.15),
                Pos2::new(box_r.right() - 2.5, c.y - r * 0.15),
            ];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
    }
}
