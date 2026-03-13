//! # Genesis Render
//!
//! Bevy 0.15 rendering plugin for the Genesis Engine.
//!
//! This crate bridges the simulation data in [`ParticleStore`] to Bevy's ECS
//! rendering pipeline.  It uses instanced rendering (a shared low-poly sphere
//! mesh + per-particle transforms) for thousands of particles, thin cylinders
//! for bonds, an orbital camera, day/night lighting, and a selection resource
//! for the inspector UI.

use bevy::prelude::*;
use genesis_sim::components::ParticleType;
use genesis_sim::config::SimConfig;
use genesis_sim::particle_store::ParticleStore;
use genesis_sim::resources::*;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// Top-level render plugin – add this to the Bevy [`App`] to get 3-D
/// visualisation of the running simulation.
pub struct GenesisRenderPlugin;

impl Plugin for GenesisRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_rendering)
            .add_systems(
                Update,
                (
                    sync_particles,
                    sync_bonds,
                    update_camera,
                    update_lighting,
                ),
            );
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marker component on every Bevy entity that represents a single particle
/// from the simulation.  `store_index` maps back into [`ParticleStore`].
#[derive(Component)]
struct ParticleVisual {
    store_index: usize,
}

/// Marker component for bond-cylinder entities.
#[derive(Component)]
struct BondVisual;

/// Orbital camera controller state.  The camera orbits around `target` at
/// the given `distance`, with `yaw` / `pitch` angles (radians).
#[derive(Component)]
struct OrbitCamera {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub target: Vec3,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Cached material handles – one per particle type, plus special materials
/// for deposits and bonds.
#[derive(Resource)]
struct ParticleMaterials {
    /// One material per [`ParticleType`] variant (indexed 0..6).
    materials: [Handle<StandardMaterial>; 6],
    /// Semi-transparent grey for deposit particles.
    deposit_material: Handle<StandardMaterial>,
    /// Semi-transparent unlit white for bond cylinders.
    bond_material: Handle<StandardMaterial>,
}

/// Cached mesh handles shared by all particle / bond entities.
#[derive(Resource)]
struct ParticleMeshes {
    sphere: Handle<Mesh>,
    cylinder: Handle<Mesh>,
}

/// Tracks which particle (if any) is currently selected for inspection.
#[derive(Resource, Default)]
pub struct SelectedParticle {
    pub index: Option<usize>,
}

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

/// One-time initialisation: create shared meshes, materials, the camera, and
/// scene lighting.
fn setup_rendering(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<SimConfig>,
) {
    // ------------------------------------------------------------------
    // Shared meshes
    // ------------------------------------------------------------------

    // Low-poly icosphere for particles (subdivision level 2 ≈ 162 verts).
    let sphere = meshes.add(Sphere::new(0.3).mesh().ico(2).unwrap());
    // Unit cylinder used for bonds (scaled at runtime).
    let cylinder = meshes.add(Cylinder::new(0.05, 1.0).mesh());

    // ------------------------------------------------------------------
    // Per-type materials
    // ------------------------------------------------------------------

    // Each of the six particle types gets a distinctive colour with a
    // subtle emissive glow so they remain visible even in shadow.
    let type_colors: [[f32; 3]; 6] = [
        [0.3, 0.6, 1.0], // Alpha    — blue
        [0.2, 1.0, 0.3], // Beta     — green
        [1.0, 0.5, 0.0], // Catalyst — orange
        [1.0, 1.0, 0.2], // Data     — yellow
        [0.7, 0.3, 1.0], // Membrane — purple
        [1.0, 0.2, 0.2], // Motor    — red
    ];

    let mats: [Handle<StandardMaterial>; 6] = std::array::from_fn(|i| {
        let [r, g, b] = type_colors[i];
        materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            emissive: LinearRgba::new(r * 0.3, g * 0.3, b * 0.3, 1.0),
            perceptual_roughness: 0.6,
            metallic: 0.1,
            ..default()
        })
    });

    // Deposits are translucent grey.
    let deposit_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.5, 0.5, 0.5, 0.5),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // Bonds are translucent, unlit white lines.
    let bond_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.8, 0.8, 0.3),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.insert_resource(ParticleMaterials {
        materials: mats,
        deposit_material: deposit_mat,
        bond_material: bond_mat,
    });
    commands.insert_resource(ParticleMeshes { sphere, cylinder });
    commands.insert_resource(SelectedParticle::default());

    // ------------------------------------------------------------------
    // Camera
    // ------------------------------------------------------------------

    let ws = config.world_size;
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, ws * 0.3, ws * 1.2))
            .looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCamera {
            distance: ws * 1.2,
            yaw: 0.0,
            pitch: 0.3,
            target: Vec3::ZERO,
        },
    ));

    // ------------------------------------------------------------------
    // Lighting
    // ------------------------------------------------------------------

    // Soft ambient fill.
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
    });

    // Primary directional "sun" light with shadows.
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.8),
            illuminance: 8000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.5, 0.0)),
    ));

    // Cool-toned point light at the world origin for fill.
    commands.spawn((
        PointLight {
            color: Color::srgb(0.5, 0.7, 1.0),
            intensity: 50000.0,
            range: ws * 2.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::ZERO),
    ));
}

