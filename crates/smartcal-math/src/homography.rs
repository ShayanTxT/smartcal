use nalgebra::{DMatrix, DVector, Matrix3, Vector3};
use smartcal_core::{Point2D, Result, SmartCalError};

pub fn apply_homography(m: &[f64; 9], p: Point2D) -> Option<Point2D> {
    let x = m[0] * p.x + m[1] * p.y + m[2];
    let y = m[3] * p.x + m[4] * p.y + m[5];
    let w = m[6] * p.x + m[7] * p.y + m[8];
    if !w.is_finite() || w.abs() < 1e-12 {
        return None;
    }
    let px = x / w;
    let py = y / w;
    if px.is_finite() && py.is_finite() {
        Some(Point2D::new(px, py))
    } else {
        None
    }
}

pub fn apply_affine(a: &[f64; 6], p: Point2D) -> Point2D {
    Point2D::new(
        a[0] * p.x + a[1] * p.y + a[2],
        a[3] * p.x + a[4] * p.y + a[5],
    )
}

pub fn mat3_to_array(m: &Matrix3<f64>) -> [f64; 9] {
    [
        m[(0, 0)],
        m[(0, 1)],
        m[(0, 2)],
        m[(1, 0)],
        m[(1, 1)],
        m[(1, 2)],
        m[(2, 0)],
        m[(2, 1)],
        m[(2, 2)],
    ]
}

pub fn array_to_mat3(a: &[f64; 9]) -> Matrix3<f64> {
    Matrix3::new(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8])
}

pub fn multiply_mat3(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    let ma = array_to_mat3(a);
    let mb = array_to_mat3(b);
    mat3_to_array(&(ma * mb))
}

pub fn invert_mat3(a: &[f64; 9]) -> Option<[f64; 9]> {
    let m = array_to_mat3(a);
    m.try_inverse().map(|inv| mat3_to_array(&inv))
}

fn normalize_matrix_scale(mut m: Matrix3<f64>) -> Matrix3<f64> {
    if m[(2, 2)].abs() > 1e-12 {
        m /= m[(2, 2)];
    } else {
        let norm = m.norm();
        if norm > 1e-12 {
            m /= norm;
        }
    }
    m
}

/// Hartley isotropic point normalization: centroid at origin, mean distance sqrt(2).
fn hartley_normalize(points: &[Point2D]) -> (Vec<Point2D>, Matrix3<f64>) {
    let n = points.len() as f64;
    let cx = points.iter().map(|p| p.x).sum::<f64>() / n;
    let cy = points.iter().map(|p| p.y).sum::<f64>() / n;

    let mean_dist = points
        .iter()
        .map(|p| ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt())
        .sum::<f64>()
        / n;

    let scale = if mean_dist > 1e-9 {
        std::f64::consts::SQRT_2 / mean_dist
    } else {
        1.0
    };

    let t = Matrix3::new(
        scale,
        0.0,
        -scale * cx,
        0.0,
        scale,
        -scale * cy,
        0.0,
        0.0,
        1.0,
    );

    let normalized = points
        .iter()
        .map(|p| Point2D::new(scale * (p.x - cx), scale * (p.y - cy)))
        .collect();

    (normalized, t)
}

fn triangle_area2(a: Point2D, b: Point2D, c: Point2D) -> f64 {
    ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs()
}

/// Checks whether a 4-point tuple has any nearly collinear triple.
pub fn has_collinear_triple(pts: &[Point2D]) -> bool {
    if pts.len() < 3 {
        return true;
    }
    let min_x = pts.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = pts.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = pts.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = pts.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    let span = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2)).sqrt();
    if span < 1e-6 {
        return true;
    }
    let area_thresh = span * span * 1e-4;
    for i in 0..pts.len() {
        for j in (i + 1)..pts.len() {
            for k in (j + 1)..pts.len() {
                if triangle_area2(pts[i], pts[j], pts[k]) < area_thresh {
                    return true;
                }
            }
        }
    }
    false
}

