use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    message::Message,
    ClientConfig,
};
use tracing::{error, warn};

use crate::{error::KafkaError, models::SensorReading};

/// Async Kafka consumer that deserialises raw bytes into [`SensorReading`].
pub struct KafkaSensorConsumer {
    consumer: StreamConsumer,
    pub topic: String,
}

impl KafkaSensorConsumer {
    /// Connect to the given broker(s) and subscribe to `topic`.
    pub async fn new(brokers: &str, topic: &str, group_id: &str) -> Self {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "latest")
            .set("session.timeout.ms", "6000")
            .create()
            .expect("Kafka StreamConsumer creation failed");

        consumer
            .subscribe(&[topic])
            .expect("Kafka subscription failed");

        Self {
            consumer,
            topic: topic.to_string(),
        }
    }

    /// Poll for the next message.  Returns `None` on transient errors so
    /// the caller can keep running.
    pub async fn poll(&self) -> Option<SensorReading> {
        match self.consumer.recv().await {
            Ok(msg) => match Self::parse_message(&msg) {
                Ok(r) => Some(r),
                Err(e) => {
                    warn!("kafka parse error: {e}");
                    None
                }
            },
            Err(e) => {
                error!("kafka receive error: {e}");
                None
            }
        }
    }

    fn parse_message(
        msg: &rdkafka::message::BorrowedMessage<'_>,
    ) -> Result<SensorReading, KafkaError> {
        let payload = msg
            .payload()
            .ok_or_else(|| KafkaError::Receive("empty payload".into()))?;
        let reading: SensorReading = serde_json::from_slice(payload)?;
        Ok(reading)
    }
}
