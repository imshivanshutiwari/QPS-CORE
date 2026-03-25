use nalgebra::Matrix6;

/// Enforce positive semi-definiteness on a covariance matrix by symmetrising
/// it and clamping diagonal elements to a minimum value.
///
/// This guards against numerical drift that can make the matrix indefinite
/// after many update cycles.
pub fn enforce_psd(p: &mut Matrix6<f64>, min_diag: f64) {
    // Symmetrise: P ← (P + Pᵀ) / 2
    let sym = (*p + p.transpose()) * 0.5;
    *p = sym;

    // Clamp diagonal
    for i in 0..6 {
        if p[(i, i)] < min_diag {
            p[(i, i)] = min_diag;
        }
    }
}

/// Return the trace of the covariance matrix (sum of diagonal variances).
/// Useful as a scalar measure of overall uncertainty.
pub fn trace(p: &Matrix6<f64>) -> f64 {
    (0..6).map(|i| p[(i, i)]).sum()
}