/// Direct Linear Transform (Normalized DLT) solving H such that dst ~ H * src.
pub fn solve_homography_dlt(src: &[Point2D], dst: &[Point2D]) -> Result<Matrix3<f64>> {
    if src.len() != dst.len() || src.len() < 4 {
        return Err(SmartCalError::DegenerateGeometry(
            "At least 4 matching point pairs are required for homography".into(),
        ));
    }

    if src.len() == 4 && (has_collinear_triple(src) || has_collinear_triple(dst)) {
        return Err(SmartCalError::DegenerateGeometry(
            "Calibration points are collinear or collapsed".into(),
        ));
    }

    let (src_n, t_src) = hartley_normalize(src);
    let (dst_n, t_dst) = hartley_normalize(dst);

    let n = src.len();
    // Pad to at least 9 rows so thin SVD in nalgebra produces full 9x9 V^T even when n == 4.
    let mut a = DMatrix::<f64>::zeros((2 * n).max(9), 9);
    for i in 0..n {
        let x = src_n[i].x;
        let y = src_n[i].y;
        let u = dst_n[i].x;
        let v = dst_n[i].y;

        let r0 = 2 * i;
        a[(r0, 0)] = -x;
        a[(r0, 1)] = -y;
        a[(r0, 2)] = -1.0;
        a[(r0, 6)] = u * x;
        a[(r0, 7)] = u * y;
        a[(r0, 8)] = u;

        let r1 = 2 * i + 1;
        a[(r1, 3)] = -x;
        a[(r1, 4)] = -y;
        a[(r1, 5)] = -1.0;
        a[(r1, 6)] = v * x;
        a[(r1, 7)] = v * y;
        a[(r1, 8)] = v;
    }

    let svd = a.svd(false, true);
    let v_t = svd.v_t.ok_or_else(|| {
        SmartCalError::DegenerateGeometry("SVD failed while computing homography".into())
    })?;

    // Last row of V^T corresponds to smallest singular value.
    let h_row = v_t.row(8);
    let h_norm = Matrix3::new(
        h_row[0], h_row[1], h_row[2], h_row[3], h_row[4], h_row[5], h_row[6], h_row[7], h_row[8],
    );

    let t_dst_inv = t_dst.try_inverse().ok_or_else(|| {
        SmartCalError::DegenerateGeometry("Destination point distribution is singular".into())
    })?;

    let h = normalize_matrix_scale(t_dst_inv * h_norm * t_src);
    if !h.iter().all(|v| v.is_finite()) {
        return Err(SmartCalError::DegenerateGeometry(
            "Computed homography contains non-finite entries".into(),
        ));
    }

    Ok(h)
}

/// Refines a homography matrix by minimizing squared Euclidean reprojection error via Levenberg-Marquardt.
pub fn refine_homography_lm(
    initial: Matrix3<f64>,
    src: &[Point2D],
    dst: &[Point2D],
) -> Matrix3<f64> {
    if src.len() < 4 || src.len() != dst.len() {
        return initial;
    }

    let mut h = normalize_matrix_scale(initial);
    if h[(2, 2)].abs() < 1e-9 {
        return initial;
    }

    let mut params = [
        h[(0, 0)],
        h[(0, 1)],
        h[(0, 2)],
        h[(1, 0)],
        h[(1, 1)],
        h[(1, 2)],
        h[(2, 0)],
        h[(2, 1)],
    ];

    let eval_residuals = |p: &[f64; 8]| -> Option<(Vec<f64>, f64)> {
        let mut res = Vec::with_capacity(src.len() * 2);
        let mut cost = 0.0;
        for (s, d) in src.iter().zip(dst.iter()) {
            let denom = p[6] * s.x + p[7] * s.y + 1.0;
            if denom.abs() < 1e-10 || !denom.is_finite() {
                return None;
            }
            let px = (p[0] * s.x + p[1] * s.y + p[2]) / denom;
            let py = (p[3] * s.x + p[4] * s.y + p[5]) / denom;
            let rx = px - d.x;
            let ry = py - d.y;
            cost += rx * rx + ry * ry;
            res.push(rx);
            res.push(ry);
        }
        Some((res, cost))
    };

    let Some((mut residuals, mut current_cost)) = eval_residuals(&params) else {
        return initial;
    };

    let mut lambda = 1e-3_f64;
    let m = src.len() * 2;

    for _ in 0..35 {
        let mut j = DMatrix::<f64>::zeros(m, 8);
        for (idx, s) in src.iter().enumerate() {
            let denom = params[6] * s.x + params[7] * s.y + 1.0;
            if denom.abs() < 1e-10 {
                break;
            }
            let u_num = params[0] * s.x + params[1] * s.y + params[2];
            let v_num = params[3] * s.x + params[4] * s.y + params[5];
            let inv_d = 1.0 / denom;
            let inv_d2 = inv_d * inv_d;

            let r0 = 2 * idx;
            j[(r0, 0)] = s.x * inv_d;
            j[(r0, 1)] = s.y * inv_d;
            j[(r0, 2)] = inv_d;
            j[(r0, 6)] = -u_num * s.x * inv_d2;
            j[(r0, 7)] = -u_num * s.y * inv_d2;

            let r1 = 2 * idx + 1;
            j[(r1, 3)] = s.x * inv_d;
            j[(r1, 4)] = s.y * inv_d;
            j[(r1, 5)] = inv_d;
            j[(r1, 6)] = -v_num * s.x * inv_d2;
            j[(r1, 7)] = -v_num * s.y * inv_d2;
        }

        let r_vec = DVector::from_column_slice(&residuals);
        let jt = j.transpose();
        let mut h_mat = &jt * &j;
        let g = &jt * &r_vec;

        for d in 0..8 {
            h_mat[(d, d)] += lambda * (h_mat[(d, d)].abs().max(1e-6));
        }

        let Some(step) = h_mat.lu().solve(&(-g)) else {
            lambda *= 10.0;
            continue;
        };

        if step.norm() < 1e-10 {
            break;
        }

        let mut candidate = params;
        for d in 0..8 {
            candidate[d] += step[d];
        }

        if let Some((cand_res, cand_cost)) = eval_residuals(&candidate) {
            if cand_cost < current_cost {
                let rel_improv = (current_cost - cand_cost) / (current_cost + 1e-12);
                params = candidate;
                residuals = cand_res;
                current_cost = cand_cost;
                lambda = (lambda * 0.3).max(1e-9);
                if rel_improv < 1e-10 {
                    break;
                }
            } else {
                lambda = (lambda * 5.0).min(1e9);
            }
        } else {
            lambda = (lambda * 10.0).min(1e9);
        }
    }

    h = Matrix3::new(
        params[0], params[1], params[2], params[3], params[4], params[5], params[6], params[7], 1.0,
    );
    normalize_matrix_scale(h)
}

