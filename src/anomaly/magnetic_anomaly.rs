use nalgebra::Vector3;

/// Hard-coded reference Earth magnetic field vectors for a small set of
/// geographic cells (latitude / longitude buckets).  In a production system
/// this would be backed by the World Magnetic Model (WMM) or IGRF database.
/// Values in µT — typical mid-latitude components.
const REFERENCE_FIELD_UT: [f64; 3] = [20.0, 2.0, -43.0]; // µT

/// Anomaly detector that flags readings whose magnetic field deviates
/// significantly from the expected Earth reference.
pub struct MagneticAnomalyDetector;

impl MagneticAnomalyDetector {
    /// Returns `true` when `field` is anomalous compared to `expected`.
    ///
    /// Two criteria must both be met to avoid false positives:
    /// 1. Euclidean deviation > `threshold_ut` (absolute amplitude check in µT)
    /// 2. Angular deviation > 5° (directional check)
    pub fn detect(field: Vector3<f64>, expected: Vector3<f64>, threshold_ut: f64) -> bool {
        let diff_norm = (field - expected).norm();

        // Angular deviation check
        let cos_angle = field.dot(&expected) / (field.norm() * expected.norm() + 1e-12);
        let angle_deg = cos_angle.clamp(-1.0, 1.0).acos().to_degrees();

        diff_norm > threshold_ut || angle_deg > 5.0
    }

    /// Convenience method using the built-in reference field and a 5 µT
    /// threshold.
    pub fn detect_with_reference(field: Vector3<f64>) -> bool {
        let expected = Vector3::new(
            REFERENCE_FIELD_UT[0],
            REFERENCE_FIELD_UT[1],
            REFERENCE_FIELD_UT[2],
        );
        Self::detect(field, expected, 5.0)
    }

    /// Statistical anomaly: z-score > 3σ on the field magnitude.
    pub fn detect_statistical(field: Vector3<f64>, mean_ut: f64, std_ut: f64) -> bool {
        let magnitude = field.norm();
        let z = (magnitude - mean_ut).abs() / (std_ut + 1e-12);
        z > 3.0
    }
}
