# T_Music_Bot RPC Client v1.1.0

A high-performance Discord Rich Presence client for T_Music_Bot with a built-in visualizer overlay.

## ✨ What's New in v1.1.0
- **TypeScript Migration:** Rewritten in TypeScript for 100% logic stability and type safety.
- **Smart Thumbnail System:**
  - Auto-upgrades YouTube thumbnails to **MaxRes (1280x720)** quality.
  - Implements **Smart Cropping (12.5% rule)** to remove black bars from SD/HQ thumbnails.
  - Instant loading: Thumbnails and avatars appear immediately on refresh.
- **Auto-FFmpeg Setup:** 
  - The app now detects if FFmpeg is missing and offers to download a minimal version automatically.
  - No manual installation required for the visualizer to work.
- **Improved UI Logic:**
  - Perfect centering for "Paused" and "Status" (Resting) modes.
  - Visualizer and "LIVE" indicator are strictly hidden when paused or in status mode.
  - Highly optimized rendering for high-refresh-rate monitors (up to 240Hz).
- **Size Optimization:** Executable size reduced to <50MB using Brotli compression.

## 🚀 Installation
1. Download the latest `t-music-bot-rpc.exe` from the [Releases](https://github.com/TehPig/t-music-bot-rpc/releases) page.
2. Run the executable.
3. If FFmpeg is not found, click **Yes** on the prompt to install the visualizer components automatically.

## 🛠 Configuration
The application generates a `settings.json` on first run.
- **audioDevice:** Set this to your VoiceMeeter or System output (Run with `--list` to see names).
- **overlay.port:** Default is `3000`. Access your overlay at `http://localhost:3000`.

## 💻 Development
```bash
npm install
npm run build    # Compile TypeScript
npm run package  # Generate standalone .exe
```

## 📝 License
MIT
