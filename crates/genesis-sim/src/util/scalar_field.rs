//! 3D scalar field with toroidal (wrapping) boundary conditions.
//!
//! Used for nutrients, pheromones, metabolites, waves, and symbol channels.
//! Each field is a cube of `size³` cells stored in a flat `Vec<f32>`.
//! Supports sampling, injection, gradient computation, diffusion, and decay.

use bevy::math::Vec3;

/// A 3D scalar field on a regular grid with toroidal (wrapping) boundaries.
///
/// The field stores `size × size × size` floating-point values in a flat array.
/// Boundary conditions wrap around in all three axes, creating a torus topology
/// that avoids edge artifacts in the simulation.
///
/// Typical resolutions:
/// - Nutrients: 20³ = 8,000 cells
/// - Pheromones: 14³ = 2,744 cells
/// - Metabolites / waves / symbols: varies
pub struct ScalarField {
    /// The scalar data stored in row-major order: index = x * size² + y * size + z.
    pub data: Vec<f32>,
    /// Resolution per axis (total cells = size³).
    pub size: usize,
    /// Pre-allocated swap buffer used during diffusion to avoid allocation.
    swap: Vec<f32>,
}

impl ScalarField {
    /// Create a new scalar field initialized to zero.
    pub fn new(size: usize) -> Self {
        let total = size * size * size;
        Self {
            data: vec![0.0; total],
            size,
            swap: vec![0.0; total],
        }
    }

    /// Convert (x, y, z) grid coordinates to a flat array index.
    ///
    /// Coordinates wrap toroidally: negative values and values >= size
    /// are mapped into the valid range [0, size).
    #[inline]
    fn idx(&self, x: i32, y: i32, z: i32) -> usize {
        let s = self.size as i32;
        let wx = ((x % s) + s) % s;
        let wy = ((y % s) + s) % s;
        let wz = ((z % s) + s) % s;
        (wx as usize) * self.size * self.size + (wy as usize) * self.size + (wz as usize)
    }

    /// Get the value at grid coordinates (x, y, z). Coordinates wrap toroidally.
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> f32 {
        self.data[self.idx(x, y, z)]
    }

