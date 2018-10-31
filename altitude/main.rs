mod altitude;
use libm;
use nalgebra as na;

fn main() {
    let mut ekf = altitude::ASL_EKF::new();
    let baro_base = 97420.0;
    for i in 0..100 {
        let height:f32 = 100.0 + 50.0 * libm::sinf((i as f32) * 3.14 / 180.0);
        let pressure = baro_base - height;
        let fused = ekf.step(na::Vector2::new(pressure, height));
        println!("Pressure: {}, height: {}, fused: {}", pressure, height, fused[0]);
    }
}
