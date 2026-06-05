import http from "http";
import WebSocket from "ws";
import fs from "fs";
import path from "path";
import { PATHS, GLOBAL_STATE, TrackUpdate, VisualizerConfig } from "../utils/constants";

let httpServer: http.Server | null = null;
let wss: WebSocket.Server | null = null;

export const broadcast = (data: any): void => {
  if (!GLOBAL_STATE.overlayClients || GLOBAL_STATE.isShuttingDown) return;
  const msg = JSON.stringify(data);
  GLOBAL_STATE.overlayClients.forEach((c) => {
    if (c.readyState === WebSocket.OPEN) c.send(msg);
  });
};

const processThumbnail = (url?: string): string => {
  if (!url) return "/assets/music.png";
  if (url.includes('ytimg.com')) {
    return url.replace(/\/maxresdefault\.jpg/, '/hqdefault.jpg').replace(/\/(?:default|hqdefault|sddefault|mqdefault)\.jpg/, '/maxresdefault.jpg');
  }
  return url;
};

export const updateTrack = (data: TrackUpdate): void => {
  if (data && data.details && data.state) {
    let artistName = data.state;
    if (artistName.toLowerCase().startsWith("by ")) artistName = artistName.substring(3).trim();
    const escapedArtist = artistName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const separators = ["\\s*-\\s*", "\\s*–\\s*", "\\s*—\\s*", ":\\s*", "\\|\\s*", "\\s*~\\s*", "\\s*by\\s+"];
    const sep_join = separators.join("|");

    const artistRegex = new RegExp(`(?:^|\\s+)${escapedArtist}(?:${sep_join})`, "i");
    data.details = data.details.replace(artistRegex, "").trim();

    const suffixRegex = new RegExp(`(?:${sep_join})\\s*${escapedArtist}\\s*$`, "i");
    data.details = data.details.replace(suffixRegex, "").trim();

    const junkPatterns = [
      /\s*[\[\(\]]?(?:Copyright[\s-]*Freel?|Official Music Video|Official Video|Official Audio|Music Video|Lyrics|Audio|Video)[^\]\)]*[\]\)]?\s*/gi,
      /\s*[\(\[].*?(?:Video|Audio|Lyrics|Version|Remix).*?[\)\]]\s*/gi,
      /\[Copyright[\s-]*Freel?\]?/gi,
      /\s*No\.\s*\d+\s*/gi,
    ];
    junkPatterns.forEach((p) => { data.details = data.details!.replace(p, " "); });
    data.details = data.details.replace(/\s\s+/g, " ").trim();
    data.thumbnail = processThumbnail(data.thumbnail);
  }
  GLOBAL_STATE.lastTrack = data;
  broadcast({ type: "track_update", data: data || {} });
};

export const start = (): http.Server | null => {
  if (httpServer) return httpServer;
  const s = GLOBAL_STATE.settings;
  if (!s) return null;

  httpServer = http.createServer((req, res) => {
      if (req.url && req.url.includes("/assets/")) {
        const fileName = path.basename(req.url);
        const target = [path.join(PATHS.assets, fileName), path.join(PATHS.internalAssets, fileName)].find((p) => fs.existsSync(p));
        if (target) {
          const ext = path.extname(target).toLowerCase();
          const mime = ext === ".png" ? "image/png" : ext === ".ico" ? "image/x-icon" : "application/octet-stream";
          res.writeHead(200, { "Content-Type": mime, "Cache-Control": "public, max-age=86400" });
          return res.end(fs.readFileSync(target));
        }
      }
      res.writeHead(200, { "Content-Type": "text/html", "Cache-Control": "no-store" });
      try { res.end(getHTML(s.overlay.layout, s.overlay.visualizer, GLOBAL_STATE.lastTrack)); } catch (e: any) { res.end(`Error: ${e.message}`); }
    }).listen(s.overlay.port);

  wss = new WebSocket.Server({ server: httpServer });
  wss.on("connection", (ws) => {
    GLOBAL_STATE.overlayClients.push(ws);
    if (GLOBAL_STATE.lastTrack) ws.send(JSON.stringify({ type: "track_update", data: GLOBAL_STATE.lastTrack }));
    ws.on("close", () => { GLOBAL_STATE.overlayClients = GLOBAL_STATE.overlayClients.filter((c) => c !== ws); });
  });
  return httpServer;
};

