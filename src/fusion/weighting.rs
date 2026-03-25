use crate::models::SensorReading;

/// Compute an adaptive weight for a reading based on its quality score.
///
/// Returns a value in (0, 1] using a quadratic boost for high-quality sensors.
#[inline]
pub fn compute_weight(reading: &SensorReading) -> f64 {
    // Quadratic weighting: emphasises sensors with quality > 0.9
    reading.quality * reading.quality
}
