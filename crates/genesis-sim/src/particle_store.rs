//! Centralised Structure-of-Arrays particle storage for the Genesis simulation.
//!
//! Instead of using individual Bevy ECS entities for the simulation hot-path,
//! we store all particle data in flat, parallel arrays indexed by `usize`.
//! This gives us:
//!   - Cache-friendly iteration (hot fields packed together)
//!   - O(1) index-based access (critical for neighbor queries)
//!   - Easy compaction of dead particles
//!
//! Bevy entities are used only for rendering synchronisation. The simulation
//! systems operate directly on this `ParticleStore` resource.

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use crate::components::{ParticleType, CellRole, NUM_TYPES, MAX_BONDS};

// ---------------------------------------------------------------------------
// SimRng — Deterministic PRNG resource (Mulberry32)
// ---------------------------------------------------------------------------

/// Deterministic pseudo-random number generator stored as a Bevy resource.
///
/// Uses the Mulberry32 algorithm, identical to the TypeScript reference
/// implementation, ensuring that the same seed produces the same simulation.
#[derive(Resource)]
pub struct SimRng {
    state: u32,
}

impl SimRng {
    /// Create a new RNG with the given seed.
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// Generate the next pseudo-random `f32` in `[0.0, 1.0)`.
    ///
    /// Uses the Mulberry32 algorithm (same as the TypeScript `mulberry32`).
    #[inline]
    pub fn next(&mut self) -> f32 {
        self.state = self.state.wrapping_add(0x6D2B_79F5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        let result = t ^ (t >> 14);
        (result as f64 / 4_294_967_296.0) as f32
    }

    /// Generate a random float in `[lo, hi)`.
    #[inline]
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next() * (hi - lo)
    }
}

impl Default for SimRng {
    fn default() -> Self {
        Self::new(42)
    }
}

// ---------------------------------------------------------------------------
// ParticleStore — SoA particle data
// ---------------------------------------------------------------------------

/// Structure-of-Arrays storage for all particle data.
///
/// Each particle is identified by its **index** into these parallel arrays.
/// The `id` field contains a unique, monotonically-increasing particle ID
/// that is stable across compaction (used for bond references).
///
/// ## Layout
///
/// Fields are grouped by access pattern:
/// - **Identity**: `id`, `ptype`, `alive`, `is_deposit`
/// - **Physics**: `x/y/z`, `vx/vy/vz`
/// - **Biology**: `energy`, `signal`, `memory`, `phase`, `age`
/// - **Bonds**: `bonds` (HashSet of partner particle IDs)
/// - **Organism**: `organism_id` (-1 = no organism)
/// - **Advanced**: genetics, cell roles, symbols, tools, culture, cognition
#[derive(Resource)]
pub struct ParticleStore {
    // ---- Identity ----
    /// Unique particle ID (monotonically increasing, stable across compaction).
    pub id: Vec<u32>,
    /// Particle type (Alpha, Beta, Catalyst, Data, Membrane, Motor).
    pub ptype: Vec<ParticleType>,
    /// Whether this particle is alive. Dead particles are removed on cleanup.
    pub alive: Vec<bool>,
    /// Whether this particle is an inert energy deposit.
    pub is_deposit: Vec<bool>,

    // ---- Physics ----
    /// World-space X position.
    pub x: Vec<f32>,
    /// World-space Y position.
    pub y: Vec<f32>,
    /// World-space Z position.
    pub z: Vec<f32>,
    /// Velocity X component (world-units per tick).
    pub vx: Vec<f32>,
    /// Velocity Y component.
    pub vy: Vec<f32>,
    /// Velocity Z component.
    pub vz: Vec<f32>,

    // ---- Biology ----
    /// Energy reserve. Particles with zero energy may die.
    pub energy: Vec<f32>,
    /// Signal value (0.0–1.0) propagated through bonds.
    pub signal: Vec<f32>,
    /// Particle-local memory for simple learning / state.
    pub memory: Vec<f32>,
    /// Phase accumulator for oscillatory behaviours.
    pub phase: Vec<f32>,
    /// Age in simulation ticks since spawning.
    pub age: Vec<u32>,

    // ---- Bonds ----
    /// Set of bonded partner particle IDs (by particle id, not index).
    /// Each particle can have up to `MAX_BONDS` bonds.
    pub bonds: Vec<HashSet<u32>>,

