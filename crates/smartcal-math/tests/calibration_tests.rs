use smartcal_core::{
    AxisBounds, AxisRange, CalibrationConfig, CalibrationMethod, CalibrationQuality,
    CalibrationType, DisplayInfo, Point2D, SmartCalError,
};
use smartcal_math::{
    FilteredTouch, RawSample, SingleTouchTracker, apply_homography, compute_calibration,
    evaluate_model_raw_to_screen, filter_touch_samples, generate_target_grid, solve_homography_dlt,
    solve_homography_ransac,
};
use std::time::{Duration, Instant};

fn test_display_projector() -> DisplayInfo {
    DisplayInfo {
        id: "Display 2".into(),
        name: "HDMI-1".into(),
        friendly_label: "Classroom Projector".into(),
        width: 1920,
        height: 1080,
        x: 1920,
        y: 0,
        refresh_hz: 60.0,
        scale_factor: 1.0,
        primary: false,
        connected: true,
        physical_width_mm: None,
        physical_height_mm: None,
    }
}

fn synthetic_projective_raw(screen: Point2D, bounds: &AxisBounds) -> Point2D {
    // Invert a tilted projector homography: map normalized screen -> warped raw coordinates
    let sx = screen.x / 1920.0;
    let sy = screen.y / 1080.0;
    // Perspective keystone + 90-deg rotation + offset + scaling
    let denom = 1.0 + 0.14 * sx - 0.08 * sy;
    let u = (0.85 - 0.72 * sy + 0.05 * sx) / denom;
    let v = (0.12 + 0.76 * sx + 0.04 * sy) / denom;
    Point2D::new(
        bounds.x.min as f64 + u * (bounds.x.max - bounds.x.min) as f64,
        bounds.y.min as f64 + v * (bounds.y.max - bounds.y.min) as f64,
    )
}

#[test]
fn four_point_homography_recovers_perspective_and_rotation() {
    let disp = test_display_projector();
    let bounds = AxisBounds {
        x: AxisRange {
            min: 0,
            max: 32767,
            ..Default::default()
        },
        y: AxisRange {
            min: 0,
            max: 32767,
            ..Default::default()
        },
    };
    let targets = generate_target_grid(disp.width, disp.height, CalibrationType::Points4, 0.08);

    let captured: Vec<FilteredTouch> = targets
        .iter()
        .map(|t| {
            let raw = synthetic_projective_raw(*t, &bounds);
            FilteredTouch {
                raw_center: raw,
                raw_normalized: Point2D::new(raw.x / 32767.0, raw.y / 32767.0),
                accepted_samples: 15,
                rejected_outliers: 0,
                spread_norm: 0.001,
                duration_ms: 250,
            }
        })
        .collect();

    let cfg = CalibrationConfig {
        calibration_type: CalibrationType::Points4,
        method: CalibrationMethod::Homography,
        ..Default::default()
    };

    let out = compute_calibration(&disp, 3840, 1080, &bounds, &cfg, &targets, &captured)
        .expect("4-point homography must succeed");

    assert!(
        out.error.average < 0.01,
        "avg error was {}",
        out.error.average
    );
    assert!(
        out.error.maximum < 0.05,
        "max error was {}",
        out.error.maximum
    );
    assert_eq!(out.error.quality, CalibrationQuality::Excellent);

    // Check multi-monitor X11 CTM maps projector points strictly into x >= 0.5 (second half of 3840x1080!)
    let center_raw = synthetic_projective_raw(Point2D::new(960.0, 540.0), &bounds);
    let center_norm = Point2D::new(center_raw.x / 32767.0, center_raw.y / 32767.0);
    let virt_norm = apply_homography(&out.model.x11_ctm, center_norm).unwrap();
    assert!(
        (virt_norm.x - 0.75).abs() < 0.01,
        "virt_norm.x={}",
        virt_norm.x
    );
    assert!(
        (virt_norm.y - 0.50).abs() < 0.01,
        "virt_norm.y={}",
        virt_norm.y
    );
}

#[test]
fn nine_point_ransac_rejects_bad_tap_outlier() {
    let disp = test_display_projector();
    let bounds = AxisBounds::default();
    let targets = generate_target_grid(disp.width, disp.height, CalibrationType::Points9, 0.08);

    let mut captured: Vec<FilteredTouch> = targets
        .iter()
        .map(|t| {
            let raw = synthetic_projective_raw(*t, &bounds);
            FilteredTouch {
                raw_center: raw,
                raw_normalized: Point2D::new(raw.x / 32767.0, raw.y / 32767.0),
                accepted_samples: 14,
                rejected_outliers: 1,
                spread_norm: 0.002,
                duration_ms: 260,
            }
        })
        .collect();

    // Inject a massive accidental touch error on point #4!
    captured[4].raw_center.x += 5500.0;
    captured[4].raw_normalized.x += 5500.0 / 32767.0;

    let src_norm: Vec<Point2D> = captured.iter().map(|c| c.raw_normalized).collect();
    let (h, inliers) = solve_homography_ransac(&src_norm, &targets, 20.0)
        .expect("RANSAC homography should handle single outlier");

    assert!(!inliers[4], "Outlier point 4 should be rejected by RANSAC");
    let inlier_count = inliers.iter().filter(|b| **b).count();
    assert_eq!(inlier_count, 8);

    // Verify clean point 0 still projects accurately
    let h_arr = [
        h[(0, 0)],
        h[(0, 1)],
        h[(0, 2)],
        h[(1, 0)],
        h[(1, 1)],
        h[(1, 2)],
        h[(2, 0)],
        h[(2, 1)],
        h[(2, 2)],
    ];
    let p0 = apply_homography(&h_arr, src_norm[0]).unwrap();
    assert!(p0.distance_to(&targets[0]) < 1.0);
}

