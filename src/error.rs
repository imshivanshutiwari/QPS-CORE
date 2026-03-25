use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("quality score {0:.2} is below threshold 0.80")]
    LowQuality(f64),
    #[error("magnetic field component {0:.2} µT is out of valid range ±1000 µT")]
    OutOfBounds(f64),
    #[error("timestamp is zero or negative")]
    InvalidTimestamp,
}

#[derive(Debug, Error)]
pub enum KafkaError {
    #[error("kafka receive error: {0}")]
    Receive(String),
    #[error("json parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("postgres error: {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("validation: {0}")]
    Validation(#[from] ValidationError),
    #[error("kafka: {0}")]
    Kafka(#[from] KafkaError),
    #[error("storage: {0}")]
    Storage(#[from] StorageError),
    #[error("no valid readings to fuse")]
    EmptyFusion,
    #[error("matrix inversion failed")]
    MatrixInversion,
    #[error("gRPC error: {0}")]
    Grpc(String),
}
