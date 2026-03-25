use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use nalgebra::{Vector3, Vector6};

use qps_core::filtering::kalman_filter::KalmanFilter;
use qps_core::fusion::sensor_fusion::SensorFusionEngine;
use qps_core::models::SensorReading;

fn make_kf() -> KalmanFilter {
    let s = Vector6::new(4_209_000.0_f64, 0.0, 4_640_000.0, 1.0, 0.5, 0.0);
    KalmanFilter::new(s, 100.0, 1.0)
}

fn make_reading(q: f64) -> SensorReading {
    SensorReading {
        sensor_id:      "bench".into(),
        timestamp:      1_000_000,
        magnetic_field: [20.0, 2.0, -43.0], // µT
        quality:        q,
        latitude:       Some(51.5),
        longitude:      Some(-0.1),
        altitude:       Some(10.0),
    }
}

fn bench_kalman_predict(c: &mut Criterion) {
    let mut kf = make_kf();
    c.bench_function("kalman_predict", |b| {
        b.iter(|| kf.predict(black_box(0.01)));
    });
}

fn bench_kalman_update(c: &mut Criterion) {
    let mut kf = make_kf();
    let meas = Vector3::new(4_209_000.0_f64, 0.0, 4_640_000.0);
    c.bench_function("kalman_update", |b| {
        b.iter(|| kf.update(black_box(meas)).unwrap());
    });
}

fn bench_kalman_predict_update(c: &mut Criterion) {
    let mut kf = make_kf();
    let meas = Vector3::new(4_209_000.0_f64, 0.0, 4_640_000.0);
    c.bench_function("kalman_predict_update", |b| {
        b.iter(|| {
            kf.predict(black_box(0.01));
            kf.update(black_box(meas)).unwrap();
        });
    });
}

fn bench_sensor_fusion(c: &mut Criterion) {
    for n in [1_usize, 4, 8, 16] {
        let readings: Vec<SensorReading> = (0..n).map(|i| make_reading(0.85 + 0.01 * i as f64)).collect();
        c.bench_with_input(
            BenchmarkId::new("sensor_fusion", n),
            &readings,
            |b, r| b.iter(|| SensorFusionEngine::fuse(black_box(r)).unwrap()),
        );
    }
}

fn bench_1000_predict_update(c: &mut Criterion) {
    let meas = Vector3::new(4_209_000.0_f64, 0.0, 4_640_000.0);
    c.bench_function("1000_predict_update_cycles", |b| {
        b.iter(|| {
            let mut kf = make_kf();
            for _ in 0..1000 {
                kf.predict(0.01);
                kf.update(meas).unwrap();
            }
            kf
        });
    });
}

criterion_group!(
    benches,
    bench_kalman_predict,
    bench_kalman_update,
    bench_kalman_predict_update,
    bench_sensor_fusion,
    bench_1000_predict_update,
);
criterion_main!(benches);
