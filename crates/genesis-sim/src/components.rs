//! ECS Components for the Genesis Engine particle simulation.
//!
//! This module defines every Bevy ECS component used to represent particles,
//! their physical state, biological properties, social structures, and
//! emergent cultural/cognitive features.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Total number of distinct particle types in the simulation.
pub const NUM_TYPES: usize = 6;

/// Maximum bonds a single particle can form simultaneously.
pub const MAX_BONDS: usize = 4;

/// Human-readable names for each particle type, indexed by `ParticleType::as_index()`.
pub const TYPE_NAMES: [&str; NUM_TYPES] = [
    "Alpha",
    "Beta",
    "Catalyst",
    "Data",
    "Membrane",
    "Motor",
];

/// RGB colours for each particle type (values in 0.0–1.0).
pub const TYPE_COLORS: [[f32; 3]; NUM_TYPES] = [
    [0.40, 0.76, 1.00], // Alpha   – light blue
    [1.00, 0.55, 0.25], // Beta    – orange
    [0.30, 1.00, 0.40], // Catalyst – green
    [0.85, 0.50, 1.00], // Data    – purple
    [0.70, 0.70, 0.70], // Membrane – grey
    [1.00, 1.00, 0.30], // Motor   – yellow
];

// ---------------------------------------------------------------------------
// ParticleType
// ---------------------------------------------------------------------------

/// The fundamental type of a particle, governing its interaction affinities,
/// bonding behaviour, and role potential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect)]
pub enum ParticleType {
    Alpha    = 0,
    Beta     = 1,
    Catalyst = 2,
    Data     = 3,
    Membrane = 4,
    Motor    = 5,
}

impl ParticleType {
    /// Numeric index (matches the discriminant).
    #[inline]
    pub fn as_index(self) -> usize {
        self as usize
    }

    /// Construct from a numeric index. Panics if `idx >= NUM_TYPES`.
    #[inline]
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::Alpha,
            1 => Self::Beta,
            2 => Self::Catalyst,
            3 => Self::Data,
            4 => Self::Membrane,
            5 => Self::Motor,
            _ => panic!("Invalid particle type index: {idx}"),
        }
    }

    /// Try to construct from a numeric index, returning `None` for out-of-range.
    #[inline]
    pub fn try_from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Self::Alpha),
            1 => Some(Self::Beta),
            2 => Some(Self::Catalyst),
            3 => Some(Self::Data),
            4 => Some(Self::Membrane),
            5 => Some(Self::Motor),
            _ => None,
        }
    }

    /// Human-readable name.
    #[inline]
    pub fn name(self) -> &'static str {
        TYPE_NAMES[self.as_index()]
    }

    /// RGB colour as `[f32; 3]`.
    #[inline]
    pub fn color_rgb(self) -> [f32; 3] {
        TYPE_COLORS[self.as_index()]
    }

    /// Iterator over all particle types.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..NUM_TYPES).map(Self::from_index)
    }
}

impl Default for ParticleType {
    fn default() -> Self {
        Self::Alpha
    }
}

// ---------------------------------------------------------------------------
// Core identity & lifecycle
// ---------------------------------------------------------------------------

/// Unique numeric identifier for a particle (monotonically increasing).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub struct ParticleId(pub u32);

/// Tag component: the particle is alive and should be simulated.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct Alive;

// ---------------------------------------------------------------------------
// Physics
// ---------------------------------------------------------------------------

/// Linear velocity (world-units per tick).
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct Velocity(pub Vec3);

/// Accumulated force for the current tick — zeroed at the start of each physics step.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct Force(pub Vec3);

// ---------------------------------------------------------------------------
// Energy & age
// ---------------------------------------------------------------------------

/// The particle's energy reserve. Particles with zero energy may die.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct Energy(pub f32);

/// Age in simulation ticks since spawning.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct Age(pub u32);

// ---------------------------------------------------------------------------
// Signalling & memory
// ---------------------------------------------------------------------------

/// Signal value (0.0–1.0) propagated through bonds.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct Signal(pub f32);

/// Particle-local memory value used for simple learning / state.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct Memory(pub f32);

/// Phase accumulator for oscillatory / cyclic behaviours.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct Phase(pub f32);

// ---------------------------------------------------------------------------
// Genetics & epigenetics
// ---------------------------------------------------------------------------

/// Gene expression level (0.0–1.0).
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct GeneExpression(pub f32);

