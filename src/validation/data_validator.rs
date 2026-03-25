use crate::{error::ValidationError, models::SensorReading};

/// Stateless validator for incoming sensor readings.
pub struct DataValidator;

impl DataValidator {
    /// Validate a [`SensorReading`], returning a [`ValidationError`] if any
    /// constraint is violated.
    pub fn validate(reading: &SensorReading) -> Result<(), ValidationError> {
        // Timestamp must be positive
        if reading.timestamp <= 0 {
            return Err(ValidationError::InvalidTimestamp);
        }

        // Quality must meet minimum threshold
        if reading.quality < 0.8 {
            return Err(ValidationError::LowQuality(reading.quality));
        }

        // Magnetic field components must be in ±1000 nT
        for &v in &reading.magnetic_field {
            if v.abs() > 1000.0 {
                return Err(ValidationError::OutOfBounds(v));
            }
        }

        Ok(())
    }
}
