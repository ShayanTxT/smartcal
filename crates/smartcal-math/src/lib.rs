pub mod calibrate;
pub mod filter;
pub mod homography;
pub mod polynomial;
pub mod tps;

pub use calibrate::{
    CalibrationOutput, compute_calibration, evaluate_model_norm_to_screen,
    evaluate_model_raw_to_screen, generate_target_grid, grade_quality, is_quality_acceptable,
    map_raw_to_screen, map_raw_via_libinput_affine, map_screen_to_raw,
};
pub use filter::{
    FilteredTouch, RawSample, SingleTouchTracker, denormalize_raw_point, filter_touch_samples,
    median_filter_points, median_of, moving_average_points, normalize_raw_point,
};
pub use homography::{
    apply_affine, apply_homography, compose_output_norm_homography, compose_x11_ctm,
    has_collinear_triple, identity_mat3, invert_mat3, multiply_mat3, output_only_ctm,
    refine_homography_lm, solve_affine_least_squares, solve_homography_dlt,
    solve_homography_ransac,
};
