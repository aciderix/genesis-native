//! Spatial hash grid for efficient neighbor queries in 3D particle simulations.
//!
//! Uses a packed 30-bit key (10 bits per axis) to hash 3D positions into cells.
//! Supports O(1) insertion and 3×3×3 neighbor queries for finding nearby particles.

use bevy::prelude::*;
use std::collections::HashMap;

/// Spatial hash grid for O(1) neighbor lookups.
///
/// Cells are indexed by a packed 30-bit key: 10 bits for each of x, y, z.
/// This gives 1024 unique buckets per axis, which is more than sufficient
/// for the simulation's world size.
#[derive(Resource)]
pub struct SpatialGrid {
    /// Size of each grid cell — should match or exceed the interaction radius.
    cell_size: f32,
    /// Map from packed cell key to the list of particle indices in that cell.
    cells: HashMap<u32, Vec<usize>>,
}

impl SpatialGrid {
    /// Create a new spatial grid with the given cell size.
    ///
    /// The cell size should be at least as large as the maximum interaction
    /// radius so that all potential neighbors fall within the 3×3×3 query volume.
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    /// Compute the packed 30-bit cell key for a world-space position.
    ///
    /// Each axis is quantized to a 10-bit integer (0..1023) by flooring
    /// the coordinate divided by cell_size and masking to 10 bits.
    /// The three 10-bit values are packed into a single u32:
    ///   bits [29..20] = x, bits [19..10] = y, bits [9..0] = z
    #[inline]
    fn key(x: f32, y: f32, z: f32, cs: f32) -> u32 {
        let ix = (x / cs).floor() as i32;
        let iy = (y / cs).floor() as i32;
        let iz = (z / cs).floor() as i32;
        (((ix as u32) & 0x3FF) << 20) | (((iy as u32) & 0x3FF) << 10) | ((iz as u32) & 0x3FF)
    }

    /// Decompose a packed key back into its three 10-bit cell coordinates.
    #[inline]
    fn unpack_key(key: u32) -> (i32, i32, i32) {
        let x = ((key >> 20) & 0x3FF) as i32;
        let y = ((key >> 10) & 0x3FF) as i32;
        let z = (key & 0x3FF) as i32;
        (x, y, z)
    }

    /// Pack three 10-bit cell coordinates into a single key.
    #[inline]
    fn pack_key(cx: i32, cy: i32, cz: i32) -> u32 {
        (((cx as u32) & 0x3FF) << 20) | (((cy as u32) & 0x3FF) << 10) | ((cz as u32) & 0x3FF)
    }

    /// Remove all particles from the grid. Call once per frame before re-inserting.
    pub fn clear(&mut self) {
        // We clear each vec but keep the HashMap entries to avoid re-allocation.
        for bucket in self.cells.values_mut() {
            bucket.clear();
        }
    }

    /// Insert a particle (by index) at the given world-space position.
    pub fn insert(&mut self, idx: usize, x: f32, y: f32, z: f32) {
        let k = Self::key(x, y, z, self.cell_size);
        self.cells.entry(k).or_insert_with(|| Vec::with_capacity(8)).push(idx);
    }

    /// Query all particle indices in the 27 neighboring cells (3×3×3 cube)
    /// centered on the cell containing the given world-space position.
    ///
    /// Returns a newly-allocated `Vec<usize>`. For hot loops, prefer
    /// [`query_into`] to reuse an output buffer.
    pub fn query(&self, x: f32, y: f32, z: f32) -> Vec<usize> {
        let mut out = Vec::new();
        self.query_into(x, y, z, &mut out);
        out
    }

    /// Query all particle indices in the 27 neighboring cells, appending
    /// results to the provided `out` vector (which is cleared first).
    ///
    /// This avoids per-query allocation and is the preferred method in
    /// performance-critical code paths.
    pub fn query_into(&self, x: f32, y: f32, z: f32, out: &mut Vec<usize>) {
        out.clear();

        let k = Self::key(x, y, z, self.cell_size);
        let (cx, cy, cz) = Self::unpack_key(k);

        // Iterate over the 3×3×3 neighborhood of cells.
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                for dz in -1i32..=1 {
                    let nk = Self::pack_key(cx + dx, cy + dy, cz + dz);
                    if let Some(bucket) = self.cells.get(&nk) {
                        out.extend_from_slice(bucket);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_query_finds_nearby() {
        let mut grid = SpatialGrid::new(1.0);
        grid.insert(0, 0.5, 0.5, 0.5);
        grid.insert(1, 0.8, 0.8, 0.8);
        grid.insert(2, 5.0, 5.0, 5.0); // far away

        let neighbors = grid.query(0.5, 0.5, 0.5);
        assert!(neighbors.contains(&0));
        assert!(neighbors.contains(&1));
        assert!(!neighbors.contains(&2));
    }

    #[test]
    fn clear_removes_all() {
        let mut grid = SpatialGrid::new(1.0);
        grid.insert(0, 0.0, 0.0, 0.0);
        grid.clear();

        let neighbors = grid.query(0.0, 0.0, 0.0);
        assert!(neighbors.is_empty());
    }

    #[test]
    fn query_into_reuses_buffer() {
        let mut grid = SpatialGrid::new(1.0);
        grid.insert(0, 1.0, 1.0, 1.0);
        grid.insert(1, 1.5, 1.5, 1.5);

        let mut buf = Vec::new();
        grid.query_into(1.2, 1.2, 1.2, &mut buf);
        assert!(buf.contains(&0));
        assert!(buf.contains(&1));
    }
}
