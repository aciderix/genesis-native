//! Genesis Engine — Native Bevy application entry point
//!
//! Supports both GUI mode (default) and headless CLI mode (`--headless`).
//!
//! # GUI mode (default)
//! ```sh
//! cargo run --release
//! ```
//!
//! # Headless mode (no GPU required)
//! ```sh
//! cargo run --release -- --headless --ticks 1000 --seed 42 --json
//! ```

use bevy::prelude::*;
use genesis_sim::GenesisSimPlugin;

// CLI parsing — native only (not available on WASM)
#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;

/// Genesis Engine — Artificial Life Simulator
#[cfg(not(target_arch = "wasm32"))]
#[derive(Parser, Debug)]
#[command(name = "genesis", version, about = "Genesis Engine — Artificial Life Simulator")]
struct Cli {
    /// Run in headless mode (no window, no GPU required)
    #[arg(long)]
    headless: bool,

    /// Number of ticks to simulate in headless mode
    #[arg(long, default_value_t = 1000)]
    ticks: u64,

    /// Simulation seed
    #[arg(long)]
    seed: Option<u32>,

    /// Output final stats as JSON (stdout)
    #[arg(long)]
    json: bool,

    /// Print progress every N ticks (0 = final report only)
    #[arg(long, default_value_t = 0)]
    report_every: u64,

    /// Save final state to a JSON file
    #[arg(long)]
    save: Option<String>,

    /// Simulation speed multiplier (default: 200 in headless)
    #[arg(long)]
    speed: Option<f32>,
}

fn main() {
    // ── WASM: always GUI ────────────────────────────────────────────────
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        run_gui(GenesisSimPlugin::default());
        return;
    }

    // ── Native: parse CLI args ──────────────────────────────────────────
    #[cfg(not(target_arch = "wasm32"))]
    {
        let cli = Cli::parse();
        let sim_plugin = GenesisSimPlugin { seed: cli.seed };

        if cli.headless {
            run_headless(sim_plugin, &cli);
        } else {
            run_gui(sim_plugin);
        }
    }
}

// ---------------------------------------------------------------------------
// GUI mode
// ---------------------------------------------------------------------------

/// Launch the full graphical application (window + 3D rendering + egui UI).
fn run_gui(sim_plugin: GenesisSimPlugin) {
    use genesis_render::GenesisRenderPlugin;
    use genesis_ui::GenesisUiPlugin;

    App::new()
        // ── Core Bevy plugins ──────────────────────────────────────────
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Genesis Engine v6.1".into(),
                resolution: (1280.0, 720.0).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                canvas: Some("#bevy-canvas".to_string()),
                ..default()
            }),
            ..default()
        }))
        // ── Genesis crates ─────────────────────────────────────────────
        .add_plugins(sim_plugin)
        .add_plugins(GenesisRenderPlugin)
        .add_plugins(GenesisUiPlugin)
        // ── Launch ─────────────────────────────────────────────────────
        .run();
}

// ---------------------------------------------------------------------------
// Headless mode (native only)
// ---------------------------------------------------------------------------

/// Configuration resource for headless execution.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource)]
struct HeadlessConfig {
    target_ticks: u64,
    report_every: u64,
    json_output: bool,
    save_path: Option<String>,
    last_report_tick: u64,
    speed: f32,
    start_time: std::time::Instant,
}

/// Run the simulation headless — no window, no GPU, pure computation.
/// Prints stats at the end and exits with code 0.
#[cfg(not(target_arch = "wasm32"))]
fn run_headless(sim_plugin: GenesisSimPlugin, cli: &Cli) {
    use std::time::Duration;

    let speed = cli.speed.unwrap_or(200.0);

    if !cli.json {
        eprintln!("🧬 Genesis Engine — Headless Mode");
        eprintln!(
            "   Seed:   {}",
            cli.seed
                .map_or("random".to_string(), |s| s.to_string())
        );
        eprintln!("   Ticks:  {}", cli.ticks);
        eprintln!("   Speed:  {}×", speed);
        eprintln!();
    }

    App::new()
        .add_plugins(
            MinimalPlugins.set(
                bevy::app::ScheduleRunnerPlugin::run_loop(Duration::ZERO),
            ),
        )
        .add_plugins(sim_plugin)
        .insert_resource(HeadlessConfig {
            target_ticks: cli.ticks,
            report_every: cli.report_every,
            json_output: cli.json,
            save_path: cli.save.clone(),
            last_report_tick: 0,
            speed,
            start_time: std::time::Instant::now(),
        })
        .add_systems(Startup, boost_headless_speed)
        .add_systems(Update, headless_monitor)
        .run();
}

