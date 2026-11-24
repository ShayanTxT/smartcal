use smartcal_core::{AxisBounds, CalibrationConfig, Point2D, Result, SmartCalError};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct FilteredTouch {
    pub raw_center: Point2D,
    pub raw_normalized: Point2D,
    pub accepted_samples: usize,
    pub rejected_outliers: usize,
    pub spread_norm: f64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct RawSample {
    pub raw: Point2D,
    pub timestamp: Instant,
}

/// Ensures only ONE touch contact (primary slot/tracking ID) is accepted during calibration.
#[derive(Debug, Clone, Default)]
pub struct SingleTouchTracker {
    active_slot: Option<i32>,
    active_tracking_id: Option<i32>,
    current_slot: i32,
    touch_down: bool,
    latest_x: Option<f64>,
    latest_y: Option<f64>,
}

impl SingleTouchTracker {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn set_current_slot(&mut self, slot: i32) {
        self.current_slot = slot;
    }

    pub fn on_tracking_id(&mut self, id: i32) {
        if id >= 0 {
            if self.active_tracking_id.is_none() {
                self.active_tracking_id = Some(id);
                self.active_slot = Some(self.current_slot);
                self.touch_down = true;
            }
        } else if self.active_slot == Some(self.current_slot) {
            self.active_tracking_id = None;
            self.active_slot = None;
            self.touch_down = false;
        }
    }

    pub fn on_btn_touch(&mut self, down: bool) {
        if down {
            if self.active_slot.is_none() {
                self.active_slot = Some(0);
            }
            self.touch_down = true;
        } else {
            self.touch_down = false;
            self.active_slot = None;
            self.active_tracking_id = None;
        }
    }

    pub fn is_primary_slot(&self) -> bool {
        match self.active_slot {
            Some(s) => s == self.current_slot,
            None => self.current_slot == 0,
        }
    }

    pub fn update_mt_x(&mut self, x: f64) {
        if self.is_primary_slot() {
            self.latest_x = Some(x);
        }
    }

    pub fn update_mt_y(&mut self, y: f64) {
        if self.is_primary_slot() {
            self.latest_y = Some(y);
        }
    }

    pub fn update_abs_x(&mut self, x: f64) {
        if self.active_slot.is_none() || self.active_slot == Some(0) {
            self.latest_x = Some(x);
        }
    }

    pub fn update_abs_y(&mut self, y: f64) {
        if self.active_slot.is_none() || self.active_slot == Some(0) {
            self.latest_y = Some(y);
        }
    }

    pub fn is_touch_down(&self) -> bool {
        self.touch_down
    }

    pub fn current_point(&self) -> Option<Point2D> {
        match (self.latest_x, self.latest_y) {
            (Some(x), Some(y)) => Some(Point2D::new(x, y)),
            _ => None,
        }
    }
}

pub fn normalize_raw_point(raw: Point2D, bounds: &AxisBounds) -> Point2D {
    let dx = (bounds.x.max - bounds.x.min) as f64;
    let dy = (bounds.y.max - bounds.y.min) as f64;
    let u = if dx.abs() > 1.0 {
        (raw.x - bounds.x.min as f64) / dx
    } else {
        0.5
    };
    let v = if dy.abs() > 1.0 {
        (raw.y - bounds.y.min as f64) / dy
    } else {
        0.5
    };
    Point2D::new(u, v)
}

pub fn denormalize_raw_point(norm: Point2D, bounds: &AxisBounds) -> Point2D {
    let dx = (bounds.x.max - bounds.x.min) as f64;
    let dy = (bounds.y.max - bounds.y.min) as f64;
    Point2D::new(
        bounds.x.min as f64 + norm.x * dx,
        bounds.y.min as f64 + norm.y * dy,
    )
}

pub fn median_of(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        0.5 * (sorted[mid - 1] + sorted[mid])
    } else {
        sorted[mid]
    }
}

/// Applies component-wise sliding median filter of window size `window` (must be odd >= 3).
pub fn median_filter_points(points: &[Point2D], window: usize) -> Vec<Point2D> {
    if points.len() < 3 || window < 3 {
        return points.to_vec();
    }
    let half = window / 2;
    points
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let start = i.saturating_sub(half);
            let end = (i + half + 1).min(points.len());
            let xs: Vec<f64> = points[start..end].iter().map(|p| p.x).collect();
            let ys: Vec<f64> = points[start..end].iter().map(|p| p.y).collect();
            Point2D::new(median_of(&xs), median_of(&ys))
        })
        .collect()
}

/// Applies moving average smoothing over filtered points.
pub fn moving_average_points(points: &[Point2D], window: usize) -> Vec<Point2D> {
    if points.len() < 2 || window < 2 {
        return points.to_vec();
    }
    let half = window / 2;
    points
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let start = i.saturating_sub(half);
            let end = (i + half + 1).min(points.len());
            let count = (end - start) as f64;
            let sx: f64 = points[start..end].iter().map(|p| p.x).sum();
            let sy: f64 = points[start..end].iter().map(|p| p.y).sum();
            Point2D::new(sx / count, sy / count)
        })
        .collect()
}

