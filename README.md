# 🧬 Genesis Engine v6.1 — Native

A high-performance 3D artificial life simulation built with **Rust** and **Bevy 0.15**.

![CI](https://github.com/aciderix/genesis-native/actions/workflows/ci.yml/badge.svg)

## ✨ Features

- **Particle-based life simulation** — organisms, metabolism, reproduction, colonies
- **Real-time 3D rendering** with Bevy's GPU-accelerated pipeline
- **Interactive UI** via egui (parameter tweaking, stats, controls)
- **Spatial grid** + scalar fields for efficient physics
- **Chemical signals**, bonding, culture & metacognition systems
- **Cross-platform** — Windows, macOS, Linux, Web (WASM)

## 📦 Project Structure

```
genesis-native/
├── src/main.rs                    # Entry point — wires everything together
├── crates/
│   ├── genesis-sim/               # Core simulation engine
│   │   └── src/
│   │       ├── systems/           # ECS systems (forces, metabolism, reproduction…)
│   │       ├── util/              # Spatial grid, scalar fields
│   │       ├── components.rs      # Bevy ECS components
│   │       ├── resources.rs       # Shared simulation resources
│   │       └── config.rs          # Simulation parameters
│   ├── genesis-render/            # 3D rendering (Bevy meshes, materials, camera)
│   └── genesis-ui/                # egui interface panels
├── .github/workflows/
│   ├── ci.yml                     # CI: build on every push (Linux/Win/Mac/WASM)
│   └── release.yml                # Auto-release on version tags
└── Cargo.toml                     # Workspace manifest
```

## 🚀 Quick Start

### Prerequisites

1. **Install Rust** (if not already):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
   On Windows: download [rustup-init.exe](https://rustup.rs)

2. **System dependencies** (for Bevy rendering):

   | Platform | Command |
   |---|---|
   | **macOS** | Nothing extra needed ✅ |
   | **Linux (Ubuntu/Debian)** | `sudo apt install pkg-config libx11-dev libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev` |
   | **Windows** | Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (C++ workload) |

### Build & Run

```bash
git clone https://github.com/aciderix/genesis-native.git
cd genesis-native
cargo run --release
```

> ⚡ **Important**: Always use `--release` — Bevy runs ~10× slower in debug mode.

First build takes ~3–5 minutes (downloading + compiling dependencies). Subsequent builds: ~10 seconds.

## 🌐 Web Build (WASM)

```bash
# Add WASM target
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

# Build
cargo build --release --target wasm32-unknown-unknown

# Generate JS bindings
wasm-bindgen \
  target/wasm32-unknown-unknown/release/genesis.wasm \
  --out-dir web-dist --target web --no-typescript
```

Then serve `web-dist/` with any HTTP server.

## 📱 Mobile (Experimental)

### Android
```bash
rustup target add aarch64-linux-android
cargo install cargo-ndk
cargo ndk -t arm64-v8a build --release
```

### iOS
```bash
rustup target add aarch64-apple-ios
cargo build --release --target aarch64-apple-ios
```

> ⚠️ Mobile requires UI adaptation (touch controls, larger buttons, battery-aware particle count).

## 🏗️ CI/CD

| Workflow | Trigger | What it does |
|---|---|---|
| **CI** | Push / PR to `main` | Builds for Linux, Windows, macOS, WASM |
| **Release** | Push tag `v*` | Builds + uploads binaries to GitHub Releases |

### Creating a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers automatic builds and creates a GitHub Release with downloadable binaries for all platforms.

## 🎮 Controls

| Action | Input |
|---|---|
| Orbit camera | Right-click + drag |
| Zoom | Scroll wheel |
| Pan | Middle-click + drag |
| Toggle UI panels | egui sidebar |

## 📊 Performance

| Target | Relative Speed | Status |
|---|---|---|
| Native (PC) | ⭐⭐⭐⭐⭐ 100% | ✅ Ready |
| Web (WASM) | ⭐⭐⭐ ~60% | ✅ Ready |
| Android | ⭐⭐⭐⭐ ~80% | ⚠️ UI adaptation needed |
| iOS | ⭐⭐⭐⭐ ~80% | ⚠️ UI adaptation + Mac required |

## 📄 License

MIT
