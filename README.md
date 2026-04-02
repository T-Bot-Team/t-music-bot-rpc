# T_Music_Bot RPC Client

A high-performance, standalone Discord Rich Presence client and OBS overlay for T_Music_Bot. Designed for maximum visual quality with minimum resource footprint.

## 🚀 Key Features

### 🎧 Discord Rich Presence
- **Dynamic Presence:** Shows exactly what you are listening to, including track title, artist, and playback status.
- **Smart Auth:** Secure 6-digit pairing code authentication to link with your Discord account.
- **State Logic:** Robust handling of Playing, Paused, and "Resting" (Status) states.

### 📺 OBS Visualizer Overlay
- **Professional Layout:** High-resolution (1360x440) overlay designed for OBS browser sources.
- **Audio-Responsive Visualizer:** Smooth, high-refresh-rate frequency bars (optimized for up to 240Hz).
- **Customizable Modes:** Support for various modes including classic bars, wave, center-bars, and neon.
- **Low Latency:** Real-time audio processing with dynamic hop-sizing to ensure zero delay between music and visuals.

### 🖼️ Smart Thumbnail Engine
- **MaxRes Quality:** Automatically forces YouTube thumbnails to **1280x720 (maxresdefault)** for the sharpest possible look.
- **Smart Crop (12.5% Rule):** Programmatically removes black bars from legacy 4:3 YouTube thumbnails, providing a clean 16:9 cinematic experience.
- **Instant Loading:** Zero-delay image rendering ensures thumbnails appear the moment the track changes.

### 🛠️ Intelligent System Integration
- **Auto-FFmpeg Setup:** Detects if audio capture dependencies are missing and offers a native one-click automatic installation.
- **System Tray Controller:** Runs in the background with a system tray icon showing real-time WebSocket and RPC connection status.
- **Portable & Lean:** Standalone executable (~40MB) with no external dependencies required for the core RPC.

## 📦 Installation & Usage
1. Download the latest `t-music-bot-rpc.exe` from the **Releases** tab.
2. Run the application.
3. **Setup:**
   - If prompted, enter the 6-digit pairing code from Discord (`/rpc connect`).
   - If you want the visualizer, click **Yes** on the automatic FFmpeg setup prompt.
4. **OBS Setup:** Add a "Browser Source" pointing to `http://localhost:3000` with width `1360` and height `440`.

## ⚙️ Configuration
The `settings.json` file allows for deep customization:
- `audioDevice`: The exact name of your system audio output (Run with `--list` to see options).
- `visualizer.samples`: Buffer size for audio analysis (Default: 2048).
- `visualizer.bars`: Number of frequency bands to display (Default: 64).
- `overlay.port`: The local port for the browser overlay.

## 💻 Development
Built with TypeScript for rock-solid stability and type-safe state management.
```bash
npm install      # Install dependencies
npm run build    # Compile TypeScript
npm run package  # Generate standalone binaries
```

## 📝 License
MIT
