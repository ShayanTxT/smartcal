use nalgebra::{DMatrix, DVector};
use smartcal_core::{Point2D, Result, SmartCalError};

fn basis_terms(p: Point2D, degree: usize) -> Vec<f64> {
    let x = p.x;
    let y = p.y;
    match degree {
        1 => vec![1.0, x, y],
        2 => vec![1.0, x, y, x * x, x * y, y * y],
        _ => vec![
            1.0,
            x,
            y,
            x * x,
            x * y,
            y * y,
            x * x * x,
            x * x * y,
            x * y * y,
            y * y * y,
        ],
    }
}

pub fn required_points_for_degree(degree: usize) -> usize {
    match degree {
        1 => 3,
        2 => 6,
        _ => 10,
    }
}

/// Fits a regularized bivariate polynomial mapping normalized raw coordinates -> screen pixels.
pub fn fit_polynomial(
    src: &[Point2D],
    dst: &[Point2D],
    degree: usize,
    ridge: f64,
) -> Result<(usize, Vec<f64>, Vec<f64>)> {
    let deg = degree.clamp(1, 3);
    let min_pts = required_points_for_degree(deg);
    if src.len() != dst.len() || src.len() < min_pts {
        return Err(SmartCalError::DegenerateGeometry(format!(
            "Degree-{deg} polynomial requires at least {min_pts} points, got {}",
            src.len()
        )));
    }

    let n = src.len();
    let cols = min_pts;
    let mut a = DMatrix::<f64>::zeros(n, cols);
    let mut bx = DVector::<f64>::zeros(n);
    let mut by = DVector::<f64>::zeros(n);

    for i in 0..n {
        let terms = basis_terms(src[i], deg);
        for (c, val) in terms.into_iter().enumerate() {
            a[(i, c)] = val;
        }
        bx[i] = dst[i].x;
        by[i] = dst[i].y;
    }

    let at = a.transpose();
    let mut ata = &at * &a;
    // Do not regularize constant/linear terms as aggressively; small ridge prevents singular Vandermonde.
    for c in 0..cols {
        let weight = if c < 3 { ridge * 0.01 } else { ridge };
        ata[(c, c)] += weight;
    }

    let rhs_x = &at * &bx;
    let rhs_y = &at * &by;

    let lu = ata.lu();
    let cx = lu.solve(&rhs_x).ok_or_else(|| {
        SmartCalError::DegenerateGeometry("Polynomial X normal matrix is singular".into())
    })?;
    let cy = lu.solve(&rhs_y).ok_or_else(|| {
        SmartCalError::DegenerateGeometry("Polynomial Y normal matrix is singular".into())
    })?;

    Ok((deg, cx.as_slice().to_vec(), cy.as_slice().to_vec()))
}

pub fn eval_polynomial(
    degree: usize,
    coeff_x: &[f64],
    coeff_y: &[f64],
    p: Point2D,
) -> Option<Point2D> {
    let terms = basis_terms(p, degree);
    if coeff_x.len() != terms.len() || coeff_y.len() != terms.len() {
        return None;
    }
    let x = terms.iter().zip(coeff_x.iter()).map(|(t, c)| t * c).sum();
    let y = terms.iter().zip(coeff_y.iter()).map(|(t, c)| t * c).sum();
    Some(Point2D::new(x, y))
}
