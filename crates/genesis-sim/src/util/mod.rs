//! Utility modules for the Genesis simulation.
//!
//! - [`spatial_grid`]: Spatial hash grid for O(1) neighbor lookups.
//! - [`scalar_field`]: 3D scalar field with toroidal boundary conditions.

pub mod scalar_field;
pub mod spatial_grid;

pub use scalar_field::ScalarField;
pub use spatial_grid::SpatialGrid;