/// Robust Homography estimation using RANSAC + Normalized DLT + LM refinement.
pub fn solve_homography_ransac(
    src: &[Point2D],
    dst: &[Point2D],
    inlier_threshold_px: f64,
) -> Result<(Matrix3<f64>, Vec<bool>)> {
    let n = src.len();
    if n != dst.len() || n < 4 {
        return Err(SmartCalError::DegenerateGeometry(
            "Need at least 4 points for RANSAC homography".into(),
        ));
    }
    if n == 4 {
        let h = refine_homography_lm(solve_homography_dlt(src, dst)?, src, dst);
        return Ok((h, vec![true; 4]));
    }

    let thresh_sq = inlier_threshold_px * inlier_threshold_px;
    let mut best_inliers = vec![false; n];
    let mut best_count = 0usize;
    let mut best_err_sum = f64::INFINITY;

    let mut evaluate_subset = |idx: [usize; 4]| {
        let sub_src = [src[idx[0]], src[idx[1]], src[idx[2]], src[idx[3]]];
        let sub_dst = [dst[idx[0]], dst[idx[1]], dst[idx[2]], dst[idx[3]]];
        if has_collinear_triple(&sub_src) || has_collinear_triple(&sub_dst) {
            return;
        }
        let Ok(h) = solve_homography_dlt(&sub_src, &sub_dst) else {
            return;
        };
        let arr = mat3_to_array(&h);

        let mut inliers = vec![false; n];
        let mut count = 0usize;
        let mut err_sum = 0.0;
        for i in 0..n {
            if let Some(pred) = apply_homography(&arr, src[i]) {
                let d2 = (pred.x - dst[i].x).powi(2) + (pred.y - dst[i].y).powi(2);
                if d2 <= thresh_sq {
                    inliers[i] = true;
                    count += 1;
                    err_sum += d2;
                } else {
                    err_sum += thresh_sq;
                }
            } else {
                err_sum += thresh_sq * 2.0;
            }
        }

        if count > best_count || (count == best_count && err_sum < best_err_sum) {
            best_count = count;
            best_err_sum = err_sum;
            best_inliers = inliers;
        }
    };

    if n <= 12 {
        // Exhaustive deterministic check (at most 495 subsets for 12 points).
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    for l in (k + 1)..n {
                        evaluate_subset([i, j, k, l]);
                    }
                }
            }
        }
    } else {
        // Deterministic LCG sampler over 400 hypotheses.
        let mut seed: u64 = 0x5EED_CAFE_BABE_1234;
        let mut next_rand = || -> usize {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 32) as usize) % n
        };
        for _ in 0..400 {
            let mut picks = [0usize; 4];
            let mut filled = 0;
            let mut guard = 0;
            while filled < 4 && guard < 32 {
                guard += 1;
                let cand = next_rand();
                if !picks[..filled].contains(&cand) {
                    picks[filled] = cand;
                    filled += 1;
                }
            }
            if filled == 4 {
                evaluate_subset(picks);
            }
        }
    }

    if best_count < 4 {
        let h_all = refine_homography_lm(solve_homography_dlt(src, dst)?, src, dst);
        return Ok((h_all, vec![true; n]));
    }

    let inlier_src: Vec<Point2D> = src
        .iter()
        .zip(best_inliers.iter())
        .filter(|(_, keep)| **keep)
        .map(|(p, _)| *p)
        .collect();
    let inlier_dst: Vec<Point2D> = dst
        .iter()
        .zip(best_inliers.iter())
        .filter(|(_, keep)| **keep)
        .map(|(p, _)| *p)
        .collect();

    let h_refit = solve_homography_dlt(&inlier_src, &inlier_dst)?;
    let h_final = refine_homography_lm(h_refit, &inlier_src, &inlier_dst);
    Ok((h_final, best_inliers))
}

