# 🧬 Genesis Engine v6.1 — Native Edition

A high-performance artificial life simulation built with **Rust** and **Bevy 0.15**.
Particles self-organize into organisms, colonies, and evolving ecosystems.

![License](https://img.shields.io/badge/license-MIT-blue)
![Build](https://img.shields.io/github/actions/workflow/status/aciderix/genesis-native/ci.yml?label=CI)

## ✨ Features

- **6 particle types** (Alpha, Beta, Catalyst, Data, Membrane, Motor)
- **Emergent life**: bonds → organisms → colonies → reproduction → evolution
- **Day/Night cycle** with solar energy dynamics
- **Cultural evolution** & metacognition systems
- **Full 3D rendering** with orbital camera
- **Real-time UI** with HUD, charts, inspector, event log
- **Touch controls** for mobile (pinch-to-zoom, drag-to-orbit)
- **Cross-platform**: Windows, macOS, Linux, Web (WASM), Android, iOS

## 🚀 Quick Start

### Prerequisites

Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build & Run (Desktop)

```bash
git clone https://github.com/aciderix/genesis-native.git
cd genesis-native
cargo run --release
```

> **Note**: First compile takes ~3-5 min (Bevy pulls many deps). `--release` is crucial — debug mode is ~10× slower.

### 🌐 Web (WASM)

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

### 📱 Android

```bash
rustup target add aarch64-linux-android
cargo install cargo-ndk
cargo ndk -t arm64-v8a build --release
```

### 📱 iOS (requires macOS + Xcode)

```bash
rustup target add aarch64-apple-ios
cargo build --release --target aarch64-apple-ios
```

## 🖥️ Headless CLI Mode

Run the simulation without a window or GPU — perfect for CI, testing, and scripting:

```bash
# Basic headless run (1000 ticks, random seed)
./genesis --headless

# Specific seed, 5000 ticks, JSON output
./genesis --headless --ticks 5000 --seed 42 --json

# Progress every 100 ticks + save final state
./genesis --headless --ticks 10000 --seed 42 --report-every 100 --save state.json

# Pipe JSON to jq for analysis
./genesis --headless --ticks 2000 --seed 42 --json | jq '.organisms'
```

### CLI Options

| Flag | Description | Default |
|---|---|---|
| `--headless` | Run without GUI (no GPU needed) | off |
| `--ticks N` | Number of simulation ticks to run | 1000 |
| `--seed N` | Deterministic PRNG seed | random |
| `--json` | Output stats as JSON to stdout | off |
| `--report-every N` | Print progress every N ticks | 0 (final only) |
| `--save FILE` | Save final state to JSON file | — |

### JSON Output Schema

```json
{
  "tick": 5000,
  "particles": 1847,
  "organisms": 23,
  "bonds": 1205,
  "colonies": 3,
  "max_generation": 12,
  "total_energy": 28450.0,
  "total_reproductions": 67,
  "total_predations": 15,
  "total_symbiogenesis": 2,
  "total_sexual_repro": 8
}
```

## 🎮 Controls

### Desktop (Mouse + Keyboard)

| Input | Action |
|---|---|
| **Right/Middle mouse drag** | Rotate camera |
| **Scroll wheel** | Zoom in/out |
| **W/A/S/D** | Pan camera |
| **Q/E** | Move camera up/down |
| **Space** | Pause/Play |
| **1/2/3/4** | Speed 1×/5×/10×/20× |
| **H/C/I/E** | Toggle HUD/Charts/Inspector/Events |
| **R** | Reset simulation |

### Mobile (Touch)

| Gesture | Action |
|---|---|
| **1-finger drag** | Rotate camera |
| **Pinch** | Zoom in/out |
| **2-finger drag** | Pan camera |

## 🏗️ Architecture

```
genesis-native/
├── src/main.rs                 # App entry point
├── crates/
│   ├── genesis-sim/            # Core simulation engine
│   │   ├── components.rs       # ParticleType, CellRole
│   │   ├── config.rs           # SimConfig
│   │   ├── resources.rs        # SimStats, EventLog, etc.
│   │   ├── particle_store.rs   # SoA particle data
│   │   └── systems/            # 14 simulation systems
│   ├── genesis-render/         # Bevy 3D rendering + camera
│   └── genesis-ui/             # egui HUD & panels
└── .github/workflows/
    ├── ci.yml                  # Build on every push
    └── release.yml             # Auto-release on tags
```

## 📦 Releases

Binaries are automatically built for Windows, macOS, and Linux when a version tag is pushed:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This creates a GitHub Release with downloadable binaries.

## 📄 License

MIT