    /// Set the value at grid coordinates (x, y, z). Coordinates wrap toroidally.
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, v: f32) {
        let i = self.idx(x, y, z);
        self.data[i] = v;
    }

    /// Add a value to the cell at grid coordinates (x, y, z). Coordinates wrap toroidally.
    #[inline]
    pub fn add(&mut self, x: i32, y: i32, z: i32, v: f32) {
        let i = self.idx(x, y, z);
        self.data[i] += v;
    }

    /// Convert a world-space coordinate to the nearest grid coordinate.
    ///
    /// Maps the range `[-world_size/2, +world_size/2]` → `[0, size)`,
    /// with wrapping for coordinates outside the domain.
    ///
    /// Algorithm (matching TypeScript):
    /// ```text
    /// grid = floor(((wx / world_size) + 0.5) * size)
    /// grid = ((grid % size) + size) % size
    /// ```
    #[inline]
    pub fn world_to_grid(&self, wx: f32, world_size: f32) -> i32 {
        let s = self.size as i32;
        let g = ((wx / world_size + 0.5) * self.size as f32).floor() as i32;
        ((g % s) + s) % s
    }

    /// Sample the field value at a world-space position.
    ///
    /// Converts the world position to grid coordinates and returns the
    /// value of the containing cell (nearest-neighbor sampling).
    #[inline]
    pub fn sample(&self, wx: f32, wy: f32, wz: f32, world_size: f32) -> f32 {
        let gx = self.world_to_grid(wx, world_size);
        let gy = self.world_to_grid(wy, world_size);
        let gz = self.world_to_grid(wz, world_size);
        self.get(gx, gy, gz)
    }

    /// Inject (add) an amount of scalar at a world-space position.
    ///
    /// Converts the world position to grid coordinates and adds the
    /// given amount to the containing cell.
    #[inline]
    pub fn inject(&mut self, wx: f32, wy: f32, wz: f32, world_size: f32, amount: f32) {
        let gx = self.world_to_grid(wx, world_size);
        let gy = self.world_to_grid(wy, world_size);
        let gz = self.world_to_grid(wz, world_size);
        self.add(gx, gy, gz, amount);
    }

    /// Compute the gradient of the field at a world-space position.
    ///
    /// Uses central finite differences on the 6 axis-aligned neighbors:
    /// ```text
    /// gx = (field[x+1,y,z] - field[x-1,y,z]) / 2
    /// gy = (field[x,y+1,z] - field[x,y-1,z]) / 2
    /// gz = (field[x,y,z+1] - field[x,y,z-1]) / 2
    /// ```
    ///
    /// Returns a `Vec3` pointing in the direction of increasing field value.
    pub fn gradient(&self, wx: f32, wy: f32, wz: f32, world_size: f32) -> Vec3 {
        let gx = self.world_to_grid(wx, world_size);
        let gy = self.world_to_grid(wy, world_size);
        let gz = self.world_to_grid(wz, world_size);

        // Central differences along each axis (wrapping handled by get()).
        let dx = (self.get(gx + 1, gy, gz) - self.get(gx - 1, gy, gz)) * 0.5;
        let dy = (self.get(gx, gy + 1, gz) - self.get(gx, gy - 1, gz)) * 0.5;
        let dz = (self.get(gx, gy, gz + 1) - self.get(gx, gy, gz - 1)) * 0.5;

        Vec3::new(dx, dy, dz)
    }

    /// Diffuse the field using a single Jacobi iteration with 6 face-neighbors.
    ///
    /// For each cell, the new value is a weighted blend of the current value
    /// and the average of its 6 axis-aligned neighbors:
    /// ```text
    /// new[i] = old[i] * (1 - rate) + (sum_of_6_neighbors / 6) * rate
    /// ```
    ///
    /// `rate` controls diffusion speed (0 = no diffusion, 1 = full averaging).
    /// Uses an internal swap buffer to avoid extra allocation.
    pub fn diffuse(&mut self, rate: f32) {
        let s = self.size as i32;
        let inv6 = rate / 6.0;
        let keep = 1.0 - rate;

        for x in 0..s {
            for y in 0..s {
                for z in 0..s {
                    let center = self.get(x, y, z);
                    let neighbors = self.get(x - 1, y, z)
                        + self.get(x + 1, y, z)
                        + self.get(x, y - 1, z)
                        + self.get(x, y + 1, z)
                        + self.get(x, y, z - 1)
                        + self.get(x, y, z + 1);

                    let i = self.idx(x, y, z);
                    self.swap[i] = center * keep + neighbors * inv6;
                }
            }
        }

        // Swap data and swap buffer.
        std::mem::swap(&mut self.data, &mut self.swap);
    }

    /// Decay all values toward zero by a multiplicative factor.
    ///
    /// ```text
    /// data[i] *= (1 - rate)
    /// ```
    ///
    /// `rate` of 0.01 means 1% decay per step.
    pub fn decay(&mut self, rate: f32) {
        let factor = 1.0 - rate;
        for v in self.data.iter_mut() {
            *v *= factor;
        }
    }

    /// Sum of all values in the field.
    pub fn total(&self) -> f32 {
        self.data.iter().sum()
    }

    /// Sum of absolute values in the field.
    pub fn total_abs(&self) -> f32 {
        self.data.iter().map(|v| v.abs()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_field_is_zero() {
        let f = ScalarField::new(4);
        assert_eq!(f.total(), 0.0);
        assert_eq!(f.data.len(), 64); // 4³
    }

    #[test]
    fn set_get_roundtrip() {
        let mut f = ScalarField::new(4);
        f.set(1, 2, 3, 42.0);
        assert_eq!(f.get(1, 2, 3), 42.0);
    }

    #[test]
    fn wrapping_coordinates() {
        let mut f = ScalarField::new(4);
        f.set(0, 0, 0, 7.0);
        // -4 wraps to 0, 4 wraps to 0
        assert_eq!(f.get(-4, 0, 0), 7.0);
        assert_eq!(f.get(4, 0, 0), 7.0);
        assert_eq!(f.get(0, -4, 0), 7.0);
        assert_eq!(f.get(0, 0, 4), 7.0);
    }

    #[test]
    fn add_accumulates() {
        let mut f = ScalarField::new(4);
        f.add(0, 0, 0, 3.0);
        f.add(0, 0, 0, 5.0);
        assert_eq!(f.get(0, 0, 0), 8.0);
    }

    #[test]
    fn inject_and_sample() {
        let mut f = ScalarField::new(10);
        let ws = 10.0;
        // World center (0,0,0) maps to grid center (5,5,5)
        f.inject(0.0, 0.0, 0.0, ws, 100.0);
        let v = f.sample(0.0, 0.0, 0.0, ws);
        assert_eq!(v, 100.0);
    }

    #[test]
    fn decay_reduces_values() {
        let mut f = ScalarField::new(2);
        f.set(0, 0, 0, 100.0);
        f.decay(0.1); // 10% decay
        assert!((f.get(0, 0, 0) - 90.0).abs() < 1e-5);
    }

    #[test]
    fn diffuse_spreads_values() {
        let mut f = ScalarField::new(4);
        f.set(2, 2, 2, 6.0);
        let before = f.total();
        f.diffuse(0.5);
        let after = f.total();
        // Diffusion should approximately conserve total mass.
        assert!((before - after).abs() < 1e-4);
        // Center value should have decreased.
        assert!(f.get(2, 2, 2) < 6.0);
    }

    #[test]
    fn gradient_points_uphill() {
        let mut f = ScalarField::new(10);
        let ws = 10.0;
        // Create a gradient along x: higher values at higher x
        for x in 0..10i32 {
            for y in 0..10i32 {
                for z in 0..10i32 {
                    f.set(x, y, z, x as f32);
                }
            }
        }
        // Sample gradient at the center
        let g = f.gradient(0.0, 0.0, 0.0, ws);
        // x-component should be positive (increasing x direction)
        assert!(g.x > 0.0);
        // y and z components should be ~0
        assert!(g.y.abs() < 1e-5);
        assert!(g.z.abs() < 1e-5);
    }
}
