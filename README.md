# T_Music_Bot RPC (Rust)

A Discord Rich Presence client and OBS overlay for **T_Music_Bot**. Written in Rust, one codebase for Windows, macOS and Linux, and it stays in your system tray once it's paired. The idea is simple: something you can leave running all day without ever noticing it's there.

---

## Layout modes & visualizer

Everything is configured from the web dashboard at `http://localhost:3000/settings` — double-click the tray icon to open it. The overlay OBS captures has three layouts:

- **Full** — cover art, track info, progress bar and spectrum bars along the bottom. When playback pauses, the background dims behind a blur.
- **Compact bar** — track info side-by-side with the spectrum bars. Sits nicely on the top or bottom edge of a frame.
- **Compact minimal** — a small card with the avatar and track name. Fits in a corner without eating screen space.

### Spectrum engine

The visualizer captures desktop audio through CPAL — WASAPI loopback on Windows, PulseAudio/PipeWire on Linux, CoreAudio on macOS — so it sees whatever your system is already playing, virtual cables like VB-Cable or Voicemeeter included.

- Bands can be spaced logarithmically (matches how ears hear, more resolution down in the bass) or linearly (even spread across the whole range).
- Capture suspends itself when no overlay is listening or nothing is playing, so idle cost is basically nothing.
- Bar inertia has three presets: `Reactive` for fast electronic tracks, `Balanced` as a middle ground, `Smooth` if you'd rather watch slow movement for hours.

### Colors

Take the palette straight from the album cover and the text, bars and borders all follow it. Or lock a static gradient, or shuffle your own palette on every new track. There's a neon glow option behind the bars too — it looks good, but it's the first thing to switch off if your GPU is already busy.

---

## Guides & documentation

The wiki covers everything that reads better as a walkthrough:

* 🔌 **[Audio Isolation & Routing Guide](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Audio-Isolation-&-Routing-Guide)** — keeping games, voice chat and system sounds out of the visualizer using VB-Cable, Voicemeeter, or PipeWire/CoreAudio.
* 🖥️ **[OBS Studio Integration Tour](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/OBS-Studio-Integration-Tour)** — browser source setup, viewport scaling, transparency filters, performance settings.
* 🎛️ **[Dashboard Tour](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Dashboard-Tour)** — what every setting in the control panel actually does.
* ⚡ **[Performance Benchmarks](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Performance-Benchmarks)** — CPU/RAM numbers for this Rust build against the old TypeScript client.
* 🛠️ **[Compiling Guide](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Compiling-Guide)** — building from source, with a walkthrough of the codebase.
* 🤝 **[Contributing Guidelines](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Contributing-Guidelines)** — workflow, code expectations, how to send a PR.

---

## Configuration settings

Settings live in a `settings.json` file: `%APPDATA%/T_Music_Bot_RPC` on Windows, `~/Library/Application Support/T_Music_Bot_RPC` on macOS, `~/.config/t-music-bot-rpc` on Linux. Open it straight from the **Open settings.json** button on the dashboard, or use the sliders — both write to the same file.

| Section | Key | Default | Description |
| :--- | :--- | :--- | :--- |
| **Core** | `code` | `""` | 6-digit Discord pairing authorization code. |
| **Core** | `userId` | `""` | Numeric Discord User ID (auto-detected on connect). |
| **Overlay** | `port` | `3000` | Web server port for dashboard (`/settings`) and overlay (`/`). |
| **Overlay** | `layout` | `"full"` | Active layout (`"full"`, `"compact-bar"`, or `"compact-minimal"`). |
| **Audio** | `manageBus` | `false` | Linux only. Creates a dedicated virtual sink so the visualizer always reads full-volume output, while `listenVolume` controls what you hear — lowering your volume no longer flattens the bars. On Windows and macOS, use VB-Cable/Voicemeeter instead (see the routing guide above). |
| **Audio** | `listenVolume` | `75` | What you hear from the managed bus, in percent (`0 - 150`). |
| **Visualizer** | `enabled` | `false` | Enable/disable CPAL audio loopback visualizer engine. |
| **Visualizer** | `audioDevice` | `"default"`| Capture device id. `"default"` = desktop audio (WASAPI loopback on Windows, default monitor source on Linux, virtual loopback/mic on macOS). Custom values are stable device ids persisted from the picker; devices are displayed Windows-style as `Name (ID)`, and legacy name-based settings still resolve. |
| **Visualizer** | `fps` | `60` | Spectrum rendering frames per second (`30 - 240`). |
| **Visualizer** | `samples` | `4096` | FFT frequency window size (`512 - 16384`). |
| **Visualizer** | `bars` | `64` | Visualizer frequency bars count. |
| **Visualizer** | `smoothing` | `6` | Inertia/decay inertia coefficient (`1 - 20`). |
| **Visualizer** | `sensitivity`| `45` | Lower threshold in decibels for bar scaling (`10 - 100`). |
| **Visualizer** | `multiplier` | `25` | Height multiplier scaling factor. |

---

## Getting started

1. Download the binary for your OS from the **Releases** page:
   * **Windows (64-bit):** `T_Music_Bot_RPC.exe`
   * **Linux (x86_64):** `T_Music_Bot-RPC`
   * **macOS (Intel):** `T_Music_Bot-RPC_Intel`
   * **macOS (Apple Silicon):** `T_Music_Bot-RPC_Apple_Silicon`
2. Run it.
3. On Discord, type `/rpc connect` to get a pairing code.
4. Enter the code in the setup dialog. From then on the client sits quietly in the system tray.
5. Double-click the tray icon whenever you want the dashboard back.

Building from source instead? Head to the [Compiling Guide](https://github.com/T-Bot-Team/t-music-bot-rpc/wiki/Compiling-Guide).

---

## License
[CC BY-NC-ND 4.0](LICENSE)
