use crate::filter::{FilteredTouch, denormalize_raw_point, normalize_raw_point};
use crate::homography::{
    apply_affine, apply_homography, compose_output_norm_homography, compose_x11_ctm, mat3_to_array,
    refine_homography_lm, solve_affine_least_squares, solve_homography_dlt,
    solve_homography_ransac,
};
use crate::polynomial::{eval_polynomial, fit_polynomial, required_points_for_degree};
use crate::tps::{eval_tps, fit_tps};
use smartcal_core::{
    AxisBounds, CalibrationConfig, CalibrationMethod, CalibrationModelData, CalibrationPointRecord,
    CalibrationQuality, CalibrationType, DisplayInfo, ErrorStats, Point2D, Result, SmartCalError,
};

#[derive(Debug, Clone)]
pub struct CalibrationOutput {
    pub model: CalibrationModelData,
    pub matrix: [f64; 9],
    pub points: Vec<CalibrationPointRecord>,
    pub error: ErrorStats,
}

/// Generates calibration target pixel positions strictly inside the chosen display.
pub fn generate_target_grid(
    width: u32,
    height: u32,
    cal_type: CalibrationType,
    margin_ratio: f64,
) -> Vec<Point2D> {
    let m = margin_ratio.clamp(0.03, 0.25);
    let w = width.max(100) as f64;
    let h = height.max(100) as f64;
    let (cols, rows) = cal_type.grid_dims();

    let mut targets = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        let ty = if rows == 1 {
            0.5
        } else {
            m + (1.0 - 2.0 * m) * (r as f64 / (rows - 1) as f64)
        };
        for c in 0..cols {
            let tx = if cols == 1 {
                0.5
            } else {
                m + (1.0 - 2.0 * m) * (c as f64 / (cols - 1) as f64)
            };
            targets.push(Point2D::new(tx * w, ty * h));
        }
    }
    targets
}

pub fn grade_quality(avg_px: f64, max_px: f64) -> CalibrationQuality {
    if avg_px <= 3.0 && max_px <= 6.5 {
        CalibrationQuality::Excellent
    } else if avg_px <= 7.0 && max_px <= 14.0 {
        CalibrationQuality::Good
    } else if avg_px <= 15.0 && max_px <= 28.0 {
        CalibrationQuality::Acceptable
    } else {
        CalibrationQuality::Poor
    }
}

pub fn is_quality_acceptable(quality: CalibrationQuality) -> bool {
    !matches!(quality, CalibrationQuality::Poor)
}

/// Evaluates a calibrated model mapping raw coordinates -> local monitor pixels.
pub fn evaluate_model_raw_to_screen(
    model: &CalibrationModelData,
    raw_bounds: &AxisBounds,
    raw_point: Point2D,
    method: CalibrationMethod,
) -> Option<Point2D> {
    let norm = normalize_raw_point(raw_point, raw_bounds);
    evaluate_model_norm_to_screen(model, norm, method)
}

pub fn evaluate_model_norm_to_screen(
    model: &CalibrationModelData,
    norm: Point2D,
    method: CalibrationMethod,
) -> Option<Point2D> {
    match method {
        CalibrationMethod::Affine => {
            // libinput_affine maps [0,1] -> [0,1] output norm, while homography_norm_to_screen maps to px.
            apply_homography(&model.homography_norm_to_screen, norm)
        }
        CalibrationMethod::PolynomialDeg2 | CalibrationMethod::PolynomialDeg3 => {
            if let (Some(deg), Some(cx), Some(cy)) =
                (model.poly_degree, &model.poly_coeff_x, &model.poly_coeff_y)
            {
                eval_polynomial(deg, cx, cy, norm)
            } else {
                apply_homography(&model.homography_norm_to_screen, norm)
            }
        }
        CalibrationMethod::HybridTps => {
            let base = apply_homography(&model.homography_norm_to_screen, norm)?;
            if let Some(tps) = &model.tps_residual {
                let residual = eval_tps(tps, norm);
                Some(Point2D::new(base.x + residual.x, base.y + residual.y))
            } else {
                Some(base)
            }
        }
        CalibrationMethod::Homography | CalibrationMethod::HomographyRansac => {
            apply_homography(&model.homography_norm_to_screen, norm)
        }
    }
}

