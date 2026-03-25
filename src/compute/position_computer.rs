use nalgebra::Vector6;

use crate::{
    compute::coordinate_transform::ecef_to_geodetic,
    models::GeoPosition,
};

/// Converts a Kalman-filter state vector into a [`GeoPosition`].
///
/// The state vector is [x, y, z, vx, vy, vz] in ECEF metres / m·s⁻¹.
pub struct PositionComputer;

impl PositionComputer {
    pub fn compute(state: Vector6<f64>, timestamp: i64, anomaly: bool) -> GeoPosition {
        let (lat, lon, alt) = ecef_to_geodetic(state[0], state[1], state[2]);

        // Accuracy estimate: ±2 m baseline (could be improved using covariance)
        let accuracy = 2.0_f64;

        GeoPosition {
            latitude:  lat,
            longitude: lon,
            altitude:  alt,
            velocity:  [state[3], state[4], state[5]],
            timestamp,
            accuracy,
            anomaly,
        }
    }
}
