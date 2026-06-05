use crate::models::{OverlayConfig, TrackUpdate};

fn get_widget_styles(
    padding: &str,
    inner_radius: &str,
    bg_pure: &str,
    overlay_global_enabled: f32,
    widget_width: &str,
    widget_height: &str,
    gap: &str,
    border_css: &str,
    edge_opacity: f32,
    full_bg_opacity: f32,
    bg_opacity: f32,
    thumb_size: &str,
    show_thumb_style: &str,
    avatar_offset: &str,
    avatar_size: &str,
    title_size: &str,
    text_color_css: &str,
    artist_size: &str,
    show_progress_style: &str,
    element_bg: &str,
    viz_global_enabled: &str,
    viz_opacity: f32,
    viz_display: &str,
    debug_display: &str,
) -> String {
    format!(
        r##"@font-face {{ font-family: 'Inter'; src: url('/assets/inter.woff2') format('woff2'); font-weight: 400; font-style: normal; }}
        @font-face {{ font-family: 'Inter'; src: url('/assets/inter-bold.woff2') format('woff2'); font-weight: 700; font-style: normal; }}
        :root {{
            --widget-padding: {padding};
            --inner-radius: {inner_radius};
            --outer-radius: calc(var(--inner-radius) + var(--widget-padding));
            --bg-pure: {bg_pure};
        }}
        html, body {{ background: transparent !important; margin: 0; padding: 0; overflow: hidden; width: 100vw; height: 100vh; display: flex; align-items: flex-start; justify-content: flex-start; }}
        body {{ font-family: 'Inter', sans-serif; }}
        #scale-wrapper {{ display: inline-flex; width: max-content; height: max-content; position: relative; opacity: {overlay_global_enabled}; transition: opacity 0.5s cubic-bezier(0.4, 0, 0.2, 1); }}
        #widget {{ display: inline-flex; align-items: center; justify-content: center; padding: var(--widget-padding); border-radius: var(--outer-radius); width: calc({widget_width} - 4px); height: calc({widget_height} - 4px); box-sizing: border-box; position: relative; gap: {gap}; overflow: hidden; {border_css} margin: 2px; transition: opacity 0.5s cubic-bezier(0.4, 0, 0.2, 1), background 0.5s cubic-bezier(0.4, 0, 0.2, 1), border 0.5s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.5s cubic-bezier(0.4, 0, 0.2, 1); isolation: isolate; }}
        #bg-blur {{ position: absolute; inset: -40px; z-index: -2; filter: blur(60px); opacity: 0; background-size: cover; background-position: center; transition: opacity 0.5s cubic-bezier(0.4, 0, 0.2, 1); pointer-events: none; }}
        #bg-blur.visible {{ opacity: {edge_opacity}; }}
        #bg-full {{ position: absolute; inset: 0; z-index: -3; background-size: cover; background-position: center; opacity: 0; transition: opacity 0.5s cubic-bezier(0.4, 0, 0.2, 1); pointer-events: none; }}
        #bg-full.visible {{ opacity: {full_bg_opacity}; }}
        #widget::before {{ content: ''; position: absolute; inset: 0; background: var(--bg-pure) !important; z-index: -4; opacity: {bg_opacity}; pointer-events: none; }}
        #img-container {{ position: relative !important; flex-shrink: 0; z-index: 10; display: {show_thumb_style}; width: {thumb_size}; height: {thumb_size}; transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1); }}
        #track-img-wrapper {{ width: 100%; height: 100%; border-radius: var(--inner-radius); overflow: hidden; position: relative; z-index: 100; box-shadow: 0 10px 30px rgba(0,0,0,0.6); transition: opacity 0.5s cubic-bezier(0.4, 0, 0.2, 1), visibility 0.5s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.5s cubic-bezier(0.4, 0, 0.2, 1); }}
        #track-img {{ width: 100%; height: 100%; object-fit: cover; opacity: 1; }}
        #avatar-img {{ position: absolute; top: {avatar_offset}; left: {avatar_offset}; width: {avatar_size}; height: {avatar_size}; z-index: 110; border-radius: 50%; object-fit: cover; box-shadow: 0 4px 15px rgba(0,0,0,0.5); border: none; transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1); opacity: 1; transform: translate(0, 0); }}
        #avatar-img.loading {{ opacity: 0; }}
        #info {{ display: flex; flex-direction: column; justify-content: center; min-height: 100px; flex-grow: 1; min-width: 0; transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1); margin-bottom: 15px; }}
        #info.centered {{ align-items: center; text-align: center; }}
        #text-wrapper {{ display: flex; flex-direction: column; gap: 0px; overflow: hidden; width: 100%; box-sizing: border-box; filter: drop-shadow(0 4px 12px rgba(0,0,0,0.9)); }}
        #title {{ font-weight: bold; font-size: {title_size}; margin: 0; padding-bottom: 0.15em; white-space: nowrap; line-height: normal; transition: color 0.5s cubic-bezier(0.4, 0, 0.2, 1); overflow: hidden; position: relative; width: 100%; }}
        #title span {{ display: inline-block; min-width: 100%; text-align: inherit; {text_color_css} }}
        #widget.sync-active:not(.no-cover) #title span {{ background: none !important; -webkit-background-clip: initial !important; -webkit-text-fill-color: initial !important; color: inherit !important; }}
        #widget.no-cover #title span,
        #widget.idle #title span,
        #widget.offline #title span {{ background: none !important; -webkit-background-clip: initial !important; -webkit-text-fill-color: initial !important; color: #38bdf8 !important; }}
        #widget.no-cover #artist,
        #widget.idle #artist,
        #widget.offline #artist {{ color: rgba(255, 255, 255, 0.7) !important; }}
        #widget.no-cover,
        #widget.compact-minimal {{
            gap: calc({gap} + 12px) !important;
            padding: var(--widget-padding) !important;
            justify-content: flex-start !important;
        }}
        #widget.no-cover #img-container,
        #widget.compact-minimal #img-container {{
            width: {avatar_size} !important;
            height: {avatar_size} !important;
            position: relative !important;
            left: auto !important;
            top: auto !important;
            display: flex !important;
            align-items: center !important;
            justify-content: center !important;
            padding: 0 !important;
            margin: 0 !important;
            z-index: 100 !important;
            flex-shrink: 0 !important;
        }}
        #widget.no-cover #info,
        #widget.compact-minimal #info {{
            width: auto !important;
            height: auto !important;
            margin: 0 !important;
            padding: 0 !important;
            box-sizing: border-box !important;
            flex-grow: 1 !important;
            min-width: 0 !important;
            display: flex !important;
            flex-direction: column !important;
            justify-content: center !important;
            align-items: flex-start !important;
            transform: none !important;
            transition: none !important;
        }}
        #widget.no-cover #title,
        #widget.no-cover #artist,
        #widget.compact-minimal #title,
        #widget.compact-minimal #artist {{
            text-align: left !important;
        }}
        #widget.no-cover #title span,
        #widget.compact-minimal #title span {{
            text-align: left !important;
        }}
        #widget.no-cover.centered-widget,
        #widget.compact-minimal.centered-widget {{
            justify-content: center !important;
        }}
        #widget.no-cover.centered-widget #info,
        #widget.compact-minimal.centered-widget #info {{
            align-items: center !important;
        }}
        #widget.no-cover.centered-widget #title,
        #widget.no-cover.centered-widget #artist,
        #widget.compact-minimal.centered-widget #title,
        #widget.compact-minimal.centered-widget #artist {{
            text-align: center !important;
        }}
        #widget.no-cover.centered-widget #title span,
        #widget.compact-minimal.centered-widget #title span {{
            text-align: center !important;
        }}
        #widget.no-cover #avatar-img,
        #widget.compact-minimal #avatar-img {{
            position: absolute !important;
            top: 50% !important;
            left: 50% !important;
            transform: translate(-50%, -50%) !important;
            box-shadow: 0 10px 30px rgba(0,0,0,0.6) !important;
            opacity: 1 !important;
            width: {avatar_size} !important;
            height: {avatar_size} !important;
            border-radius: 50% !important;
            transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1) !important;
        }}
        #widget.no-cover #track-img-wrapper,
        #widget.compact-minimal #track-img-wrapper {{
            opacity: 0 !important;
            visibility: hidden !important;
            pointer-events: none !important;
            position: absolute !important;
            transition: opacity 0.5s cubic-bezier(0.4, 0, 0.2, 1), visibility 0.5s cubic-bezier(0.4, 0, 0.2, 1) !important;
        }}
        #widget.paused #viz-canvas,
        #widget.offline #progress-area,
        #widget.offline #viz-canvas,
        #widget.idle #progress-area,
        #widget.idle #viz-canvas {{
            display: none !important;
        }}
        #widget.paused #track-img {{
            filter: brightness(0.7);
        }}

        #pause-overlay {{
            position: absolute;
            inset: 0;
            background: rgba(0, 0, 0, 0.4);
            backdrop-filter: blur(4px);
            -webkit-backdrop-filter: blur(4px);
            display: flex;
            align-items: center;
            justify-content: center;
            opacity: 0;
            transition: opacity 0.4s ease;
            z-index: 105;
        }}
        #pause-overlay svg {{
            width: 30%;
            height: 30%;
            color: #ffffff;
            filter: drop-shadow(0 2px 8px rgba(0,0,0,0.5));
        }}
        #widget.paused #pause-overlay {{
            opacity: 1;
        }}
        #widget.compact-bar #info {{
            min-height: 0 !important;
            margin-bottom: 0 !important;
        }}
        #widget.compact-minimal #info {{
            min-height: 0 !important;
            margin-bottom: 0 !important;
        }}
        @keyframes scrollText {{ 0%, 15% {{ transform: translateX(0); }} 85%, 100% {{ transform: translateX(var(--scroll-dist)); }} }}
        @keyframes maskPosition {{
            0%, 15% {{
                -webkit-mask-position: -30px 0;
                mask-position: -30px 0;
            }}
            85%, 100% {{
                -webkit-mask-position: 0px 0;
                mask-position: 0px 0;
            }}
        }}
        #title.scrolling {{ text-align: left !important; }}
        #title.scrolling span {{ animation: scrollText var(--scroll-duration) ease-in-out infinite alternate; text-align: left !important; }}
        #title.fade-edge.scrolling {{
            -webkit-mask-image: linear-gradient(to right, transparent 0px, black 30px, black calc(100% - 30px), transparent 100%);
            mask-image: linear-gradient(to right, transparent 0px, black 30px, black calc(100% - 30px), transparent 100%);
            -webkit-mask-size: calc(100% + 30px) 100%;
            mask-size: calc(100% + 30px) 100%;
            animation: maskPosition var(--scroll-duration) ease-in-out infinite alternate !important;
        }}
        #title.fade-edge:not(.scrolling) {{
            -webkit-mask-image: linear-gradient(to right, black calc(100% - 30px), transparent 100%);
            mask-image: linear-gradient(to right, black calc(100% - 30px), transparent 100%);
        }}
        #artist {{ font-size: {artist_size}; color: rgba(255,255,255,0.7); margin: -4px 0 0 0; padding-bottom: 6px; white-space: nowrap; overflow: hidden; text-align: inherit; }}
        #progress-area {{ display: none; align-items: center; width: 100%; gap: 15px; margin-top: 14px; opacity: 0; transition: opacity 0.4s ease; }}
        #progress-area.visible {{ display: {show_progress_style} !important; opacity: 1; }}
        #progress-container {{ flex-grow: 1; height: 24px; background: {element_bg}; border: 1px solid rgba(255, 255, 255, 0.15); border-radius: 50px; overflow: hidden; box-shadow: inset 0 2px 5px rgba(0,0,0,0.5), 0 0 0 1px rgba(0,0,0,0.3); min-width: 200px; }}
        #progress-bar {{ width: 0%; height: 100%; border-radius: 50px; transition: width 0.1s linear, background 0.3s ease; box-shadow: inset 0 1px 2px rgba(255,255,255,0.4), inset 0 -1px 2px rgba(0,0,0,0.2), 0 0 4px rgba(0,0,0,0.6); }}
        #time-text {{ font-size: calc({artist_size} * 0.9); font-weight: 700; font-variant-numeric: tabular-nums; text-shadow: 0 2px 5px rgba(0,0,0,0.8); white-space: nowrap; color: rgba(255, 255, 255, 0.8) !important; }}
        #live-indicator {{ display: none; align-items: center; justify-content: flex-end; gap: 12px; color: #ff4444; font-weight: bold; font-size: calc({artist_size} * 0.7); text-transform: uppercase; letter-spacing: 2px; text-shadow: 0 0 15px rgba(255,68,68,0.4); margin-top: 5px; width: 100%; }}
        #live-indicator.visible {{ display: flex; }}
        #live-dot {{ width: 12px; height: 12px; background-color: #ff4444; border-radius: 50%; box-shadow: 0 0 10px #ff4444; animation: pulse 1.5s infinite; }}
        @keyframes pulse {{ 0% {{ transform: scale(1); opacity: 1; }} 50% {{ transform: scale(1.3); opacity: 0.5; }} 100% {{ transform: scale(1); opacity: 1; }} }}
        #viz-canvas {{ display: none; margin-top: 5px; width: 100%; height: 100px; visibility: {viz_global_enabled}; opacity: {viz_opacity}; transition: opacity 0.4s ease, visibility 0.4s ease; }}
        #viz-canvas.visible {{ display: {viz_display} !important; }}
        #fps-text {{ position: absolute; bottom: 15px; right: 30px; font-size: 1.2em; color: rgba(255,255,255,0.3); font-family: monospace; display: {debug_display}; z-index: 1000; }}
        #widget.idle #info, #widget.offline #info {{ min-height: 100px; }}
        #widget.idle #progress-area, #widget.idle #viz-canvas, #widget.idle #fps-text,
        #widget.offline #progress-area, #widget.offline #viz-canvas, #widget.offline #fps-text {{ display: none !important; opacity: 0 !important; visibility: hidden !important; }}
        #widget.compact-bar #time-text {{ display: none !important; }}
        #widget.compact-minimal #progress-area {{ display: none !important; }}
        #widget.compact-minimal #track-img-wrapper {{ display: none !important; }}
        #widget.compact-bar #viz-canvas, #widget.compact-minimal #viz-canvas {{ display: none !important; }}
        "##,
        padding = padding,
        inner_radius = inner_radius,
        bg_pure = bg_pure,
        overlay_global_enabled = overlay_global_enabled,
        widget_width = widget_width,
        widget_height = widget_height,
        gap = gap,
        border_css = border_css,
        edge_opacity = edge_opacity,
        full_bg_opacity = full_bg_opacity,
        bg_opacity = bg_opacity,
        thumb_size = thumb_size,
        show_thumb_style = show_thumb_style,
        avatar_offset = avatar_offset,
        avatar_size = avatar_size,
        title_size = title_size,
        text_color_css = text_color_css,
        artist_size = artist_size,
        show_progress_style = show_progress_style,
        element_bg = element_bg,
        viz_display = viz_display,
        viz_global_enabled = viz_global_enabled,
        viz_opacity = viz_opacity,
        debug_display = debug_display,
    )
}

