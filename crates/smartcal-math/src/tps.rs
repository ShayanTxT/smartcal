use nalgebra::{DMatrix, DVector};
use smartcal_core::{Point2D, Result, SmartCalError, TpsModelData};

fn tps_kernel(r2: f64) -> f64 {
    if r2 <= 1e-16 {
        0.0
    } else {
        r2 * (r2 + 1e-12).ln()
    }
}

/// Fits a regularized 2D Thin-Plate Spline mapping src -> target residuals.
pub fn fit_tps(src: &[Point2D], target: &[Point2D], smoothing: f64) -> Result<TpsModelData> {
    let n = src.len();
    if n != target.len() || n < 3 {
        return Err(SmartCalError::DegenerateGeometry(
            "At least 3 control points are required for Thin-Plate Spline".into(),
        ));
    }

    let dim = n + 3;
    let mut l = DMatrix::<f64>::zeros(dim, dim);
    let mut vx = DVector::<f64>::zeros(dim);
    let mut vy = DVector::<f64>::zeros(dim);

    for i in 0..n {
        for j in 0..n {
            if i == j {
                l[(i, j)] = smoothing;
            } else {
                let dx = src[i].x - src[j].x;
                let dy = src[i].y - src[j].y;
                l[(i, j)] = tps_kernel(dx * dx + dy * dy);
            }
        }
        l[(i, n)] = 1.0;
        l[(i, n + 1)] = src[i].x;
        l[(i, n + 2)] = src[i].y;

        l[(n, i)] = 1.0;
        l[(n + 1, i)] = src[i].x;
        l[(n + 2, i)] = src[i].y;

        vx[i] = target[i].x;
        vy[i] = target[i].y;
    }

    let svd = l.svd(true, true);
    let sol_x = svd
        .solve(&vx, 1e-10)
        .map_err(|e| SmartCalError::DegenerateGeometry(format!("TPS X solve failed: {e}")))?;
    let sol_y = svd
        .solve(&vy, 1e-10)
        .map_err(|e| SmartCalError::DegenerateGeometry(format!("TPS Y solve failed: {e}")))?;

    Ok(TpsModelData {
        control_points: src.iter().map(|p| [p.x, p.y]).collect(),
        weights_x: sol_x.as_slice()[..n].to_vec(),
        weights_y: sol_y.as_slice()[..n].to_vec(),
        affine_x: [sol_x[n], sol_x[n + 1], sol_x[n + 2]],
        affine_y: [sol_y[n], sol_y[n + 1], sol_y[n + 2]],
    })
}

pub fn eval_tps(model: &TpsModelData, p: Point2D) -> Point2D {
    let mut rx = model.affine_x[0] + model.affine_x[1] * p.x + model.affine_x[2] * p.y;
    let mut ry = model.affine_y[0] + model.affine_y[1] * p.x + model.affine_y[2] * p.y;

    for (i, cp) in model.control_points.iter().enumerate() {
        let dx = p.x - cp[0];
        let dy = p.y - cp[1];
        let k = tps_kernel(dx * dx + dy * dy);
        rx += model.weights_x[i] * k;
        ry += model.weights_y[i] * k;
    }
    Point2D::new(rx, ry)
}
