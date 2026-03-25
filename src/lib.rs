//! QPS-CORE — High-Performance Quantum Positioning Core Engine
//!
//! # Pipeline
//! ```text
//! Kafka → Validate → Fuse → Kalman Filter → Anomaly → Position → gRPC
//! ```

pub mod anomaly;
pub mod api;
pub mod compute;
pub mod error;
pub mod filtering;
pub mod fusion;
pub mod ingestion;
pub mod models;
pub mod storage;
pub mod validation;