export const stop = (): void => {
  if (wss) {
    wss.clients.forEach(client => {
        try { client.send(JSON.stringify({ type: "program_shutdown" })); } catch(e) {}
        client.terminate();
    });
    wss.close();
  }
  if (httpServer) httpServer.close();
};

export const getHTML = (layout: string, viz: VisualizerConfig, lastTrack: TrackUpdate | null): string => {
  const track = lastTrack || {};
  const isPaused = !!track.paused;
  const isIdle = !track.details;
  const isStatusOnly = !!track.details && !track.end_timestamp && !track.start_timestamp;
  
  const thumb = track.thumbnail || "/assets/music.png";
  const title = track.details || (isPaused ? "Paused" : "Not Playing");
  const artist = track.state || "T_Music_Bot";

  const randomHex = () => "#" + Math.floor(Math.random()*16777215).toString(16).padStart(6, '0');
  const colorTop = viz.colorTop || randomHex();
  const colorBot = viz.colorBottom || randomHex();

  // Layout sizing (matches Rust CSS logic)
  let width = 1360, height = 440;
  if (layout === "compact-bar") { width = 1000; height = 220; }
  else if (layout === "compact-minimal") { width = 800; height = 160; }

  return `<!DOCTYPE html><html><head><style>
          @import url('https://fonts.googleapis.com/css2?family=Lexend:wght@400;700&display=swap');
          html, body { background: transparent !important; margin: 0; padding: 0; overflow: hidden; width: \${width}px; height: \${height}px; }
          body { font-family: 'Lexend', sans-serif; }
          #scale-wrapper { width: \${width}px; height: \${height}px; position: relative; background: transparent !important; }
          #bg-blur { position: absolute; top: 0; left: 0; width: 100%; height: 100%; z-index: -1; filter: blur(60px); opacity: 0; background-size: cover; background-position: center; border-radius: 48px; pointer-events: none; transition: opacity 0.3s; background-color: transparent !important; }
          #bg-blur.visible { opacity: 0.5; }
          #widget { display: flex; align-items: center; background: transparent !important; color: white; padding: 60px; border-radius: 48px; width: 100%; height: 100%; box-sizing: border-box; position: relative; gap: 40px; overflow: hidden; border: none !important; }
          #widget.playing::before { content: ''; position: absolute; inset: 0; background: transparent !important; z-index: -1; border-radius: 48px; }
          #widget.shutdown { opacity: 0 !important; visibility: hidden !important; }
          #img-container { position: relative; width: 320px; height: 320px; flex-shrink: 0; background: transparent !important; z-index: 100; }
          #avatar-img { position: absolute; top: -15px; left: -15px; width: 110px; height: 110px; z-index: 120; object-fit: cover; background: white !important; border: 4px solid rgba(255,255,255,0.4); border-radius: 50%; box-shadow: 0 4px 12px rgba(0,0,0,0.5); }
          #track-img-wrapper { width: 100%; height: 100%; border-radius: 32px; overflow: hidden; background: transparent !important; position: relative; z-index: 100; box-shadow: 0 8px 24px rgba(0,0,0,0.5); }
          #track-img { width: 100%; height: 100%; object-fit: cover; background: transparent !important; transition: none; }
          #track-img.crop-yt { transform: scale(1.35); }
          #info { display: flex; flex-direction: column; justify-content: center; height: 320px; flex-grow: 1; min-width: 0; overflow: hidden; }
          #widget.playing #info { justify-content: space-between; }
          #widget.idle #info, #widget.offline #info, #widget.paused #info, #widget.status-only #info { justify-content: center !important; }
          #text-wrapper { width: 100%; display: flex; flex-direction: column; overflow: hidden; text-align: left; }
          #widget.idle #text-wrapper, #widget.paused #text-wrapper, #widget.status-only #text-wrapper, #widget.offline #text-wrapper { text-align: center !important; width: 100%; }
          #title { font-weight: bold; font-size: 3.8em; margin: 0; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; line-height: 1.2; text-shadow: 0 4px 12px rgba(0,0,0,0.9); }
          #artist { font-size: 2.8em; color: #ccc; margin: 0; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; }
          #progress-area { display: none; align-items: center; width: 100%; gap: 15px; margin-top: 10px; }
          #progress-area.visible { display: flex !important; }
          #progress-container { flex-grow: 1; height: 14px; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.08); border-radius: 7px; overflow: hidden; position: relative; }
          #progress-bar { width: 0%; height: 100%; background: linear-gradient(90deg, \${colorBot}, \${colorTop}); box-shadow: 0 0 10px \${colorTop}80; border-radius: 6px; transition: width 0.1s linear; }
          #time-text { font-size: 2.8em; color: #fff; font-weight: 500; font-variant-numeric: tabular-nums; white-space: nowrap; text-shadow: 0 0 5px rgba(0,0,0,0.5); }
          #live-indicator { display: none; align-items: center; justify-content: flex-end; gap: 12px; color: #ff4444; font-weight: bold; font-size: 2.2em; text-transform: uppercase; letter-spacing: 2px; text-shadow: 0 0 15px rgba(255,68,68,0.4); margin: 10px 0; width: 100%; padding-right: 20px; box-sizing: border-box; }
          #live-indicator.visible { display: flex; }
          #live-dot { width: 18px; height: 18px; background-color: #ff4444; border-radius: 50%; box-shadow: 0 0 10px #ff4444; animation: pulse 1.5s infinite; }
          @keyframes pulse { 0% { transform: scale(1); opacity: 1; } 50% { transform: scale(1.3); opacity: 0.5; } 100% { transform: scale(1); opacity: 1; } }
          #viz-canvas { width: 880px; height: 100px; margin: 0; \${viz.enabled ? "" : "display: none;"} background: transparent !important; }
          #fps-text { position: absolute; bottom: 10px; right: 20px; font-size: 1.2em; color: rgba(255,255,255,0.4); font-family: monospace; z-index: 200; }
          
          /* LAYOUT OVERRIDES */
          #widget.compact-bar { padding: 30px; border-radius: 32px; gap: 30px; }
          #widget.compact-bar #img-container { width: 160px; height: 160px; }
          #widget.compact-bar #info { height: 160px; }
          #widget.compact-bar #title { font-size: 2.4em; }
          #widget.compact-bar #artist { font-size: 1.8em; }
          #widget.compact-bar #viz-canvas { display: none !important; }

          #widget.compact-minimal { padding: 25px; border-radius: 24px; gap: 20px; }
          #widget.compact-minimal #img-container { display: none; }
          #widget.compact-minimal #info { height: 110px; text-align: center; }
          #widget.compact-minimal #text-wrapper { text-align: center; }
          #widget.compact-minimal #title { font-size: 2.2em; }
          #widget.compact-minimal #artist { font-size: 1.6em; }
          #widget.compact-minimal #progress-area, #widget.compact-minimal #viz-canvas { display: none !important; }

          /* Visibility Overrides */
          #widget.idle #progress-area, #widget.idle #viz-canvas, #widget.idle #live-indicator,
          #widget.status-only #progress-area, #widget.status-only #viz-canvas, #widget.status-only #live-indicator,
          #widget.paused #viz-canvas, #widget.paused #live-indicator,
          #widget.offline #progress-area, #widget.offline #viz-canvas, #widget.offline #live-indicator,
          #widget.offline #img-container { display: none !important; opacity: 0 !important; visibility: hidden !important; }
          #widget.offline #artist { color: #ff4444 !important; font-weight: bold; }
      </style></head><body><div id="scale-wrapper"><div id="bg-blur" class="\${isIdle ? "" : "visible"}" style="background-image: url('\${thumb}');"></div><div id="widget" class="\${layout} \${isIdle ? "idle visible" : (isStatusOnly ? "status-only visible" : (isPaused ? "paused playing visible" : "playing visible"))}"><div id="img-container"><img id="avatar-img" src="/assets/avatar.png" loading="eager" /><div id="track-img-wrapper"><img id="track-img" src="\${thumb}" loading="eager" /></div></div><div id="info"><div id="text-wrapper"><div id="title">\${title}</div><div id="artist">\${artist}</div></div><div id="live-indicator"><div id="live-dot"></div><span>LIVE</span></div><div id="progress-area"><div id="progress-container"><div id="progress-bar"></div></div><div id="time-text">0:00 / 0:00</div></div><canvas id="viz-canvas"></canvas></div><div id="fps-text">0 FPS</div></div></div>
        <script>
            let track = \${JSON.stringify(track)}, isIdle = \${isIdle}, bins = new Uint8Array(\${viz.bars}).fill(0), isOffline = false;
            const widget = document.getElementById("widget"), bgBlur = document.getElementById("bg-blur");
            const canvas = document.getElementById("viz-canvas"), ctx = canvas.getContext("2d", { alpha: true });

            function setThumbnail(url) {
                const img = document.getElementById('track-img');
                if (!url || !url.includes('ytimg.com')) {
                    img.src = url || "/assets/music.png";
                    bgBlur.style.backgroundImage = 'url(' + (url || "/assets/music.png") + ')';
                    img.classList.remove('crop-yt');
                    return;
                }
                const maxres = url.replace(/\\/maxresdefault\\.jpg/, '/hqdefault.jpg').replace(/\\/(?:default|hqdefault|sddefault|mqdefault)\\.jpg/, '/maxresdefault.jpg');
                const hq = url.replace(/\\/(?:default|maxresdefault|sddefault|mqdefault)\\.jpg/, '/hqdefault.jpg');
                const temp = new Image();
                temp.onload = () => {
                    const final = (temp.width === 120 && temp.height === 90) ? hq : maxres;
                    img.src = final; bgBlur.style.backgroundImage = 'url(' + final + ')';
                    img.classList.toggle('crop-yt', final === hq);
                    bgBlur.classList.add("visible");
                };
                temp.onerror = () => { img.src = hq; bgBlur.style.backgroundImage = 'url(' + hq + ')'; img.classList.add('crop-yt'); bgBlur.classList.add("visible"); };
                temp.src = maxres;
            }

            function connectWS() {
                const socket = new WebSocket("ws://" + window.location.host);
                socket.onopen = () => { if (isOffline) window.location.reload(); isOffline = false; widget.classList.remove("offline"); };
                socket.onmessage = (e) => {
                    const m = JSON.parse(e.data);
                    if (m.type === "program_shutdown") { isOffline = true; widget.classList.add("shutdown"); resetDisplay(); return; }
                    if (m.type === "track_update") {
                        track = m.data; 
                        isIdle = !track || !track.details;
                        const isPaused = !!track.paused;
                        const isStatus = !!track.details && !track.end_timestamp && !track.start_timestamp;
                        
                        widget.classList.toggle("idle", isIdle); 
                        widget.classList.toggle("playing", !isIdle && !isStatus);
                        widget.classList.toggle("paused", isPaused);
                        widget.classList.toggle("status-only", isStatus);
                        
                        if (track.thumbnail && (!isIdle || isPaused)) setThumbnail(track.thumbnail);
                        else if (isIdle) resetDisplay();
                        document.getElementById("title").innerText = track.details || (track.paused ? "Paused" : "Not Playing");
                        document.getElementById("artist").innerText = track.state || "T_Music_Bot";
                    }
                    if (m.type === "fft_data") { bins.set(m.bins); }
                };
                socket.onclose = () => { if(!isOffline) setOffline("DISCONNECTED", "Retrying connection..."); setTimeout(connectWS, 2000); };
            }

            function setOffline(title, sub) {
                isOffline = true; widget.classList.add("offline"); widget.classList.remove("playing");
                document.getElementById("title").innerText = title; document.getElementById("artist").innerText = sub;
                resetDisplay();
            }

            function resetDisplay() {
                bgBlur.style.backgroundImage = 'none'; bgBlur.classList.remove("visible");
                const tImg = document.getElementById('track-img');
                if (tImg) { tImg.src = "/assets/music.png"; tImg.classList.remove('crop-yt'); }
                bins.fill(0); ctx.clearRect(0, 0, canvas.width, canvas.height);
            }

            connectWS();
            if (track.thumbnail) setThumbnail(track.thumbnail);

            const BARS = \${viz.bars}, userW = \${viz.barWidth || 10}, userG = \${viz.barGap || 5};
            const cTop = "\${colorTop}", cBot = "\${colorBot}";
            const heights = new Float32Array(BARS).fill(0);
            canvas.width = 880; canvas.height = 100;
            const totalUnits = (userW * BARS) + (userG * (BARS - 1)), barW = (canvas.width / totalUnits) * userW, gap = (canvas.width / totalUnits) * userG;
            let lastFrameTime = performance.now(), frameCount = 0, lastFpsUpdate = lastFrameTime;
            const targetFps = \${viz.fps}, frameTarget = 1000 / targetFps;
            const barGrad = ctx.createLinearGradient(0, 0, 0, canvas.height);
            barGrad.addColorStop(0, cTop); barGrad.addColorStop(1, cBot);

            const fluidity = \${viz.visualFluidity || 0.22};
            const horizontalSmoothing = \${!!viz.horizontalSmoothing};

            function draw() {
                requestAnimationFrame(draw);
                if (isOffline) return;
                const now = performance.now(), elapsed = now - lastFrameTime;
                if (targetFps >= 144 || elapsed >= frameTarget - 0.5) {
                    lastFrameTime = now - (elapsed % frameTarget);
                    render();
                    frameCount++;
                    if (now - lastFpsUpdate >= 1000) {
                        const fpsEl = document.getElementById("fps-text");
                        if (fpsEl) fpsEl.innerText = frameCount + " FPS";
                        frameCount = 0; lastFpsUpdate = now;
                    }
                }
            }

            function render() {
                const liveIndicator = document.getElementById("live-indicator"), progressArea = document.getElementById("progress-area");
                const isPaused = widget.classList.contains("paused");
                const isStatus = widget.classList.contains("status-only");
                const hasEnd = !!track.end_timestamp;
                const hasStart = !!track.start_timestamp;
                
                if (!isOffline && !isIdle && !isStatus) {
                    const tNow = Date.now();
                    const start = (track.start_timestamp < 10000000000 ? track.start_timestamp * 1000 : track.start_timestamp) || tNow;
                    const end = (track.end_timestamp < 10000000000 ? track.end_timestamp * 1000 : track.end_timestamp) || 0;
                    const posRaw = track.position || 0;
                    const pos = (posRaw < 10000000000 ? posRaw * 1000 : posRaw);
                    
                    const total = end > start ? end - start : 0;
                    const current = isPaused ? pos : Math.max(0, Math.min(total, tNow - start));
                    
                    const isLive = !hasEnd && hasStart && !isPaused;
                    if (liveIndicator) liveIndicator.classList.toggle("visible", isLive);

                    const shouldShowProgress = hasEnd && total > 0;
                    if (progressArea) progressArea.classList.toggle("visible", shouldShowProgress);

                    if (shouldShowProgress) {
                        const progBar = document.getElementById("progress-bar"), timeText = document.getElementById("time-text");
                        if (progBar) progBar.style.width = Math.min(100, (current/total)*100) + "%";
                        const fmt = (ms) => { 
                            const s = Math.floor(Math.max(0, ms)/1000), h = Math.floor(s/3600), m = Math.floor((s % 3600) / 60), sc = s % 60; 
                            const hPart = h > 0 ? h + ":" : "";
                            const mPart = h > 0 ? m.toString().padStart(2, "0") : m;
                            return hPart + mPart + ":" + sc.toString().padStart(2, "0"); 
                        };
                        if (timeText) timeText.innerText = fmt(current) + " / " + fmt(total);
                    }
                } else {
                    if (liveIndicator) liveIndicator.classList.remove("visible");
                    if (progressArea) progressArea.classList.remove("visible");
                }
                
                ctx.clearRect(0, 0, canvas.width, canvas.height);
                if (isIdle || isPaused || isStatus || !hasStart || !\${viz.enabled}) return;

                const cHeight = canvas.height;
                for(let i=0; i<BARS; i++) {
                    const center = (bins[i] / 255) * cHeight;
                    const l = i > 0 ? (bins[i-1] / 255) * cHeight : 0;
                    const r = i < BARS - 1 ? (bins[i+1] / 255) * cHeight : 0;
                    let target = center;
                    if (l * 0.65 > target) target = l * 0.65;
                    if (r * 0.65 > target) target = r * 0.65;
                    if (target > heights[i]) heights[i] = target;
                    else {
                        const diff = heights[i] - target;
                        heights[i] -= (diff * 0.22 > 1) ? diff * 0.22 : 1;
                    }
                }
                ctx.fillStyle = barGrad;
                const showGlow = \${viz.glow} && BARS <= 128;
                const limitH = (h, pad) => Math.min(canvas.height - pad, Math.max(pad, h));
                switch("\${viz.mode}") {
                    case "wave":
                        ctx.strokeStyle = barGrad; ctx.lineWidth = 6; ctx.lineJoin = "round"; ctx.lineCap = "round";
                        if (showGlow) { ctx.shadowBlur = 15; ctx.shadowColor = cTop; }
                        ctx.beginPath();
                        const step = BARS > 128 ? Math.floor(BARS / 64) : 1;
                        const pts = [];
                        for(let i=0; i<BARS; i += step) pts.push({x: i * (barW + gap) + barW/2, y: canvas.height - limitH(heights[i], 10)});
                        if(pts.length > 0) {
                            ctx.moveTo(pts[0].x, pts[0].y);
                            for (let i = 0; i < pts.length - 1; i++) {
                                const xc = (pts[i].x + pts[i + 1].x) / 2, yc = (pts[i].y + pts[i + 1].y) / 2;
                                ctx.quadraticCurveTo(pts[i].x, pts[i].y, xc, yc);
                            }
                            ctx.stroke(); ctx.shadowBlur = 0; ctx.lineTo(canvas.width, canvas.height); ctx.lineTo(0, canvas.height); ctx.closePath();
                            ctx.globalAlpha = 0.2; ctx.fill(); ctx.globalAlpha = 1.0;
                        }
                        break;
                    case "center-bars":
                        const rC = \${viz.rounded} ? Math.min(barW / 2, 6) : 0;
                        if (showGlow) { ctx.shadowBlur = 15; ctx.shadowColor = cTop; }
                        const midY = canvas.height / 2; ctx.beginPath();
                        for(let i=0; i<BARS; i++) {
                            const h = limitH(heights[i], 4) / 2, x = i * (barW + gap);
                            if (rC > 0) ctx.roundRect(x, midY - h, barW, h * 2, [rC, rC, rC, rC]);
                            else ctx.rect(x, midY - h, barW, h * 2);
                        }
                        ctx.fill(); ctx.shadowBlur = 0;
                        break;
                    default:
                        const r = \${viz.rounded} ? Math.min(barW / 2, 6) : 0;
                        if (showGlow) { ctx.shadowBlur = 15; ctx.shadowColor = cTop; }
                        ctx.beginPath();
                        for(let i=0; i<BARS; i++) {
                            const h = limitH(heights[i], 4), x = i * (barW + gap), y = canvas.height - h;
                            if (r > 0) ctx.roundRect(x, y, barW, h, [r, r, 0, 0]); else ctx.rect(x, y, barW, h);
                        }
                        ctx.fill();
                        ctx.shadowBlur = 0;
                        ctx.fillStyle = cTop; ctx.beginPath();
                        for(let i=0; i<BARS; i++) ctx.rect(i * (barW + gap), canvas.height - limitH(heights[i], 4), barW, 2);
                        ctx.fill();
                }
            } draw();
        </script></body></html>\`;
};