/// Processes a sequence of raw touch samples for a single calibration target:
/// 1. Discards initial transient samples
/// 2. Verifies minimum touch duration (when multiple samples span time)
/// 3. Applies median filter + MAD / max_deviation outlier rejection
/// 4. Applies moving average to compute stable touch center
pub fn filter_touch_samples(
    samples: &[RawSample],
    bounds: &AxisBounds,
    config: &CalibrationConfig,
    existing_norm_points: &[Point2D],
) -> Result<FilteredTouch> {
    if samples.is_empty() {
        return Err(SmartCalError::TouchDetectionInaccurate);
    }

    let first_t = samples.first().map(|s| s.timestamp).unwrap();
    let last_t = samples.last().map(|s| s.timestamp).unwrap();
    let duration_ms = last_t.duration_since(first_t).as_millis() as u64;

    if samples.len() >= 3 && duration_ms < config.min_touch_duration_ms / 2 {
        return Err(SmartCalError::TouchDetectionInaccurate);
    }

    // Discard initial landing transient samples unless total count is too small.
    let skip = if samples.len() > config.initial_discard_samples + 3 {
        config.initial_discard_samples
    } else if samples.len() > 2 {
        1
    } else {
        0
    };
    let trimmed = &samples[skip..];

    let norm_pts: Vec<Point2D> = trimmed
        .iter()
        .map(|s| normalize_raw_point(s.raw, bounds))
        .collect();

    let raw_pts: Vec<Point2D> = trimmed.iter().map(|s| s.raw).collect();

    // Check normalized bounds sanity (reject completely bogus coordinates far outside [0,1]).
    if norm_pts.iter().any(|p| {
        !p.x.is_finite()
            || !p.y.is_finite()
            || p.x < -0.25
            || p.x > 1.25
            || p.y < -0.25
            || p.y > 1.25
    }) {
        return Err(SmartCalError::TouchDetectionInaccurate);
    }

    // Median smoothing before MAD outlier detection.
    let smoothed_norm = median_filter_points(&norm_pts, 3);
    let xs: Vec<f64> = smoothed_norm.iter().map(|p| p.x).collect();
    let ys: Vec<f64> = smoothed_norm.iter().map(|p| p.y).collect();
    let med_x = median_of(&xs);
    let med_y = median_of(&ys);

    let dev_x: Vec<f64> = xs.iter().map(|x| (x - med_x).abs()).collect();
    let dev_y: Vec<f64> = ys.iter().map(|y| (y - med_y).abs()).collect();
    // Scaled MAD (~1.4826 * MAD approximates std dev for normal distributions).
    let mad_x = (median_of(&dev_x) * 1.4826).max(0.0025);
    let mad_y = (median_of(&dev_y) * 1.4826).max(0.0025);

    let thresh_x = (config.outlier_sigma * mad_x).min(config.max_deviation_norm);
    let thresh_y = (config.outlier_sigma * mad_y).min(config.max_deviation_norm);

    let mut kept_raw = Vec::new();
    let mut kept_norm = Vec::new();
    let mut rejected = 0usize;

    for (raw_p, norm_p) in raw_pts.iter().zip(norm_pts.iter()) {
        let dist = ((norm_p.x - med_x).powi(2) + (norm_p.y - med_y).powi(2)).sqrt();
        if (norm_p.x - med_x).abs() <= thresh_x
            && (norm_p.y - med_y).abs() <= thresh_y
            && dist <= config.max_deviation_norm
        {
            kept_raw.push(*raw_p);
            kept_norm.push(*norm_p);
        } else {
            rejected += 1;
        }
    }

    // Require at least half of trimmed samples (or >=1 when single-event fallback) to be consistent.
    if kept_raw.is_empty() || (trimmed.len() >= 4 && kept_raw.len() * 2 < trimmed.len()) {
        return Err(SmartCalError::TouchDetectionInaccurate);
    }

    // Moving average across inliers, then centroid.
    let ma_raw = moving_average_points(&kept_raw, 3);
    let ma_norm = moving_average_points(&kept_norm, 3);
    let cnt = ma_raw.len() as f64;
    let center_raw = Point2D::new(
        ma_raw.iter().map(|p| p.x).sum::<f64>() / cnt,
        ma_raw.iter().map(|p| p.y).sum::<f64>() / cnt,
    );
    let center_norm = Point2D::new(
        ma_norm.iter().map(|p| p.x).sum::<f64>() / cnt,
        ma_norm.iter().map(|p| p.y).sum::<f64>() / cnt,
    );

    // Compute RMS spread in normalized coordinates.
    let spread_norm = (kept_norm
        .iter()
        .map(|p| (p.x - center_norm.x).powi(2) + (p.y - center_norm.y).powi(2))
        .sum::<f64>()
        / cnt)
        .sqrt();

    if spread_norm > config.max_deviation_norm {
        return Err(SmartCalError::TouchDetectionInaccurate);
    }

    // Duplicate touch guard: reject if virtually identical to an earlier distinct grid target.
    if existing_norm_points
        .iter()
        .any(|prev| prev.distance_to(&center_norm) < 0.025)
    {
        return Err(SmartCalError::TouchDetectionInaccurate);
    }

    Ok(FilteredTouch {
        raw_center: center_raw,
        raw_normalized: center_norm,
        accepted_samples: kept_raw.len(),
        rejected_outliers: rejected,
        spread_norm,
        duration_ms,
    })
}
