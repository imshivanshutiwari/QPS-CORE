use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::{error::StorageError, models::{GeoPosition, SensorReading}};

/// PostgreSQL persistence layer.  Stores raw readings and derived positions
/// for later analysis / audit.
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    /// Connect and run any pending migrations.
    pub async fn connect(database_url: &str) -> Result<Self, StorageError> {
        let pool = PgPool::connect(database_url).await?;
        // Run embedded migrations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sensor_readings (
                id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                sensor_id   TEXT        NOT NULL,
                timestamp   BIGINT      NOT NULL,
                mag_x       DOUBLE PRECISION NOT NULL,
                mag_y       DOUBLE PRECISION NOT NULL,
                mag_z       DOUBLE PRECISION NOT NULL,
                quality     DOUBLE PRECISION NOT NULL
            );
            CREATE TABLE IF NOT EXISTS positions (
                id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                reading_id  UUID        NOT NULL,
                latitude    DOUBLE PRECISION NOT NULL,
                longitude   DOUBLE PRECISION NOT NULL,
                altitude    DOUBLE PRECISION NOT NULL,
                vel_x       DOUBLE PRECISION NOT NULL,
                vel_y       DOUBLE PRECISION NOT NULL,
                vel_z       DOUBLE PRECISION NOT NULL,
                accuracy    DOUBLE PRECISION NOT NULL,
                anomaly     BOOLEAN      NOT NULL
            );
            "#,
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }

    /// Persist a raw sensor reading.  Returns the generated UUID.
    pub async fn insert_reading(&self, r: &SensorReading) -> Result<Uuid, StorageError> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO sensor_readings
              (id, created_at, sensor_id, timestamp, mag_x, mag_y, mag_z, quality)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(Utc::now())
        .bind(&r.sensor_id)
        .bind(r.timestamp)
        .bind(r.magnetic_field[0])
        .bind(r.magnetic_field[1])
        .bind(r.magnetic_field[2])
        .bind(r.quality)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    /// Persist a computed position linked to its source reading.
    pub async fn insert_position(
        &self,
        reading_id: Uuid,
        p: &GeoPosition,
    ) -> Result<Uuid, StorageError> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO positions
              (id, created_at, reading_id, latitude, longitude, altitude,
               vel_x, vel_y, vel_z, accuracy, anomaly)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(id)
        .bind(Utc::now())
        .bind(reading_id)
        .bind(p.latitude)
        .bind(p.longitude)
        .bind(p.altitude)
        .bind(p.velocity[0])
        .bind(p.velocity[1])
        .bind(p.velocity[2])
        .bind(p.accuracy)
        .bind(p.anomaly)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }
}
