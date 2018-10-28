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
