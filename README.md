<div align="center">

# ⚛️ QPS-CORE

### High-Performance Quantum Positioning System — Core Engine

[![Rust](https://img.shields.io/badge/language-Rust-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen)](#)
[![gRPC](https://img.shields.io/badge/API-gRPC-blueviolet?logo=grpc)](https://grpc.io/)
[![Kafka](https://img.shields.io/badge/ingestion-Kafka-black?logo=apachekafka)](https://kafka.apache.org/)
[![PostgreSQL](https://img.shields.io/badge/storage-PostgreSQL-316192?logo=postgresql&logoColor=white)](https://www.postgresql.org/)
[![Redis](https://img.shields.io/badge/cache-Redis-red?logo=redis&logoColor=white)](https://redis.io/)
[![Docker](https://img.shields.io/badge/container-Docker-2496ED?logo=docker&logoColor=white)](https://www.docker.com/)

> **QPS-CORE** is a production-grade, real-time positioning engine that fuses magnetic-field sensor data through a 6-DOF Kalman filter to compute accurate geographic positions — all at **100 Hz**, streamed via gRPC.

</div>

---

## 📑 Table of Contents

- [Overview](#-overview)
- [Architecture](#-architecture)
- [Features](#-features)
- [Tech Stack](#-tech-stack)
- [gRPC API](#-grpc-api)
- [Configuration](#-configuration)
- [Getting Started](#-getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
  - [Running Locally](#running-locally)
  - [Running with Docker](#running-with-docker)
- [Make Targets](#-make-targets)
- [Testing & Benchmarking](#-testing--benchmarking)
- [Project Structure](#-project-structure)
- [Internal Parameters](#-internal-parameters)
- [Contributing](#-contributing)
- [License](#-license)

---

## 🔭 Overview

**QPS-CORE** is the real-time core engine of the Quantum Positioning System. It ingests raw magnetic-field readings from distributed sensors via **Apache Kafka**, validates and fuses them, runs them through a **6-DOF Extended Kalman Filter**, detects magnetic anomalies, and outputs precise WGS-84 latitude/longitude/altitude coordinates — all over a **bidirectional gRPC stream** at up to 100 Hz.

The system is designed for:
- 🚀 **Ultra-low latency** — 10 ms pipeline ticks (100 Hz)
- 🏗️ **Production-grade reliability** — PostgreSQL persistence + Redis cache
- 🔬 **Scientific accuracy** — ECEF/WGS-84 coordinate transforms, tunable noise matrices
- 🧩 **Extensibility** — Modular Rust crate with clear module boundaries

---

## 🏛️ Architecture

```
┌───────────────────────────────────────────────────────────────────────────┐
│                          QPS-CORE Pipeline (100 Hz)                       │
│                                                                           │
│  ┌──────────┐   ┌───────────┐   ┌─────────────┐   ┌────────────────────┐ │
│  │  Kafka   │──▶│  Validate │──▶│ Sensor Fuse │──▶│  Kalman Filter     │ │
│  │ Consumer │   │ & Quality │   │  (weighted) │   │  (6-DOF, ECEF)     │ │
│  └──────────┘   └───────────┘   └─────────────┘   └────────┬───────────┘ │
│                                                            │               │
│  ┌──────────┐   ┌───────────┐   ┌─────────────┐           │               │
│  │  gRPC    │◀──│ Coordinate│◀──│  Anomaly    │◀──────────┘               │
│  │  Stream  │   │ Transform │   │  Detection  │                           │
│  └──────────┘   │ ECEF→WGS84│   └─────────────┘                           │
│                 └───────────┘                                             │
│                       │                                                   │
│              ┌────────▼────────┐                                          │
│              │   PostgreSQL    │  ←  Persistent storage                   │
│              │   Redis Cache   │  ←  Fast caching layer                   │
│              └─────────────────┘                                          │
└───────────────────────────────────────────────────────────────────────────┘
```

**Data flow:**

1. **Ingestion** — Kafka consumer polls magnetic-field readings (async, 10 ms ticks)
2. **Validation** — Quality threshold (≥ 0.80), field bounds (±1000 µT), timestamp checks
3. **Fusion** — Quality-weighted vector averaging across multiple sensors
4. **Kalman Filtering** — Predict → Update cycle in ECEF Cartesian space
5. **Anomaly Detection** — Reference-field comparison, angular deviation (> 5°), Z-score outliers
6. **Coordinate Transform** — ECEF → WGS-84 (latitude, longitude, altitude)
7. **Storage** — Raw readings and computed positions saved to PostgreSQL
8. **API** — Results streamed back to clients over gRPC

---

## ✨ Features

| Feature | Description |
|---|---|
| **100 Hz Real-Time Pipeline** | 10 ms tick-based processing loop driven by Tokio async runtime |
| **6-DOF Kalman Filter** | Tracks 3D position + 3D velocity in ECEF coordinates |
| **Multi-Sensor Fusion** | Quality-weighted averaging of concurrent sensor readings |
| **Magnetic Anomaly Detection** | Reference-field deviation, angular checks, Z-score statistics |
| **ECEF ↔ WGS-84 Transforms** | Precise geodetic coordinate conversion |
| **Bidirectional gRPC Streaming** | Per-client Kalman filter instance with real-time output |
| **Kafka Ingestion** | Scalable, fault-tolerant sensor data stream intake |
| **PostgreSQL Persistence** | Auto-schema creation; stores raw readings and computed positions |
| **Redis Cache Layer** | Optional fast cache for extensibility |
| **Docker Support** | Multi-stage build for a lean production image |
| **Criterion Benchmarks** | Performance benchmarks for Kalman filter hot path |

---

## 🛠️ Tech Stack

| Layer | Technology | Version |
|---|---|---|
| Language | **Rust** | 2021 edition |
| Async Runtime | **Tokio** | full features |
| Message Queue | **Apache Kafka** (rdkafka) | 0.36 |
| RPC Framework | **gRPC / Tonic** | 0.12 |
| IDL | **Protocol Buffers** | via prost |
| Database | **PostgreSQL** (sqlx) | 0.7 |
| Cache | **Redis** | 0.25 |
| Linear Algebra | **nalgebra** | 0.33 |
| Serialization | **serde / serde_json** | latest |
| Logging | **tracing** + JSON subscriber | latest |
| Error Handling | **thiserror** + **anyhow** | latest |
| Benchmarks | **Criterion** | 0.5 |
| Containerization | **Docker** | multi-stage |

---

## 📡 gRPC API

The service is defined in [`proto/qps.proto`](proto/qps.proto).

### Service

```protobuf
service Qps {
  rpc StreamPosition(stream SensorInput) returns (stream PositionOutput);
}
```

### `SensorInput` — Request Message

| Field | Type | Description |
|---|---|---|
| `sensor_id` | `string` | Unique sensor identifier |
| `timestamp` | `int64` | Unix time in **nanoseconds** |
| `mag_x` | `double` | Magnetic field X component [µT] |
| `mag_y` | `double` | Magnetic field Y component [µT] |
| `mag_z` | `double` | Magnetic field Z component [µT] |
| `quality` | `double` | Quality score [0.0 – 1.0] |
| `latitude` | `double` | GPS hint — latitude [°] |
| `longitude` | `double` | GPS hint — longitude [°] |
| `altitude` | `double` | GPS hint — altitude [m] |

### `PositionOutput` — Response Message

| Field | Type | Description |
|---|---|---|
| `request_id` | `string` | Echo of `sensor_id` |
| `timestamp` | `int64` | Unix time in **nanoseconds** |
| `latitude` | `double` | Computed latitude [°] |
| `longitude` | `double` | Computed longitude [°] |
| `altitude` | `double` | Computed altitude [m] |
| `vel_x` | `double` | Velocity X — ECEF frame [m/s] |
| `vel_y` | `double` | Velocity Y — ECEF frame [m/s] |
| `vel_z` | `double` | Velocity Z — ECEF frame [m/s] |
| `anomaly` | `bool` | `true` if a magnetic anomaly was detected |
| `accuracy` | `double` | Estimated position accuracy [m] |

### Default Server Address

```
[::]:50051   # IPv6 dual-stack, configurable via GRPC_ADDR
```

---

## ⚙️ Configuration

All configuration is read from environment variables (or a `.env` file at project root).

| Variable | Default | Description |
|---|---|---|
| `KAFKA_BROKERS` | `localhost:9092` | Comma-separated list of Kafka broker addresses |
| `KAFKA_TOPIC` | `sensor-readings` | Kafka topic to subscribe to |
| `KAFKA_GROUP_ID` | `qps-core-group` | Kafka consumer group ID |
| `REDIS_URL` | `redis://127.0.0.1:6379/` | Redis connection URL |
| `DATABASE_URL` | `postgres://qps:qps@localhost:5432/qps_core` | PostgreSQL connection string |
| `GRPC_ADDR` | `[::]:50051` | gRPC server bind address |
| `RUST_LOG` | `qps_core=info` | Log level filter (tracing format) |

**Example `.env` file:**

```dotenv
KAFKA_BROKERS=localhost:9092
KAFKA_TOPIC=sensor-readings
KAFKA_GROUP_ID=qps-core-group
REDIS_URL=redis://127.0.0.1:6379/
DATABASE_URL=postgres://qps:qps@localhost:5432/qps_core
GRPC_ADDR=[::]:50051
RUST_LOG=qps_core=info
```

---

## 🚀 Getting Started

### Prerequisites

| Requirement | Minimum Version | Notes |
|---|---|---|
| [Rust](https://rustup.rs/) | 1.77+ | Install via `rustup` |
| [Apache Kafka](https://kafka.apache.org/downloads) | 2.0+ | Required for sensor ingestion |
| [PostgreSQL](https://www.postgresql.org/download/) | 12+ | Required for persistence |
| [Redis](https://redis.io/download/) | 5+ | Required for cache layer |
| [Protocol Buffers compiler](https://grpc.io/docs/protoc-installation/) (`protoc`) | 3.x | Required to compile `.proto` files |
| [Docker](https://docs.docker.com/get-docker/) *(optional)* | 20+ | For containerized deployment |

### Installation

```bash
# 1. Clone the repository
git clone https://github.com/imshivanshutiwari/QPS-CORE.git
cd QPS-CORE

# 2. Copy and edit the environment file
cp .env .env.local
# Edit .env.local with your service addresses

# 3. Build the release binary
cargo build --release
# or
make build
```

### Running Locally

```bash
# Ensure Kafka, PostgreSQL, and Redis are running, then:
cargo run --release
# or
make run
```

The server will start, auto-create the required PostgreSQL tables, and begin listening for gRPC connections on `[::]:50051` and Kafka messages on the configured topic.

### Running with Docker

```bash
# Build the Docker image (multi-stage, optimized)
docker build -t qps-core:latest .
# or
make docker

# Run the container, passing in your environment file
docker run --rm \
  --env-file .env \
  -p 50051:50051 \
  qps-core:latest
```

---

## 🔧 Make Targets

| Target | Command | Description |
|---|---|---|
| `build` | `cargo build --release` | Compile release binary |
| `run` | `cargo run --release` | Build and run the service |
| `test` | `cargo test -- --test-threads=4` | Run all unit + integration tests |
| `bench` | `cargo bench` | Run Criterion performance benchmarks |
| `fmt` | `cargo fmt` | Format source code with rustfmt |
| `lint` | `cargo clippy` | Static analysis with Clippy |
| `docker` | `docker build -t qps-core:latest .` | Build Docker image |
| `clean` | `cargo clean` | Remove build artifacts |

```bash
# Examples
make build
make test
make bench
make lint
```

---

## 🧪 Testing & Benchmarking

### Unit & Integration Tests

```bash
make test
# or: cargo test -- --test-threads=4
```

- **`tests/unit_tests.rs`** — Tests individual components (Kalman filter, anomaly detection, coordinate transforms, validation, fusion, etc.)
- **`tests/integration_tests.rs`** — End-to-end pipeline tests covering the full data flow

### Benchmarks

```bash
make bench
# or: cargo bench
```

- **`benches/kalman_bench.rs`** — Criterion benchmarks for the Kalman filter predict/update cycle, the most performance-critical code path in the pipeline.

---

## 📂 Project Structure

```
QPS-CORE/
├── Cargo.toml                  # Project manifest & dependencies
├── Cargo.lock                  # Locked dependency versions
├── Makefile                    # Build / run / test shortcuts
├── Dockerfile                  # Multi-stage Docker build
├── build.rs                    # Proto compilation (tonic-build)
├── .env                        # Default environment variables
│
├── proto/
│   └── qps.proto               # gRPC service & message definitions
│
├── src/
│   ├── main.rs                 # Binary entry point — 100 Hz pipeline loop
│   ├── lib.rs                  # Library crate entry (re-exports)
│   ├── models.rs               # Shared data models (SensorReading, Position…)
│   ├── error.rs                # Custom error types (QpsError)
│   │
│   ├── ingestion/              # Kafka consumer & stream handler
│   ├── validation/             # Input validation & quality assessment
│   ├── fusion/                 # Multi-sensor fusion & quality weighting
│   ├── filtering/              # Kalman filter, state predictor, covariance
│   ├── anomaly/                # Magnetic anomaly detection & map matching
│   ├── compute/                # ECEF ↔ Geodetic transforms & position output
│   ├── storage/                # PostgreSQL persistence & Redis cache
│   └── api/                    # gRPC server & handlers
│
├── tests/
│   ├── unit_tests.rs           # Component-level tests
│   └── integration_tests.rs    # Full pipeline tests
│
└── benches/
    └── kalman_bench.rs         # Kalman filter performance benchmarks
```

---

## 🔬 Internal Parameters

These constants are compiled in and can be tuned for your deployment environment:

| Parameter | Value | Location | Purpose |
|---|---|---|---|
| Pipeline tick | **10 ms** (100 Hz) | `main.rs` | Data collection interval |
| Initial ECEF position | **(4,209,000 · 0 · 4,640,000) m** | `main.rs` | Kalman filter seed (mid-latitude) |
| Initial position noise | **1,000 m** | `main.rs` | Covariance P₀ — position |
| Initial velocity noise | **10 m/s** | `main.rs` | Covariance P₀ — velocity |
| Measurement noise R | **2 m²** | `kalman_filter.rs` | Position uncertainty |
| Process noise Q | **0.1 – 1.0** | `kalman_filter.rs` | Motion model uncertainty |
| Reference magnetic field | **[20.0, 2.0, −43.0] µT** | multiple | Earth field reference (mid-latitude) |
| Anomaly field threshold | **5.0 µT** | `main.rs` | Deviation to flag anomaly |
| Angular anomaly threshold | **> 5°** | `magnetic_anomaly.rs` | Directional anomaly detection |
| Quality score threshold | **0.80** | `data_validator.rs` | Minimum accepted quality |
| Magnetic field bounds | **± 1,000 µT** | `data_validator.rs` | Per-component safety limits |
| gRPC output buffer | **256 messages** | `grpc_server.rs` | Per-client async channel depth |

---

## 🤝 Contributing

Contributions, issues and feature requests are welcome!

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feat/my-feature`
3. **Commit** your changes with a descriptive message
4. **Push** to your fork and open a **Pull Request**

Please make sure your code passes the linter and tests before submitting:

```bash
make fmt
make lint
make test
```

---

## 📄 License

This project is licensed under the **MIT License** — see the [LICENSE](LICENSE) file for details.

---

<div align="center">

Made with ❤️ and Rust · [QPS-CORE](https://github.com/imshivanshutiwari/QPS-CORE)

</div>