// ---------------------------------------------------------------------------
// Particle sync
// ---------------------------------------------------------------------------

/// Every frame, synchronise [`ParticleStore`] positions / types to Bevy
/// entities.
///
/// **Strategy:** maintain a pool of pre-spawned entities.  If the pool is
/// too small we spawn more; surplus entities are hidden.  This avoids the
/// cost of spawning / despawning thousands of entities each frame.
fn sync_particles(
    mut commands: Commands,
    store: Res<ParticleStore>,
    materials: Res<ParticleMaterials>,
    meshes_res: Res<ParticleMeshes>,
    mut visuals: Query<(
        Entity,
        &mut ParticleVisual,
        &mut Transform,
        &mut Visibility,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    // Collect indices of living particles.
    let alive_indices: Vec<usize> = (0..store.len()).filter(|&i| store.alive[i]).collect();

    let existing_count = visuals.iter().count();
    let needed = alive_indices.len();

    // Grow the pool if necessary.
    if needed > existing_count {
        for _ in 0..(needed - existing_count) {
            commands.spawn((
                Mesh3d(meshes_res.sphere.clone()),
                MeshMaterial3d(materials.materials[0].clone()),
                Transform::default(),
                Visibility::Hidden,
                ParticleVisual { store_index: 0 },
            ));
        }
    }

    // Walk existing entities and assign them to alive particles.
    let mut visual_iter = visuals.iter_mut();
    for &si in &alive_indices {
        if let Some((_, mut pv, mut transform, mut vis, mut mat)) = visual_iter.next() {
            pv.store_index = si;
            transform.translation = Vec3::new(store.x[si], store.y[si], store.z[si]);

            // Slightly scale particles by energy so high-energy ones look
            // larger – clamped to avoid extremes.
            let scale = 0.25 + (store.energy[si] / 20.0).min(0.5);
            transform.scale = Vec3::splat(scale);

            *vis = Visibility::Visible;

            // Choose material: deposits get a special translucent look,
            // otherwise pick the type-specific colour.
            let mat_handle = if store.is_deposit[si] {
                &materials.deposit_material
            } else {
                &materials.materials[store.ptype[si].as_index() as usize]
            };
            mat.0 = mat_handle.clone();
        }
    }

    // Hide any remaining pool entities that are not in use this frame.
    for (_, _, _, mut vis, _) in visual_iter {
        *vis = Visibility::Hidden;
    }
}

// ---------------------------------------------------------------------------
// Bond sync
// ---------------------------------------------------------------------------

/// Render bonds as thin cylinders stretched between bonded particle pairs.
///
/// Bonds are deduplicated by only drawing `id_a < id_b` pairs.
fn sync_bonds(
    mut commands: Commands,
    store: Res<ParticleStore>,
    materials: Res<ParticleMaterials>,
    meshes_res: Res<ParticleMeshes>,
    mut bonds_q: Query<(Entity, &BondVisual, &mut Transform, &mut Visibility)>,
) {
    // Collect unique bond pairs (lower id → higher id).
    let mut bond_pairs: Vec<(usize, usize)> = Vec::new();
    for i in 0..store.len() {
        if !store.alive[i] {
            continue;
        }
        for &bid in &store.bonds[i] {
            if let Some(&j) = store.id_to_index.get(&bid) {
                if store.id[i] < bid && store.alive[j] {
                    bond_pairs.push((i, j));
                }
            }
        }
    }

    let existing = bonds_q.iter().count();
    let needed = bond_pairs.len();

    // Grow pool if needed.
    if needed > existing {
        for _ in 0..(needed - existing) {
            commands.spawn((
                Mesh3d(meshes_res.cylinder.clone()),
                MeshMaterial3d(materials.bond_material.clone()),
                Transform::default(),
                Visibility::Hidden,
                BondVisual,
            ));
        }
    }

    let mut bond_iter = bonds_q.iter_mut();
    for &(ia, ib) in &bond_pairs {
        if let Some((_, _, mut transform, mut vis)) = bond_iter.next() {
            let a = Vec3::new(store.x[ia], store.y[ia], store.z[ia]);
            let b = Vec3::new(store.x[ib], store.y[ib], store.z[ib]);
            let mid = (a + b) * 0.5;
            let diff = b - a;
            let len = diff.length();

            // Skip degenerate zero-length bonds.
            if len < 0.001 {
                *vis = Visibility::Hidden;
                continue;
            }

            // Orient the unit Y-cylinder along the bond direction and scale
            // its Y-axis to the bond length.
            let dir = diff / len;
            let rot = Quat::from_rotation_arc(Vec3::Y, dir);
            transform.translation = mid;
            transform.rotation = rot;
            transform.scale = Vec3::new(1.0, len, 1.0);
            *vis = Visibility::Visible;
        }
    }

    // Hide surplus pool entities.
    for (_, _, _, mut vis) in bond_iter {
        *vis = Visibility::Hidden;
    }
}

// ---------------------------------------------------------------------------
// Camera
// ---------------------------------------------------------------------------

/// Orbital camera controlled by mouse and keyboard:
///
/// - **Right / Middle mouse drag** – rotate (yaw + pitch)
/// - **Scroll wheel** – zoom in / out
/// - **W / A / S / D** – pan target horizontally
/// - **Q / E** – raise / lower target
fn update_camera(
    mut camera_q: Query<(&mut Transform, &mut OrbitCamera)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut scroll: EventReader<bevy::input::mouse::MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut cam)) = camera_q.get_single_mut() else {
        return;
    };

    // ---- Rotation (mouse drag) ----
    if mouse_button.pressed(MouseButton::Right) || mouse_button.pressed(MouseButton::Middle) {
        for ev in mouse_motion.read() {
            cam.yaw -= ev.delta.x * 0.005;
            cam.pitch = (cam.pitch - ev.delta.y * 0.005).clamp(-1.5, 1.5);
        }
    } else {
        // Consume unneeded events to avoid accumulation.
        mouse_motion.clear();
    }

    // ---- Zoom (scroll wheel) ----
    for ev in scroll.read() {
        cam.distance = (cam.distance - ev.y * cam.distance * 0.1).clamp(5.0, 200.0);
    }

    // ---- Pan (keyboard) ----
    let speed = 20.0 * time.delta_secs();
    let forward = Vec3::new(cam.yaw.sin(), 0.0, cam.yaw.cos());
    let right = Vec3::new(cam.yaw.cos(), 0.0, -cam.yaw.sin());

    if keyboard.pressed(KeyCode::KeyW) {
        cam.target += forward * speed;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        cam.target -= forward * speed;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        cam.target -= right * speed;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        cam.target += right * speed;
    }
    if keyboard.pressed(KeyCode::KeyQ) {
        cam.target.y += speed;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        cam.target.y -= speed;
    }

    // ---- Apply ----
    let eye = cam.target
        + Vec3::new(
            cam.yaw.sin() * cam.pitch.cos() * cam.distance,
            cam.pitch.sin() * cam.distance,
            cam.yaw.cos() * cam.pitch.cos() * cam.distance,
        );
    *transform = Transform::from_translation(eye).looking_at(cam.target, Vec3::Y);
}

// ---------------------------------------------------------------------------
// Day / Night lighting
// ---------------------------------------------------------------------------

/// Smoothly adjust ambient and directional light intensity + colour based on
/// the simulation's [`DayNightState`] resource (`solar_now` ∈ [0, 1]).
fn update_lighting(
    day_night: Res<DayNightState>,
    mut ambient: ResMut<AmbientLight>,
    mut dir_lights: Query<&mut DirectionalLight>,
) {
    let day = day_night.solar_now.clamp(0.0, 1.0);

    // Ambient brightness ramps from a dim 100 lx at night to 400 lx midday.
    let night_brightness = 100.0;
    let day_brightness = 400.0;
    ambient.brightness = night_brightness + (day_brightness - night_brightness) * day;

    // Directional light intensity follows the sun factor.
    for mut light in dir_lights.iter_mut() {
        light.illuminance = 2000.0 + 8000.0 * day;

        // Colour shifts from warm (sunrise/sunset) to cool (midday).
        let r = 1.0;
        let g = 0.85 + 0.15 * day;
        let b = 0.7 + 0.3 * day;
        light.color = Color::srgb(r, g, b);
    }
}
