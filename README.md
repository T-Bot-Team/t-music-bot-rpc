# T_Music_Bot RPC Client v1.1.0 (LTS)

A high-performance, type-safe Discord Rich Presence client for T_Music_Bot featuring a low-latency OBS visualizer overlay. This version (v1.1.0) is the final Long Term Support release for the Node.js implementation.

## ✨ Key Features & Improvements

### 🚀 Core Architecture
- **Full TypeScript Migration:** The entire project has been rewritten in TypeScript, providing 100% logic stability and a robust blueprint for future native ports.
- **Improved State Management:** Advanced detection for Paused, Streaming (LIVE), and Status (Resting) states to ensure accurate presence and overlay data.
- **Enhanced Connection Logic:** Standardized headers and detailed handshaking diagnostics to ensure reliable connections when running as a standalone executable.

### 🎨 Smart Thumbnail Engine
- **MaxRes Upgrade:** Automatically upgrades YouTube thumbnails to **1280x720 (maxresdefault)** quality regardless of the source resolution.
- **Smart Cropping:** Implements a CSS-based **12.5% Smart Crop** to remove legacy black bars from 4:3 thumbnails (SD/HQ), ensuring a clean 16:9 cinematic look.
- **Instant Loading:** Optimized image handling with `eager` priority loading for immediate appearance on track changes.

### 📺 High-Performance Visualizer
- **Ultra-Low Latency:** Uses a **Dynamic Hop Size** that scales with your target FPS (supporting up to 240Hz monitors).
- **"Catch-Up" Logic:** Aggressive buffer management prevents visualizer lag, ensuring the bars always sync perfectly with the audio.
- **Strict UI Hard-Lock:** CSS-level overrides ensure the visualizer and live indicators are physically hidden when the player is paused or idle.

### 📦 Optimized Distribution
- **Auto-FFmpeg Setup:** Native system prompt detects missing FFmpeg and offers to download a minimal binary automatically—no manual installation required.
- **Size Optimization:** Executable size reduced to **~40MB** using Brotli compression and surgical asset pruning.
- **LTS Stability:** This branch is considered feature-complete and optimized for long-term use.

## 🚀 Installation
1. Download the latest `t-music-bot-rpc.exe` from the [Releases](https://github.com/TehPig/t-music-bot-rpc/releases) page.
2. Run the executable.
3. If FFmpeg is missing, click **Yes** on the automatic setup prompt to enable the visualizer.

## 🛠 Configuration
The application generates a `settings.json` on the first run.
- **audioDevice:** Set this to your system output (Run with `--list` to see available names).
- **overlay.port:** Default is `3000`. Access your overlay at `http://localhost:3000`.

## 💻 Development
```bash
npm install
npm run build    # Compile & Minify
npm run package  # Generate standalone LTS binary
```

## 📝 License
MIT
