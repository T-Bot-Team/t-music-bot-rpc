use crate::{info, AppState, OverlayConfig, Settings, TrackUpdate};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct ServerHandle {
    pub tx: broadcast::Sender<String>,
}

pub async fn start_server(state: AppState) {
    let (port, tx) = {
        let s = state.read().await;
        (
            s.settings.as_ref().unwrap().overlay.port,
            s.overlay_tx.clone(),
        )
    };

    let server_handle = Arc::new(ServerHandle { tx });

    let app = Router::new()
        .route("/", get(overlay_handler))
        .route("/ws", get(ws_handler))
        .route("/settings", get(settings_gui_handler))
        .route("/setup", get(setup_gui_handler))
        .route(
            "/api/settings",
            get(get_settings_handler).post(update_settings_handler),
        )
        .route("/api/devices", get(get_devices_handler))
        .route("/assets/:path", get(asset_handler))
        .with_state((state.clone(), server_handle));

    let addr = format!("127.0.0.1:{}", port);
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            info!("[Server] Overlay active on http://{}", addr);
            axum::serve(listener, app).await.unwrap();
        }
        Err(e) => {
            let err_msg = format!("CRITICAL: Permission Denied or Port {} is already in use.\n\nError: {}\n\nPlease change the port in settings.json or close the conflicting application.", port, e);
            info!("[Server] {}", err_msg);
            crate::utils::show_error_popup(&err_msg);
            std::process::exit(1);
        }
    }
}

async fn asset_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl IntoResponse {
    let clean_path = path.trim_start_matches('/');
    let mime = match clean_path.split('.').last() {
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    };

    let content: Option<&'static [u8]> = match clean_path {
        "avatar.png" => Some(include_bytes!("../assets/avatar.png")),
        "music.png" => Some(include_bytes!("../assets/music.png")),
        "icon.png" => Some(include_bytes!("../assets/icon.png")),
        "icon.ico" => Some(include_bytes!("../assets/icon.ico")),
        _ => None,
    };

    if let Some(c) = content {
        ([(axum::http::header::CONTENT_TYPE, mime)], c).into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}

async fn overlay_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let s = state.read().await;
    let settings = s.settings.as_ref().unwrap();
    let layout_override = params.get("layout").cloned();
    Html(get_html(
        &settings.overlay,
        s.last_track.as_ref(),
        layout_override,
    ))
}

async fn settings_gui_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    let s = state.read().await;
    let code = s.settings.as_ref().unwrap().code.clone().unwrap_or_default();
    if code.is_empty() || code.len() != 6 {
        return axum::response::Redirect::to("/setup").into_response();
    }
    Html(include_str!("gui.html")).into_response()
}

async fn setup_gui_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    let s = state.read().await;
    let code = s.settings.as_ref().unwrap().code.clone().unwrap_or_default();
    if !code.is_empty() && code.len() == 6 {
        return axum::response::Redirect::to("/settings").into_response();
    }
    Html(include_str!("setup.html")).into_response()
}

async fn get_devices_handler() -> impl IntoResponse {
    axum::Json(crate::utils::get_audio_devices())
}

async fn get_settings_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    axum::Json(state.read().await.settings.clone())
}