    // ---- Organism membership ----
    /// Organism ID this particle belongs to. -1 = not in any organism.
    pub organism_id: Vec<i32>,

    // ---- Advanced: genetics & epigenetics ----
    /// Combo bonus factor derived from bond-pattern interactions.
    pub combo_bonus: Vec<f32>,
    /// Gene expression level (0.0–1.0).
    pub gene_expr: Vec<f32>,
    /// Specialised cell role within an organism.
    pub cell_role: Vec<CellRole>,
    /// Epigenetic weight modifier (multiplicative, default 1.0).
    pub epi_weight: Vec<f32>,

    // ---- Advanced: symbolic & cultural ----
    /// Symbol code channel (0 = none, 1–8 = active channel).
    pub symbol_code: Vec<u8>,
    /// Particle ID of held tool (-1 = none).
    pub held_tool: Vec<i32>,
    /// Cultural meme identifier (0 = none).
    pub cultural_meme: Vec<u16>,
    /// Meta-cognition level (0.0 = none, higher = deeper).
    pub meta_cog_level: Vec<f32>,

    // ---- Advanced: immune system ----
    /// Immune signature — organisms with the same signature cooperate.
    pub signature: Vec<u32>,

    // ---- Index mapping ----
    /// Maps particle ID → current array index. Rebuilt on compaction.
    pub id_to_index: HashMap<u32, usize>,
    /// Next particle ID to assign (monotonically increasing).
    pub next_id: u32,

    // ---- Cached aggregates ----
    /// Count of alive, non-deposit particles (updated on rebuild_index).
    pub alive_count: usize,
}

impl Default for ParticleStore {
    fn default() -> Self {
        Self {
            id: Vec::new(),
            ptype: Vec::new(),
            alive: Vec::new(),
            is_deposit: Vec::new(),
            x: Vec::new(),
            y: Vec::new(),
            z: Vec::new(),
            vx: Vec::new(),
            vy: Vec::new(),
            vz: Vec::new(),
            energy: Vec::new(),
            signal: Vec::new(),
            memory: Vec::new(),
            phase: Vec::new(),
            age: Vec::new(),
            bonds: Vec::new(),
            organism_id: Vec::new(),
            combo_bonus: Vec::new(),
            gene_expr: Vec::new(),
            cell_role: Vec::new(),
            epi_weight: Vec::new(),
            symbol_code: Vec::new(),
            held_tool: Vec::new(),
            cultural_meme: Vec::new(),
            meta_cog_level: Vec::new(),
            signature: Vec::new(),
            id_to_index: HashMap::new(),
            next_id: 0,
            alive_count: 0,
        }
    }
}

impl ParticleStore {
    /// Total number of slots (alive + dead, before compaction).
    #[inline]
    pub fn len(&self) -> usize {
        self.id.len()
    }

