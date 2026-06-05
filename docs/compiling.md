# Compiling Guide

This guide covers setting up the development environment, the project's codebase architecture, and common development workflows for **T_Music_Bot RPC (Rust)**.

---

## 🛠️ Environment Setup

### 1. Prerequisites
To compile and run this project, you need the **Rust toolchain** and build dependencies:
- **rustup**: Download and run the installer from [rustup.rs](https://rustup.rs/).
- **Windows Only**: Visual Studio Build Tools with C++ compiler packages.
- **macOS Only**: Xcode Command Line Tools installed via `xcode-select --install`.
- **Linux Only**: Development headers for ALSA and GTK (e.g. `libasound2-dev libgtk-3-dev libappindicator3-dev`).

### 2. Initial Setup
Clone the repository and compile the dependencies:
```bash
cargo build
```

---

## 🏗️ Codebase Architecture

The project consists of a native Rust backend that hosts an Axum web server and drives the Discord RPC client + CPAL-based audio capture loop.

### 📂 Directory Layout
- **`src/`**: Rust backend implementation.
  - `main.rs`: Entry point. Initializes state, tray icon, and runtime loops.
  - `models.rs`: Serialization structures and default configuration schemas.
  - `config.rs`: Helper utilities for reading/writing configuration files.
  - `server/`: Axum web server configuration and request handlers.
    - `mod.rs`: Axum router setup and static asset loader.
    - `handlers.rs`: Endpoints for settings update, client discovery, and audio device listing.
    - `templates.rs`: Dynamic canvas rendering and overlay HTML generators.
  - `rpc/`: Discord Rich Presence connection and status synchronization.
  - `visualizer/`: Real-time CPAL audio loopback capture thread and FFT (Fast Fourier Transform) math engine.
  - `ui/`: Embedded Web UI layouts and static styling/scripts.
- **`ui/`**: Root UI source templates (development duplicates/references).
- **`docs/`**: Feature tours, setup guides, and benchmarks.

---

## 🔄 UI Development Workflow

### Static File Embedding
For maximum performance and self-containment, the HTML/CSS/JS frontend files are compiled directly into the final executable using Rust's `include_str!` and `include_bytes!` macros.
- Changing files under `src/ui/` requires recompiling the Rust binary.

To test changes to UI templates or scripts:
1. Edit the source files (e.g., [gui.html](file:///c:/Users/TehPig/Desktop/@Projects/t-music-bot-rpc-rust/src/ui/gui.html) or [gui.css](file:///c:/Users/TehPig/Desktop/@Projects/t-music-bot-rpc-rust/src/ui/assets/gui.css)).
2. Recompile and run:
   ```bash
   cargo run
   ```
3. Refresh your web browser.

---

## 🧑‍💻 Verification & Build Checks

Before making a contribution, verify that everything compiles correctly.

### Check Syntax & Borrow Checker
```bash
cargo check
```

### Apply Code Formatting
```bash
cargo fmt --all
```

### Run Linter
```bash
cargo clippy --all-targets --all-features
```

### Create Release Binary
To compile the highly optimized standalone executable:
```bash
cargo build --release
```
The compiled output is located at: `target/release/t-music-bot-rpc.exe` (on Windows).