/// In headless mode, crank up the simulation speed so we burn through
/// ticks as fast as the CPU allows.
#[cfg(not(target_arch = "wasm32"))]
fn boost_headless_speed(
    mut sim_config: ResMut<genesis_sim::config::SimConfig>,
    headless_config: Res<HeadlessConfig>,
) {
    sim_config.speed = headless_config.speed;
}

/// Monitor simulation progress and exit when the target tick count is reached.
#[cfg(not(target_arch = "wasm32"))]
fn headless_monitor(
    store: Res<genesis_sim::particle_store::ParticleStore>,
    stats: Res<genesis_sim::resources::SimStats>,
    counters: Res<genesis_sim::resources::SimCounters>,
    phylogeny: Res<genesis_sim::resources::PhylogenyTree>,
    mut hcfg: ResMut<HeadlessConfig>,
    mut exit: EventWriter<AppExit>,
) {
    let tick = counters.tick;

    // ── Periodic progress report ────────────────────────────────────────
    if hcfg.report_every > 0
        && tick > 0
        && tick - hcfg.last_report_tick >= hcfg.report_every
    {
        hcfg.last_report_tick = tick;
        if !hcfg.json_output {
            eprintln!(
                "  tick {:>6} | particles {:>5} | organisms {:>4} | bonds {:>5} | colonies {:>3} | gen {:>3} | energy {:>8.0}",
                tick,
                stats.particle_count,
                stats.organism_count,
                stats.bond_count,
                stats.colony_count,
                stats.max_generation,
                stats.total_energy,
            );
        }
    }

    // ── Target reached → output stats, optionally save, then exit ───────
    if tick >= hcfg.target_ticks {
        // Optional: save state to JSON file
        if let Some(ref path) = hcfg.save_path {
            let json = genesis_sim::saveload::serialize_state(&store, &counters, &phylogeny);
            match std::fs::write(path, &json) {
                Ok(_) => {
                    if !hcfg.json_output {
                        let kb = json.len() / 1024;
                        eprintln!("  💾 Saved to {} ({} KB)", path, kb);
                    }
                }
                Err(e) => {
                    eprintln!("  ❌ Save failed: {}", e);
                }
            }
        }

        // Output final stats
        if hcfg.json_output {
            let elapsed = hcfg.start_time.elapsed().as_secs_f64();
            let ticks_per_sec = tick as f64 / elapsed;
            let output = serde_json::json!({
                "tick": tick,
                "speed": hcfg.speed,
                "elapsed_seconds": (elapsed * 100.0).round() / 100.0,
                "ticks_per_second": (ticks_per_sec).round(),
                "particles": stats.particle_count,
                "organisms": stats.organism_count,
                "bonds": stats.bond_count,
                "colonies": stats.colony_count,
                "max_generation": stats.max_generation,
                "total_energy": stats.total_energy,
                "total_reproductions": counters.total_repro,
                "total_predations": counters.total_pred,
                "total_symbiogenesis": counters.total_symbiogenesis,
                "total_sexual_repro": counters.total_sexual_repro,
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        } else {
            eprintln!();
            eprintln!("═══════════════════════════════════════════════");
            eprintln!("  🧬 Simulation complete — tick {}", tick);
            eprintln!("═══════════════════════════════════════════════");
            eprintln!("  Particles:       {:>6}", stats.particle_count);
            eprintln!("  Organisms:       {:>6}", stats.organism_count);
            eprintln!("  Bonds:           {:>6}", stats.bond_count);
            eprintln!("  Colonies:        {:>6}", stats.colony_count);
            eprintln!("  Max Generation:  {:>6}", stats.max_generation);
            eprintln!("  Total Energy:    {:>10.0}", stats.total_energy);
            eprintln!("  Reproductions:   {:>6}", counters.total_repro);
            eprintln!("  Predations:      {:>6}", counters.total_pred);
            eprintln!("═══════════════════════════════════════════════");
        }

        exit.send(AppExit::Success);
    }
}
