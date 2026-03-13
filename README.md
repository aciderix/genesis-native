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
