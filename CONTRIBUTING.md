# Contributing to T_Music_Bot RPC

Thank you for your interest in contributing to T_Music_Bot RPC! We welcome pull requests, bug reports, and suggestions to make this client even better.

Please take a moment to review this document to understand our workflow and development guidelines.

---

## 🛠️ Getting Started

### Prerequisites
To build and run this project, you need the **Rust toolchain** and build dependencies:
- **Rustup**: The official installer for Rust. You can download it from [rustup.rs](https://rustup.rs/).
- **Cargo**: Included with Rustup (acts as the package manager and build tool).
- **Windows**: Visual Studio C++ Build Tools installed.
- **macOS**: Xcode Command Line Tools installed (`xcode-select --install`).
- **Linux**: ALSA and GTK development libraries installed (`libasound2-dev libgtk-3-dev`).

### Quick Start Setup
1. Clone this repository to your local machine.
2. Build the project using Cargo:
   ```bash
   cargo build
   ```
3. Run the development build:
   ```bash
   cargo run
   ```

*For more details on codebase architecture, UI testing, and compiling binaries, refer to the [Compiling Guide](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Compiling-Guide).*

---

## 📜 Coding Standards & Guidelines

To maintain code quality, security, and performance, all contributions must adhere to the following rules:

### 1. Rust Best Practices
- **Idiomatic Rust**: Write clean, readable code. Avoid overly clever or nested abstractions when a simpler approach is clearer.
- **Safety**: Ensure there are no memory leaks or resource leaks. Use smart pointers, and avoid unsafe code unless absolutely necessary for low-level OS interaction.
- **No Git Access**: Do not execute git commands inside the application code or modify files inside the `.git` directories.

### 2. Frontend Development (HTML/CSS/JS)
- **Vanilla Only**: Use standard, plain vanilla HTML, CSS, and JavaScript.
- **No Tailwind CSS**: Avoid Tailwind CSS or utility-first frameworks unless explicitly requested. Maximize performance using vanilla CSS styles.
- **Optimized Rendering**: Keep execution paths in high-frequency rendering loops (e.g. visualizer anim loops) extremely clean. Avoid thrashing the DOM (e.g. cache layout values like `getBoundingClientRect()` outside render frames).

---

## 🧑‍💻 Development Workflow & Verification

Before submitting a Pull Request, verify that your changes build and follow the style guidelines:

### 1. Code Formatting
Make sure your Rust code is properly formatted according to style guidelines:
```bash
cargo fmt --all
```
You can check formatting without applying changes:
```bash
cargo fmt --all --check
```

### 2. Code Quality & Linting
Run Rust's linter (Clippy) to catch common mistakes and code smell:
```bash
cargo clippy --all-targets --all-features
```

### 3. Build Checks
Always run `cargo check` and verify that the release build works:
```bash
cargo check
cargo build --release
```

---

## 📥 Submitting a Pull Request

1. **Create a Branch**: Create a feature branch off of the main branch.
2. **Commit Atomically**: Keep your commits small and focused on a single logical change.
3. **Verify Locally**: Run formatting, lint, and build checks locally before pushing.
4. **Open a PR**: Open a Pull Request on GitHub, fill out the pull request template, and wait for feedback!
