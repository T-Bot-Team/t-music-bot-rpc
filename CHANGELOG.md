# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.1.1] - 2026-11-24

### Tray & Audio Capture
- **Active Device Display**: Added a dedicated status item (`Audio: <Device Name>`) directly visible in the tray menu that updates dynamically whenever the device changes.
- **Unmistakable Selection Indicator**: Added visual `✓  ` checkmark prefix directly into the tray menu item text, guaranteeing immediate visibility across all desktop environments (GNOME AppIndicator, KDE, XFCE, Windows, macOS).
- **Desktop/Playback Exclusivity**: Filtered out all microphone and recording devices from the visualizer capture list, listing only desktop audio monitors and playback loopback sinks.
- **Tray Exclusivity**: Dedicated audio device selection exclusively to the system tray menu, removing redundant selectors from the web dashboard.

### Audio Bus & Settings
- **Cross-Platform Availability**: Made "Music Volume (What You Hear)" available across all platforms (Linux, Windows, macOS) with robust state synchronization.
- **Settings UI Polish**: Cleaned up obsolete toggles, enhanced custom select dropdowns with optgroups, keyboard accessibility (`:focus-visible`), and styled scrollbars.
- **Overlay Refinements**: Added system font fallbacks, bounds clamping, and dynamic color accent border support.

## [2.1.0] - 2026-11-23

### Packaging
- **Windows**: Packaged as `T_Music_Bot_RPC.zip` with embedded PE version/icon resource metadata (`FileVersion` 2.1.0.0, `ProductVersion` 2.1.0.0) and autostart/tray documentation.
- **macOS**: Packaged into `.app` bundles with `LSUIElement` (tray-only mode) and `.dmg` installers for both Intel (`x86_64`) and Apple Silicon (`aarch64`). Added ad-hoc/Developer ID codesigning and Gatekeeper right-click launch instructions.
- **Linux**: Packaged as `T_Music_Bot-RPC.tar.gz` including `.desktop` launcher and 256x256 application icon. Documented runtime dependencies (`gtk3`, `libpulse`, `openssl`) and GNOME AppIndicator extension requirements.

### Stability & Robustness
- **Error Handling**: Audited and eliminated unwrap/expect panic paths across non-test codebase.
  - Corrupted or unreadable `settings.json` now safely falls back to defaults without crashing.
  - Missing or disconnected audio devices gracefully degrade to "no visualizer" mode without crashing the process.
  - Handled WebSocket client closures and server port conflicts gracefully with backoff retry rather than hot restart loops.
  - Discord IPC packet parser now enforces safe 16MB maximum length limits against malformed packets.
- **Clippy**: Resolved all warnings across all targets to enforce a strict `-D warnings` build gate.

### Updater
- Wired up `self-replace` binary replacement on all three platforms.
- Preserved strict version checking (`is_newer_release`) to prevent older or equal releases from re-downloading.
- Ensured atomic binary replacement: failed or partial downloads leave the running binary completely untouched with zero destructive deletions.
- Fixed platform asset detection to correctly match Apple Silicon, Intel, Linux, and Windows executables while ignoring archive files (`.zip`, `.dmg`, `.tar.gz`).

### Performance
- Verified idle CPU utilization stays `< 0.2%` (well below 1% threshold).
- Verified memory footprint remains completely flat at `~40 MB` RSS with zero drift.
