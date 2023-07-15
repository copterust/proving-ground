mod altitude;
use libm;
use nalgebra as na;
use rand;

const LOOPSIZE: i32 = 5000;

//ground-truth AGL to sonar measurement, empirically determined:
// see http://diydrones.com/profiles/blogs/altitude-hold-with-mb1242-sonar
fn sonarfun(agl: f32) -> f32 {
    0.933 * agl - 2.894
}

fn main() {
    let mut ekf = altitude::ASL_EKF::new();
    let baro_base = 97420.0;

    for i in 0..LOOPSIZE {
        //  Model up-and-down motion with a sine wave
        let count = i as f32;
        let sine = libm::sinf(count / (LOOPSIZE as f32) * 2.0 * 3.141592);
        let baro = baro_base + sine * 20.;

        // Add noise to simulated sonar at random intervals
        let sonar: f32 = sonarfun(50. * (1. - sine))
            + if rand::random::<f32>() > 0.9 { 50. } else { 0. };
        let fused = ekf.step(na::Vector2::new(baro, sonar));
        println!("Pressure: {}, height: {}, fused: {}", baro, sonar, fused[0]);
    }
}