fn get_widget_scripts(
    track_json: &str,
    bars_val: u32,
    is_static: bool,
    global_sync: bool,
    viz_mode: &str,
    is_random: bool,
    v_top: &str,
    v_bot: &str,
    text_anim_enabled: &str,
    text_anim_mode: &str,
    text_anim_duration: u32,
    edge_opacity: f32,
    full_bg_opacity: f32,
    show_bg_blur: bool,
    show_thumb_bg: bool,
    viz_fps: u32,
    viz_enabled: bool,
    show_viz_layout: bool,
    bar_w: u32,
    bar_g: u32,
    viz_physics: bool,
    viz_glow: bool,
    rounded: bool,
) -> String {
    format!(
        r##"
        const urlParams = new URLSearchParams(window.location.search);
        const overrideViz = urlParams.get('visualizer');
        const overrideProgress = urlParams.get('progress');
        let track = {track_json}, bins = new Uint8Array({bars_val}).fill(0), isOffline = false, isFirstLoad = true;
        const widget = document.getElementById("widget"), bgBlur = document.getElementById("bg-blur"), bgFull = document.getElementById("bg-full");
        const canvas = document.getElementById("viz-canvas"), ctx = canvas.getContext("2d", {{ alpha: true }});
        const progressArea = document.getElementById("progress-area");
        if (overrideProgress === 'false' && progressArea) progressArea.style.display = 'none';

        // 🚀 CACHED CANVAS DIMENSIONS FOR STABLE 165+ FPS RENDERING
        const DPR = window.devicePixelRatio || 1;
        let rect = canvas.getBoundingClientRect(), dW = rect.width || 1360, dH = rect.height || 100;
        function updateCanvasSize() {{
            rect = canvas.getBoundingClientRect();
            dW = rect.width || 1360;
            dH = rect.height || 100;
            const targetW = Math.floor(dW * DPR);
            const targetH = Math.floor(dH * DPR);
            if (canvas.width !== targetW || canvas.height !== targetH) {{
                canvas.width = targetW;
                canvas.height = targetH;
            }}
        }}
        window.addEventListener('resize', updateCanvasSize);
        const randomHex = () => {{
            const h = Math.random();
            const s = 0.85;
            const l = 0.65;
            const hue2rgb = (p, q, t) => {{
                if (t < 0) t += 1;
                if (t > 1) t -= 1;
                if (t < 1/6) return p + (q - p) * 6 * t;
                if (t < 1/2) return q;
                if (t < 2/3) return p + (q - p) * (2/3 - t) * 6;
                return p;
            }};
            const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
            const p = 2 * l - q;
            const r = Math.round(hue2rgb(p, q, h + 1/3) * 255);
            const g = Math.round(hue2rgb(p, q, h) * 255);
            const b = Math.round(hue2rgb(p, q, h - 1/3) * 255);
            return "#" + [r, g, b].map(x => x.toString(16).padStart(2, '0')).join('');
        }};
        const isStatic = {is_static}, isGlobalSync = {global_sync}, mode = "{viz_mode}";
        let cTop = isGlobalSync ? "#38bdf8" : ({is_random} ? randomHex() : "{v_top}");
        let cBot = isGlobalSync ? "#38bdf8" : ({is_random} ? randomHex() : "{v_bot}");
        if (isStatic || isGlobalSync) cBot = cTop;
        const progBar = document.getElementById("progress-bar"), timeText = document.getElementById("time-text"), liveIndicator = document.getElementById("live-indicator"), trackImg = document.getElementById('track-img');
        const titleEl = document.getElementById('title'), trackImgWrapper = document.getElementById('track-img-wrapper'), avatarImg = document.getElementById('avatar-img');
        trackImg.onerror = () => {{
            trackImg.src = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
            widget.classList.add('no-cover');
        }};
        function updateTitleFade() {{ 
            titleEl.classList.remove('scrolling'); 
            titleEl.style.transform = "translateX(0)"; 
            // Force a synchronous reflow to ensure the browser registers the animation reset instantly
            void titleEl.offsetWidth;
            
            if ({text_anim_enabled} && titleEl.clientWidth > 0 && titleEl.scrollWidth > titleEl.clientWidth + 5) {{ 
                titleEl.classList.add('fade-edge'); 
                const dist = titleEl.scrollWidth - titleEl.clientWidth; 
                titleEl.style.setProperty('--scroll-dist', `-${{dist + 20}}px`); 
                
                const animMode = "{text_anim_mode}";
                let duration = {text_anim_duration};
                if (animMode === "dynamic") {{
                    duration = Math.max(4, Math.floor(dist / 40)); 
                }}
                titleEl.style.setProperty('--scroll-duration', `${{duration}}s`);
                
                // Force a second reflow to apply custom property variables before restarting the animation
                void titleEl.offsetWidth;
                titleEl.classList.add('scrolling'); 
            }} else {{ 
                titleEl.classList.remove('fade-edge'); 
            }} 
        }}
        window.addEventListener('resize', updateTitleFade);
        function applyColorsFromImg(imgEl) {{
            const thumb = imgEl.src || "";
            const isNoCover = isOffline || (track && track.status === 'idle') || thumb.endsWith('/assets/music.png') || thumb.endsWith('/assets/icon.png') || thumb.startsWith('data:image/');
            widget.classList.toggle("no-cover", isNoCover);

            if (isGlobalSync && !isOffline && !isNoCover) {{ 
                try {{ 
                    const tC = document.createElement('canvas'); 
                    tC.width = 1; 
                    tC.height = 1; 
                    const tCtx = tC.getContext('2d'); 
                    tCtx.drawImage(imgEl, 0, 0, 1, 1); 
                    let [r, g, b] = tCtx.getImageData(0, 0, 1, 1).data; 
                    let luminance = 0.299 * r + 0.587 * g + 0.114 * b; 
                    if (luminance < 70) {{ 
                        const boost = (70 - luminance) / 70; 
                        r = Math.min(255, Math.floor(r + (255 - r) * boost)); 
                        g = Math.min(255, Math.floor(g + (255 - g) * boost)); 
                        b = Math.min(255, Math.floor(b + (255 - b) * boost)); 
                    }}
                    const domColor = `rgb(${{r}},${{g}},${{b}})`; 
                    cTop = domColor; 
                    cBot = domColor; 
                    titleEl.style.color = domColor; 
                    avatarImg.style.borderColor = domColor; 
                }} catch(e) {{ 
                    console.log("CORS block"); 
                }}
            }} else if ({is_random} && !isOffline && !isNoCover) {{
                titleEl.style.color = cTop;
                avatarImg.style.borderColor = cTop;
            }} else {{
                titleEl.style.color = '';
                avatarImg.style.borderColor = '';
            }}
            if ((isGlobalSync || {is_random}) && !isOffline && !isNoCover) {{
                widget.style.border = '2px solid transparent';
                widget.style.background = 'linear-gradient(var(--bg-pure), var(--bg-pure)) padding-box, linear-gradient(90deg, ' + cBot + ', ' + cTop + ') border-box';
            }} else {{
                widget.style.border = '';
                widget.style.background = '';
            }}
            progBar.style.background = isStatic ? cTop : `linear-gradient(90deg, ${{cBot}}, ${{cTop}})`; 
            progBar.style.boxShadow = `0 0 10px ${{cTop}}80`;
            
            // Dynamic Progress Background Sync
            if (cTop.startsWith('#')) {{
                document.getElementById('progress-container').style.background = cTop + '26';
            }} else {{
                document.getElementById('progress-container').style.background = cTop.replace('rgb', 'rgba').replace(')', ', 0.15)');
            }}
        }}
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
                    el.src = el === trackImg ? "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7" : "/assets/icon.png"; 
                    el.classList.remove('loading'); 
                    if (el === trackImg) {{
                        widget.classList.add('no-cover');
                    }}
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
        trackImg.onload = () => {{ 
            if (trackImg.src.endsWith('/assets/icon.png') || trackImg.src.endsWith('/assets/music.png') || trackImg.src.startsWith('data:image/')) {{ 
                trackImgWrapper.style.boxShadow = 'none'; 
            }} else {{ 
                trackImgWrapper.style.boxShadow = '0 10px 30px rgba(0,0,0,0.6)'; 
            }}
            applyColorsFromImg(trackImg);
            
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
            
            // Safe fine-grained state toggling (prevents overwriting layout and no-cover classes)
            widget.classList.remove("offline", "idle", "paused", "playing", "sync-active");
            widget.classList.add(isOffline ? "offline" : (isIdle ? "idle" : (isPaused ? "paused" : "playing")));
            if (isPaused && !isIdle && !isOffline) widget.classList.add("playing");
            if ((isGlobalSync || {is_random}) && !isOffline) widget.classList.add("sync-active");
            
            const isCentered = document.getElementById("info").classList.contains("centered");
            widget.classList.toggle("centered-widget", isCentered);
            
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
            setTimeout(updateTitleFade, 50);
            if (mode === "background" || mode === "sides" || mode === "mirrored") {{ canvas.style.position = "absolute"; canvas.style.inset = "0"; canvas.style.width = "100%"; canvas.style.height = "100%"; canvas.style.zIndex = "-1"; canvas.style.pointerEvents = "none"; canvas.style.margin = "0"; }} else {{ canvas.style.position = "relative"; canvas.style.inset = "auto"; canvas.style.width = "100%"; canvas.style.height = "100px"; canvas.style.zIndex = "1"; canvas.style.pointerEvents = "auto"; canvas.style.marginTop = "15px"; }}
            const thumb = track.thumbnail || "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"; 
            if (trackImg.src !== thumb && !trackImg.src.endsWith(thumb)) {{
                trackImg.src = thumb;
            }} 
            updateImageSmoothly(avatarImg, "/assets/avatar.png", isFirstLoad);
            const isNoCover = isOffline || isIdle || thumb.endsWith('/assets/music.png') || thumb.endsWith('/assets/icon.png') || thumb.startsWith('data:image/');
            widget.classList.toggle("no-cover", isNoCover);
            
            const hasRealCover = !isOffline && !isIdle && track.thumbnail && !track.thumbnail.endsWith('/assets/music.png') && !track.thumbnail.endsWith('/assets/icon.png') && !track.thumbnail.startsWith('data:image/');
            const shouldShowBlur = hasRealCover;
            
            if ({show_bg_blur} && shouldShowBlur) {{
                updateBgSmoothly(bgBlur, track.thumbnail, '{edge_opacity}', isFirstLoad);
                bgBlur.classList.add("visible");
            }} else {{
                bgBlur.classList.remove("visible");
            }}
            if ({show_thumb_bg} && shouldShowBlur) {{
                updateBgSmoothly(bgFull, track.thumbnail, '{full_bg_opacity}', isFirstLoad);
                bgFull.classList.add("visible");
            }} else {{
                bgFull.classList.remove("visible");
            }}
            if (isNoCover) {{
                titleEl.style.color = '';
                avatarImg.style.borderColor = '';
                widget.style.border = '';
                widget.style.background = '';
            }} else {{
                if (isFirstLoad && (!isGlobalSync || trackImg.src.endsWith(thumb))) {{
                    applyColorsFromImg(trackImg);
                }}
            }}
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
            updateCanvasSize();
        }}
        updateUI(); // Setup instantly before WS connects
        let wasConnected = false;
        function connect() {{ 
            const socket = new WebSocket("ws://" + location.host + "/ws"); 
            socket.onopen = () => {{ 
                isOffline = false; 
                if (wasConnected) {{
                    location.reload();
                    return;
                }}
                wasConnected = true;
                updateUI(); 
            }}; 
            socket.onmessage = (e) => {{ 
                const m = JSON.parse(e.data); 
                if (m.type === "track" || m.type === "track_update") {{ track = m.data; isOffline = false; updateUI(); }} 
                if (m.type === "fft_data") {{ bins.set(m.bins); }} 
                if (m.type === "settings_update") {{ location.reload(); }} 
                if (m.type === "requires_manual_id") {{ isOffline = false; updateUI(); }}
            }}; 
            socket.onclose = () => {{ isOffline = true; updateUI(); setTimeout(connect, 2000); }}; 
        }}
        const THE_BARS = {bars_val}, heights = new Float32Array(THE_BARS).fill(0); 
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
                if (liveIndicator.classList.contains("visible") !== isLive) {{
                    liveIndicator.classList.toggle("visible", isLive);
                }}

                const isProgressVisible = progressArea.classList.contains("visible");
                if (isLive) {{ 
                    if (isProgressVisible) progressArea.classList.remove("visible"); 
                }} else if (total > 0) {{ 
                    if (overrideProgress !== 'false' && !isProgressVisible) progressArea.classList.add("visible"); 
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
                    const timeStr = fmt(current) + " / " + fmt(total);
                    if (timeText.innerText !== timeStr) {{
                        timeText.innerText = timeStr;
                    }}
                }} else {{ 
                    if (isProgressVisible) progressArea.classList.remove("visible"); 
                }}
            }} else {{ 
                if (progressArea.classList.contains("visible")) progressArea.classList.remove("visible"); 
                if (liveIndicator.classList.contains("visible")) liveIndicator.classList.remove("visible"); 
            }}
            ctx.setTransform(DPR, 0, 0, DPR, 0, 0); ctx.clearRect(0, 0, dW, dH);
            const isPausedOrInactive = isPaused || isOffline || isIdle || !{viz_enabled} || !{show_viz_layout} || overrideViz === 'false';
            if (isPausedOrInactive) {{
                if (canvas.classList.contains("visible")) {{
                    canvas.classList.remove("visible");
                    canvas.style.opacity = '0';
                    canvas.style.visibility = 'hidden';
                }}
                return;
            }} else {{
                if (!canvas.classList.contains("visible")) {{
                    canvas.classList.add("visible");
                    canvas.style.opacity = '1';
                    canvas.style.visibility = 'visible';
                }}
            }} 
            const totalUnits = ({bar_w} * THE_BARS) + ({bar_g} * (THE_BARS - 1)), barW = (dW / totalUnits) * {bar_w}, gap = (dW / totalUnits) * {bar_g}, physicsEnabled = {viz_physics};
            let barGrad = ctx.createLinearGradient(0, 0, 0, dH); if (isStatic) {{ barGrad.addColorStop(0, cTop); barGrad.addColorStop(1, cTop); }} else {{ barGrad.addColorStop(0, cTop); barGrad.addColorStop(1, cBot); }}
            let binSum = 0;
            for(let i=0; i<THE_BARS; i++) {{ 
                binSum += bins[i]; 
                heights[i] = (bins[i] / 255) * (dH * 0.85);
            }}
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
        }}
        connect();
        draw(performance.now());
        "##,
        track_json = track_json,
        bars_val = bars_val,
        is_static = is_static,
        global_sync = global_sync,
        viz_mode = viz_mode,
        is_random = is_random,
        v_top = v_top,
        v_bot = v_bot,
        text_anim_enabled = text_anim_enabled,
        text_anim_mode = text_anim_mode,
        text_anim_duration = text_anim_duration,
        edge_opacity = edge_opacity,
        full_bg_opacity = full_bg_opacity,
        show_bg_blur = show_bg_blur,
        show_thumb_bg = show_thumb_bg,
        viz_fps = viz_fps,
        viz_enabled = viz_enabled,
        show_viz_layout = show_viz_layout,
        bar_w = bar_w,
        bar_g = bar_g,
        viz_physics = viz_physics,
        viz_glow = viz_glow,
        rounded = rounded,
    )
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

    let initial_details = track.details.as_deref().unwrap_or("");
    let initial_is_idle = track.status == "idle" || initial_details.trim().is_empty() || initial_details.to_lowercase() == "resting...";
    let initial_thumb = if initial_is_idle { "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7" } else { track.thumbnail.as_deref().unwrap_or("data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7") };
    let initial_no_cover = initial_is_idle || initial_thumb.ends_with("/assets/music.png") || initial_thumb.ends_with("/assets/icon.png") || initial_thumb.starts_with("data:image/");
    let is_dynamic_active = (overlay.global_sync || viz.gradient.random) && !initial_no_cover;
    let initial_border_css = if is_dynamic_active {
        "border: 2px solid transparent; background: transparent;".to_string()
    } else {
        border_css.clone()
    };

    let element_bg = get_css(&overlay.element_style);
    let bg_pure = &overlay.background.color;
    let show_progress_style = if show_progress_layout { "flex" } else { "none" };
    let bars_val = viz.bars;

    let widget_styles = get_widget_styles(
        padding,
        inner_radius,
        bg_pure,
        if overlay.enabled { 1.0 } else { 0.0 },
        widget_width,
        widget_height,
        gap,
        &initial_border_css,
        overlay.thumbnail_opacity,
        overlay.thumbnail_background_opacity,
        overlay.background_opacity,
        thumb_size,
        if show_thumb { "block" } else { "none" },
        avatar_offset,
        avatar_size,
        title_size,
        &text_color_css,
        artist_size,
        show_progress_style,
        &element_bg,
        if viz.enabled { "visible" } else { "hidden" },
        if viz.enabled { 1.0 } else { 0.0 },
        if viz.enabled && show_viz_layout { "block" } else { "none" },
        if viz.debug_fps { "block" } else { "none" },
    );

    let widget_scripts = get_widget_scripts(
        &track_json,
        bars_val,
        viz.gradient.is_static,
        overlay.global_sync,
        &viz.mode,
        viz.gradient.random,
        &viz.gradient.top,
        &viz.gradient.bottom,
        if text_anim_enabled { "true" } else { "false" },
        text_anim_mode,
        text_anim_duration,
        overlay.thumbnail_opacity,
        overlay.thumbnail_background_opacity,
        overlay.show_background,
        overlay.show_thumbnail_background,
        viz.fps,
        viz.enabled,
        show_viz_layout,
        viz.bar_width,
        viz.bar_gap,
        viz.horizontal_smoothing,
        viz.glow,
        viz.rounded,
    );

    let center_class = if overlay.center_text { "centered" } else { "" };

    format!(
        r##"<!DOCTYPE html><html><head><style>
        {widget_styles}
        </style></head><body><div id="scale-wrapper"><div id="widget" class="offline {layout_class}"><div id="bg-blur"></div><div id="bg-full"></div><div id="img-container"><img id="avatar-img" src="/assets/avatar.png" alt="Avatar" /><div id="track-img-wrapper"><img id="track-img" src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7" crossorigin="anonymous" /><div id="pause-overlay"><svg viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg></div></div></div><div id="info" class="{center_class}"><div id="text-wrapper"><div id="title"><span>Server Offline</span></div><div id="artist">Waiting for Rust...</div></div><div id="live-indicator"><div id="live-dot"></div><span>LIVE</span></div><div id="progress-area"><div id="progress-container"><div id="progress-bar"></div></div><div id="time-text">0:00 / 0:00</div></div><canvas id="viz-canvas"></canvas></div><div id="fps-text">0 FPS</div></div></div><script>
        {widget_scripts}
        </script></body></html>"##,
        widget_styles = widget_styles,
        layout_class = layout_mode,
        center_class = center_class,
        widget_scripts = widget_scripts,
    )
}