#[test]
fn polynomial_and_hybrid_tps_correct_nonlinear_barrel_distortion() {
    let disp = test_display_projector();
    let bounds = AxisBounds::default();
    let targets = generate_target_grid(disp.width, disp.height, CalibrationType::Points16, 0.08);

    let captured: Vec<FilteredTouch> = targets
        .iter()
        .map(|t| {
            let sx = t.x / 1920.0 - 0.5;
            let sy = t.y / 1080.0 - 0.5;
            let r2 = sx * sx + sy * sy;
            // Strong radial barrel distortion
            let u = 0.5 + sx * (1.0 + 0.25 * r2);
            let v = 0.5 + sy * (1.0 + 0.25 * r2);
            let raw = Point2D::new(u * 32767.0, v * 32767.0);
            FilteredTouch {
                raw_center: raw,
                raw_normalized: Point2D::new(u, v),
                accepted_samples: 16,
                rejected_outliers: 0,
                spread_norm: 0.001,
                duration_ms: 250,
            }
        })
        .collect();

    for method in [
        CalibrationMethod::PolynomialDeg3,
        CalibrationMethod::HybridTps,
    ] {
        let cfg = CalibrationConfig {
            calibration_type: CalibrationType::Points16,
            method,
            ..Default::default()
        };
        let out = compute_calibration(&disp, 1920, 1080, &bounds, &cfg, &targets, &captured)
            .expect("Nonlinear model must fit");
        assert!(
            out.error.average < 4.0,
            "{method} avg error too high: {}",
            out.error.average
        );

        let probe_raw = Point2D::new(0.5 * 32767.0, 0.5 * 32767.0);
        let mapped = evaluate_model_raw_to_screen(&out.model, &bounds, probe_raw, method).unwrap();
        assert!(mapped.distance_to(&Point2D::new(960.0, 540.0)) < 5.0);
    }
}

#[test]
fn collinear_four_points_rejected_cleanly() {
    let src = vec![
        Point2D::new(0.1, 0.1),
        Point2D::new(0.2, 0.2),
        Point2D::new(0.3, 0.3),
        Point2D::new(0.4, 0.4),
    ];
    let dst = generate_target_grid(1920, 1080, CalibrationType::Points4, 0.1);
    assert!(solve_homography_dlt(&src, &dst).is_err());
}

#[test]
fn touch_sample_filter_discards_initial_wobble_and_outliers() {
    let bounds = AxisBounds::default();
    let cfg = CalibrationConfig {
        initial_discard_samples: 2,
        min_touch_duration_ms: 150,
        max_deviation_norm: 0.03,
        ..Default::default()
    };

    let t0 = Instant::now();
    let mut samples = Vec::new();
    // 2 noisy initial landing samples
    samples.push(RawSample {
        raw: Point2D::new(12000.0, 12000.0),
        timestamp: t0,
    });
    samples.push(RawSample {
        raw: Point2D::new(15000.0, 14500.0),
        timestamp: t0 + Duration::from_millis(20),
    });
    // Stable touch around (16000, 16000)
    for i in 0..12 {
        let jitter = if i == 5 { 900.0 } else { (i % 3) as f64 * 12.0 };
        samples.push(RawSample {
            raw: Point2D::new(16000.0 + jitter, 16000.0 - jitter * 0.5),
            timestamp: t0 + Duration::from_millis(40 + i * 20),
        });
    }

    let res = filter_touch_samples(&samples, &bounds, &cfg, &[])
        .expect("Stable touch with initial wobble + 1 outlier spike should filter cleanly");

    assert!((res.raw_center.x - 16012.0).abs() < 60.0);
    assert!((res.raw_center.y - 15994.0).abs() < 60.0);
    assert!(res.rejected_outliers >= 1);
}

#[test]
fn touch_sample_filter_rejects_wild_swipe() {
    let bounds = AxisBounds::default();
    let cfg = CalibrationConfig::default();
    let t0 = Instant::now();
    let samples: Vec<RawSample> = (0..10)
        .map(|i| RawSample {
            raw: Point2D::new(5000.0 + i as f64 * 2500.0, 5000.0),
            timestamp: t0 + Duration::from_millis(i * 30),
        })
        .collect();

    let err = filter_touch_samples(&samples, &bounds, &cfg, &[]).unwrap_err();
    assert!(matches!(err, SmartCalError::TouchDetectionInaccurate));
}

#[test]
fn single_touch_tracker_ignores_secondary_finger() {
    let mut tracker = SingleTouchTracker::default();
    tracker.set_current_slot(0);
    tracker.on_tracking_id(101);
    tracker.update_mt_x(5000.0);
    tracker.update_mt_y(6000.0);

    // Second finger arrives in slot 1!
    tracker.set_current_slot(1);
    tracker.on_tracking_id(102);
    tracker.update_mt_x(29000.0);
    tracker.update_mt_y(29000.0);

    // Primary coordinate must stay locked to slot 0!
    assert_eq!(tracker.current_point(), Some(Point2D::new(5000.0, 6000.0)));
}