/// Epigenetic weight modifier (multiplicative).
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct EpiWeight(pub f32);

impl Default for EpiWeight {
    fn default() -> Self {
        Self(1.0)
    }
}

/// Bonus factor derived from combo interactions.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct ComboBonus(pub f32);

// ---------------------------------------------------------------------------
// Bonds
// ---------------------------------------------------------------------------

/// Up to `MAX_BONDS` covalent-like bonds to other particles.
#[derive(Component, Debug, Clone, Reflect)]
pub struct Bonds {
    /// Bonded partner entities (`None` = empty slot).
    pub partners: [Option<Entity>; MAX_BONDS],
    /// Number of active bonds (cached for fast checks).
    pub count: u8,
}

impl Default for Bonds {
    fn default() -> Self {
        Self {
            partners: [None; MAX_BONDS],
            count: 0,
        }
    }
}

impl Bonds {
    /// Create a new empty bonds component.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when no more bond slots are available.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.count as usize >= MAX_BONDS
    }

    /// Returns `true` if `entity` is already bonded.
    #[inline]
    pub fn has(&self, entity: Entity) -> bool {
        self.partners.iter().any(|p| *p == Some(entity))
    }

    /// Try to add a bond. Returns `true` on success, `false` if full or duplicate.
    pub fn add(&mut self, entity: Entity) -> bool {
        if self.has(entity) || self.is_full() {
            return false;
        }
        for slot in self.partners.iter_mut() {
            if slot.is_none() {
                *slot = Some(entity);
                self.count += 1;
                return true;
            }
        }
        false
    }

    /// Remove a bond partner. Returns `true` if the partner was found and removed.
    pub fn remove(&mut self, entity: Entity) -> bool {
        for slot in self.partners.iter_mut() {
            if *slot == Some(entity) {
                *slot = None;
                self.count = self.count.saturating_sub(1);
                return true;
            }
        }
        false
    }

    /// Iterate over all current bond partners.
    pub fn iter(&self) -> impl Iterator<Item = Entity> + '_ {
        self.partners.iter().filter_map(|p| *p)
    }

    /// Number of active bonds.
    #[inline]
    pub fn len(&self) -> usize {
        self.count as usize
    }

    /// Returns `true` if there are no active bonds.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// ---------------------------------------------------------------------------
// Cell roles (within organisms)
// ---------------------------------------------------------------------------

/// Specialised role that a particle can assume when it is part of an organism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
pub enum CellRole {
    #[default]
    None       = 0,
    Sensor     = 1,
    Digester   = 2,
    MotorCell  = 3,
    Defense    = 4,
    Reproducer = 5,
}

impl CellRole {
    #[inline]
    pub fn as_index(self) -> usize {
        self as usize
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::None,
            1 => Self::Sensor,
            2 => Self::Digester,
            3 => Self::MotorCell,
            4 => Self::Defense,
            5 => Self::Reproducer,
            _ => Self::None,
        }
    }
}

/// Component wrapper for `CellRole`.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct CellRoleComp(pub CellRole);

// ---------------------------------------------------------------------------
// Organism & colony membership
// ---------------------------------------------------------------------------

/// Marks a particle as belonging to an organism.
/// `organism_id == -1` means "not part of any organism" (mirrors the TS convention).
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct OrganismMember {
    pub organism_id: i32,
}

impl Default for OrganismMember {
    fn default() -> Self {
        Self { organism_id: -1 }
    }
}

impl OrganismMember {
    /// Returns `true` if this particle currently belongs to an organism.
    #[inline]
    pub fn has_organism(&self) -> bool {
        self.organism_id >= 0
    }

    /// Returns the organism ID as `Option<u32>`, mapping `-1` → `None`.
    #[inline]
    pub fn organism(&self) -> Option<u32> {
        if self.organism_id >= 0 {
            Some(self.organism_id as u32)
        } else {
            None
        }
    }
}

/// Colony membership. `-1` means "no colony".
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct ColonyMember(pub i32);

impl Default for ColonyMember {
    fn default() -> Self {
        Self(-1)
    }
}

impl ColonyMember {
    #[inline]
    pub fn has_colony(&self) -> bool {
        self.0 >= 0
    }

    #[inline]
    pub fn colony(&self) -> Option<u32> {
        if self.0 >= 0 { Some(self.0 as u32) } else { None }
    }
}

// ---------------------------------------------------------------------------
// Deposits
// ---------------------------------------------------------------------------

