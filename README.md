# T_Music_Bot RPC

A lightweight, cross-platform Discord Rich Presence (RPC) client for **T_Music_Bot**. This client displays your current music activity from the bot directly on your Discord profile and provides a high-quality OBS visualizer overlay.

## Features

- **Cross-Platform:** Native support for Windows and Linux.
- **Lightweight:** Minimal CPU and memory usage (~40MB executable).
- **Auto-Setup:** Easy pairing process via Discord with automatic 6-digit code detection.
- **GUI & Tray:** Integrated system tray with a clean setup interface and real-time status updates.
- **OBS Visualizer Overlay:** Amazing-looking 1360x440 audio-responsive overlay with customizable modes (classic, wave, neon, etc.).

## Installation

### For Users
Download the latest version for your operating system from the [Releases](https://github.com/TehPig/t-music-bot-rpc/releases) page.

#### Windows
1. Run `t-music-bot-rpc.exe`.
2. Follow the GUI instructions to pair with Discord.
3. If prompted, click **Yes** to automatically install the Visualizer component.
4. The app will minimize to your system tray.

#### Linux
1. Give the binary execution permissions: `chmod +x t-music-bot-rpc-linux`.
2. Run the binary.
3. **Note:** On Linux, the app will automatically background itself. Use the `--foreground` flag if you want to keep it in the terminal.
4. **Note:** On some Linux environments, you may need to manually enter your Discord User ID in `settings.json` if automatic detection fails.

---

## Configuration Guide

The `settings.json` file is generated automatically on the first run. Below are the available options and their valid ranges:

### Core Settings
| Key | Type | Description |
| :--- | :--- | :--- |
| `code` | String | The 6-digit pairing code from Discord (`/rpc connect`). |
| `userId` | String | Your Discord User ID (Numeric). Auto-detected on most systems. |

### Overlay Settings (`overlay`)
| Key | Default | Description |
| :--- | :--- | :--- |
| `enabled` | `false` | Enable/Disable the browser overlay server. |
| `port` | `3000` | The local port used to access the overlay. |

### Visualizer Settings (`overlay.visualizer`)
| Key | Range | Default | Description |
| :--- | :--- | :--- | :--- |
| `enabled` | `true/false` | `false` | Enable audio capture and rendering. |
| `audioDevice` | String | `""` | Exact name of your audio output (Use `--list` to find). |
| `fps` | `30 - 240` | `60` | Target refresh rate for the animation. |
| `samples` | `512 - 16384`| `2048` | FFT Sample size. Must be a power of 2. |
| `bars` | `1 - 2048` | `64` | Number of frequency bars to display. |
| `smoothing` | `1 - 20` | `3` | How much the bars "lag" behind the audio. |
| `sensitivity` | `1 - 100` | `40` | Responsiveness to quiet sounds. |
| `multiplier` | `1 - 100` | `40` | Visual height multiplier for the bars. |
| `colorTop` | Hex | `#7cf6ff` | The color at the top of the bars. |
| `colorBottom`| Hex | `#1a69a8` | The color at the base of the bars. |
| `mode` | String | `"bars"` | See **Visualizer Modes** below. |
| `rounded` | `true/false` | `true` | Enable rounded corners on bars. |
| `glow` | `true/false` | `false` | Enable neon outer glow (Performance heavy). |

### Visualizer Modes
- `bars`: Standard vertical frequency bars.
- `wave`: Smooth animated waveform.
- `particles`: Floating dots representing frequencies.
- `neon-bars`: Thin, high-contrast neon styling.
- `led`: Segmented block-style frequency meter.
- `outline`: Hollow bar outlines.
- `center-bars`: Bars that grow outwards from the vertical center.

---

## Configuration & Logs

Settings and logs are stored in the same directory as the executable:
- `settings.json`: Stores your pairing code, audio device selection, and overlay preferences.
- `logs.txt`: Contains application logs for troubleshooting.
- `.lock`: A temporary file used to prevent multiple instances from running.

## OBS Setup
To use the overlay in OBS:
1. Add a **Browser Source**.
2. URL: `http://localhost:3000` (port can be changed in settings).
3. Width: `1360`
4. Height: `440`

---

## Command Line Options

- `--list`: List all available audio output devices for the visualizer.
- `--quiet` or `-q`: Disable most logging to console and `logs.txt`.
- `--foreground`: (Linux only) Prevent the app from backgrounding itself.
- `--debug-fft`: Enable detailed FFT analysis logging.

## Building from Source

### Prerequisites
- [Node.js](https://nodejs.org/) (v18.16.0 or higher recommended)

### Steps
1. **Clone the repository:**
   ```bash
   git clone https://github.com/TehPig/t-music-bot-rpc.git
   cd t-music-bot-rpc
   ```

2. **Install dependencies:**
   ```bash
   npm install
   ```

3. **Build the binaries:**
   ```bash
   npm run build    # Compiles TypeScript and prepares dist/ folder
   npm run package  # Generates standalone binaries
   ```

---

## License
[CC BY-NC-ND 4.0](LICENSE)

This project is licensed under the Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International License.

- **Attribution:** You must give appropriate credit.
- **Non-Commercial:** You may not use the material for commercial purposes.
- **No-Derivatives:** If you remix, transform, or build upon the material, you may not distribute the modified material.
