use nalgebra::Vector3;

use crate::{error::PipelineError, models::SensorReading};
use super::weighting::compute_weight;

/// Multi-sensor fusion engine.  Uses quality-weighted averaging so that
/// high-confidence sensors contribute more to the fused output.
pub struct SensorFusionEngine;

impl SensorFusionEngine {
    /// Fuse a slice of validated sensor readings into a single magnetic
    /// field vector [x, y, z] in nanoTesla.
    ///
    /// Returns [`PipelineError::EmptyFusion`] if the slice is empty or all
    /// weights are effectively zero.
    pub fn fuse(readings: &[SensorReading]) -> Result<Vector3<f64>, PipelineError> {
        if readings.is_empty() {
            return Err(PipelineError::EmptyFusion);
        }

        let mut weighted_sum = Vector3::zeros();
        let mut total_weight = 0.0_f64;

        for r in readings {
            let w = compute_weight(r);
            let vec = Vector3::new(
                r.magnetic_field[0],
                r.magnetic_field[1],
                r.magnetic_field[2],
            );
            weighted_sum += vec * w;
            total_weight += w;
        }

        if total_weight < 1e-12 {
            return Err(PipelineError::EmptyFusion);
        }

        Ok(weighted_sum / total_weight)
    }
}
