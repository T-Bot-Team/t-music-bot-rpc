use crate::models::{OverlayConfig, TrackUpdate};

pub fn get_html(
    overlay: &OverlayConfig,
    last_track: Option<&TrackUpdate>,
    layout_override: Option<String>,
) -> String {
    let viz = &overlay.visualizer;
    let track = last_track.cloned().unwrap_or_default();
    let track_json = serde_json::to_string(&track).unwrap();
    let layout_mode = layout_override.unwrap_or(overlay.layout.clone());

    let (padding, title_size, artist_size, thumb_size, gap, show_thumb, show_viz_layout, show_progress_layout, inner_radius, widget_width, widget_height, avatar_size, avatar_offset) = match layout_mode.as_str() {
        "compact-bar" => ("20px", "2.6em", "1.8em", "160px", "30px", true, false, true, "16px", "800px", "200px", "64px", "-10px"),
        "compact-minimal" => ("15px", "2.0em", "1.4em", "48px", "20px", true, false, false, "8px", "540px", "100px", "48px", "0px"),
        _ => ("40px", "3.8em", "2.8em", "320px", "40px", true, true, true, "32px", "1360px", "440px", "110px", "-15px"), 
    };

    let text_anim_enabled = overlay.enable_text_animation;
    let text_anim_mode = &overlay.text_animation_mode;
    let text_anim_duration = overlay.text_animation_duration;

    let get_css = |cfg: &crate::models::GradientConfig| {
        if cfg.is_static { cfg.top.clone() }
        else { format!("linear-gradient(90deg, {}, {})", cfg.bottom, cfg.top) }
    };

    let text_color_css = if overlay.global_sync || viz.gradient.random {
        "color: #ffffff;".to_string()
    } else if overlay.text_style.is_static {
        format!("color: {};", overlay.text_style.top)
    } else {
        format!("background: {}; -webkit-background-clip: text; -webkit-text-fill-color: transparent;", get_css(&overlay.text_style))
    };

    let border_css = if overlay.border_style.is_static {
        format!("border: 2px solid {};", overlay.border_style.top)
    } else {
        format!(
            "border: 2px solid transparent; background: linear-gradient(var(--bg-pure), var(--bg-pure)) padding-box, {} border-box;",
            get_css(&overlay.border_style)
        )
    };

    let element_bg = get_css(&overlay.element_style);
    let bg_pure = &overlay.background.color;
    let show_progress_style = if show_progress_layout { "flex" } else { "none" };
    let bars_val = viz.bars;

    format!(
        r##"<!DOCTYPE html><html><head><style>
        @font-face {{ font-family: 'Inter'; src: url('/assets/inter.woff2') format('woff2'); font-weight: 400; font-style: normal; }}
        @font-face {{ font-family: 'Inter'; src: url('/assets/inter-bold.woff2') format('woff2'); font-weight: 700; font-style: normal; }}
        :root {{
            --widget-padding: {padding};
            --inner-radius: {inner_radius};
            --outer-radius: calc(var(--inner-radius) + var(--widget-padding));
            --bg-pure: {bg_pure};
        }}
        html, body {{ background: transparent !important; margin: 0; padding: 0; overflow: hidden; width: 100vw; height: 100vh; display: flex; align-items: flex-start; justify-content: flex-start; }}
        body {{ font-family: 'Inter', sans-serif; }}
        #scale-wrapper {{ display: inline-flex; width: max-content; height: max-content; position: relative; opacity: {overlay_global_enabled}; transition: opacity 0.3s ease; }}
        #widget {{ display: inline-flex; align-items: center; justify-content: center; padding: var(--widget-padding); border-radius: var(--outer-radius); width: {widget_width}; height: {widget_height}; box-sizing: border-box; position: relative; gap: {gap}; overflow: hidden; {border_css} transition: all 0.3s ease; }}
        #bg-blur {{ position: absolute; inset: -40px; z-index: -2; filter: blur(60px); opacity: 0; background-size: cover; background-position: center; transition: opacity 0.5s ease; pointer-events: none; }}
        #bg-blur.visible {{ opacity: {edge_opacity}; }}
        #bg-full {{ position: absolute; inset: 0; z-index: -3; background-size: cover; background-position: center; opacity: 0; transition: opacity 0.5s ease; pointer-events: none; }}
        #bg-full.visible {{ opacity: {full_bg_opacity}; }}
        #widget::before {{ content: ''; position: absolute; inset: 0; background: var(--bg-pure) !important; z-index: -4; opacity: {bg_opacity}; pointer-events: none; }}
        #img-container {{ flex-shrink: 0; position: relative; width: {thumb_size}; height: {thumb_size}; z-index: 10; display: {show_thumb_style}; }}
        #track-img-wrapper {{ width: 100%; height: 100%; border-radius: var(--inner-radius); overflow: hidden; position: relative; z-index: 100; box-shadow: 0 10px 30px rgba(0,0,0,0.6); transition: box-shadow 0.3s ease; }}
        #track-img {{ width: 100%; height: 100%; object-fit: cover; transition: opacity 0.4s ease; opacity: 1; }}
        #track-img.loading {{ opacity: 0; }}
        #avatar-img {{ position: absolute; top: {avatar_offset}; left: {avatar_offset}; width: {avatar_size}; height: {avatar_size}; z-index: 110; border-radius: 50%; object-fit: cover; box-shadow: 0 4px 15px rgba(0,0,0,0.5); border: none; transition: opacity 0.4s ease; opacity: 1; }}
        #avatar-img.loading {{ opacity: 0; }}
        #info {{ display: flex; flex-direction: column; justify-content: center; min-height: 100px; flex-grow: 1; min-width: 0; transition: all 0.3s ease; margin-bottom: 15px; }}
        #info.centered {{ align-items: center; text-align: center; }}
        #text-wrapper {{ display: flex; flex-direction: column; gap: 0px; overflow: visible; width: 100%; box-sizing: border-box; filter: drop-shadow(0 4px 12px rgba(0,0,0,0.9)); }}
        #title {{ font-weight: bold; font-size: {title_size}; margin: 0; padding-bottom: 0.15em; white-space: nowrap; line-height: normal; transition: color 0.3s ease; overflow: hidden; position: relative; width: 100%; }}
        #title span {{ display: inline-block; min-width: 100%; {text_color_css} }}
        #widget.sync-active #title span {{ background: none !important; -webkit-background-clip: initial !important; -webkit-text-fill-color: initial !important; color: inherit !important; }}
        @keyframes scrollText {{ 0%, 15% {{ transform: translateX(0); }} 85%, 100% {{ transform: translateX(var(--scroll-dist)); }} }}
        #title.scrolling span {{ animation: scrollText var(--scroll-duration) ease-in-out infinite alternate; }}
        #title.fade-edge {{ -webkit-mask-image: linear-gradient(to right, transparent 0%, black 5%, black 95%, transparent 100%); mask-image: linear-gradient(to right, transparent 0%, black 5%, black 95%, transparent 100%); }}
        #artist {{ font-size: {artist_size}; color: rgba(255,255,255,0.7); margin: -4px 0 0 0; padding-bottom: 6px; white-space: nowrap; overflow: hidden; -webkit-mask-image: linear-gradient(to right, transparent 0%, black 5%, black 95%, transparent 100%); mask-image: linear-gradient(to right, transparent 0%, black 5%, black 95%, transparent 100%); }}
        #progress-area {{ display: none; align-items: center; width: 100%; gap: 15px; margin-top: 14px; opacity: 0; transition: opacity 0.4s ease; }}
        #progress-area.visible {{ display: {show_progress_style} !important; opacity: 1; }}
        #progress-container {{ flex-grow: 1; height: 24px; background: {element_bg}; border: 1px solid rgba(255, 255, 255, 0.15); border-radius: 50px; overflow: hidden; box-shadow: inset 0 2px 5px rgba(0,0,0,0.5), 0 0 0 1px rgba(0,0,0,0.3); min-width: 200px; }}
        #progress-bar {{ width: 0%; height: 100%; border-radius: 50px; transition: width 0.1s linear, background 0.3s ease; box-shadow: inset 0 1px 2px rgba(255,255,255,0.4), inset 0 -1px 2px rgba(0,0,0,0.2), 0 0 4px rgba(0,0,0,0.6); }}
        #time-text {{ font-size: calc({artist_size} * 0.9); font-weight: 700; font-variant-numeric: tabular-nums; text-shadow: 0 2px 5px rgba(0,0,0,0.8); white-space: nowrap; color: {text_color_fallback}; }}
        #live-indicator {{ display: none; align-items: center; justify-content: flex-end; gap: 12px; color: #ff4444; font-weight: bold; font-size: calc({artist_size} * 0.7); text-transform: uppercase; letter-spacing: 2px; text-shadow: 0 0 15px rgba(255,68,68,0.4); margin-top: 5px; width: 100%; }}
        #live-indicator.visible {{ display: flex; }}
        #live-dot {{ width: 12px; height: 12px; background-color: #ff4444; border-radius: 50%; box-shadow: 0 0 10px #ff4444; animation: pulse 1.5s infinite; }}
        @keyframes pulse {{ 0% {{ transform: scale(1); opacity: 1; }} 50% {{ transform: scale(1.3); opacity: 0.5; }} 100% {{ transform: scale(1); opacity: 1; }} }}
        #viz-canvas {{ display: {viz_display}; margin-top: 5px; width: 100%; height: 100px; visibility: {viz_global_enabled}; opacity: {viz_opacity}; transition: opacity 0.4s ease, visibility 0.4s ease; }}
        #fps-text {{ position: absolute; bottom: 15px; right: 30px; font-size: 1.2em; color: rgba(255,255,255,0.3); font-family: monospace; display: {debug_display}; z-index: 1000; }}
        #widget.idle #img-container, #widget.offline #img-container {{ display: none !important; }}
        #widget.idle #info, #widget.offline #info {{ justify-content: center; text-align: center; align-items: center; min-height: 100px; }}
        #widget.idle #progress-area, #widget.idle #viz-canvas, #widget.idle #fps-text,
        #widget.offline #progress-area, #widget.offline #viz-canvas, #widget.offline #fps-text {{ display: none !important; opacity: 0 !important; visibility: hidden !important; }}
        #widget.compact-bar #time-text {{ display: none !important; }}
        #widget.compact-minimal #progress-area {{ display: none !important; }}
        #widget.compact-minimal #track-img-wrapper {{ display: none !important; }}
        #widget.compact-bar #viz-canvas, #widget.compact-minimal #viz-canvas {{ display: none !important; }}
        </style></head><body><div id="scale-wrapper"><div id="widget" class="offline {layout_class}"><div id="bg-blur"></div><div id="bg-full"></div><div id="img-container"><img id="avatar-img" src="/assets/avatar.png" alt="Avatar" /><div id="track-img-wrapper"><img id="track-img" src="/assets/music.png" crossorigin="anonymous" /></div></div><div id="info" class="{center_class}"><div id="text-wrapper"><div id="title"><span>Server Offline</span></div><div id="artist">Waiting for Rust...</div></div><div id="live-indicator"><div id="live-dot"></div><span>LIVE</span></div><div id="progress-area"><div id="progress-container"><div id="progress-bar"></div></div><div id="time-text">0:00 / 0:00</div></div><canvas id="viz-canvas"></canvas></div><div id="fps-text">0 FPS</div></div></div><script>
        const urlParams = new URLSearchParams(window.location.search);
        const overrideViz = urlParams.get('visualizer');
        const overrideProgress = urlParams.get('progress');
        let track = {track_json}, bins = new Uint8Array({bars_val}).fill(0), isOffline = false, isFirstLoad = true;
        const widget = document.getElementById("widget"), bgBlur = document.getElementById("bg-blur"), bgFull = document.getElementById("bg-full");
        const canvas = document.getElementById("viz-canvas"), ctx = canvas.getContext("2d", {{ alpha: true }});
        const progressArea = document.getElementById("progress-area");
        if (overrideProgress === 'false' && progressArea) progressArea.style.display = 'none';
        const randomHex = () => "#" + Math.floor(Math.random()*16777215).toString(16).padStart(6, '0');
        const isStatic = {is_static}, isGlobalSync = {global_sync}, mode = "{viz_mode}";
        let cTop = {is_random} ? randomHex() : "{v_top}", cBot = {is_random} ? randomHex() : "{v_bot}";
        if (isStatic) cBot = cTop;
        const progBar = document.getElementById("progress-bar"), timeText = document.getElementById("time-text"), liveIndicator = document.getElementById("live-indicator"), trackImg = document.getElementById('track-img');
        const titleEl = document.getElementById('title'), trackImgWrapper = document.getElementById('track-img-wrapper'), avatarImg = document.getElementById('avatar-img');
        function updateTitleFade() {{ 
            titleEl.classList.remove('scrolling'); 
            titleEl.style.transform = "translateX(0)"; 
            if ({text_anim_enabled} && titleEl.scrollWidth > titleEl.clientWidth) {{ 
                titleEl.classList.add('fade-edge'); 
                const dist = titleEl.scrollWidth - titleEl.clientWidth; 
                titleEl.style.setProperty('--scroll-dist', `-${{dist + 20}}px`); 
                
                const animMode = "{text_anim_mode}";
                let duration = {text_anim_duration};
                if (animMode === "dynamic") {{
                    duration = Math.max(4, Math.floor(dist / 40)); 
                }}
                titleEl.style.setProperty('--scroll-duration', `${{duration}}s`);
                titleEl.classList.add('scrolling'); 
            }} else {{ 
                titleEl.classList.remove('fade-edge'); 
            }} 
        }}
        window.addEventListener('resize', updateTitleFade);
        function updateImageSmoothly(el, newSrc, instant = false) {{ 
            if (!newSrc || el.src.endsWith(newSrc)) return; 
            if (instant) {{ el.src = newSrc; return; }}
            el.classList.add('loading'); 
            const temp = new Image(); 
            temp.onload = () => {{ 
                setTimeout(() => {{
                    el.src = newSrc; 
                    el.classList.remove('loading'); 
                }}, 400);
            }}; 
            temp.onerror = () => {{ 
                setTimeout(() => {{
                    el.src = "/assets/music.png"; 
                    el.classList.remove('loading'); 
                }}, 400);
            }}; 
            temp.src = newSrc; 
        }}
        function updateBgSmoothly(el, newSrc, targetOpacity, instant = false) {{
            if (!newSrc || el.style.backgroundImage.includes(newSrc)) return;
            if (instant) {{
                el.style.backgroundImage = `url("${{newSrc}}")`;
                el.style.opacity = targetOpacity;
                return;
            }}
            el.style.opacity = '0';
            setTimeout(() => {{
                el.style.backgroundImage = `url("${{newSrc}}")`;
                el.style.opacity = targetOpacity;
            }}, 500);
        }}
        trackImg.onload = () => {{ if (trackImg.src.endsWith('/assets/music.png')) {{ trackImgWrapper.style.boxShadow = 'none'; }} else {{ trackImgWrapper.style.boxShadow = '0 10px 30px rgba(0,0,0,0.6)'; }}
            if ({is_random}) {{ cTop = randomHex(); cBot = isStatic ? cTop : randomHex(); }}
            if (isGlobalSync && !isOffline) {{ try {{ const temp = document.createElement('canvas'); temp.width = 1; temp.height = 1; const tCtx = temp.getContext('2d'); tCtx.drawImage(trackImg, 0, 0, 1, 1); let [r, g, b] = tCtx.getImageData(0, 0, 1, 1).data; let luminance = 0.299 * r + 0.587 * g + 0.114 * b; if (luminance < 70) {{ const boost = (70 - luminance) / 70; r = Math.min(255, Math.floor(r + (255 - r) * boost)); g = Math.min(255, Math.floor(g + (255 - g) * boost)); b = Math.min(255, Math.floor(b + (255 - b) * boost)); }}
                    const domColor = `rgb(${{r}},${{g}},${{b}})`; cTop = domColor; cBot = domColor; titleEl.style.color = domColor; avatarImg.style.borderColor = domColor; document.getElementById('progress-container').style.background = `rgba(${{r}},${{g}},${{b}},0.2)`; }} catch(e) {{ console.log("CORS block"); }}
            }} else if ({is_random} && !isOffline) {{
                titleEl.style.color = cTop;
            }} progBar.style.background = isStatic ? cTop : `linear-gradient(90deg, ${{cBot}}, ${{cTop}})`; progBar.style.boxShadow = `0 0 10px ${{cTop}}80`;
            if (isFirstLoad) {{
                requestAnimationFrame(() => requestAnimationFrame(() => {{
                    widget.style.opacity = '';
                    widget.style.transition = '';
                    isFirstLoad = false;
                }}));
            }}
        }};
        function updateUI() {{ 
            const hasDetails = track && track.details && track.details.trim().length > 0 && track.details.toLowerCase() !== "resting...";
            const isIdle = !track || track.status === 'idle' || !hasDetails; 
            const isPaused = !!track.paused; 
            
            if (isFirstLoad) {{
                widget.style.transition = 'none';
                widget.style.opacity = '0';
            }}
            widget.className = (isOffline ? "offline" : (isIdle ? "idle" : (isPaused ? "paused playing" : "playing"))) + " {layout_class}" + (isGlobalSync && !isOffline ? " sync-active" : ""); 
            
            titleEl.innerHTML = `<span>${{isOffline ? "Server Offline" : (hasDetails ? track.details : "Resting...")}}</span>`; 
            if (isOffline || isIdle) {{
                titleEl.style.color = '';
            }} else if (isGlobalSync) {{
                // Handled in trackImg.onload
            }} else if ({is_random}) {{
                titleEl.style.color = cTop;
            }} else {{
                titleEl.style.color = '';
            }}
            document.getElementById("artist").innerText = isOffline ? "Waiting for connection..." : (track.state || "Browsing for music");
            updateTitleFade();
            if (mode === "background" || mode === "sides" || mode === "mirrored") {{ canvas.style.position = "absolute"; canvas.style.inset = "0"; canvas.style.width = "100%"; canvas.style.height = "100%"; canvas.style.zIndex = "-1"; canvas.style.pointerEvents = "none"; canvas.style.margin = "0"; }} else {{ canvas.style.position = "relative"; canvas.style.inset = "auto"; canvas.style.width = "100%"; canvas.style.height = "100px"; canvas.style.zIndex = "1"; canvas.style.pointerEvents = "auto"; canvas.style.marginTop = "15px"; }}
            const thumb = (isIdle || isOffline) ? "/assets/music.png" : (track.thumbnail || "/assets/music.png"); 
            updateImageSmoothly(trackImg, thumb, isFirstLoad); 
            updateImageSmoothly(avatarImg, "/assets/avatar.png", isFirstLoad);
            if({show_bg_blur}) {{ updateBgSmoothly(bgBlur, thumb, '{edge_opacity}', isFirstLoad); bgBlur.classList.add("visible"); }} else {{ bgBlur.classList.remove("visible"); }}
            if({show_thumb_bg}) {{ updateBgSmoothly(bgFull, thumb, '{full_bg_opacity}', isFirstLoad); bgFull.classList.add("visible"); }} else {{ bgFull.classList.remove("visible"); }}
            
            if (isFirstLoad) {{
                // Fallback in case onload doesn't fire
                setTimeout(() => {{
                    if (isFirstLoad) {{
                        widget.style.opacity = '';
                        widget.style.transition = '';
                        isFirstLoad = false;
                    }}
                }}, 500);
            }}
        }}
        updateUI(); // Setup instantly before WS connects
        function connect() {{ 
            const socket = new WebSocket("ws://" + location.host + "/ws"); 
            socket.onopen = () => {{ isOffline = false; updateUI(); }}; 
            socket.onmessage = (e) => {{ 
                const m = JSON.parse(e.data); 
                if (m.type === "track" || m.type === "track_update") {{ track = m.data; isOffline = false; updateUI(); }} 
                if (m.type === "fft_data") {{ bins.set(m.bins); }} 
                if (m.type === "settings_update") {{ location.reload(); }} 
                if (m.type === "requires_manual_id") {{ isOffline = false; updateUI(); }}
            }}; 
            socket.onclose = () => {{ isOffline = true; updateUI(); setTimeout(connect, 2000); }}; 
        }}
        const THE_BARS = {bars_val}, heights = new Float32Array(THE_BARS).fill(0), DPR = window.devicePixelRatio || 1; 
        let lastFrame = performance.now(), frameCount = 0, lastFpsUpdate = lastFrame; const targetFps = {viz_fps}, frameInterval = 1000 / targetFps;
        function draw(now) {{ requestAnimationFrame(draw); if (isOffline) return; const elapsed = now - lastFrame; if (targetFps < 240 && elapsed < frameInterval - 1) return; lastFrame = now; const isPaused = widget.classList.contains("paused"), isIdle = widget.classList.contains("idle");
            if (!isOffline) {{ 
                const rawStart = track.startTimestamp || track.start_timestamp || 0;
                const rawEnd = track.endTimestamp || track.end_timestamp || 0;
                const start = (rawStart < 10000000000 && rawStart > 0) ? rawStart * 1000 : rawStart;
                const end = (rawEnd < 10000000000 && rawEnd > 0) ? rawEnd * 1000 : rawEnd;
                const posRaw = track.position || 0;
                const pos = posRaw < 10000000000 ? posRaw * 1000 : posRaw;
                
                const tNow = Date.now();
                const total = end > start ? end - start : 0;
                const current = isPaused ? pos : (total > 0 ? Math.min(total, Math.max(0, tNow - start)) : Math.max(0, tNow - start));
                
                const isLive = (!rawEnd || rawEnd === 0) && rawStart > 0 && !isPaused && pos === 0; 
                liveIndicator.classList.toggle("visible", isLive);

                if (isLive) {{ 
                    if(overrideProgress !== 'false') progressArea.classList.add("visible"); 
                    progBar.style.width = "100%"; 
                    timeText.innerText = "Live"; 
                }} else if (total > 0) {{ 
                    if(overrideProgress !== 'false') progressArea.classList.add("visible"); 
                    const pct = Math.min(100, (current / total) * 100); 
                    progBar.style.width = pct + "%"; 
                    const fmt = (ms) => {{ 
                        const s = Math.floor(Math.max(0, ms)/1000);
                        const h = Math.floor(s/3600);
                        const m = Math.floor((s % 3600) / 60);
                        const sc = s % 60;
                        const hPart = h > 0 ? h + ":" : "";
                        const mPart = h > 0 ? m.toString().padStart(2, "0") : m.toString();
                        return hPart + mPart + ":" + sc.toString().padStart(2, "0");
                    }}; 
                    timeText.innerText = fmt(current) + " / " + fmt(total); 
                }} else {{ 
                    progressArea.classList.remove("visible"); 
                }}
            }} else {{ 
                progressArea.classList.remove("visible"); 
                liveIndicator.classList.remove("visible"); 
            }}
            const rect = canvas.getBoundingClientRect(), dW = rect.width || 1360, dH = rect.height || 100; if (canvas.width !== Math.floor(dW * DPR) || canvas.height !== Math.floor(dH * DPR)) {{ canvas.width = Math.floor(dW * DPR); canvas.height = Math.floor(dH * DPR); }} ctx.setTransform(DPR, 0, 0, DPR, 0, 0); ctx.clearRect(0, 0, dW, dH);
            if(isPaused || isOffline || isIdle || !{viz_enabled} || !{show_viz_layout} || overrideViz === 'false') return;
            const totalUnits = ({bar_w} * THE_BARS) + ({bar_g} * (THE_BARS - 1)), barW = (dW / totalUnits) * {bar_w}, gap = (dW / totalUnits) * {bar_g}, physicsEnabled = {viz_physics};
            let barGrad = ctx.createLinearGradient(0, 0, 0, dH); if (isStatic) {{ barGrad.addColorStop(0, cTop); barGrad.addColorStop(1, cTop); }} else {{ barGrad.addColorStop(0, cTop); barGrad.addColorStop(1, cBot); }}
            let binSum = 0;
            for(let i=0; i<THE_BARS; i++) {{ 
                binSum += bins[i]; 
                heights[i] = (bins[i] / 255) * dH;
            }} 
            if (binSum > 0 && !isIdle && !isPaused && !isOffline) {{ canvas.style.opacity = '1'; canvas.style.visibility = 'visible'; }} else {{ canvas.style.opacity = '0'; canvas.style.visibility = 'hidden'; }}
            if ({viz_glow}) {{ 
                ctx.shadowBlur = 18 * DPR; 
                ctx.shadowOffsetY = 0; 
                ctx.shadowColor = cTop; 
                ctx.globalCompositeOperation = "screen";
            }} 
            ctx.fillStyle = barGrad; ctx.strokeStyle = barGrad;
            if (mode === "background") {{ const r = {rounded} ? Math.min(barW / 2, 6) : 0; ctx.beginPath(); const maxHeight = dH * 0.70; for(let i=0; i<THE_BARS; i++) {{ const h = Math.max(4, (heights[i] / dH) * maxHeight), x = i * (barW + gap), y = dH - h; if (r > 0) ctx.roundRect(x, y, barW, h, [r, r, 0, 0]); else ctx.rect(x, y, barW, h); }} ctx.globalAlpha = 0.3; ctx.fill(); ctx.globalAlpha = 1.0; }}
            else if (mode === "wave") {{ ctx.lineWidth = 6; ctx.lineJoin = "round"; ctx.lineCap = "round"; ctx.beginPath(); const pts = []; for(let i=0; i<THE_BARS; i++) pts.push({{x: i * (barW + gap) + barW/2, y: dH - Math.max(4, heights[i])}}); if(pts.length > 0) {{ ctx.moveTo(pts[0].x, pts[0].y); for (let i = 0; i < pts.length - 1; i++) {{ const xc = (pts[i].x + pts[i + 1].x) / 2, yc = (pts[i].y + pts[i + 1].y) / 2; ctx.quadraticCurveTo(pts[i].x, pts[i].y, xc, yc); }} ctx.stroke(); ctx.lineTo(dW, dH); ctx.lineTo(0, dH); ctx.globalAlpha = 0.2; ctx.fill(); ctx.globalAlpha = 1.0; }} }}
            else if (mode === "center-bars") {{ const rC = {rounded} ? Math.min(barW / 2, 6) : 0; const midY = dH / 2; ctx.beginPath(); for(let i=0; i<THE_BARS; i++) {{ const h = Math.max(4, heights[i]) / 2, x = i * (barW + gap); if (rC > 0) ctx.roundRect(x, midY - h, barW, h * 2, [rC, rC, rC, rC]); else ctx.rect(x, midY - h, barW, h * 2); }} ctx.fill(); }}
            else if (mode === "sides") {{
                const r = {rounded} ? Math.min(barW / 2, 4) : 0;
                const sideBars = Math.min(THE_BARS, Math.floor(dH / (barW + gap)));
                ctx.beginPath();
                for(let i=0; i<sideBars; i++) {{
                    const w = Math.max(4, (heights[i] / dH) * (dW * 0.25)), y = dH - (i * (barW + gap)) - barW;
                    if (r > 0) {{
                        ctx.roundRect(0, y, w, barW, [0, r, r, 0]);
                        ctx.roundRect(dW - w, y, w, barW, [r, 0, 0, r]);
                    }} else {{
                        ctx.rect(0, y, w, barW);
                        ctx.rect(dW - w, y, w, barW);
                    }}
                }}
                ctx.globalAlpha = 0.3; ctx.fill(); ctx.globalAlpha = 1.0;
            }}
            else if (mode === "mirrored") {{
                const r = {rounded} ? Math.min(barW / 2, 6) : 0;
                ctx.beginPath();
                for(let i=0; i<THE_BARS; i++) {{
                    const h = Math.max(4, heights[i]) / 2, x = i * (barW + gap);
                    if (r > 0) {{
                        ctx.roundRect(x, 0, barW, h, [0, 0, r, r]);
                        ctx.roundRect(x, dH - h, barW, h, [r, r, 0, 0]);
                    }} else {{
                        ctx.rect(x, 0, barW, h);
                        ctx.rect(x, dH - h, barW, h);
                    }}
                }}
                ctx.globalAlpha = 0.3; ctx.fill(); ctx.globalAlpha = 1.0;
            }}
            else if (mode === "outline") {{
                ctx.lineWidth = 3; ctx.strokeStyle = barGrad;
                ctx.beginPath();
                for(let i=0; i<THE_BARS; i++) {{
                    const x = i * (barW + gap) + barW/2, y = dH - heights[i];
                    if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
                }}
                ctx.stroke();
            }}
            else if (mode === "led") {{
                const dotH = 6, dotG = 2;
                for(let i=0; i<THE_BARS; i++) {{
                    const h = heights[i], x = i * (barW + gap);
                    const dots = Math.floor(h / (dotH + dotG));
                    for(let j=0; j<dots; j++) {{
                        ctx.fillRect(x, dH - (j * (dotH + dotG)) - dotH, barW, dotH);
                    }}
                }}
            }}
            else if (mode === "particles") {{
                for(let i=0; i<THE_BARS; i++) {{
                    const h = heights[i], x = i * (barW + gap) + barW/2;
                    const y = Math.max(barW / 2, dH - h);
                    ctx.beginPath();
                    ctx.arc(x, y, barW / 2, 0, Math.PI * 2);
                    ctx.fill();
                }}
            }}
            else if (mode === "neon") {{
                const r = {rounded} ? Math.min(barW / 2, 6) : 0;
                ctx.lineWidth = 2; ctx.strokeStyle = barGrad;
                ctx.beginPath();
                for(let i=0; i<THE_BARS; i++) {{
                    const h = Math.max(4, heights[i]), x = i * (barW + gap), y = dH - h;
                    if (r > 0) ctx.roundRect(x, y, barW, h, [r, r, 0, 0]); else ctx.rect(x, y, barW, h);
                }}
                ctx.stroke(); ctx.shadowBlur = 0; ctx.fillStyle = cTop; ctx.beginPath();
                for(let i=0; i<THE_BARS; i++) ctx.rect(i * (barW + gap), dH - Math.max(4, heights[i]), barW, 2);
                ctx.fill();
            }}
            else {{
                const r = {rounded} ? Math.min(barW / 2, 6) : 0;
                ctx.beginPath();
                for(let i=0; i<THE_BARS; i++) {{
                    const h = Math.max(4, heights[i]), x = i * (barW + gap), y = dH - h;
                    if (r > 0) ctx.roundRect(x, y, barW, h, [r, r, 0, 0]); else ctx.rect(x, y, barW, h);
                }}
                ctx.fill();
            }}
            ctx.shadowBlur = 0; frameCount++; if(now - lastFpsUpdate >= 1000) {{ document.getElementById("fps-text").innerText = frameCount + " FPS"; frameCount = 0; lastFpsUpdate = now; }}
        }} connect(); draw(performance.now());
    </script></body></html>"##,
        padding = padding,
        inner_radius = inner_radius,
        bg_pure = bg_pure,
        overlay_global_enabled = if overlay.enabled { 1.0 } else { 0.0 },
        widget_width = widget_width,
        widget_height = widget_height,
        gap = gap,
        border_css = border_css,
        edge_opacity = overlay.thumbnail_opacity,
        full_bg_opacity = overlay.thumbnail_background_opacity,
        bg_opacity = overlay.background_opacity,
        thumb_size = thumb_size,
        show_thumb_style = if show_thumb { "block" } else { "none" },
        avatar_offset = avatar_offset,
        avatar_size = avatar_size,
        title_size = title_size,
        text_color_css = text_color_css,
        text_anim_enabled = if text_anim_enabled { "true" } else { "false" },
        text_anim_mode = text_anim_mode,
        text_anim_duration = text_anim_duration,
        artist_size = artist_size,
        show_progress_style = show_progress_style,
        element_bg = element_bg,
        text_color_fallback = overlay.text_style.top,
        viz_display = if viz.enabled && show_viz_layout { "block" } else { "none" },
        viz_global_enabled = if viz.enabled { "visible" } else { "hidden" },
        viz_opacity = if viz.enabled { 1.0 } else { 0.0 },
        debug_display = if viz.debug_fps { "block" } else { "none" },
        layout_class = layout_mode,
        center_class = if overlay.center_text { "centered" } else { "" },
        track_json = track_json,
        bars_val = bars_val,
        is_static = viz.gradient.is_static,
        global_sync = overlay.global_sync,
        viz_mode = viz.mode,
        is_random = viz.gradient.random,
        v_top = viz.gradient.top,
        v_bot = viz.gradient.bottom,
        show_bg_blur = overlay.show_background,
        show_thumb_bg = overlay.show_thumbnail_background,
        viz_fps = viz.fps,
        viz_enabled = viz.enabled,
        show_viz_layout = show_viz_layout,
        bar_w = viz.bar_width,
        bar_g = viz.bar_gap,
        viz_physics = viz.horizontal_smoothing,
        viz_glow = viz.glow,
        rounded = viz.rounded,
    )
}