/// Computes the complete calibration model, multi-monitor X11 CTM, libinput affine matrix,
/// per-point errors, and overall quality grade.
pub fn compute_calibration(
    display: &DisplayInfo,
    virt_desktop_w: u32,
    virt_desktop_h: u32,
    _raw_bounds: &AxisBounds,
    config: &CalibrationConfig,
    expected_screen: &[Point2D],
    captured: &[FilteredTouch],
) -> Result<CalibrationOutput> {
    if expected_screen.len() != captured.len() || expected_screen.len() < 4 {
        return Err(SmartCalError::DegenerateGeometry(
            "Insufficient captured points for calibration".into(),
        ));
    }

    let norm_src: Vec<Point2D> = captured.iter().map(|c| c.raw_normalized).collect();
    let screen_dst: Vec<Point2D> = expected_screen.to_vec();

    let screen_diag = ((display.width as f64).powi(2) + (display.height as f64).powi(2)).sqrt();
    let ransac_thresh = (screen_diag * 0.015).clamp(12.0, 45.0);

    // Always compute a robust homography baseline (needed for X11 CTM, diagnostics, and fallback).
    let (h_mat, inlier_mask) = match config.method {
        CalibrationMethod::Homography => {
            let h = refine_homography_lm(
                solve_homography_dlt(&norm_src, &screen_dst)?,
                &norm_src,
                &screen_dst,
            );
            (h, vec![true; norm_src.len()])
        }
        _ => solve_homography_ransac(&norm_src, &screen_dst, ransac_thresh)?,
    };

    let mut h_arr = mat3_to_array(&h_mat);
    let mut poly_degree = None;
    let mut poly_coeff_x = None;
    let mut poly_coeff_y = None;
    let mut tps_residual = None;

    // Fit method-specific high-order model if selected and enough points are available.
    match config.method {
        CalibrationMethod::Affine => {
            let aff = solve_affine_least_squares(&norm_src, &screen_dst)?;
            h_arr = [
                aff[0], aff[1], aff[2], aff[3], aff[4], aff[5], 0.0, 0.0, 1.0,
            ];
        }
        CalibrationMethod::PolynomialDeg2 | CalibrationMethod::PolynomialDeg3 => {
            let wanted = if config.method == CalibrationMethod::PolynomialDeg3 {
                3
            } else {
                2
            };
            let deg = if norm_src.len() >= required_points_for_degree(wanted) {
                wanted
            } else if norm_src.len() >= required_points_for_degree(2) {
                2
            } else {
                1
            };
            if deg >= 2 {
                let (d, cx, cy) = fit_polynomial(&norm_src, &screen_dst, deg, 1e-6)?;
                poly_degree = Some(d);
                poly_coeff_x = Some(cx);
                poly_coeff_y = Some(cy);
            }
        }
        CalibrationMethod::HybridTps => {
            let residuals: Vec<Point2D> = norm_src
                .iter()
                .zip(screen_dst.iter())
                .map(|(s, d)| {
                    let base = apply_homography(&h_arr, *s).unwrap_or(*d);
                    Point2D::new(d.x - base.x, d.y - base.y)
                })
                .collect();
            if let Ok(tps) = fit_tps(&norm_src, &residuals, 0.002) {
                tps_residual = Some(tps);
            }
        }
        CalibrationMethod::Homography | CalibrationMethod::HomographyRansac => {}
    }

    // Compute output-normalized affine matrix for libinput (`[0,1] -> [0,1]` within target display).
    let disp_w = display.width.max(1) as f64;
    let disp_h = display.height.max(1) as f64;
    let dst_output_norm: Vec<Point2D> = screen_dst
        .iter()
        .map(|p| Point2D::new(p.x / disp_w, p.y / disp_h))
        .collect();
    let libinput_affine = solve_affine_least_squares(&norm_src, &dst_output_norm)?;

    let homography_norm_to_output_norm =
        compose_output_norm_homography(&h_arr, display.width, display.height);
    let x11_ctm = compose_x11_ctm(
        &h_arr,
        display.x,
        display.y,
        virt_desktop_w.max(display.width),
        virt_desktop_h.max(display.height),
    );

    let ransac_inliers = inlier_mask.iter().filter(|b| **b).count();
    let model = CalibrationModelData {
        homography_norm_to_screen: h_arr,
        homography_norm_to_output_norm,
        x11_ctm,
        libinput_affine,
        poly_degree,
        poly_coeff_x,
        poly_coeff_y,
        tps_residual,
        ransac_inliers,
        total_points: norm_src.len(),
    };

    let mut points = Vec::with_capacity(norm_src.len());
    let mut sum_err = 0.0;
    let mut sum_sq_err = 0.0;
    let mut max_err = 0.0_f64;

    for i in 0..norm_src.len() {
        let predicted = evaluate_model_norm_to_screen(&model, norm_src[i], config.method)
            .ok_or_else(|| {
                SmartCalError::DegenerateGeometry("Projective singularity during evaluation".into())
            })?;
        let dx = predicted.x - screen_dst[i].x;
        let dy = predicted.y - screen_dst[i].y;
        let px_err = (dx * dx + dy * dy).sqrt();

        sum_err += px_err;
        sum_sq_err += px_err * px_err;
        max_err = max_err.max(px_err);

        points.push(CalibrationPointRecord {
            index: i + 1,
            expected_screen: [screen_dst[i].x, screen_dst[i].y],
            raw_center: [captured[i].raw_center.x, captured[i].raw_center.y],
            raw_normalized: [captured[i].raw_normalized.x, captured[i].raw_normalized.y],
            predicted_screen: [predicted.x, predicted.y],
            error_dx: dx,
            error_dy: dy,
            pixel_error: px_err,
            accepted_samples: captured[i].accepted_samples,
            rejected_outliers: captured[i].rejected_outliers,
            is_ransac_inlier: inlier_mask[i],
        });
    }

    let n_f = norm_src.len() as f64;
    let avg = sum_err / n_f;
    let rms = (sum_sq_err / n_f).sqrt();
    let quality = grade_quality(avg, max_err);

    Ok(CalibrationOutput {
        matrix: h_arr,
        model,
        points,
        error: ErrorStats {
            average: avg,
            maximum: max_err,
            rms,
            quality,
        },
    })
}

