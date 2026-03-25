use nalgebra::{Matrix3, Matrix3x6, Matrix6, Vector3, Vector6};

use crate::error::PipelineError;

/// 6-DOF Kalman filter tracking position (x, y, z) and velocity (vx, vy, vz)
/// in an ECEF-like Cartesian frame.
///
/// State vector:  [x, y, z, vx, vy, vz]ᵀ
/// Measurement:   [x, y, z]ᵀ  (direct position / magnetic-derived position)
pub struct KalmanFilter {
    /// Current state estimate
    pub state:            Vector6<f64>,
    /// State covariance matrix P
    pub covariance:       Matrix6<f64>,
    /// Process noise matrix Q
    pub process_noise:    Matrix6<f64>,
    /// Measurement noise matrix R
    pub measurement_noise: Matrix3<f64>,
}

impl KalmanFilter {
    /// Create a new filter with sensible defaults.
    ///
    /// * `initial_state` – [x, y, z, vx, vy, vz] in metres / m/s
    /// * `pos_noise`     – initial position uncertainty (metres)
    /// * `vel_noise`     – initial velocity uncertainty (m/s)
    pub fn new(initial_state: Vector6<f64>, pos_noise: f64, vel_noise: f64) -> Self {
        let mut covariance = Matrix6::zeros();
        for i in 0..3 {
            covariance[(i, i)] = pos_noise * pos_noise;
        }
        for i in 3..6 {
            covariance[(i, i)] = vel_noise * vel_noise;
        }

        // Process noise: small acceleration uncertainty
        let q_pos = 0.1_f64;
        let q_vel = 1.0_f64;
        let mut process_noise = Matrix6::zeros();
        for i in 0..3 {
            process_noise[(i, i)] = q_pos * q_pos;
        }
        for i in 3..6 {
            process_noise[(i, i)] = q_vel * q_vel;
        }

        // Measurement noise: ±2 m position accuracy
        let r = 2.0_f64;
        let measurement_noise = Matrix3::from_diagonal_element(r * r);

        Self {
            state: initial_state,
            covariance,
            process_noise,
            measurement_noise,
        }
    }

    /// **Predict step** – propagate the state forward by `dt` seconds using a
    /// constant-velocity model.
    ///
    /// State transition:
    /// ```text
    /// x  ← x  + vx·dt
    /// y  ← y  + vy·dt
    /// z  ← z  + vz·dt
    /// vx ← vx
    /// vy ← vy
    /// vz ← vz
    /// ```
    /// Covariance:  P ← F·P·Fᵀ + Q
    pub fn predict(&mut self, dt: f64) {
        // Build state transition matrix F (identity + velocity coupling)
        let mut f = Matrix6::identity();
        f[(0, 3)] = dt;
        f[(1, 4)] = dt;
        f[(2, 5)] = dt;

        self.state = f * self.state;
        self.covariance = f * self.covariance * f.transpose() + self.process_noise;
    }

    /// **Update step** – incorporate a position measurement [x, y, z].
    ///
    /// Equations:
    /// ```text
    /// y = z − H·x̂         (innovation)
    /// S = H·P·Hᵀ + R       (innovation covariance)
    /// K = P·Hᵀ·S⁻¹         (Kalman gain)
    /// x̂ ← x̂ + K·y
    /// P  ← (I − K·H)·P
    /// ```
    pub fn update(&mut self, measurement: Vector3<f64>) -> Result<(), PipelineError> {
        // Observation matrix H: extracts position from state vector
        #[rustfmt::skip]
        let h = Matrix3x6::new(
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
        );

        // Innovation
        let y = measurement - h * self.state;

        // Innovation covariance S = HPHᵀ + R
        let s = h * self.covariance * h.transpose() + self.measurement_noise;

        // Kalman gain K = PHᵀ S⁻¹
        let s_inv = s.try_inverse().ok_or(PipelineError::MatrixInversion)?;
        let k = self.covariance * h.transpose() * s_inv;

        // State update
        self.state = self.state + k * y;

        // Covariance update (Joseph form for numerical stability)
        let i_kh = Matrix6::identity() - k * h;
        self.covariance = i_kh * self.covariance * i_kh.transpose()
            + k * self.measurement_noise * k.transpose();

        Ok(())
    }

    /// Return the current position estimate [x, y, z].
    #[inline]
    pub fn position(&self) -> Vector3<f64> {
        Vector3::new(self.state[0], self.state[1], self.state[2])
    }

    /// Return the current velocity estimate [vx, vy, vz].
    #[inline]
    pub fn velocity(&self) -> Vector3<f64> {
        Vector3::new(self.state[3], self.state[4], self.state[5])
    }
}