async fn update_settings_handler(
    State((state, handle)): State<(AppState, Arc<ServerHandle>)>,
    body: String,
) -> impl IntoResponse {
    match serde_json::from_str::<Settings>(&body) {
        Ok(new_settings) => {
            let mut s = state.write().await;
            s.settings = Some(new_settings.clone());
            crate::utils::save_settings(&new_settings);
            info!("[Server] Settings updated. Triggering visualizer restart...");
            let _ = handle
                .tx
                .send(serde_json::json!({ "type": "settings_update" }).to_string());
            serde_json::json!({ "status": "success" }).to_string().into_response()
        }
        Err(e) => {
            info!("[Server] Settings Update Failed: {}", e);
            (axum::http::StatusCode::BAD_REQUEST, e.to_string()).into_response()
        }
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State((state, handle)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, handle))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, handle: Arc<ServerHandle>) {
    let mut rx = handle.tx.subscribe();
    {
        let s = state.read().await;
        let track = s.last_track.clone().unwrap_or_default();
        let _ = socket
            .send(Message::Text(
                serde_json::json!({ "type": "track_update", "data": track })
                    .to_string()
                    .into(),
            ))
            .await;
    }
    loop {
        tokio::select! {
            res = rx.recv() => {
                match res {
                    Ok(msg) => { if socket.send(Message::Text(msg.into())).await.is_err() { break; } }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = socket.recv() => { if msg.is_none() { break; } }
        }
    }
}

pub fn get_html(
    overlay: &OverlayConfig,
    last_track: Option<&TrackUpdate>,
    layout_override: Option<String>,
) -> String {
    let viz = &overlay.visualizer;
    let track = last_track.cloned().unwrap_or_default();
    let track_json = serde_json::to_string(&track).unwrap();

    let layout_mode = layout_override.unwrap_or(overlay.layout.clone());

    let (padding, title_size, artist_size, thumb_size, gap, show_thumb, show_viz_layout, show_progress_layout, inner_radius, widget_width, widget_height) = match layout_mode.as_str() {
        "compact-bar" => ("20px", "2.6em", "1.8em", "160px", "30px", true, false, true, "16px", "800px", "200px"),
        "compact-minimal" => ("15px", "2.2em", "1.6em", "0px", "0px", false, false, false, "8px", "600px", "160px"),
        _ => ("40px", "3.8em", "2.8em", "320px", "40px", true, true, true, "32px", "1360px", "440px"), 
    };

    let show_progress_style = if show_progress_layout { "flex" } else { "none" };

    format!(
        r##"<!DOCTYPE html><html><head><style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');
        :root {{
            --widget-padding: {padding};
            --inner-radius: {inner_radius};
            --outer-radius: calc(var(--inner-radius) + var(--widget-padding));
        }}
        html, body {{ background: transparent !important; margin: 0; padding: 0; overflow: hidden; width: 100vw; height: 100vh; color: {text_color}; display: flex; align-items: flex-start; justify-content: flex-start; }}
        body {{ font-family: 'Inter', sans-serif; }}
        #scale-wrapper {{ display: inline-flex; width: max-content; height: max-content; position: relative; opacity: {overlay_global_enabled}; transition: opacity 0.3s ease; }}
        
        #widget {{ 
            display: inline-flex; 
            align-items: center; 
            justify-content: center; 
            padding: var(--widget-padding); 
            border-radius: var(--outer-radius); 
            width: {widget_width}; 
            height: {widget_height}; 
            box-sizing: border-box; 
            position: relative; 
            gap: {gap}; 
            overflow: hidden; 
            border: 2px solid {border_color}; 
            transition: all 0.3s ease; 
        }}
        
        #bg-blur {{ position: absolute; inset: -40px; z-index: -2; filter: blur(60px); opacity: 0; background-size: cover; background-position: center; transition: opacity 0.5s ease; pointer-events: none; }}
        #bg-blur.visible {{ opacity: {edge_opacity}; }}
        #bg-full {{ position: absolute; inset: 0; z-index: -3; background-size: cover; background-position: center; opacity: 0; transition: opacity 0.5s ease; pointer-events: none; }}
        #bg-full.visible {{ opacity: {full_bg_opacity}; }}
        #widget::before {{ content: ''; position: absolute; inset: 0; background: {bg_color} !important; z-index: -4; opacity: {bg_opacity}; pointer-events: none; }}
        
        #img-container {{ flex-shrink: 0; position: relative; width: {thumb_size}; height: {thumb_size}; z-index: 10; display: {show_thumb_style}; }}
        #track-img-wrapper {{ width: 100%; height: 100%; border-radius: var(--inner-radius); overflow: hidden; position: relative; z-index: 100; box-shadow: 0 10px 30px rgba(0,0,0,0.6); transition: box-shadow 0.3s ease; }}
        #track-img {{ width: 100%; height: 100%; object-fit: cover; }}
        
        #avatar-img {{ position: absolute; top: -15px; left: -15px; width: 110px; height: 110px; z-index: 110; border-radius: 50%; object-fit: cover; box-shadow: 0 4px 15px rgba(0,0,0,0.5); border: none; transition: all 0.3s ease; }}
        #widget.compact-minimal #avatar-img {{ width: 56px; height: 56px; top: -10px; left: -10px; }}
        #widget.compact-bar #avatar-img {{ width: 72px; height: 72px; top: -12px; left: -12px; }}

        #info {{ display: flex; flex-direction: column; justify-content: center; min-height: 100px; flex-grow: 1; min-width: 0; transition: all 0.3s ease; }}
        #info.centered {{ align-items: center; text-align: center; }}
        #text-wrapper {{ display: flex; flex-direction: column; gap: 4px; overflow: visible; width: 100%; box-sizing: border-box; }}
        
        #title {{ font-weight: bold; font-size: {title_size}; margin: 0; padding-bottom: 0.15em; white-space: nowrap; line-height: normal; text-shadow: 0 4px 12px rgba(0,0,0,0.9); transition: color 0.3s ease; overflow: visible; }}
        #title.fade-edge {{ -webkit-mask-image: linear-gradient(to right, black 85%, transparent 100%); mask-image: linear-gradient(to right, black 85%, transparent 100%); }}
        
        #artist {{ font-size: {artist_size}; color: rgba(255,255,255,0.7); margin: 0; white-space: nowrap; overflow: hidden; text-shadow: 0 2px 8px rgba(0,0,0,0.8); -webkit-mask-image: linear-gradient(to right, black 85%, transparent 100%); mask-image: linear-gradient(to right, black 85%, transparent 100%); }}
        #progress-area {{ display: none; align-items: center; width: 100%; gap: 15px; margin-top: 15px; opacity: 0; transition: opacity 0.4s ease; }}
        #progress-area.visible {{ display: {show_progress_style} !important; opacity: 1; }}
        #progress-container {{ flex-grow: 1; height: 24px; background: {element_color}; border-radius: 50px; overflow: hidden; box-shadow: inset 0 2px 5px rgba(0,0,0,0.5); min-width: 200px; }}
        #progress-bar {{ width: 0%; height: 100%; border-radius: 50px; transition: width 0.1s linear, background 0.3s ease; }}
        #time-text {{ font-size: calc({artist_size} * 0.9); font-weight: 700; font-variant-numeric: tabular-nums; text-shadow: 0 2px 5px rgba(0,0,0,0.8); white-space: nowrap; }}
        
        #live-indicator {{ display: none; align-items: center; justify-content: flex-end; gap: 12px; color: #ff4444; font-weight: bold; font-size: calc({artist_size} * 0.7); text-transform: uppercase; letter-spacing: 2px; text-shadow: 0 0 15px rgba(255,68,68,0.4); margin-top: 5px; width: 100%; }}
        #live-indicator.visible {{ display: flex; }}
        #live-dot {{ width: 12px; height: 12px; background-color: #ff4444; border-radius: 50%; box-shadow: 0 0 10px #ff4444; animation: pulse 1.5s infinite; }}
        @keyframes pulse {{ 0% {{ transform: scale(1); opacity: 1; }} 50% {{ transform: scale(1.3); opacity: 0.5; }} 100% {{ transform: scale(1); opacity: 1; }} }}
        
        #viz-canvas {{ display: {viz_display}; margin-top: 15px; width: 100%; height: 100px; visibility: {viz_global_enabled}; opacity: {viz_opacity}; transition: opacity 0.4s ease, visibility 0.4s ease; }}
        #fps-text {{ position: absolute; bottom: 10px; right: 20px; font-size: 1.2em; color: rgba(255,255,255,0.3); font-family: monospace; display: {debug_display}; z-index: 1000; }}
        
        #widget.idle #img-container, #widget.offline #img-container {{ display: none !important; }}
        #widget.idle #info, #widget.offline #info {{ justify-content: center; text-align: center; align-items: center; min-height: 100px; }}
        #widget.idle #progress-area, #widget.idle #viz-canvas, #widget.idle #fps-text,
        #widget.offline #progress-area, #widget.offline #viz-canvas, #widget.offline #fps-text {{ display: none !important; opacity: 0 !important; visibility: hidden !important; }}
        #widget.compact-bar #time-text {{ display: none !important; }}
        #widget.compact-minimal #progress-area {{ display: none !important; }}
        #widget.compact-bar #viz-canvas, #widget.compact-minimal #viz-canvas {{ display: none !important; }}
        </style></head><body><div id="scale-wrapper">        <div id="widget" class="offline {layout_class}">
            <div id="bg-blur"></div><div id="bg-full"></div>
            <div id="img-container">
                <img id="avatar-img" src="/assets/avatar.png" alt="Avatar" />
                <div id="track-img-wrapper"><img id="track-img" src="/assets/music.png" crossorigin="anonymous" /></div>
            </div>
            <div id="info" class="{center_class}">
                <div id="text-wrapper"><div id="title">Server Offline</div><div id="artist">Waiting for Rust...</div></div>
                <div id="live-indicator"><div id="live-dot"></div><span>LIVE</span></div>
                <div id="progress-area"><div id="progress-container"><div id="progress-bar"></div></div><div id="time-text">0:00 / 0:00</div></div>
                <canvas id="viz-canvas"></canvas>
            </div>
            <div id="fps-text">0 FPS</div>
        </div>
    </div><script>
        const urlParams = new URLSearchParams(window.location.search);
        const overrideViz = urlParams.get('visualizer');
        const overrideProgress = urlParams.get('progress');

        let track = {track_json}, bins = new Uint8Array({bars}).fill(0), isOffline = true;
        const widget = document.getElementById("widget"), bgBlur = document.getElementById("bg-blur"), bgFull = document.getElementById("bg-full");
        const canvas = document.getElementById("viz-canvas"), ctx = canvas.getContext("2d", {{ alpha: true }});
        
        const progressArea = document.getElementById("progress-area");
        if (overrideProgress === 'false' && progressArea) progressArea.style.display = 'none';

        const randomHex = () => "#" + Math.floor(Math.random()*16777215).toString(16).padStart(6, '0');
        const isStatic = {is_static}, isGlobalSync = {global_sync}, mode = "{viz_mode}";
        let cTop = {is_random} ? randomHex() : "{v_top}", cBot = {is_random} ? randomHex() : "{v_bot}";
        if (isStatic) cBot = cTop;
        const progBar = document.getElementById("progress-bar"), timeText = document.getElementById("time-text"), liveIndicator = document.getElementById("live-indicator"), trackImg = document.getElementById('track-img');
        const titleEl = document.getElementById('title');
        const trackImgWrapper = document.getElementById('track-img-wrapper');
        const avatarImg = document.getElementById('avatar-img');

        function updateTitleFade() {{
            if (titleEl.scrollWidth > titleEl.clientWidth) {{
                titleEl.classList.add('fade-edge');
            }} else {{
                titleEl.classList.remove('fade-edge');
            }}
        }}

        window.addEventListener('resize', updateTitleFade);

        trackImg.onload = () => {{
            if (trackImg.src.endsWith('/assets/music.png')) {{
                trackImgWrapper.style.boxShadow = 'none';
            }} else {{
                trackImgWrapper.style.boxShadow = '0 10px 30px rgba(0,0,0,0.6)';
            }}

            if (isGlobalSync && !isOffline) {{
                try {{
                    const temp = document.createElement('canvas'); temp.width = 1; temp.height = 1;
                    const tCtx = temp.getContext('2d'); tCtx.drawImage(trackImg, 0, 0, 1, 1);
                    let [r, g, b] = tCtx.getImageData(0, 0, 1, 1).data;
                    
                    let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
                    if (luminance < 70) {{
                        const boost = (70 - luminance) / 70;
                        r = Math.min(255, Math.floor(r + (255 - r) * boost));
                        g = Math.min(255, Math.floor(g + (255 - g) * boost));
                        b = Math.min(255, Math.floor(b + (255 - b) * boost));
                    }}

                    const domColor = `rgb(${{r}},${{g}},${{b}})`;
                    cTop = domColor; cBot = domColor; 
                    titleEl.style.color = domColor;
                    avatarImg.style.borderColor = domColor;
                    document.getElementById('progress-container').style.background = `rgba(${{r}},${{g}},${{b}},0.2)`;
                }} catch(e) {{ console.log("CORS block"); }}
            }} else if (!isGlobalSync) {{ 
                titleEl.style.color = "{text_color}"; 
                document.getElementById('progress-container').style.background = "{element_color}";
            }}
            progBar.style.background = isStatic ? cTop : `linear-gradient(90deg, ${{cBot}}, ${{cTop}})`;
            progBar.style.boxShadow = `0 0 10px ${{cTop}}80`;
        }};

        function updateUI() {{
            const isIdle = !track || !track.details || track.details.includes("Disconnected") || track.details.includes("Resting...") || track.details.includes("Not Playing") || track.state === "Resting..." || track.state === "Not Playing";
            const isPaused = !!track.paused;
            widget.className = (isOffline ? "offline" : (isIdle ? "idle" : (isPaused ? "paused playing" : "playing"))) + " {layout_class}";
            titleEl.innerText = isOffline ? "Server Offline" : (track.details || "Resting...");
            document.getElementById("artist").innerText = isOffline ? "Waiting for connection..." : (track.state || "Browsing for music");
            
            updateTitleFade();

            if (isIdle || isPaused || isOffline) {{
                canvas.style.opacity = '0'; canvas.style.visibility = 'hidden';
            }} else {{
                canvas.style.opacity = '1'; canvas.style.visibility = 'visible';
            }}

            const thumb = (isIdle || isOffline) ? "/assets/music.png" : (track.thumbnail || "/assets/music.png");
            trackImg.src = thumb; 
            if({show_bg_blur}) {{ bgBlur.style.backgroundImage = `url("${{thumb}}")`; bgBlur.classList.add("visible"); }} else {{ bgBlur.classList.remove("visible"); }}
            if({show_thumb_bg}) {{ bgFull.style.backgroundImage = `url("${{thumb}}")`; bgFull.classList.add("visible"); }} else {{ bgFull.classList.remove("visible"); }}
        }}

        function connect() {{
            const socket = new WebSocket("ws://" + location.host + "/ws");
            socket.onopen = () => {{ isOffline = false; updateUI(); }};
            socket.onmessage = (e) => {{
                const m = JSON.parse(e.data);
                if (m.type === "track_update") {{ track = m.data; updateUI(); }}
                if (m.type === "fft_data") {{ bins.set(m.bins); }}
                if (m.type === "settings_update") {{ location.reload(); }}
            }};
            socket.onclose = () => {{ isOffline = true; updateUI(); setTimeout(connect, 2000); }};
        }}

        const BARS = {bars}, heights = new Float32Array(BARS).fill(0), DPR = Math.max(window.devicePixelRatio || 1, 2); 
        let lastFrame = performance.now(), frameCount = 0, lastFpsUpdate = lastFrame;
        const targetFps = {viz_fps};
        const frameInterval = 1000 / targetFps;

        function draw(now) {{
            requestAnimationFrame(draw);
            
            const elapsed = now - lastFrame;
            if (elapsed < frameInterval) return;
            lastFrame = now - (elapsed % frameInterval);

            if (isOffline) return;
            
            const isIdle = widget.classList.contains("idle"), isPaused = widget.classList.contains("paused");
            
            if (!isIdle && !isOffline) {{
                const rawStart = track.startTimestamp || track.start_timestamp || 0;
                const rawEnd = track.endTimestamp || track.end_timestamp || 0;
                const start = rawStart < 10000000000 && rawStart > 0 ? rawStart * 1000 : rawStart;
                const end = rawEnd < 10000000000 && rawEnd > 0 ? rawEnd * 1000 : rawEnd;
                const posRaw = track.position || 0;
                const pos = posRaw < 10000000000 ? posRaw * 1000 : posRaw;
                
                const tNow = Date.now(), total = end > start ? end - start : 0, current = isPaused ? pos : Math.max(0, tNow - start);
                const isLive = (!rawEnd || rawEnd === 0) && rawStart > 0 && !isPaused;
                liveIndicator.classList.toggle("visible", isLive);
                
                if (isLive) {{
                    if(overrideProgress !== 'false') progressArea.classList.add("visible");
                    progBar.style.width = "100%";
                    timeText.innerText = "Live";
                }} else if (total > 0) {{
                    if(overrideProgress !== 'false') progressArea.classList.add("visible");
                    progBar.style.width = Math.min(100, (current / total) * 100) + "%";
                    const fmt = (ms) => {{ 
                        const s = Math.floor(Math.max(0, ms)/1000), h = Math.floor(s/3600), m = Math.floor((s % 3600) / 60), sc = s % 60; 
                        const hPart = h > 0 ? h + ":" : "";
                        const mPart = h > 0 ? m.toString().padStart(2, "0") : m.toString();
                        return hPart + mPart + ":" + sc.toString().padStart(2, "0"); 
                    }};
                    timeText.innerText = fmt(current) + " / " + fmt(total);
                }} else {{
                    progressArea.classList.remove("visible");
                }}
            }} else {{ progressArea.classList.remove("visible"); liveIndicator.classList.remove("visible"); }}

            const baseW = 1360, baseH = 440;
            if (canvas.width !== baseW * DPR) {{ canvas.width = baseW * DPR; canvas.height = baseH * DPR; }}
            ctx.setTransform(DPR, 0, 0, DPR, 0, 0); ctx.clearRect(0, 0, baseW, baseH);
            
            if(isIdle || isPaused || isOffline || !{viz_enabled} || !{show_viz_layout} || overrideViz === 'false') return;

            const totalUnits = ({bar_w} * BARS) + ({bar_g} * (BARS - 1)), barW = (baseW / totalUnits) * {bar_w}, gap = (baseW / totalUnits) * {bar_g};
            const physicsEnabled = {viz_physics};
            const barGrad = ctx.createLinearGradient(0, 0, 0, baseH); barGrad.addColorStop(0, cTop); barGrad.addColorStop(1, cBot);

            for(let i=0; i<BARS; i++) {{
                let target = (bins[i] / 255) * baseH;
                if (physicsEnabled) {{
                    const l = i > 0 ? (bins[i-1] / 255) * baseH : 0, r = i < BARS - 1 ? (bins[i+1] / 255) * baseH : 0;
                    if (l * 0.75 > target) target = l * 0.75; if (r * 0.75 > target) target = r * 0.75;
                    if (target > heights[i]) heights[i] = target; else heights[i] -= (heights[i] - target) * 0.22;
                }} else {{
                    heights[i] = target;
                }}
            }}

            if ({viz_glow}) {{ ctx.shadowBlur = 15; ctx.shadowColor = cTop; }}
            ctx.fillStyle = barGrad; ctx.strokeStyle = barGrad;
            
            if (mode === "background") {{
                const r = {rounded} ? Math.min(barW / 2, 6) : 0; ctx.beginPath();
                const maxHeight = baseH * 0.70; // Fill bottom 70% max
                for(let i=0; i<BARS; i++) {{ const h = Math.max(4, (heights[i] / baseH) * maxHeight), x = i * (barW + gap), y = baseH - h; if (r > 0) ctx.roundRect(x, y, barW, h, [r, r, 0, 0]); else ctx.rect(x, y, barW, h); }}
                ctx.globalAlpha = 0.3; ctx.fill(); ctx.globalAlpha = 1.0;
            }} else if (mode === "wave") {{
                ctx.lineWidth = 6; ctx.lineJoin = "round"; ctx.lineCap = "round"; ctx.beginPath();
                const pts = []; for(let i=0; i<BARS; i++) pts.push({{x: i * (barW + gap) + barW/2, y: baseH - Math.max(4, heights[i])}});
                if(pts.length > 0) {{
                    ctx.moveTo(pts[0].x, pts[0].y);
                    for (let i = 0; i < pts.length - 1; i++) {{
                        const xc = (pts[i].x + pts[i + 1].x) / 2, yc = (pts[i].y + pts[i + 1].y) / 2;
                        ctx.quadraticCurveTo(pts[i].x, pts[i].y, xc, yc);
                    }}
                    ctx.stroke(); ctx.lineTo(baseW, baseH); ctx.lineTo(0, baseH); ctx.globalAlpha = 0.2; ctx.fill(); ctx.globalAlpha = 1.0;
                }}
            }} else if (mode === "neon") {{
                ctx.lineWidth = 2;
                for(let i=0; i<BARS; i++) {{
                    const h = Math.max(4, heights[i]);
                    ctx.strokeRect(i*(barW+gap), baseH - h, barW, h);
                }}
            }} else if (mode === "led") {{
                const dotH = 6;
                for(let i=0; i<BARS; i++) {{
                    const h = Math.max(4, heights[i]);
                    for(let y = baseH; y > baseH - h; y -= dotH + 2) {{
                        ctx.fillRect(i*(barW+gap), y - dotH, barW, dotH);
                    }}
                }}
            }} else if (mode === "outline") {{
                ctx.lineWidth = 2; ctx.beginPath();
                for(let i=0; i<BARS; i++) {{
                    const h = Math.max(4, heights[i]);
                    const x = i*(barW+gap), y = baseH-h;
                    if(i===0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
                    ctx.lineTo(x+barW, y);
                }}
                ctx.stroke();
            }} else if (mode === "center-bars") {{
                const rC = {rounded} ? Math.min(barW / 2, 6) : 0; const midY = baseH / 2; ctx.beginPath();
                for(let i=0; i<BARS; i++) {{ const h = Math.max(4, heights[i]) / 2, x = i * (barW + gap); if (rC > 0) ctx.roundRect(x, midY - h, barW, h * 2, [rC, rC, rC, rC]); else ctx.rect(x, midY - h, barW, h * 2); }}
                ctx.fill();
            }} else if (mode === "mirrored") {{
                const r = {rounded} ? Math.min(barW / 2, 6) : 0;
                ctx.beginPath();
                for(let i=0; i<BARS; i++) {{
                    const curH = Math.max(4, (heights[i] / baseH) * (baseH * 0.30)); // 30% max height per side (leaves 40% gap in middle)
                    const x = i * (barW + gap);
                    if (r > 0) {{
                        ctx.roundRect(x, 0, barW, curH, [0, 0, r, r]);
                        ctx.roundRect(x, baseH - curH, barW, curH, [r, r, 0, 0]);
                    }} else {{
                        ctx.rect(x, 0, barW, curH);
                        ctx.rect(x, baseH - curH, barW, curH);
                    }}
                }}
                ctx.globalAlpha = 0.5; ctx.fill(); ctx.globalAlpha = 1.0;
            }} else if (mode === "sides") {{
                const sideBarH = baseH / BARS;
                ctx.beginPath();
                for(let i=0; i<BARS; i++) {{
                    const curW = Math.max(4, (heights[i] / baseH) * (baseW * 0.15)); // 15% max width per side
                    const y = i * sideBarH;
                    ctx.rect(0, y, curW, sideBarH - 2);
                    ctx.rect(baseW - curW, y, curW, sideBarH - 2);
                }}
                ctx.globalAlpha = 0.5; ctx.fill(); ctx.globalAlpha = 1.0;
            }} else {{
                const r = {rounded} ? Math.min(barW / 2, 6) : 0; ctx.beginPath();
                for(let i=0; i<BARS; i++) {{ const h = Math.max(4, heights[i]), x = i * (barW + gap), y = baseH - h; if (r > 0) ctx.roundRect(x, y, barW, h, [r, r, 0, 0]); else ctx.rect(x, y, barW, h); }}
                ctx.fill();
            }}
            ctx.shadowBlur = 0; frameCount++;
            if(now - lastFpsUpdate >= 1000) {{ document.getElementById("fps-text").innerText = frameCount + " FPS"; frameCount = 0; lastFpsUpdate = now; }}
        }} 
        updateUI(); connect(); draw(performance.now());
    </script></body></html>"##,
        text_color = overlay.text_color,
        border_color = overlay.border_color,
        element_color = overlay.element_color,
        bg_color = &overlay.background.color,
        bg_opacity = overlay.background_opacity,
        edge_opacity = overlay.thumbnail_opacity,
        full_bg_opacity = overlay.thumbnail_background_opacity,
        overlay_global_enabled = if overlay.enabled { 1.0 } else { 0.0 },
        bars = viz.bars,
        bar_w = viz.bar_width,
        bar_g = viz.bar_gap,
        viz_enabled = viz.enabled,
        viz_global_enabled = if viz.enabled { "visible" } else { "hidden" },
        viz_display = if viz.enabled && show_viz_layout { "block" } else { "none" },
        viz_opacity = if viz.enabled { 1.0 } else { 0.0 },
        viz_mode = viz.mode,
        is_static = viz.gradient.is_static,
        is_random = viz.gradient.random,
        v_top = viz.gradient.top,
        v_bot = viz.gradient.bottom,
        rounded = viz.rounded,
        show_bg_blur = overlay.show_background,
        show_thumb_bg = overlay.show_thumbnail_background,
        global_sync = overlay.global_sync,
        center_class = if overlay.center_text { "centered" } else { "" },
        layout_class = layout_mode,
        debug_display = if viz.debug_fps { "block" } else { "none" },
        viz_glow = viz.glow,
        viz_physics = viz.horizontal_smoothing,
        track_json = track_json,
        padding = padding,
        gap = gap,
        title_size = title_size,
        artist_size = artist_size,
        thumb_size = thumb_size,
        show_thumb_style = if show_thumb { "block" } else { "none" },
        show_viz_layout = show_viz_layout,
        viz_fps = viz.fps,
        inner_radius = inner_radius,
    )
}
