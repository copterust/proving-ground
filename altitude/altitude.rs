/// Convert above the sea level to Pascals
///
/// Returns:
/// * p: air pressure above sea level
/// Input:
/// * h: altitude above sea level (m)
///
/// References:
/// * http://www.engineeringtoolbox.com/air-altitude-pressure-d_462.html
fn asl_to_baro(h: f32) -> f32 {
    101325 * (1 - 2.25577e-5 * h).powf(5.25588)
}