/// Computes least-squares 2D affine matrix `[a, b, c, d, e, f]` mapping src -> dst.
pub fn solve_affine_least_squares(src: &[Point2D], dst: &[Point2D]) -> Result<[f64; 6]> {
    if src.len() != dst.len() || src.len() < 3 {
        return Err(SmartCalError::DegenerateGeometry(
            "At least 3 points are required for affine least squares".into(),
        ));
    }

    let n = src.len();
    let mut design = DMatrix::<f64>::zeros(n, 3);
    let mut bx = DVector::<f64>::zeros(n);
    let mut by = DVector::<f64>::zeros(n);

    for i in 0..n {
        design[(i, 0)] = src[i].x;
        design[(i, 1)] = src[i].y;
        design[(i, 2)] = 1.0;
        bx[i] = dst[i].x;
        by[i] = dst[i].y;
    }

    let svd = design.svd(true, true);
    let ax = svd
        .solve(&bx, 1e-10)
        .map_err(|e| SmartCalError::DegenerateGeometry(format!("Affine X solve failed: {e}")))?;
    let ay = svd
        .solve(&by, 1e-10)
        .map_err(|e| SmartCalError::DegenerateGeometry(format!("Affine Y solve failed: {e}")))?;

    Ok([ax[0], ax[1], ax[2], ay[0], ay[1], ay[2]])
}

/// Composes normalized->local-px homography with monitor position inside virtual desktop to form X11 CTM.
pub fn compose_x11_ctm(
    homography_norm_to_local_px: &[f64; 9],
    disp_x: i32,
    disp_y: i32,
    virt_w: u32,
    virt_h: u32,
) -> [f64; 9] {
    let vw = (virt_w.max(1)) as f64;
    let vh = (virt_h.max(1)) as f64;
    let dx = disp_x as f64;
    let dy = disp_y as f64;

    let local_px_to_virt_norm = [
        1.0 / vw,
        0.0,
        dx / vw,
        0.0,
        1.0 / vh,
        dy / vh,
        0.0,
        0.0,
        1.0,
    ];
    multiply_mat3(&local_px_to_virt_norm, homography_norm_to_local_px)
}

/// Composes normalized->local-px homography into normalized->output-normalized [0,1] space.
pub fn compose_output_norm_homography(
    homography_norm_to_local_px: &[f64; 9],
    disp_w: u32,
    disp_h: u32,
) -> [f64; 9] {
    let w = (disp_w.max(1)) as f64;
    let h = (disp_h.max(1)) as f64;
    let px_to_norm = [1.0 / w, 0.0, 0.0, 0.0, 1.0 / h, 0.0, 0.0, 0.0, 1.0];
    multiply_mat3(&px_to_norm, homography_norm_to_local_px)
}

/// Identity 3x3 matrix.
pub fn identity_mat3() -> [f64; 9] {
    [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
}

/// Computes X11 CTM mapping whole device uncalibrated [0,1] to a specific output monitor rectangle.
pub fn output_only_ctm(
    disp_x: i32,
    disp_y: i32,
    disp_w: u32,
    disp_h: u32,
    virt_w: u32,
    virt_h: u32,
) -> [f64; 9] {
    let vw = (virt_w.max(1)) as f64;
    let vh = (virt_h.max(1)) as f64;
    [
        disp_w as f64 / vw,
        0.0,
        disp_x as f64 / vw,
        0.0,
        disp_h as f64 / vh,
        disp_y as f64 / vh,
        0.0,
        0.0,
        1.0,
    ]
}

/// Unused Vector3 sanity helper check.
#[allow(dead_code)]
fn _project_vec3(m: &Matrix3<f64>, p: Point2D) -> Option<Point2D> {
    let res = m * Vector3::new(p.x, p.y, 1.0);
    if res[2].abs() < 1e-12 {
        None
    } else {
        Some(Point2D::new(res[0] / res[2], res[1] / res[2]))
    }
}
