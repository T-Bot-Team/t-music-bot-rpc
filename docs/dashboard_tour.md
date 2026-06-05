# Web Dashboard & Settings Tour

T_Music_Bot RPC features a real-time, responsive web-based configuration dashboard. To access it, run the app and navigate to **`http://localhost:3000/settings`** (or your custom port) in any browser.

This guide provides a tour of the dashboard's layout, key features, and configuration settings.

---

## 1. Top Panel: Real-Time Connection Status
At the top of the dashboard is the status bar:
- **Discord RPC Bridge**: Displays `Connected` when successfully linked to your Discord client's presence pipeline.
- **Music State**: Shows your current uploader track activity (`Playing`, `Paused`, `Idle`, or `Offline`).
- **Websocket Sync**: Reflects the active WebSocket links feeding real-time audio spectrum data.

---

## 2. Core Setup: Discord Pairing
Before the application can update your Discord profile, it must be paired with your user account:
- **Pairing Code Field**: Enter the 6-digit one-time pairing code from Discord (obtained by typing `/rpc connect` in a server with **T_Music_Bot**).
- **Auto-Link Action**: Once typed, the backend automatically detects the token exchange, saves a secure session token to `settings.json`, and clears the pairing code input for security.
- **Discord User ID (Numeric)**: Displays your auto-detected Discord User ID.

---

## 3. Visualizer Settings Card
This card controls the native high-performance CPAL audio capture loop:
- **Visualizer Enabled Toggle**: Turns the audio spectrum capture thread on or off. When disabled, the audio capture loop shuts down completely to consume 0% CPU.
- **Audio Device Dropdown**: Lists all active input and output audio hardware. Select your target device (e.g. *VB-Audio Cable*) to route audio to the FFT processor.
- **Target FPS**: Sets the rendering refresh rate (`30` to `240` FPS).
- **Bars Slider**: Customize the visualizer frequency resolution (from `1` to `2048` individual bars).
- **FFT Sample Size**: Configures the frequency processing window (`512` to `16384` samples).
  - *Tip*: Lower sizes (e.g., `1024` or `2048`) give extremely fast, snappy responsiveness. Higher sizes (e.g., `8192`) give precise, highly-resolved sub-bass frequencies.

---

## 4. Visual Styling & Color Shuffling
- **Shape Mode Selector**: Choose your visualizer style:
  - `bars`: Classic Frequency Bars.
  - `center-bars`: Pulsing Center Split.
  - `wave`: Fluid Liquid Waveform.
  - `neon`: Hollow Neon Outlines.
  - `led`: LED Dot Matrix.
  - `particles`: Floating Particles.
  - `sides`: Edges: Left & Right.
  - `background`: Full Background.
  - `mirrored`: Edges: Top & Bottom.
- **Color Profiles**:
  - **Static**: Locks the visualizer to `Color Top` and `Color Bottom` gradient values.
  - **Random Shuffle**: Randomizes visualizer and border gradient colors once on page load/config changes.
  - **Global Sync**: Automatically extracts dominant color palettes from the active track's cover art, dynamically styling the visualizer, title text, and borders.

---

## 5. Overlay Layout & URL Parameter Overrides
- **Layout Modes**:
  - `full`: Full cinematic widescreen display with background covers, FFT bars, and media details.
  - `compact-bar`: Constrained horizontal bar designed to sit at the bottom of streams.
  - `compact-minimal`: Ultra-clean coverless mode showing only the uploader avatar, title, and artist.
- **URL Parameter Overrides**:
  You can override the layout mode dynamically in OBS or browser tabs by appending query parameters to the URL:
  - `http://localhost:3000/?layout=compact-bar`
  - `http://localhost:3000/?layout=compact-minimal`
  
  This allows running multiple layout overlays concurrently across different scenes in OBS Studio without changing the default layout in your global dashboard settings!
