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
    101325.0 * (1.0 - 2.25577e-5 * h).powf(5.25588)
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
pub struct ASL_EKF {
    baseline_pressure: f32,
    p_pre: na::Matrix1<f32>, // Previous prediction noise covariance
    x: na::Matrix1<f32>, // Matrix of n states, where n = 1
    p_post: na::Matrix1<f32>, // Matrix of n multiplied by pval
    q: na::Matrix1<f32>, // Matrix of size n
    r: na::Matrix2<f32>, // Two observations
    i: na::Matrix1<f32>, // of size n

}

impl ASL_EKF {
    /// Create new fusor with default (large measurement covariance) settings.
    pub fn new() -> Self {
        let pval = 0.1;
        let qval = 1e-4;
        let rval = 0.5;
        ASL_EKF {
            p_pre: na::Matrix1::zeros(),
            x: na::Matrix1::zeros(),
            p_post: na::Matrix1::identity() * pval,
            q: na::Matrix1::identity() * qval,
            r: na::Matrix2::identity() * rval,
            i: na::Matrix1::identity(),
            baseline_pressure: 97420.0,
        }
    }

    /// State transition step
    pub fn step(&mut self, z: na::Vector2<f32>) -> na::Matrix1<f32> {
        let (new_x, f) = self.f(self.x);
        self.x = new_x;
        self.p_pre = f * self.p_post * f.transpose() + self.q;
        let (h, h_big) = self.h(self.x);
        let a = self.p_pre * h_big.transpose();
        let b = ((h_big * self.p_pre) * h_big.transpose() + self.r).try_inverse().unwrap();
        let g_big = a * b;
        self.x += g_big * (z - h/*.transpose()*/)/*.transpose()*/;
        self.p_post = (self.i - g_big * h_big) * self.p_pre;
        self.x
    }

    /// State transition function
    pub fn f(&self, x: na::Matrix1<f32>) -> (na::Matrix1<f32>, na::Matrix1<f32>) {
        (x, na::Matrix1::identity())
    }

    pub fn h(&self, x: na::Matrix1<f32>) -> (na::Matrix2x1<f32>, na::Vector2<f32>) {
        let asl = x[0];
        let s = agl_to_range(asl - baro_to_asl(self.baseline_pressure));
        let b = asl_to_baro(asl);
        let h = na::Matrix2x1::new(b, s);
        let dpdx = -0.120131 * fpow(1.0 - 2.2577e-7 * x[0], 4.25588);
        let dsdx = 0.933;
        let h_big = na::Vector2::new(dpdx, dsdx);
        (h, h_big)
    }

}