/// Tag component for particles that have become inert energy deposits.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct IsDeposit;

// ---------------------------------------------------------------------------
// Immune system
// ---------------------------------------------------------------------------

/// Immune signature — organisms with the same signature cooperate.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct Signature(pub u32);

// ---------------------------------------------------------------------------
// Symbolic & cultural layers
// ---------------------------------------------------------------------------

/// Symbol code channel. `0` = no symbol, `1..=8` = active channel.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct SymbolCode(pub u8);

/// Entity reference for a held "tool" (another particle used instrumentally).
#[derive(Component, Debug, Clone, Reflect)]
pub struct HeldTool(pub Option<Entity>);

impl Default for HeldTool {
    fn default() -> Self {
        Self(None)
    }
}

/// Cultural meme identifier. `0` = none.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct CulturalMeme(pub u16);

/// Meta-cognition level (0.0 = no meta-cognition, higher = deeper).
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
pub struct MetaCogLevel(pub f32);

// ---------------------------------------------------------------------------
// Genome
// ---------------------------------------------------------------------------

/// Simple 8-gene genome for heritable traits.
#[derive(Component, Debug, Clone, Reflect)]
pub struct Genome {
    pub genes: [f32; 8],
}

impl Default for Genome {
    fn default() -> Self {
        Self { genes: [0.0; 8] }
    }
}

impl Genome {
    /// Generate a random genome using the provided RNG closure (returns 0.0–1.0).
    pub fn random(mut rng: impl FnMut() -> f32) -> Self {
        let mut genes = [0.0f32; 8];
        for g in genes.iter_mut() {
            *g = rng();
        }
        Self { genes }
    }

    /// Uniform crossover between two parent genomes with optional mutation.
    /// `rng` should return values in 0.0–1.0.
    pub fn crossover(a: &Genome, b: &Genome, mutation_rate: f32, mut rng: impl FnMut() -> f32) -> Self {
        let mut genes = [0.0f32; 8];
        for i in 0..8 {
            // Pick gene from parent a or b with equal probability
            genes[i] = if rng() < 0.5 { a.genes[i] } else { b.genes[i] };
            // Possibly mutate
            if rng() < mutation_rate {
                genes[i] = (genes[i] + (rng() - 0.5) * 0.2).clamp(0.0, 1.0);
            }
        }
        Self { genes }
    }
}

// ---------------------------------------------------------------------------
// Bundle helper (optional convenience)
// ---------------------------------------------------------------------------

/// A bundle containing every component a freshly-spawned particle needs.
#[derive(Bundle)]
pub struct ParticleBundle {
    pub id: ParticleId,
    pub ptype: ParticleType,
    pub alive: Alive,
    pub transform: Transform,
    pub velocity: Velocity,
    pub force: Force,
    pub energy: Energy,
    pub age: Age,
    pub signal: Signal,
    pub memory: Memory,
    pub phase: Phase,
    pub gene_expr: GeneExpression,
    pub epi_weight: EpiWeight,
    pub combo_bonus: ComboBonus,
    pub bonds: Bonds,
    pub cell_role: CellRoleComp,
    pub organism: OrganismMember,
    pub colony: ColonyMember,
    pub signature: Signature,
    pub symbol_code: SymbolCode,
    pub held_tool: HeldTool,
    pub cultural_meme: CulturalMeme,
    pub meta_cog: MetaCogLevel,
    pub genome: Genome,
}

impl ParticleBundle {
    /// Create a new particle bundle with sensible defaults.
    pub fn new(id: u32, ptype: ParticleType, position: Vec3, energy: f32) -> Self {
        Self {
            id: ParticleId(id),
            ptype,
            alive: Alive,
            transform: Transform::from_translation(position),
            velocity: Velocity::default(),
            force: Force::default(),
            energy: Energy(energy),
            age: Age(0),
            signal: Signal(0.0),
            memory: Memory(0.0),
            phase: Phase(0.0),
            gene_expr: GeneExpression(0.0),
            epi_weight: EpiWeight(1.0),
            combo_bonus: ComboBonus(0.0),
            bonds: Bonds::new(),
            cell_role: CellRoleComp::default(),
            organism: OrganismMember::default(),
            colony: ColonyMember::default(),
            signature: Signature(0),
            symbol_code: SymbolCode(0),
            held_tool: HeldTool::default(),
            cultural_meme: CulturalMeme(0),
            meta_cog: MetaCogLevel(0.0),
            genome: Genome::default(),
        }
    }
}
