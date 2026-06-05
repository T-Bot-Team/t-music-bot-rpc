# Graph Report - t-music-bot-rpc-rust  (2026-06-06)

## Corpus Check
- 51 files · ~113,934 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 674 nodes · 851 edges · 57 communities (47 shown, 10 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 21 edges (avg confidence: 0.88)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `9b0f1504`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- [[_COMMUNITY_TypeScript IPC Handlers|TypeScript IPC Handlers]]
- [[_COMMUNITY_Pickr JS Color Library|Pickr JS Color Library]]
- [[_COMMUNITY_Rust Config & Domain Models|Rust Config & Domain Models]]
- [[_COMMUNITY_Dashboard Settings UI Logic|Dashboard Settings UI Logic]]
- [[_COMMUNITY_Overlay Visual Settings Schema|Overlay Visual Settings Schema]]
- [[_COMMUNITY_Overlay Visualizer Settings Schema|Overlay Visualizer Settings Schema]]
- [[_COMMUNITY_Legacy Default Settings Schema|Legacy Default Settings Schema]]
- [[_COMMUNITY_Legacy Active Settings Schema|Legacy Active Settings Schema]]
- [[_COMMUNITY_Core Architecture System Nodes|Core Architecture System Nodes]]
- [[_COMMUNITY_Axum HTTP Route Handlers|Axum HTTP Route Handlers]]
- [[_COMMUNITY_TypeScript Config Settings|TypeScript Config Settings]]
- [[_COMMUNITY_Rust System Utility Helpers|Rust System Utility Helpers]]
- [[_COMMUNITY_Global System Settings File|Global System Settings File]]
- [[_COMMUNITY_Setup Wizard UI Logic|Setup Wizard UI Logic]]
- [[_COMMUNITY_Discord IPC Named Pipes|Discord IPC Named Pipes]]
- [[_COMMUNITY_Legacy TypeScript codebase|Legacy TypeScript codebase]]
- [[_COMMUNITY_Audio Visualizer Smoothing Filter|Audio Visualizer Smoothing Filter]]
- [[_COMMUNITY_Overlay Visual Design Components|Overlay Visual Design Components]]
- [[_COMMUNITY_FFT Audio Processor|FFT Audio Processor]]
- [[_COMMUNITY_Visualizer DSP & Math Concepts|Visualizer DSP & Math Concepts]]
- [[_COMMUNITY_Rust System Tray UI|Rust System Tray UI]]
- [[_COMMUNITY_Axum HTTP Web Server|Axum HTTP Web Server]]
- [[_COMMUNITY_JavaScript RPC Tester|JavaScript RPC Tester]]
- [[_COMMUNITY_Discord RPC Client Thread|Discord RPC Client Thread]]
- [[_COMMUNITY_Legacy DSP Bands Analysis|Legacy DSP Bands Analysis]]
- [[_COMMUNITY_CICD Workflows & Docs|CI/CD Workflows & Docs]]
- [[_COMMUNITY_Setup Wizard Static Page|Setup Wizard Static Page]]
- [[_COMMUNITY_Settings Dashboard Static Page|Settings Dashboard Static Page]]
- [[_COMMUNITY_Legacy TypeScript Constants|Legacy TypeScript Constants]]
- [[_COMMUNITY_Agent Core Rules Guidelines|Agent Core Rules Guidelines]]
- [[_COMMUNITY_Community 45|Community 45]]
- [[_COMMUNITY_Community 46|Community 46]]
- [[_COMMUNITY_Community 47|Community 47]]
- [[_COMMUNITY_Community 48|Community 48]]
- [[_COMMUNITY_Community 49|Community 49]]
- [[_COMMUNITY_Community 50|Community 50]]
- [[_COMMUNITY_Community 51|Community 51]]
- [[_COMMUNITY_Community 52|Community 52]]
- [[_COMMUNITY_Community 53|Community 53]]
- [[_COMMUNITY_Community 56|Community 56]]

## God Nodes (most connected - your core abstractions)
1. `E` - 31 edges
2. `visualizer` - 22 edges
3. `overlay` - 19 edges
4. `visualizer` - 17 edges
5. `visualizer` - 17 edges
6. `el()` - 13 edges
7. `loadSettings()` - 12 edges
8. `compilerOptions` - 11 edges
9. `3. Linux: Using PipeWire / PulseAudio Virtual Sinks` - 10 edges
10. `resetSection()` - 9 edges

## Surprising Connections (you probably didn't know these)
- `handlers/server.ts` --references--> `src/setup.html`  [INFERRED]
  typescript/handlers/server.ts → src/setup.html
- `Cargo Windows Build Configuration` --conceptually_related_to--> `Main Runtime Orchestrator`  [INFERRED]
  build.rs → src/main.rs
- `Discord RPC JS Client Tester` --conceptually_related_to--> `Discord RPC worker Context`  [INFERRED]
  tester.js → src/rpc/mod.rs
- `Config IO Engine` --shares_data_with--> `Settings Persistence Store`  [EXTRACTED]
  src/config.rs → settings.json
- `Axum Web Application Server` --references--> `Pickr Color Chooser Library`  [EXTRACTED]
  src/server/mod.rs → assets/pickr.min.js

## Hyperedges (group relationships)
- **Discord Rich Presence System** — rpc_background_worker, client_pairing_auth, ipc_named_pipe_client, logic_presence_formatter [INFERRED 0.95]
- **Axum HTTP/WS Server Subsystem** — server_axum_router, handlers_http_endpoints, templates_widget_engine [INFERRED 0.95]
- **Application Settings and UI Subsystem** — config_settings_io, gui_settings_dashboard, setup_setup_wizard, settings_config_data [INFERRED 0.95]
- **CPAL Audio FFT Visualizer Subsystem** — capture_cpal_recorder, fft_dsp_analyzer [INFERRED 0.95]
- **Rust Audio Capture & Spatial Processing Pipeline** — visualizer_mod, visualizer_processor, visualizer_smoothing [INFERRED 0.95]
- **TypeScript Legacy Audio Capture Engine** — ts_handlers_visualizer, ts_handlers_server, ts_utils_constants [INFERRED 0.85]
- **Music Visualizer Layout Comparisons** — image_dark_minimal_theme, image_rounded_gradient_theme [EXTRACTED 1.00]

## Communities (57 total, 10 thin omitted)

### Community 0 - "TypeScript IPC Handlers"
Cohesion: 0.06
Nodes (29): destroyRPC(), formatTrackData(), lastMessageTime, shutdown(), state, updateActivity(), updateDiscordActivity(), broadcast() (+21 more)

### Community 1 - "Pickr JS Color Library"
Cohesion: 0.09
Nodes (14): _(), a(), b(), c(), E, g(), i, k() (+6 more)

### Community 2 - "Rust Config & Domain Models"
Cohesion: 0.07
Nodes (32): BackgroundConfig, default_animation_speed(), default_bar_gap(), default_bar_width(), default_bars(), default_bg_color(), default_bg_opacity(), default_color_bottom() (+24 more)

### Community 3 - "Dashboard Settings UI Logic"
Cohesion: 0.10
Nodes (38): applyConditionalVisibility(), applyPreset(), buildUIState(), cards, checkActivePreset(), checkbox, checkDirtyState(), checkStatus() (+30 more)

### Community 4 - "Overlay Visual Settings Schema"
Cohesion: 0.05
Nodes (43): allowFirewall, color, enabled, useGradient, bottom, isStatic, random, top (+35 more)

### Community 5 - "Overlay Visualizer Settings Schema"
Cohesion: 0.08
Nodes (27): gradient, bottom, isStatic, random, top, visualizer, animationSpeed, audioDevice (+19 more)

### Community 6 - "Legacy Default Settings Schema"
Cohesion: 0.09
Nodes (22): code, overlay, enabled, port, visualizer, userId, audioDevice, barGap (+14 more)

### Community 7 - "Legacy Active Settings Schema"
Cohesion: 0.09
Nodes (22): code, overlay, enabled, port, visualizer, userId, audioDevice, barGap (+14 more)

### Community 8 - "Core Architecture System Nodes"
Cohesion: 0.15
Nodes (21): Cargo Windows Build Configuration, cpal Low-level audio recorder Stream, Discord Auth client Bridge, Config IO Engine, Windows Firewall exception Configurator, Settings control dashboard Web App, HTTP Web and WS handlers, Discord Local IPC pipe client (+13 more)

### Community 9 - "Axum HTTP Route Handlers"
Cohesion: 0.14
Nodes (8): handle_socket(), merge_json(), overlay_handler(), update_settings_handler(), ws_handler(), get_html(), get_widget_scripts(), get_widget_styles()

### Community 10 - "TypeScript Config Settings"
Cohesion: 0.14
Nodes (13): compilerOptions, esModuleInterop, forceConsistentCasingInFileNames, module, moduleResolution, outDir, resolveJsonModule, rootDir (+5 more)

### Community 11 - "Rust System Utility Helpers"
Cohesion: 0.21
Nodes (7): check_for_updates(), check_lock(), format_overlay_track(), get_log_path(), log_message(), process_thumbnail(), show_error_popup()

### Community 12 - "Global System Settings File"
Cohesion: 0.15
Nodes (12): 1. Adding the Browser Source, 2. Layout Viewport Customization, 3. High Performance Optimization in OBS, 4. Custom CSS Overlay Styling, Clean, No-Progress Overlay:, code:css (body { background: transparent !important; }), code:css (#widget {), Compact Bar Layout: (+4 more)

### Community 13 - "Setup Wizard UI Logic"
Cohesion: 0.33
Nodes (9): checkStatus(), el(), fetchDevices(), overlayOpts, saveSettings(), toggleOverlayOptions(), toggleVizOptions(), val (+1 more)

### Community 14 - "Discord IPC Named Pipes"
Cohesion: 0.51
Nodes (5): attempt_connect(), clear_activity(), IpcClient, scan_discord_clients(), setup_pipe()

### Community 15 - "Legacy TypeScript codebase"
Cohesion: 0.22
Nodes (9): Discord Activity Metadata Scrubbing, src/setup.html, handlers/rpc.ts, handlers/server.ts, handlers/visualizer.ts, typescript/index.ts, lib/logger.ts, lib/tray.ts (+1 more)

### Community 17 - "Overlay Visual Design Components"
Cohesion: 0.43
Nodes (7): Album Art with T-Music Logo Badge, Audio Visualizer Component, Dark Minimalist Theme, Performance Metric (FPS Counter), Metadata Display (Title & Artist), Progress Bar Component, Rounded Gradient Card Theme

### Community 19 - "Visualizer DSP & Math Concepts"
Cohesion: 0.33
Nodes (6): Monstercat Spatial Smoothing, Rainmeter Temporal Smoothing Formula, Smart Pause/Wake Visualizer Stream, visualizer/mod.rs, visualizer/processor.rs, visualizer/smoothing.rs

### Community 20 - "Rust System Tray UI"
Cohesion: 0.60
Nodes (4): build_menu(), create_tray(), TrayHandle, update_tray_status_direct()

### Community 22 - "JavaScript RPC Tester"
Cohesion: 0.50
Nodes (3): activity, client, RPC

### Community 28 - "CI/CD Workflows & Docs"
Cohesion: 0.07
Nodes (36): workflows/release.yml, 1. Overlay Layout Modes, 2. Audio Spectrum & FFT Engine, 3. Dynamic Color Shuffling & Theme Engine, Build Steps, Building from Source, code:bash (git clone https://github.com/TehPig/t-music-bot-rpc.git), code:bash (cargo check) (+28 more)

### Community 44 - "Agent Core Rules Guidelines"
Cohesion: 0.40
Nodes (4): 🛑 ABSOLUTE RULES (CRITICAL - DO NOT IGNORE), 📜 Architecture & Coding Standards, 🤖 Operational Protocols & Verification, 🧠 Project Context & Core Directives

### Community 47 - "Community 47"
Cohesion: 0.06
Nodes (44): 1. Using Voicemeeter Mixer (Windows-only), 1. Windows: Using Voicemeeter Mixer, 2. macOS: Using BlackHole Loopback, 2. Using VB-Audio Virtual Cable (Windows / macOS), 2. Using VB-Audio Virtual Cable (Windows / macOS - Mixerless Setup), 3. Linux: Using PipeWire / PulseAudio Virtual Sinks, Audio Routing & Isolation Guide, Bus and Device Mappings (+36 more)

### Community 48 - "Community 48"
Cohesion: 0.25
Nodes (7): 1. Top Panel: Real-Time Connection Status, 2. Core Setup: Discord Pairing, 3. Visualizer Settings Card, 4. Visual Styling & Color Shuffling, 5. Overlay Layout Configurations, 5. Overlay Layout & URL Parameter Overrides, Web Dashboard & Settings Tour

### Community 49 - "Community 49"
Cohesion: 0.29
Nodes (6): 1. Core Metrics Benchmark Summary, 2. Architectural Comparison, 3. Web Overlay Render Optimization (DOM Reflow & Caching), Architectural & Performance Comparison: TS vs. Rust, Legacy TypeScript (Node.js) Setup, Native Rust Setup

### Community 50 - "Community 50"
Cohesion: 0.25
Nodes (8): Step 1: Install Prerequisites, Step 2: Configure Windows Default Audio, Step 3: Route Your Music Player, Step 4: Configure Voicemeeter mixing, Step 5: Bind the RPC Client Visualizer, Success!, The Goal, Voicemeeter & Virtual Cable Routing Guide

### Community 51 - "Community 51"
Cohesion: 0.10
Nodes (19): 1. Prerequisites, 2. Initial Setup, Apply Code Formatting, Check Syntax & Borrow Checker, 🏗️ Codebase Architecture, code:bash (cargo build), code:bash (cargo run), code:bash (cargo check) (+11 more)

### Community 52 - "Community 52"
Cohesion: 0.11
Nodes (18): 1. Code Formatting, 1. Rust Best Practices, 2. Code Quality & Linting, 2. Frontend Development (HTML/CSS/JS), 3. Build Checks, code:bash (cargo build), code:bash (cargo run), code:bash (cargo fmt --all) (+10 more)

### Community 53 - "Community 53"
Cohesion: 0.50
Nodes (3): Checklist, Description, Type of Change

### Community 56 - "Community 56"
Cohesion: 0.10
Nodes (19): 1. Prerequisites, 2. Initial Setup, Apply Code Formatting, Check Syntax & Borrow Checker, 🏗️ Codebase Architecture, code:bash (cargo build), code:bash (cargo run), code:bash (cargo check) (+11 more)

## Knowledge Gaps
- **264 isolated node(s):** `code`, `userId`, `sessionToken`, `1504684558065471506`, `298432708269441034` (+259 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **10 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `overlay` connect `Overlay Visual Settings Schema` to `Overlay Visualizer Settings Schema`?**
  _High betweenness centrality (0.009) - this node is a cross-community bridge._
- **Why does `visualizer` connect `Overlay Visualizer Settings Schema` to `Overlay Visual Settings Schema`?**
  _High betweenness centrality (0.006) - this node is a cross-community bridge._
- **What connects `code`, `userId`, `sessionToken` to the rest of the system?**
  _269 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `TypeScript IPC Handlers` be split into smaller, more focused modules?**
  _Cohesion score 0.06079664570230608 - nodes in this community are weakly interconnected._
- **Should `Pickr JS Color Library` be split into smaller, more focused modules?**
  _Cohesion score 0.08673469387755102 - nodes in this community are weakly interconnected._
- **Should `Rust Config & Domain Models` be split into smaller, more focused modules?**
  _Cohesion score 0.0666049953746531 - nodes in this community are weakly interconnected._
- **Should `Dashboard Settings UI Logic` be split into smaller, more focused modules?**
  _Cohesion score 0.0951219512195122 - nodes in this community are weakly interconnected._