/// Evaluates a calibrated touch event in Test Mode, mapping raw coordinates to screen pixels
/// and normalized coordinates, and clamping/reporting if outside monitor.
pub fn map_raw_to_screen(
    model: &CalibrationModelData,
    method: CalibrationMethod,
    raw_bounds: &AxisBounds,
    raw_pt: Point2D,
) -> Option<Point2D> {
    evaluate_model_raw_to_screen(model, raw_bounds, raw_pt, method)
}

/// Evaluates libinput affine matrix onto raw coordinates (used when comparing affine vs projective).
pub fn map_raw_via_libinput_affine(
    affine: &[f64; 6],
    raw_bounds: &AxisBounds,
    raw_pt: Point2D,
    disp_w: u32,
    disp_h: u32,
) -> Point2D {
    let norm = normalize_raw_point(raw_pt, raw_bounds);
    let out_norm = apply_affine(affine, norm);
    Point2D::new(out_norm.x * disp_w as f64, out_norm.y * disp_h as f64)
}

/// Re-projects local monitor screen coordinates back to raw evdev coordinates via inverse homography.
pub fn map_screen_to_raw(
    model: &CalibrationModelData,
    raw_bounds: &AxisBounds,
    screen_pt: Point2D,
) -> Option<Point2D> {
    let inv = crate::homography::invert_mat3(&model.homography_norm_to_screen)?;
    let norm = apply_homography(&inv, screen_pt)?;
    Some(denormalize_raw_point(norm, raw_bounds))
}
