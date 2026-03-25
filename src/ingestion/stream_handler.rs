use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{
    ingestion::kafka_consumer::KafkaSensorConsumer,
    models::SensorReading,
    validation::data_validator::DataValidator,
};

/// Spawns a background task that continuously polls Kafka and forwards
/// validated [`SensorReading`]s through the provided `mpsc` sender.
pub struct StreamHandler {
    consumer: Arc<KafkaSensorConsumer>,
}

impl StreamHandler {
    pub fn new(consumer: KafkaSensorConsumer) -> Self {
        Self {
            consumer: Arc::new(consumer),
        }
    }

    /// Start the ingestion loop. Returns a receiver that yields valid readings.
    /// The loop runs until the sender is dropped.
    pub fn start(&self) -> mpsc::Receiver<SensorReading> {
        let (tx, rx) = mpsc::channel::<SensorReading>(1024);
        let consumer = Arc::clone(&self.consumer);

        tokio::spawn(async move {
            info!("stream_handler: ingestion loop started on topic '{}'", consumer.topic);
            loop {
                if let Some(reading) = consumer.poll().await {
                    match DataValidator::validate(&reading) {
                        Ok(()) => {
                            if tx.send(reading).await.is_err() {
                                info!("stream_handler: downstream receiver dropped – stopping");
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("stream_handler: validation rejected reading: {e}");
                        }
                    }
                }
            }
        });

        rx
    }
}
