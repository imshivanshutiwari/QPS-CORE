use nalgebra::Vector6;

use crate::filtering::kalman_filter::KalmanFilter;

/// Advance the Kalman filter state by one prediction step.
///
/// In the pipeline this is called once per 100 Hz cycle (dt = 0.01 s) before
/// any measurement update arrives.
#[inline]
pub fn predict_next(kf: &mut KalmanFilter, dt: f64) {
    kf.predict(dt);
}

/// Return the predicted next state without mutating the filter.
pub fn peek_prediction(kf: &KalmanFilter, dt: f64) -> Vector6<f64> {
    let mut tmp = Vector6::zeros();
    for i in 0..6 {
        tmp[i] = kf.state[i];
    }
    // Apply constant-velocity model manually
    tmp[0] += kf.state[3] * dt;
    tmp[1] += kf.state[4] * dt;
    tmp[2] += kf.state[5] * dt;
    tmp
}
