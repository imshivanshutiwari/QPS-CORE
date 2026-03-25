use std::pin::Pin;

use futures::StreamExt;
use nalgebra::Vector3;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info, warn};

use crate::{
    anomaly::magnetic_anomaly::MagneticAnomalyDetector,
    compute::position_computer::PositionComputer,
    error::PipelineError,
    filtering::kalman_filter::KalmanFilter,
    fusion::sensor_fusion::SensorFusionEngine,
    models::SensorReading,
    validation::data_validator::DataValidator,
};

// Import the generated protobuf types
pub mod proto {
    tonic::include_proto!("qps");
}

use proto::{qps_server::Qps, PositionOutput, SensorInput};

/// The main gRPC service implementation.
pub struct QpsService;

#[tonic::async_trait]
impl Qps for QpsService {
    type StreamPositionStream =
        Pin<Box<dyn futures::Stream<Item = Result<PositionOutput, Status>> + Send + 'static>>;

    async fn stream_position(
        &self,
        request: Request<Streaming<SensorInput>>,
    ) -> Result<Response<Self::StreamPositionStream>, Status> {
        info!("grpc: new StreamPosition client connected");

        let mut inbound = request.into_inner();
        let (tx, rx) = mpsc::channel::<Result<PositionOutput, Status>>(256);

        tokio::spawn(async move {
            // Initialise a Kalman filter per-stream.
            // State = [x, y, z, vx, vy, vz] in ECEF metres.
            let initial = nalgebra::Vector6::new(
                4_209_000.0, // approximate ECEF X for 0°N 0°E
                0.0,
                4_640_000.0,
                0.0,
                0.0,
                0.0,
            );
            let mut kf = KalmanFilter::new(initial, 1000.0, 10.0);

            let mut last_ts: Option<i64> = None;
            const DT_DEFAULT: f64 = 0.01; // 100 Hz fallback

            while let Some(item) = inbound.next().await {
                let input = match item {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("grpc: stream error: {e}");
                        break;
                    }
                };

                // Convert proto SensorInput → domain SensorReading
                let reading = SensorReading {
                    sensor_id:      input.sensor_id.clone(),
                    timestamp:      input.timestamp,
                    magnetic_field: [input.mag_x, input.mag_y, input.mag_z],
                    quality:        input.quality,
                    latitude:       if input.latitude != 0.0 { Some(input.latitude) } else { None },
                    longitude:      if input.longitude != 0.0 { Some(input.longitude) } else { None },
                    altitude:       if input.altitude != 0.0 { Some(input.altitude) } else { None },
                };

                // Validate
                if let Err(e) = DataValidator::validate(&reading) {
                    warn!("grpc: validation failed for {}: {e}", reading.sensor_id);
                    continue;
                }

                // dt from timestamps (nanoseconds → seconds)
                let dt = match last_ts {
                    Some(prev) => {
                        let delta_ns = input.timestamp.saturating_sub(prev);
                        if delta_ns > 0 { delta_ns as f64 * 1e-9 } else { DT_DEFAULT }
                    }
                    None => DT_DEFAULT,
                };
                last_ts = Some(input.timestamp);

                // Predict
                kf.predict(dt);

                // Fuse (single reading → trivial)
                let fused = match SensorFusionEngine::fuse(&[reading.clone()]) {
                    Ok(v) => v,
                    Err(PipelineError::EmptyFusion) => continue,
                    Err(e) => {
                        error!("grpc: fusion error: {e}");
                        continue;
                    }
                };

                // Update Kalman with fused measurement
                // Map magnetic field → position delta using GPS hint if present
                let meas = if let (Some(lat), Some(lon), Some(alt)) =
                    (reading.latitude, reading.longitude, reading.altitude)
                {
                    crate::compute::coordinate_transform::geodetic_to_ecef(lat, lon, alt)
                } else {
                    // Fallback: use fused magnetic vector scaled to position space
                    fused
                };

                if let Err(e) = kf.update(meas) {
                    error!("grpc: kalman update error: {e}");
                    continue;
                }

                // Anomaly detection
                let expected = Vector3::new(20.0, 2.0, -43.0); // µT
                let anomaly = MagneticAnomalyDetector::detect(fused, expected, 5.0);

                // Compute position
                let pos = PositionComputer::compute(kf.state, input.timestamp, anomaly);

                let output = PositionOutput {
                    request_id: input.sensor_id.clone(),
                    timestamp:  pos.timestamp,
                    latitude:   pos.latitude,
                    longitude:  pos.longitude,
                    altitude:   pos.altitude,
                    vel_x:      pos.velocity[0],
                    vel_y:      pos.velocity[1],
                    vel_z:      pos.velocity[2],
                    anomaly:    pos.anomaly,
                    accuracy:   pos.accuracy,
                };

                if tx.send(Ok(output)).await.is_err() {
                    info!("grpc: client disconnected");
                    break;
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }
}
