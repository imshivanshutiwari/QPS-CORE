use nalgebra::Vector3;

/// WGS-84 ellipsoid constants
const A: f64 = 6_378_137.0;           // semi-major axis (m)
const B: f64 = 6_356_752.314_245;     // semi-minor axis (m)
const E2: f64 = 1.0 - (B * B) / (A * A); // eccentricity²

/// Convert ECEF Cartesian (x, y, z) in metres to WGS-84
/// geodetic (latitude °, longitude °, altitude m).
pub fn ecef_to_geodetic(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    // Longitude is straightforward
    let lon = y.atan2(x).to_degrees();

    // Iterative Bowring method for latitude / altitude
    let p = (x * x + y * y).sqrt();
    let mut lat = z.atan2(p * (1.0 - E2)); // initial estimate

    for _ in 0..10 {
        let sin_lat = lat.sin();
        let n = A / (1.0 - E2 * sin_lat * sin_lat).sqrt();
        lat = (z + E2 * n * sin_lat).atan2(p);
    }

    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let n = A / (1.0 - E2 * sin_lat * sin_lat).sqrt();
    let alt = if cos_lat.abs() > 1e-10 {
        p / cos_lat - n
    } else {
        z / sin_lat - n * (1.0 - E2)
    };

    (lat.to_degrees(), lon, alt)
}

/// Convert WGS-84 geodetic to ECEF Cartesian.
pub fn geodetic_to_ecef(lat_deg: f64, lon_deg: f64, alt_m: f64) -> Vector3<f64> {
    let lat = lat_deg.to_radians();
    let lon = lon_deg.to_radians();
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let n = A / (1.0 - E2 * sin_lat * sin_lat).sqrt();

    Vector3::new(
        (n + alt_m) * cos_lat * lon.cos(),
        (n + alt_m) * cos_lat * lon.sin(),
        (n * (1.0 - E2) + alt_m) * sin_lat,
    )
}
