# T_Music_Bot RPC (Rust)

A blazing-fast, lightweight, and cross-platform (Windows, macOS, Linux) Discord Rich Presence (RPC) and OBS Overlay client for **T_Music_Bot**, rewritten in native **Rust** for maximum performance and efficiency.

---

## 🎨 Layout Modes & Visualizer Features

T_Music_Bot RPC provides a premium, responsive Web UI dashboard and customizable overlays for streaming or desktop enhancement.

### 1. Overlay Layout Modes
Configure overlays at the click of a button to match your stream layout:
- **Full Cinematic Mode**: Gorgeous full-width design featuring uploader details, track metadata, progress indicators, responsive spectrum bars, and a glassmorphic pause overlay.
- **Compact Bar Mode**: A layout that places uploader details side-by-side with visualizer spectrum bars.
- **Compact Minimal Mode**: Space-saving card displaying uploader avatar and current track metadata, perfect for stream corners.

### 2. Audio Spectrum & FFT Engine
A high-accuracy frequency analyzer built directly into the client:
- **Logarithmic & Linear Mapping**: Choose logarithmic bands for natural human hearing weighting (responsive bass and mids) or linear bands for equal frequency distribution.
- **CPAL Audio Loopback**: Low-latency loopback capture that automatically binds to any default or virtual audio hardware.
- **Automated CPU Suspend**: Suspends audio thread capturing (reducing CPU usage to 0%) if no browser overlay or OBS source is active.
- **Attack/Decay Inertia**: Calibrate bar response times (rise/fall speeds) using settings presets:
  - `Reactive`: Fast, instantaneous beats (perfect for high-BPM/electronic tracks).
  - `Balanced`: Clean, standard visualizer inertia.
  - `Smooth`: Gentle, fluid transitions.

### 3. Dynamic Color Shuffling & Theme Engine
- **Global Sync Mode**: Dynamically extracts dominant colors from the active album cover art to colorize uploader texts, canvas bars, borders, and ambient glow backdrops.
- **Static Gradients**: Lock the theme to customized linear gradients.
- **Random Shuffling**: Shuffles custom color palettes automatically on every new track.
- **Neon Glow Overlays**: Adds high-fidelity ambient glow shadows behind visualizer bars.

---

## 📖 Guides & Documentation

To configure isolated audio capturing (on Windows, macOS, or Linux), integrate the visualizer into your setup, or build/contribute from source, refer to the documentation:

* 🔌 **[Audio Isolation & Routing Guide](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Audio-Isolation-&-Routing-Guide)**: Walkthrough for VB-Cable, Voicemeeter, and PipeWire/CoreAudio to isolate music visualizer audio from system sounds, games, and voice calls.
* 🖥️ **[OBS Studio Integration Tour](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/OBS-Studio-Integration-Tour)**: Step-by-step setup for browser sources, viewport scaling, transparency filters, and performance settings.
* 🎛️ **[Dashboard Tour](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Dashboard-Tour)**: Explore the settings control panel, presets, and customized visual styling options.
* ⚡ **[Performance Benchmarks](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Performance-Benchmarks)**: Detailed resource comparison showing CPU/RAM reductions of the native Rust rewrite vs. the legacy TypeScript version.
* 🛠️ **[Compiling Guide](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Compiling-Guide)**: Step-by-step instructions for compiling from source and project codebase walkthrough.
* 🤝 **[Contributing Guidelines](CONTRIBUTING.md)**: Standard workflow rules, code quality guidelines, and PR procedures for contributors.

---

## 🛠️ Configuration Settings

Settings are stored in `settings.json` inside your system's persistent application data directory (which resolves to `%APPDATA%/T_Music_Bot_RPC` on Windows, `~/Library/Application Support/T_Music_Bot_RPC` on macOS, and `~/.config/t-music-bot-rpc` on Linux). You can open this file directly via the **Open settings.json** button on the control center Web Dashboard, or configure all parameters using the UI sliders:

| Section | Key | Default | Description |
| :--- | :--- | :--- | :--- |
| **Core** | `code` | `""` | 6-digit Discord pairing authorization code. |
| **Core** | `userId` | `""` | Numeric Discord User ID (auto-detected on connect). |
| **Overlay** | `port` | `3000` | Web server port for dashboard (`/settings`) and overlay (`/`). |
| **Overlay** | `layout` | `"full"` | Active layout (`"full"`, `"compact-bar"`, or `"compact-minimal"`). |
| **Visualizer** | `enabled` | `false` | Enable/disable CPAL audio loopback visualizer engine. |
| **Visualizer** | `audioDevice` | `"default"`| Input/output loopback device name. |
| **Visualizer** | `fps` | `60` | Spectrum rendering frames per second (`30 - 240`). |
| **Visualizer** | `samples` | `4096` | FFT frequency window size (`512 - 16384`). |
| **Visualizer** | `bars` | `64` | Visualizer frequency bars count. |
| **Visualizer** | `smoothing` | `6` | Inertia/decay inertia coefficient (`1 - 20`). |
| **Visualizer** | `sensitivity`| `45` | Lower threshold in decibels for bar scaling (`10 - 100`). |
| **Visualizer** | `multiplier` | `25` | Height multiplier scaling factor. |

---

## 🚀 Getting Started

If you are an end-user, follow these quick steps:
1. Download the standalone executable for your operating system from the **Releases** page.
2. Run the application.
3. Use the `/rpc connect` slash command on Discord to obtain your pairing code.
4. Input the code in the setup dialog. Once linked successfully, the client will run silently inside your **System Tray**.
5. Double-click the tray icon to open the configuration dashboard (`http://localhost:3000/settings`).

*Note for developers: If you prefer to compile from source or run developer builds, please refer directly to the [Compiling Guide](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Compiling-Guide).*

---

## ⚖️ License
[CC BY-NC-ND 4.0](LICENSE)