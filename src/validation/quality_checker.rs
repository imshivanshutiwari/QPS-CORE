use crate::models::SensorReading;

/// Per-reading quality metrics derived during the validation phase.
#[derive(Debug, Clone)]
pub struct QualityMetrics {
    /// Normalised signal strength 0.0–1.0
    pub signal_strength: f64,
    /// Estimated noise level in µT
    pub noise_ut: f64,
    /// True if the reading passed all quality checks
    pub passed: bool,
}

/// Compute quality metrics for a single reading.
pub fn assess_quality(reading: &SensorReading) -> QualityMetrics {
    let mag_magnitude = (reading.magnetic_field[0].powi(2)
        + reading.magnetic_field[1].powi(2)
        + reading.magnetic_field[2].powi(2))
    .sqrt();

    // Typical Earth field magnitude: 25–65 µT
    let expected_min = 20.0_f64;
    let expected_max = 70.0_f64;

    let in_range = mag_magnitude >= expected_min && mag_magnitude <= expected_max;
    let noise_ut = (1.0 - reading.quality) * 0.05; // rough proxy

    QualityMetrics {
        signal_strength: reading.quality,
        noise_ut,
        passed: in_range && reading.quality >= 0.8,
    }
}
