use redis::aio::ConnectionManager;
use redis::AsyncCommands;

use crate::{error::StorageError, models::GeoPosition};

/// Redis-backed position cache.  Stores the latest `GeoPosition` under a
/// fixed key so downstream clients get sub-millisecond read access.
pub struct RedisCache {
    cm: ConnectionManager,
}

impl RedisCache {
    const POSITION_KEY: &'static str = "qps:latest_position";
    /// TTL for the cached position (seconds)
    const TTL_SECS: u64 = 5;

    /// Connect to Redis at `url` (e.g. `redis://127.0.0.1/`).
    pub async fn connect(url: &str) -> Result<Self, StorageError> {
        let client = redis::Client::open(url)?;
        let cm = ConnectionManager::new(client).await?;
        Ok(Self { cm })
    }

    /// Serialise and cache the latest position with a TTL.
    pub async fn store_position(&mut self, pos: &GeoPosition) -> Result<(), StorageError> {
        let json = serde_json::to_string(pos)?;
        let _: () = self.cm
            .set_ex(Self::POSITION_KEY, json, Self::TTL_SECS)
            .await
            .map_err(StorageError::Redis)?;
        Ok(())
    }

    /// Retrieve the latest cached position, or `None` if the key has expired
    /// or does not exist.
    pub async fn get_position(&mut self) -> Result<Option<GeoPosition>, StorageError> {
        let raw: Option<String> = self.cm
            .get(Self::POSITION_KEY)
            .await
            .map_err(StorageError::Redis)?;

        match raw {
            None => Ok(None),
            Some(json) => {
                let pos: GeoPosition = serde_json::from_str(&json)?;
                Ok(Some(pos))
            }
        }
    }
}