    /// Whether there are no particles at all.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.id.is_empty()
    }

    /// Spawn a new particle with the given type, position, and energy.
    ///
    /// Returns the index of the newly created particle. All advanced fields
    /// are initialized to sensible defaults. Velocity is given a small random
    /// jitter from the provided RNG.
    pub fn spawn(
        &mut self,
        ptype: ParticleType,
        x: f32,
        y: f32,
        z: f32,
        energy: f32,
        rng: &mut SimRng,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        let idx = self.id.len();

        // Identity
        self.id.push(id);
        self.ptype.push(ptype);
        self.alive.push(true);
        self.is_deposit.push(false);

        // Physics — small random initial velocity
        self.x.push(x);
        self.y.push(y);
        self.z.push(z);
        self.vx.push((rng.next() - 0.5) * 0.2);
        self.vy.push((rng.next() - 0.5) * 0.2);
        self.vz.push((rng.next() - 0.5) * 0.2);

        // Biology
        self.energy.push(energy);
        self.signal.push(0.0);
        self.memory.push(0.0);
        self.phase.push(rng.next() * std::f32::consts::TAU);
        self.age.push(0);

        // Bonds
        self.bonds.push(HashSet::new());

        // Organism
        self.organism_id.push(-1);

        // Advanced
        self.combo_bonus.push(0.0);
        self.gene_expr.push(0.0);
        self.cell_role.push(CellRole::None);
        self.epi_weight.push(1.0);
        self.symbol_code.push(0);
        self.held_tool.push(-1);
        self.cultural_meme.push(0);
        self.meta_cog_level.push(0.0);
        self.signature.push(0);

        // Index mapping
        self.id_to_index.insert(id, idx);
        self.alive_count += 1;

        idx
    }

    /// Rebuild the `id_to_index` mapping and `alive_count` cache.
    ///
    /// Must be called after any operation that changes alive status or
    /// compacts the arrays (e.g. `cleanup`).
    pub fn rebuild_index(&mut self) {
        self.id_to_index.clear();
        self.alive_count = 0;
        for i in 0..self.id.len() {
            if self.alive[i] {
                self.id_to_index.insert(self.id[i], i);
                if !self.is_deposit[i] {
                    self.alive_count += 1;
                }
            }
        }
    }

    /// Remove dead particles by compacting all arrays in-place.
    ///
    /// After compaction, all alive particles occupy contiguous indices
    /// starting from 0. The `id_to_index` mapping is rebuilt automatically.
    pub fn cleanup(&mut self) {
        let mut write = 0;
        for read in 0..self.id.len() {
            if self.alive[read] {
                if write != read {
                    // Copy all fields from read position to write position
                    self.id[write] = self.id[read];
                    self.ptype[write] = self.ptype[read];
                    self.alive[write] = true;
                    self.is_deposit[write] = self.is_deposit[read];
                    self.x[write] = self.x[read];
                    self.y[write] = self.y[read];
                    self.z[write] = self.z[read];
                    self.vx[write] = self.vx[read];
                    self.vy[write] = self.vy[read];
                    self.vz[write] = self.vz[read];
                    self.energy[write] = self.energy[read];
                    self.signal[write] = self.signal[read];
                    self.memory[write] = self.memory[read];
                    self.phase[write] = self.phase[read];
                    self.age[write] = self.age[read];
                    // Use mem::take to avoid cloning the HashSet
                    self.bonds[write] = std::mem::take(&mut self.bonds[read]);
                    self.organism_id[write] = self.organism_id[read];
                    self.combo_bonus[write] = self.combo_bonus[read];
                    self.gene_expr[write] = self.gene_expr[read];
                    self.cell_role[write] = self.cell_role[read];
                    self.epi_weight[write] = self.epi_weight[read];
                    self.symbol_code[write] = self.symbol_code[read];
                    self.held_tool[write] = self.held_tool[read];
                    self.cultural_meme[write] = self.cultural_meme[read];
                    self.meta_cog_level[write] = self.meta_cog_level[read];
                    self.signature[write] = self.signature[read];
                }
                write += 1;
            }
        }

        // Truncate all vectors to the compacted size
        self.id.truncate(write);
        self.ptype.truncate(write);
        self.alive.truncate(write);
        self.is_deposit.truncate(write);
        self.x.truncate(write);
        self.y.truncate(write);
        self.z.truncate(write);
        self.vx.truncate(write);
        self.vy.truncate(write);
        self.vz.truncate(write);
        self.energy.truncate(write);
        self.signal.truncate(write);
        self.memory.truncate(write);
        self.phase.truncate(write);
        self.age.truncate(write);
        self.bonds.truncate(write);
        self.organism_id.truncate(write);
        self.combo_bonus.truncate(write);
        self.gene_expr.truncate(write);
        self.cell_role.truncate(write);
        self.epi_weight.truncate(write);
        self.symbol_code.truncate(write);
        self.held_tool.truncate(write);
        self.cultural_meme.truncate(write);
        self.meta_cog_level.truncate(write);
        self.signature.truncate(write);

        // Rebuild index mapping
        self.rebuild_index();
    }

    /// Look up the current array index for a particle ID.
    ///
    /// Returns `None` if the particle has been killed or doesn't exist.
    #[inline]
    pub fn idx(&self, particle_id: u32) -> Option<usize> {
        self.id_to_index.get(&particle_id).copied()
    }

    /// Check whether the particle at `idx` has a Catalyst bonded to it.
    ///
    /// Used for catalytic effects on bonding, energy transfer, etc.
    pub fn has_catalyst(&self, idx: usize) -> bool {
        for &bid in &self.bonds[idx] {
            if let Some(&bi) = self.id_to_index.get(&bid) {
                if self.alive[bi] && self.ptype[bi] == ParticleType::Catalyst {
                    return true;
                }
            }
        }
        false
    }

    /// Count the number of bond partners of a given type for particle at `idx`.
    pub fn count_bonded_type(&self, idx: usize, ptype: ParticleType) -> usize {
        let mut count = 0;
        for &bid in &self.bonds[idx] {
            if let Some(&bi) = self.id_to_index.get(&bid) {
                if self.alive[bi] && self.ptype[bi] == ptype {
                    count += 1;
                }
            }
        }
        count
    }

    /// Get the total bond count for the particle at `idx`.
    #[inline]
    pub fn bond_count(&self, idx: usize) -> usize {
        self.bonds[idx].len()
    }

    /// Check if a particle has room for more bonds.
    #[inline]
    pub fn can_bond(&self, idx: usize) -> bool {
        self.bonds[idx].len() < MAX_BONDS
    }

    /// Form a bond between two particles (by index). Returns true if the bond
    /// was formed, false if either particle is at max bonds or already bonded.
    pub fn form_bond(&mut self, a: usize, b: usize) -> bool {
        let id_a = self.id[a];
        let id_b = self.id[b];
        if self.bonds[a].len() >= MAX_BONDS || self.bonds[b].len() >= MAX_BONDS {
            return false;
        }
        if self.bonds[a].contains(&id_b) {
            return false; // Already bonded
        }
        self.bonds[a].insert(id_b);
        self.bonds[b].insert(id_a);
        true
    }

    /// Break a bond between two particles (by index). Returns true if a bond
    /// existed and was broken.
    pub fn break_bond(&mut self, a: usize, b: usize) -> bool {
        let id_a = self.id[a];
        let id_b = self.id[b];
        let removed_a = self.bonds[a].remove(&id_b);
        let removed_b = self.bonds[b].remove(&id_a);
        removed_a || removed_b
    }

    /// Pick a random particle type using the given probability distribution.
    ///
    /// `distribution` must be a normalized array of 6 probabilities (one per type).
    /// Uses inverse CDF sampling.
    pub fn pick_type(distribution: &[f32; NUM_TYPES], rng: &mut SimRng) -> ParticleType {
        let r = rng.next();
        let mut cumulative = 0.0;
        for t in 0..NUM_TYPES {
            cumulative += distribution[t];
            if r < cumulative {
                return ParticleType::from_index(t);
            }
        }
        // Fallback (should not happen with normalized distribution)
        ParticleType::Motor
    }

    /// Get the squared distance between two particles, accounting for
    /// toroidal wrapping with the given world_size.
    pub fn distance_sq_wrapped(&self, a: usize, b: usize, world_size: f32) -> f32 {
        let ws = world_size;
        let mut dx = self.x[a] - self.x[b];
        let mut dy = self.y[a] - self.y[b];
        let mut dz = self.z[a] - self.z[b];

        // Toroidal wrapping: if distance > half world, wrap around
        if dx > ws { dx -= ws * 2.0; } else if dx < -ws { dx += ws * 2.0; }
        if dy > ws { dy -= ws * 2.0; } else if dy < -ws { dy += ws * 2.0; }
        if dz > ws { dz -= ws * 2.0; } else if dz < -ws { dz += ws * 2.0; }

        dx * dx + dy * dy + dz * dz
    }

    /// Get the displacement vector from particle `a` to particle `b`,
    /// accounting for toroidal wrapping.
    pub fn delta_wrapped(&self, a: usize, b: usize, world_size: f32) -> (f32, f32, f32) {
        let ws = world_size;
        let mut dx = self.x[b] - self.x[a];
        let mut dy = self.y[b] - self.y[a];
        let mut dz = self.z[b] - self.z[a];

        if dx > ws { dx -= ws * 2.0; } else if dx < -ws { dx += ws * 2.0; }
        if dy > ws { dy -= ws * 2.0; } else if dy < -ws { dy += ws * 2.0; }
        if dz > ws { dz -= ws * 2.0; } else if dz < -ws { dz += ws * 2.0; }

        (dx, dy, dz)
    }

    /// Get the indices of all bond partners for the particle at `idx`.
    pub fn bond_partners(&self, idx: usize) -> Vec<usize> {
        self.bonds[idx]
            .iter()
            .filter_map(|&bid| self.id_to_index.get(&bid).copied())
            .collect()
    }

    /// Get the organism ID of the particle at `idx` as `Option<u32>`.
    /// Returns `None` if the particle is not in any organism (id == -1).
    pub fn organism_id_opt(&self, idx: usize) -> Option<u32> {
        let oid = self.organism_id[idx];
        if oid >= 0 {
            Some(oid as u32)
        } else {
            None
        }
    }

    /// Count the number of deposit particles.
    pub fn deposit_count(&self) -> usize {
        (0..self.len()).filter(|&i| self.alive[i] && self.is_deposit[i]).count()
    }

    /// Check if any alive particle has at least one bond.
    pub fn has_any_bonds(&self) -> bool {
        (0..self.len()).any(|i| self.alive[i] && !self.bonds[i].is_empty())
    }

    /// Pre-allocate capacity for the expected number of particles.
    pub fn reserve(&mut self, additional: usize) {
        self.id.reserve(additional);
        self.ptype.reserve(additional);
        self.alive.reserve(additional);
        self.is_deposit.reserve(additional);
        self.x.reserve(additional);
        self.y.reserve(additional);
        self.z.reserve(additional);
        self.vx.reserve(additional);
        self.vy.reserve(additional);
        self.vz.reserve(additional);
        self.energy.reserve(additional);
        self.signal.reserve(additional);
        self.memory.reserve(additional);
        self.phase.reserve(additional);
        self.age.reserve(additional);
        self.bonds.reserve(additional);
        self.organism_id.reserve(additional);
        self.combo_bonus.reserve(additional);
        self.gene_expr.reserve(additional);
        self.cell_role.reserve(additional);
        self.epi_weight.reserve(additional);
        self.symbol_code.reserve(additional);
        self.held_tool.reserve(additional);
        self.cultural_meme.reserve(additional);
        self.meta_cog_level.reserve(additional);
        self.signature.reserve(additional);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_and_lookup() {
        let mut store = ParticleStore::default();
        let mut rng = SimRng::new(1);
        let idx = store.spawn(ParticleType::Alpha, 1.0, 2.0, 3.0, 10.0, &mut rng);
        assert_eq!(idx, 0);
        assert_eq!(store.len(), 1);
        assert_eq!(store.alive_count, 1);
        assert_eq!(store.ptype[0], ParticleType::Alpha);
        assert_eq!(store.idx(0), Some(0));
    }

    #[test]
    fn cleanup_compacts() {
        let mut store = ParticleStore::default();
        let mut rng = SimRng::new(1);
        store.spawn(ParticleType::Alpha, 0.0, 0.0, 0.0, 1.0, &mut rng);
        store.spawn(ParticleType::Beta, 1.0, 0.0, 0.0, 1.0, &mut rng);
        store.spawn(ParticleType::Catalyst, 2.0, 0.0, 0.0, 1.0, &mut rng);

        // Kill the middle particle
        store.alive[1] = false;
        store.cleanup();

        assert_eq!(store.len(), 2);
        assert_eq!(store.alive_count, 2);
        assert_eq!(store.ptype[0], ParticleType::Alpha);
        assert_eq!(store.ptype[1], ParticleType::Catalyst);
    }

    #[test]
    fn bond_formation_and_breaking() {
        let mut store = ParticleStore::default();
        let mut rng = SimRng::new(1);
        let a = store.spawn(ParticleType::Alpha, 0.0, 0.0, 0.0, 1.0, &mut rng);
        let b = store.spawn(ParticleType::Beta, 1.0, 0.0, 0.0, 1.0, &mut rng);

        assert!(store.form_bond(a, b));
        assert_eq!(store.bond_count(a), 1);
        assert_eq!(store.bond_count(b), 1);

        // Can't double-bond
        assert!(!store.form_bond(a, b));

        assert!(store.break_bond(a, b));
        assert_eq!(store.bond_count(a), 0);
        assert_eq!(store.bond_count(b), 0);
    }

    #[test]
    fn pick_type_distribution() {
        let mut rng = SimRng::new(42);
        let dist = [0.2, 0.2, 0.2, 0.15, 0.15, 0.1];
        let mut counts = [0u32; 6];
        for _ in 0..1000 {
            let t = ParticleStore::pick_type(&dist, &mut rng);
            counts[t.as_index()] += 1;
        }
        // All types should have been picked at least once
        for c in &counts {
            assert!(*c > 0);
        }
    }

    #[test]
    fn rng_deterministic() {
        let mut a = SimRng::new(999);
        let mut b = SimRng::new(999);
        for _ in 0..100 {
            assert_eq!(a.next().to_bits(), b.next().to_bits());
        }
    }
}
