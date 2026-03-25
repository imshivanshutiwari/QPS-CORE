use nalgebra::Vector3;

/// A geographic map cell with an expected magnetic field signature.
#[derive(Debug, Clone)]
pub struct MapCell {
    pub lat_min:  f64,
    pub lat_max:  f64,
    pub lon_min:  f64,
    pub lon_max:  f64,
    /// Expected magnetic field [x, y, z] in µT for this cell
    pub field_ut: [f64; 3],
}

/// Magnetic map matcher: looks up the expected field for a given position
/// and returns it.  In production this wraps the WMM/IGRF model.
pub struct MapMatcher {
    cells: Vec<MapCell>,
}

impl MapMatcher {
    pub fn new(cells: Vec<MapCell>) -> Self {
        Self { cells }
    }

    /// Return the expected magnetic field at the given coordinates, or a
    /// global default if no cell matches.
    pub fn expected_field(&self, lat: f64, lon: f64) -> Vector3<f64> {
        for cell in &self.cells {
            if lat >= cell.lat_min
                && lat <= cell.lat_max
                && lon >= cell.lon_min
                && lon <= cell.lon_max
            {
                return Vector3::new(cell.field_ut[0], cell.field_ut[1], cell.field_ut[2]);
            }
        }
        // Fallback: approximate global average (µT)
        Vector3::new(20.0, 2.0, -43.0)
    }
}
