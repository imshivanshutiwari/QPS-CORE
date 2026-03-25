use std::time::Duration;

use nalgebra::Vector3;
use tokio::{sync::mpsc, time};
use tracing::{error, info, warn};

use qps_core::{
    anomaly::magnetic_anomaly::MagneticAnomalyDetector,
    api::handlers::run_server,
    compute::position_computer::PositionComputer,
    error::PipelineError,
    filtering::kalman_filter::KalmanFilter,
    fusion::sensor_fusion::SensorFusionEngine,
    ingestion::{
        kafka_consumer::KafkaSensorConsumer, stream_handler::StreamHandler,
    },
    models::SensorReading,
};

/// 100 Hz pipeline tick interval
const PIPELINE_DT_SECS: f64 = 0.01;
const PIPELINE_TICK: Duration = Duration::from_millis(10);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env
    dotenvy::dotenv().ok();

    // Structured logging
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("qps_core=info".parse().unwrap()),
        )
        .init();

    let brokers   = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".into());
    let topic     = std::env::var("KAFKA_TOPIC").unwrap_or_else(|_| "sensor-readings".into());
    let grpc_addr = std::env::var("GRPC_ADDR").unwrap_or_else(|_| "[::]:50051".into());

    info!("QPS-CORE starting up");
    info!("kafka brokers: {brokers}, topic: {topic}");
    info!("gRPC address: {grpc_addr}");

    // Start gRPC server in background
    let grpc_addr_clone = grpc_addr.clone();
    tokio::spawn(async move {
        if let Err(e) = run_server(&grpc_addr_clone).await {
            error!("gRPC server error: {e}");
        }
    });

    // Start Kafka ingestion
    let consumer = KafkaSensorConsumer::new(&brokers, &topic, "qps-core-group").await;
    let handler  = StreamHandler::new(consumer);
    let mut rx   = handler.start();

    // Kalman filter state (ECEF metres)
    let initial = nalgebra::Vector6::new(4_209_000.0, 0.0, 4_640_000.0, 0.0, 0.0, 0.0);
    let mut kf   = KalmanFilter::new(initial, 1000.0, 10.0);

    // Reference magnetic field for anomaly detection
    let expected_field = Vector3::new(20.0_f64, 2.0, -43.0); // µT

    let mut interval = time::interval(PIPELINE_TICK);
    // Buffer to collect readings within the current tick
    let mut batch: Vec<SensorReading> = Vec::with_capacity(32);

    loop {
        // Drain all available readings into the batch
        loop {
            match rx.try_recv() {
                Ok(r) => batch.push(r),
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    error!("ingestion channel closed – shutting down pipeline");
                    return Ok(());
                }
            }
        }

        // Predict regardless of new measurements
        kf.predict(PIPELINE_DT_SECS);

        // Fuse and update if we have data
        if !batch.is_empty() {
            match SensorFusionEngine::fuse(&batch) {
                Ok(fused) => {
                    // Use GPS hint from first reading if available
                    let meas = batch.first().and_then(|r| {
                        if let (Some(lat), Some(lon), Some(alt)) =
                            (r.latitude, r.longitude, r.altitude)
                        {
                            Some(qps_core::compute::coordinate_transform::geodetic_to_ecef(
                                lat, lon, alt,
                            ))
                        } else {
                            None
                        }
                    }).unwrap_or(fused);

                    if let Err(e) = kf.update(meas) {
                        warn!("kalman update failed: {e}");
                    }

                    // Anomaly detection
                    let anomaly =
                        MagneticAnomalyDetector::detect(fused, expected_field, 5.0);

                    // Compute position from current state
                    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
                    let pos = PositionComputer::compute(kf.state, now_ns, anomaly);

                    tracing::debug!(
                        lat = pos.latitude,
                        lon = pos.longitude,
                        alt = pos.altitude,
                        anomaly = pos.anomaly,
                        "position update"
                    );
                }
                Err(PipelineError::EmptyFusion) => {}
                Err(e) => warn!("fusion error: {e}"),
            }
            batch.clear();
        }

        interval.tick().await;
    }
}
