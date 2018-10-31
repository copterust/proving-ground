use nalgebra as na;
use libm::fpow;

/// Convert above the sea level to Pascals
///
/// Returns:
/// * p: air pressure above sea level (Pa)
/// Input:
/// * h: altitude above sea level (m)
///
/// References:
/// * http://www.engineeringtoolbox.com/air-altitude-pressure-d_462.html
fn asl_to_baro(h: f32) -> f32 {
    101325 * (1 - 2.25577e-5 * h).powf(5.25588)
}

/// Convert pressure to above the sea level
///
/// Returns:
/// * h: altitude above sea level (m)
/// Input:
/// * p: air pressure
fn baro_to_asl(p: f32) -> f32 {
    (1.0 - (p / 101325.0).powf(0.190295)) * 44330.0
}

/// Ground-truth AGL to rangefinder measurement
///
/// TBD:
/// * Measure real function
///
/// Input:
/// * h: above the ground level
///
/// Output:
/// * h': corresponding rangefinder measurement
fn agl_to_range(h: f32) -> f32 {
    h
}

/// Class for fusing range and barometric sensors
struct ASL_EKF {
    baseline_pressure: f32,
    p_pre: Option<f32>, // Previous prediction noise covariance
    x: na::Matrix1<f32>, // Matrix of n states, where n = 1
    p_post: Matrix1<f32>, // Matrix of n multiplied by pval
    q: Matrix1<f32>, // Matrix of size n
    r: na::Matrix2<f32>, // Two observations
    i: na::Matrix1<f32>, // of size n

}

impl ASL_EKF {
    /// Create new fusor with default (large measurement covariance) settings.
    pub fn new() -> Self {
        let pval = 0.1;
        let qval = 1e-4;
        ASL_EKF {
            p_pre: None,
            x: na::zeros(1),
            p_post: na::identity(1) * pval,
            q: na::identity(1) * qval,
            r: na::identity(2) * rval,
            i: na::identity(1.0),
            baseline_pressure: 97420,
        }
    }

    /// State transition step
    pub fn step(&mut self, z: na::Vec2<f32>) -> na::Vec2<f32> {
        let (new_x, f) = self.f(self.x);
        self.x = new_x;
        self.p_pre = f * self.p_post * f.transpose() + self.q;
        let (h, h_big) = self.h(self.x);
        let g_big = na::dot(self.p_pre.dot(h_big.transpose()), na::inverse(h_big.dot(self.p_pre).dot(h_big.transpose()) + self.r));
        self.x += na::dot(g_big, (z - h.transpose()).transpose());
        self.p_post = na::dot(self.i - na::dot(b_big, h_big), self.p_pre);
        self.x
    }

    /// State transition function
    pub fn f(&self, x: na::Matrix1<f32>) -> (na::Matrix1<f32>, na::Matrix1<f32>) {
        (x, np.identity(1))
    }

    pub fn h(&self, x: na::Matrix1<f32>) -> (na::Vec2<f32>, na::Vec2<f32> {
        let asl = x[0];
        let s = agl_to_range(asl - baro_to_asl(self.baseline_pressure));
        let b = asl_to_baro(asl);
        let h = na::Vec2::new(b, s);
        let dpdx = -0.120131 * fpow((1 - 2.2577e-7 * x[0]), 4.25588);
        let dsdx = 0.933;
        let h_big = na::Vec2::new(dpdx, dsdx);
        (h, h_big)
    }

}
