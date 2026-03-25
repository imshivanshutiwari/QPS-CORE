/// Integration tests for the QPS-CORE pipeline.
/// These tests exercise multiple modules together.
#[cfg(test)]
mod integration {
    use nalgebra::{Vector3, Vector6};

    use qps_core::{
        anomaly::magnetic_anomaly::MagneticAnomalyDetector,
        compute::{
            coordinate_transform::geodetic_to_ecef,
            position_computer::PositionComputer,
        },
        filtering::kalman_filter::KalmanFilter,
        fusion::sensor_fusion::SensorFusionEngine,
        models::SensorReading,
        validation::data_validator::DataValidator,
    };

    fn make_reading(sensor_id: &str, quality: f64, field: [f64; 3]) -> SensorReading {
        SensorReading {
            sensor_id:      sensor_id.into(),
            timestamp:      1_000_000_000,
            magnetic_field: field,
            quality,
            latitude:       Some(48.8566),
            longitude:      Some(2.3522),
            altitude:       Some(35.0),
        }
    }

    /// Full pipeline: validate → fuse → kalman → anomaly → position
    #[test]
    fn test_full_pipeline_single_reading() {
        let reading = make_reading("sensor-1", 0.95, [20.0, 2.0, -43.0]);

        // 1. Validate
        DataValidator::validate(&reading).expect("reading should pass validation");

        // 2. Fuse
        let fused = SensorFusionEngine::fuse(&[reading.clone()]).expect("fusion should succeed");

        // 3. Kalman
        let ecef = geodetic_to_ecef(
            reading.latitude.unwrap(),
            reading.longitude.unwrap(),
            reading.altitude.unwrap(),
        );
        let initial = Vector6::new(ecef[0], ecef[1], ecef[2], 0.0, 0.0, 0.0);
        let mut kf = KalmanFilter::new(initial, 100.0, 1.0);
        kf.predict(0.01);
        kf.update(ecef).expect("kalman update should succeed");

        // 4. Anomaly
        let expected = Vector3::new(20.0, 2.0, -43.0);
        let anomaly = MagneticAnomalyDetector::detect(fused, expected, 5.0);

        // 5. Position
        let now = 1_000_000_000_i64;
        let pos = PositionComputer::compute(kf.state, now, anomaly);

        assert!(!pos.anomaly);
        assert!(pos.latitude.is_finite());
        assert!(pos.longitude.is_finite());
    }

    #[test]
    fn test_full_pipeline_multi_sensor_fusion() {
        let sensors = vec![
            make_reading("s1", 0.9,  [19.9, 1.95, -42.9]),
            make_reading("s2", 0.95, [20.1, 2.05, -43.1]),
            make_reading("s3", 1.0,  [20.0, 2.0,  -43.0]),
        ];

        // All should pass validation
        for r in &sensors {
            DataValidator::validate(r).expect("all readings should be valid");
        }

        let fused = SensorFusionEngine::fuse(&sensors).expect("multi-sensor fusion should succeed");

        // Fused value should be close to the true centre (dominated by highest quality)
        assert!((fused[0] - 20.0).abs() < 0.5);
        assert!((fused[1] - 2.0).abs() < 0.2);
    }

    #[test]
    fn test_pipeline_rejects_bad_reading() {
        let mut bad = make_reading("bad", 0.5, [20_000.0, 2_000.0, -43_000.0]);
        bad.quality = 0.3; // well below threshold
        assert!(DataValidator::validate(&bad).is_err());
    }

    #[test]
    fn test_pipeline_kalman_converges_over_many_cycles() {
        let true_ecef = geodetic_to_ecef(48.8566, 2.3522, 35.0);
        let initial = Vector6::new(true_ecef[0] + 500.0, true_ecef[1] + 500.0, true_ecef[2], 0.0, 0.0, 0.0);
        let mut kf = KalmanFilter::new(initial, 1000.0, 10.0);

        for _ in 0..500 {
            kf.predict(0.01);
            kf.update(true_ecef).expect("update must succeed");
        }

        let pos = kf.position();
        assert!((pos[0] - true_ecef[0]).abs() < 2.0);
        assert!((pos[1] - true_ecef[1]).abs() < 2.0);
    }

    #[test]
    fn test_pipeline_anomaly_is_flagged() {
        let reading = make_reading("s1", 0.95, [20.0, 2.0, -43.0]);
        let fused = SensorFusionEngine::fuse(&[reading]).unwrap();

        // Use a very different expected field to force anomaly
        let expected = Vector3::new(0.0, 0.0, 0.0);
        let anomaly = MagneticAnomalyDetector::detect(fused, expected, 5.0);
        assert!(anomaly, "large deviation should trigger anomaly");
    }

    #[test]
    fn test_position_error_under_2m_after_convergence() {
        let true_ecef = geodetic_to_ecef(51.5, -0.1, 10.0);
        let initial = Vector6::new(true_ecef[0] + 100.0, true_ecef[1], true_ecef[2], 0.0, 0.0, 0.0);
        let mut kf = KalmanFilter::new(initial, 200.0, 5.0);

        for _ in 0..300 {
            kf.predict(0.01);
            kf.update(true_ecef).unwrap();
        }

        let pos = kf.position();
        let error = (pos - true_ecef).norm();
        assert!(error < 2.0, "position error {error:.3}m exceeds 2m bound");
    }
}
