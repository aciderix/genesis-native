//! Simulation configuration and randomisation utilities.
//!
//! This module contains [`SimConfig`] — the master configuration resource that
//! governs every tunable parameter of the Genesis Engine simulation — together
//! with the base interaction matrices and a deterministic PRNG for reproducible
//! universe generation.

use bevy::prelude::*;
use crate::components::NUM_TYPES;

// ---------------------------------------------------------------------------
// Mulberry32 PRNG
// ---------------------------------------------------------------------------

/// Returns a closure that implements the Mulberry32 PRNG.
///
/// Each call to the returned closure produces a new pseudo-random `f32` in
/// the range `[0.0, 1.0)`. The algorithm is identical to the TypeScript
/// reference implementation so that the same seed yields the same universe.
pub fn mulberry32(seed: u32) -> impl FnMut() -> f32 {
    let mut state = seed;
    move || {
        state = state.wrapping_add(0x6D2B_79F5);
        let mut t = state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        let result = t ^ (t >> 14);
        (result as f32) / (u32::MAX as f32)
    }
}

// ---------------------------------------------------------------------------
// Base interaction matrices (6×6)
// ---------------------------------------------------------------------------

/// Base affinity matrix — governs attraction / repulsion between particle types.
///
/// Row = source type, Column = target type.  Positive = attraction, negative = repulsion.
/// Indices: 0=Alpha, 1=Beta, 2=Catalyst, 3=Data, 4=Membrane, 5=Motor
pub const BASE_AFFINITY: [[f32; 6]; 6] = [
    //  Alpha   Beta    Cat     Data    Memb    Motor
    [  0.10,  0.30,  0.20,  0.10,  0.40,  0.30 ], // Alpha
    [  0.30, -0.10,  0.50,  0.20,  0.30,  0.10 ], // Beta
    [  0.20,  0.50, -0.20,  0.40,  0.10,  0.20 ], // Catalyst
    [  0.10,  0.20,  0.40,  0.60,  0.10,  0.30 ], // Data
    [  0.40,  0.30,  0.10,  0.10,  0.50,  0.00 ], // Membrane
    [  0.30,  0.10,  0.20,  0.30,  0.00, -0.30 ], // Motor
];

/// Base bond-strength matrix — how strong a bond forms between two particle types.
///
/// Values in `[0.0, 1.0]`. Higher = stronger bond (harder to break).
pub const BASE_BOND_STRENGTH: [[f32; 6]; 6] = [
    //  Alpha   Beta    Cat     Data    Memb    Motor
    [  0.70,  0.50,  0.40,  0.30,  0.65,  0.50 ], // Alpha
    [  0.50,  0.30,  0.70,  0.40,  0.50,  0.30 ], // Beta
    [  0.40,  0.70,  0.20,  0.60,  0.30,  0.40 ], // Catalyst
    [  0.30,  0.40,  0.60,  0.80,  0.20,  0.50 ], // Data
    [  0.65,  0.50,  0.30,  0.20,  0.75,  0.20 ], // Membrane
    [  0.50,  0.30,  0.40,  0.50,  0.20,  0.40 ], // Motor
];

/// Base signal-conductance matrix — efficiency of signal propagation across a bond.
///
/// Values in `[0.0, 1.0]`. Higher = signal passes more easily.
pub const BASE_SIGNAL_CONDUCTANCE: [[f32; 6]; 6] = [
    //  Alpha   Beta    Cat     Data    Memb    Motor
    [  0.10,  0.20,  0.30,  0.50,  0.10,  0.20 ], // Alpha
    [  0.20,  0.40,  0.60,  0.50,  0.20,  0.30 ], // Beta
    [  0.30,  0.60,  0.20,  0.70,  0.20,  0.40 ], // Catalyst
    [  0.50,  0.50,  0.70,  0.90,  0.30,  0.60 ], // Data
    [  0.10,  0.20,  0.20,  0.30,  0.10,  0.10 ], // Membrane
    [  0.20,  0.30,  0.40,  0.60,  0.10,  0.30 ], // Motor
];

// ---------------------------------------------------------------------------
// Matrix randomisation
// ---------------------------------------------------------------------------

