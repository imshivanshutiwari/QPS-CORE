/// Unit tests covering all core modules of QPS-CORE.
/// Requires ≥30 test functions.
#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use nalgebra::{Vector3, Vector6};

    use qps_core::{
        anomaly::magnetic_anomaly::MagneticAnomalyDetector,
        anomaly::map_matcher::{MapCell, MapMatcher},
        compute::coordinate_transform::{ecef_to_geodetic, geodetic_to_ecef},
        compute::position_computer::PositionComputer,
        error::{PipelineError, ValidationError},
        filtering::{
            covariance::{enforce_psd, trace},
            kalman_filter::KalmanFilter,
            state_predictor::{peek_prediction, predict_next},
        },
        fusion::{sensor_fusion::SensorFusionEngine, weighting::compute_weight},
        models::SensorReading,
        validation::{data_validator::DataValidator, quality_checker::assess_quality},
    };

    // ── helpers ──────────────────────────────────────────────────────────────

    fn good_reading(q: f64) -> SensorReading {
        SensorReading {
            sensor_id:      "test".into(),
            timestamp:      1_000_000_000,
            magnetic_field: [20.0, 2.0, -43.0],  // µT — components within ±1000 µT
            quality:        q,
            latitude:       Some(51.5),
            longitude:      Some(-0.1),
            altitude:       Some(10.0),
        }
    }

    // ── validation tests ─────────────────────────────────────────────────────

    #[test]
    fn test_validator_accepts_good_reading() {
        assert!(DataValidator::validate(&good_reading(0.95)).is_ok());
    }

    #[test]
    fn test_validator_rejects_low_quality() {
        let r = good_reading(0.5);
        let err = DataValidator::validate(&r).unwrap_err();
        assert!(matches!(err, ValidationError::LowQuality(_)));
    }

    #[test]
    fn test_validator_rejects_zero_timestamp() {
        let mut r = good_reading(0.9);
        r.timestamp = 0;
        let err = DataValidator::validate(&r).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidTimestamp));
    }

    #[test]
    fn test_validator_rejects_negative_timestamp() {
        let mut r = good_reading(0.9);
        r.timestamp = -1;
        let err = DataValidator::validate(&r).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidTimestamp));
    }

    #[test]
    fn test_validator_rejects_out_of_bounds_x() {
        let mut r = good_reading(0.9);
        r.magnetic_field[0] = 1500.0;
        let err = DataValidator::validate(&r).unwrap_err();
        assert!(matches!(err, ValidationError::OutOfBounds(_)));
    }

    #[test]
    fn test_validator_rejects_out_of_bounds_negative() {
        let mut r = good_reading(0.9);
        r.magnetic_field[2] = -1001.0;
        let err = DataValidator::validate(&r).unwrap_err();
        assert!(matches!(err, ValidationError::OutOfBounds(_)));
    }

    #[test]
    fn test_quality_checker_in_range() {
        let r = good_reading(0.95);
        let m = assess_quality(&r);
        assert!(m.passed);
    }

    #[test]
    fn test_quality_checker_out_of_range_field() {
        let mut r = good_reading(0.95);
        // Magnitude way below Earth range
        r.magnetic_field = [0.001, 0.001, 0.001];
        let m = assess_quality(&r);
        assert!(!m.passed);
    }

    // ── fusion tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_fusion_single_reading() {
        let readings = vec![good_reading(0.9)];
        let fused = SensorFusionEngine::fuse(&readings).unwrap();
        assert_abs_diff_eq!(fused[0], 20.0, epsilon = 1e-6);
        assert_abs_diff_eq!(fused[1], 2.0, epsilon = 1e-6);
        assert_abs_diff_eq!(fused[2], -43.0, epsilon = 1e-6);
    }

    #[test]
    fn test_fusion_two_equal_quality() {
        let r1 = good_reading(0.9);
        let r2 = good_reading(0.9);
        let fused = SensorFusionEngine::fuse(&[r1, r2]).unwrap();
        assert_abs_diff_eq!(fused[0], 20.0, epsilon = 1e-3);
    }

    #[test]
    fn test_fusion_weighted_toward_higher_quality() {
        let mut r1 = good_reading(0.9);
        r1.magnetic_field = [10.0, 0.0, 0.0];

        let mut r2 = good_reading(1.0);
        r2.magnetic_field = [30.0, 0.0, 0.0];

        let fused = SensorFusionEngine::fuse(&[r1, r2]).unwrap();
        // Higher quality r2 should pull the result above the simple average
        let simple_avg = 20.0_f64;
        assert!(fused[0] > simple_avg);
    }

    #[test]
    fn test_fusion_empty_returns_error() {
        let err = SensorFusionEngine::fuse(&[]).unwrap_err();
        assert!(matches!(err, PipelineError::EmptyFusion));
    }

    #[test]
    fn test_weighting_quadratic() {
        let r = good_reading(0.9);
        let w = compute_weight(&r);
        assert_abs_diff_eq!(w, 0.81, epsilon = 1e-10);
    }

    // ── Kalman filter tests ───────────────────────────────────────────────────

    fn make_kf() -> KalmanFilter {
        let s = Vector6::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        KalmanFilter::new(s, 10.0, 1.0)
    }

    #[test]
    fn test_kalman_predict_advances_position() {
        let mut kf = make_kf();
        kf.predict(1.0);
        // x should have moved by vx * dt = 1.0
        assert_abs_diff_eq!(kf.state[0], 1.0, epsilon = 1e-9);
    }

    #[test]
    fn test_kalman_predict_does_not_change_velocity() {
        let mut kf = make_kf();
        kf.predict(1.0);
        assert_abs_diff_eq!(kf.state[3], 1.0, epsilon = 1e-9);
    }

    #[test]
    fn test_kalman_update_pulls_state_toward_measurement() {
        let mut kf = make_kf();
        let meas = Vector3::new(100.0, 0.0, 0.0);
        kf.update(meas).unwrap();
        // State x should move toward 100
        assert!(kf.state[0] > 0.0);
    }

    #[test]
    fn test_kalman_convergence() {
        let mut kf = make_kf();
        let true_pos = Vector3::new(50.0, 30.0, -10.0);
        for _ in 0..200 {
            kf.predict(0.01);
            // Add small noise (just use true position for test convergence)
            kf.update(true_pos).unwrap();
        }
        let pos = kf.position();
        assert_abs_diff_eq!(pos[0], 50.0, epsilon = 2.0);
        assert_abs_diff_eq!(pos[1], 30.0, epsilon = 2.0);
        assert_abs_diff_eq!(pos[2], -10.0, epsilon = 2.0);
    }

    #[test]
    fn test_kalman_covariance_decreases_after_updates() {
        let mut kf = make_kf();
        let initial_trace = trace(&kf.covariance);
        for _ in 0..20 {
            kf.predict(0.01);
            kf.update(Vector3::new(0.0, 0.0, 0.0)).unwrap();
        }
        let final_trace = trace(&kf.covariance);
        assert!(final_trace < initial_trace, "covariance did not decrease");
    }

    #[test]
    fn test_state_predictor_peek_does_not_mutate() {
        let kf = make_kf();
        let before = kf.state;
        let _peeked = peek_prediction(&kf, 1.0);
        assert_eq!(kf.state, before);
    }

    #[test]
    fn test_state_predictor_predict_next_mutates() {
        let mut kf = make_kf();
        let before = kf.state[0];
        predict_next(&mut kf, 1.0);
        // x should have changed
        assert_ne!(kf.state[0], before);
    }

    #[test]
    fn test_covariance_enforce_psd_symmetry() {
        let mut kf = make_kf();
        // Artificially break symmetry
        kf.covariance[(0, 1)] = 5.0;
        kf.covariance[(1, 0)] = 0.0;
        enforce_psd(&mut kf.covariance, 1e-6);
        assert_abs_diff_eq!(kf.covariance[(0, 1)], kf.covariance[(1, 0)], epsilon = 1e-10);
    }

    #[test]
    fn test_covariance_trace_positive() {
        let kf = make_kf();
        assert!(trace(&kf.covariance) > 0.0);
    }

    // ── anomaly detection tests ───────────────────────────────────────────────

    #[test]
    fn test_anomaly_no_anomaly_for_expected_field() {
        let field    = Vector3::new(20.0, 2.0, -43.0);
        let expected = Vector3::new(20.0, 2.0, -43.0);
        // Exactly equal → no anomaly
        assert!(!MagneticAnomalyDetector::detect(field, expected, 5.0));
    }

    #[test]
    fn test_anomaly_detected_large_deviation() {
        let field    = Vector3::new(26.0, 2.0, -43.0);
        let expected = Vector3::new(20.0, 2.0, -43.0);
        assert!(MagneticAnomalyDetector::detect(field, expected, 5.0));
    }

    #[test]
    fn test_anomaly_statistical_high_z() {
        let field = Vector3::new(0.0, 0.0, 1000.0);
        assert!(MagneticAnomalyDetector::detect_statistical(field, 50.0, 5.0));
    }

    #[test]
    fn test_anomaly_statistical_no_anomaly() {
        let field = Vector3::new(20.0, 2.0, -43.0);
        let magnitude = field.norm();
        assert!(!MagneticAnomalyDetector::detect_statistical(
            field,
            magnitude,
            10.0
        ));
    }

    #[test]
    fn test_anomaly_with_reference() {
        // Use a clearly anomalous field (near zero, far from Earth reference)
        let field = Vector3::new(0.0, 0.0, 100.0);
        assert!(MagneticAnomalyDetector::detect_with_reference(field));
    }

    // ── coordinate transforms ─────────────────────────────────────────────────

    #[test]
    fn test_ecef_roundtrip() {
        let lat0 = 51.5_f64;
        let lon0 = -0.1_f64;
        let alt0 = 100.0_f64;
        let ecef = geodetic_to_ecef(lat0, lon0, alt0);
        let (lat1, lon1, alt1) = ecef_to_geodetic(ecef[0], ecef[1], ecef[2]);
        assert_abs_diff_eq!(lat1, lat0, epsilon = 1e-6);
        assert_abs_diff_eq!(lon1, lon0, epsilon = 1e-6);
        assert_abs_diff_eq!(alt1, alt0, epsilon = 1e-3);
    }

    #[test]
    fn test_ecef_equator() {
        let ecef = geodetic_to_ecef(0.0, 0.0, 0.0);
        // Should be on the X axis ≈ 6 378 137 m
        assert_abs_diff_eq!(ecef[0], 6_378_137.0, epsilon = 1.0);
        assert_abs_diff_eq!(ecef[1], 0.0, epsilon = 1e-3);
    }

    #[test]
    fn test_position_computer_basic() {
        let state = Vector6::new(
            4_209_000.0_f64,
            0.0,
            4_640_000.0,
            10.0,
            0.0,
            0.0,
        );
        let pos = PositionComputer::compute(state, 1_000_000, false);
        assert!(!pos.anomaly);
        assert_abs_diff_eq!(pos.velocity[0], 10.0, epsilon = 1e-9);
    }

    // ── map matcher tests ─────────────────────────────────────────────────────

    #[test]
    fn test_map_matcher_hit() {
        let cell = MapCell {
            lat_min:  50.0,
            lat_max:  52.0,
            lon_min:  -1.0,
            lon_max:  1.0,
            field_ut: [19.0, 1.5, -44.0], // µT
        };
        let matcher = MapMatcher::new(vec![cell]);
        let f = matcher.expected_field(51.0, 0.0);
        assert_abs_diff_eq!(f[0], 19.0, epsilon = 1e-6);
    }

    #[test]
    fn test_map_matcher_miss_returns_default() {
        let matcher = MapMatcher::new(vec![]);
        let f = matcher.expected_field(0.0, 0.0);
        // Default global average (µT)
        assert_abs_diff_eq!(f[2], -43.0, epsilon = 1e-6);
    }

    // ── pipeline timing / throughput ──────────────────────────────────────────

    #[test]
    fn test_kalman_10k_updates_complete() {
        let mut kf = make_kf();
        let meas = Vector3::new(0.0, 0.0, 0.0);
        let start = std::time::Instant::now();
        for _ in 0..10_000 {
            kf.predict(0.01);
            kf.update(meas).unwrap();
        }
        let elapsed = start.elapsed();
        // In release mode: well under 50ms.  In debug mode: allow up to 10 s.
        // The primary assertion is correctness (no panic, no NaN).
        assert!(
            kf.state.iter().all(|v| v.is_finite()),
            "state contains non-finite values after 10k cycles"
        );
        assert!(
            elapsed.as_secs() < 10,
            "10k cycles took {:.3}s (pathologically slow)",
            elapsed.as_secs_f64()
        );
    }

    #[test]
    fn test_single_pipeline_cycle_under_10ms() {
        let mut kf = make_kf();
        let reading = good_reading(0.9);
        let start = std::time::Instant::now();

        let fused = SensorFusionEngine::fuse(&[reading.clone()]).unwrap();
        kf.predict(0.01);
        let meas = qps_core::compute::coordinate_transform::geodetic_to_ecef(
            reading.latitude.unwrap(),
            reading.longitude.unwrap(),
            reading.altitude.unwrap(),
        );
        kf.update(meas).unwrap();
        let expected = Vector3::new(20.0, 2.0, -43.0); // µT
        let _anomaly = MagneticAnomalyDetector::detect(fused, expected, 5.0);
        let _pos     = PositionComputer::compute(kf.state, 0, false);

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert!(
            elapsed_ms < 10.0,
            "single cycle took {elapsed_ms:.3}ms (exceeds 10ms budget)"
        );
    }
}
