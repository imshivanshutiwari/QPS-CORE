use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Raw reading arriving from a sensor (via Kafka or gRPC).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub sensor_id:      String,
    /// Unix nanoseconds
    pub timestamp:      i64,
    /// Magnetic field vector [x, y, z] in microTesla (µT)
    pub magnetic_field: [f64; 3],
    /// Quality score 0.0–1.0
    pub quality:        f64,
    /// Optional coarse GPS hint (degrees / metres)
    pub latitude:       Option<f64>,
    pub longitude:      Option<f64>,
    pub altitude:       Option<f64>,
}

/// WGS-84 geographic position + velocity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPosition {
    pub latitude:  f64,
    pub longitude: f64,
    pub altitude:  f64,
    /// Velocity in m/s [x, y, z] (ECEF frame)
    pub velocity:  [f64; 3],
    pub timestamp: i64,
    pub accuracy:  f64,
    pub anomaly:   bool,
}

/// Persisted pipeline event stored in PostgreSQL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEvent {
    pub id:         Uuid,
    pub created_at: DateTime<Utc>,
    pub reading:    SensorReading,
    pub position:   GeoPosition,
}