/// Produce a new 6×6 matrix by adding uniform noise to each cell of `base`.
///
/// Each cell is clamped to `[-1.0, 1.0]` after perturbation. `rng` must
/// return values in `[0.0, 1.0)`. `noise` controls the amplitude of the
/// random offset (e.g. `0.15` means ±0.15 from the base value).
pub fn randomize_matrix(
    base: &[[f32; 6]; 6],
    rng: &mut impl FnMut() -> f32,
    noise: f32,
) -> [[f32; 6]; 6] {
    let mut out = [[0.0f32; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            let offset = (rng() - 0.5) * 2.0 * noise;
            out[i][j] = (base[i][j] + offset).clamp(-1.0, 1.0);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// SimConfig
// ---------------------------------------------------------------------------

/// Master simulation configuration, serialisable for save / load.
///
/// All physical, biological, and runtime-control parameters live here.
/// A new config is generated deterministically from a seed via
/// [`generate_config`].
#[derive(Resource, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SimConfig {
    // ---- universe seed ----------------------------------------------------
    /// The seed used to generate this configuration.
    pub seed: u32,

    // ---- world geometry ---------------------------------------------------
    /// Half-extent of the cubic simulation volume (particles wrap at ±world_size).
    pub world_size: f32,

    // ---- population -------------------------------------------------------
    /// Initial number of particles to spawn.
    pub particle_count: usize,

    // ---- thermal vents ----------------------------------------------------
    /// Number of thermal vents in the world.
    pub vent_count: usize,
    /// Base energy output per vent per tick.
    pub vent_strength: f32,

    // ---- interaction radii ------------------------------------------------
    /// Maximum distance at which particles can interact (force / signal).
    pub interaction_radius: f32,
    /// Maximum distance at which a new bond may form.
    pub bond_distance: f32,

    // ---- energy sources ---------------------------------------------------
    /// Strength of the global solar energy field.
    pub solar_strength: f32,
    /// Direction of solar illumination (normalised).
    #[serde(with = "vec3_serde")]
    pub solar_dir: Vec3,

    // ---- environmental ----------------------------------------------------
    /// Background thermal energy — drives random jitter / diffusion.
    pub temperature: f32,

    // ---- genetics ---------------------------------------------------------
    /// Probability of a gene mutating during reproduction.
    pub mutation_rate: f32,

    // ---- type distribution ------------------------------------------------
    /// Relative probability of each particle type being spawned (sums to ~1.0).
    pub type_distribution: [f32; NUM_TYPES],

    // ---- runtime control --------------------------------------------------
    /// How many simulation ticks to run per render frame (1 / 5 / 10 / 20).
    pub speed: f32,
    /// When `true` the simulation is frozen.
    pub paused: bool,

    // ---- population caps --------------------------------------------------
    /// Hard upper limit on live particles.
    pub max_particles: usize,
    /// Hard upper limit on deposit entities.
    pub max_deposits: usize,
}

impl Default for SimConfig {
    fn default() -> Self {
        generate_config(None)
    }
}

// ---------------------------------------------------------------------------
// Config generation
// ---------------------------------------------------------------------------

/// Deterministically generate a complete [`SimConfig`] from an optional seed.
///
/// If `seed` is `None`, a default seed of `42` is used. All tuneable
/// parameters are derived from the PRNG so that sharing a seed reproduces
/// the exact same universe.
pub fn generate_config(seed: Option<u32>) -> SimConfig {
    let seed = seed.unwrap_or(42);
    let mut rng = mulberry32(seed);

    // Helper: random float in [lo, hi]
    let mut range = |lo: f32, hi: f32| -> f32 { lo + rng() * (hi - lo) };

    let world_size = range(30.0, 50.0);
    let particle_count = range(1200.0, 2000.0) as usize;
    let vent_count = range(3.0, 9.0) as usize;
    let vent_strength = range(0.4, 1.0);
    let interaction_radius = range(3.5, 5.0);
    let bond_distance = range(1.8, 2.4);
    let solar_strength = range(0.10, 0.25);
    let temperature = range(0.15, 0.50);
    let mutation_rate = range(0.05, 0.20);

    // Random solar direction (normalised)
    let sx = rng() - 0.5;
    let sy = rng() - 0.5;
    let sz = rng() - 0.5;
    let solar_dir = Vec3::new(sx, sy, sz).normalize_or(Vec3::new(0.0, 1.0, 0.0));

    // Type distribution: random weights, then normalise
    let mut td = [0.0f32; NUM_TYPES];
    let mut total = 0.0f32;
    for d in td.iter_mut() {
        let w = 0.5 + rng(); // match web: 0.5 + rng()
        *d = w;
        total += w;
    }
    for d in td.iter_mut() {
        *d /= total;
    }

    SimConfig {
        seed,
        world_size,
        particle_count,
        vent_count,
        vent_strength,
        interaction_radius,
        bond_distance,
        solar_strength,
        solar_dir,
        temperature,
        mutation_rate,
        type_distribution: td,
        speed: 1.0,
        paused: false,
        max_particles: 5000,
        max_deposits: 800,
    }
}

// ---------------------------------------------------------------------------
// Vec3 serde helper (Bevy Vec3 doesn't impl Serialize by default)
// ---------------------------------------------------------------------------

mod vec3_serde {
    use bevy::math::Vec3;
    use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Vec3Proxy {
        x: f32,
        y: f32,
        z: f32,
    }

    pub fn serialize<S: Serializer>(v: &Vec3, s: S) -> Result<S::Ok, S::Error> {
        Vec3Proxy { x: v.x, y: v.y, z: v.z }.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec3, D::Error> {
        let p = Vec3Proxy::deserialize(d)?;
        Ok(Vec3::new(p.x, p.y, p.z))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mulberry32_deterministic() {
        let mut rng1 = mulberry32(12345);
        let mut rng2 = mulberry32(12345);
        for _ in 0..100 {
            assert_eq!(rng1().to_bits(), rng2().to_bits());
        }
    }

    #[test]
    fn mulberry32_range() {
        let mut rng = mulberry32(99);
        for _ in 0..1000 {
            let v = rng();
            assert!((0.0..1.0).contains(&v), "value out of range: {v}");
        }
    }

    #[test]
    fn config_deterministic() {
        let a = generate_config(Some(777));
        let b = generate_config(Some(777));
        assert_eq!(a.world_size, b.world_size);
        assert_eq!(a.particle_count, b.particle_count);
        assert_eq!(a.type_distribution, b.type_distribution);
    }

    #[test]
    fn type_distribution_sums_to_one() {
        let cfg = generate_config(Some(42));
        let sum: f32 = cfg.type_distribution.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "sum = {sum}");
    }

    #[test]
    fn randomize_matrix_stays_bounded() {
        let mut rng = mulberry32(1);
        let m = randomize_matrix(&BASE_AFFINITY, &mut rng, 0.5);
        for row in &m {
            for &v in row {
                assert!((-1.0..=1.0).contains(&v), "value out of bounds: {v}");
            }
        }
    }
}
