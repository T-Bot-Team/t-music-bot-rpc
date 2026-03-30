import http from "http";
import WebSocket from "ws";
import fs from "fs";
import path from "path";
import { PATHS, GLOBAL_STATE, TrackUpdate, VisualizerConfig } from "../utils/constants";
// import logger from "../lib/logger"; // Assuming logger is available

let httpServer: http.Server | null = null;
let wss: WebSocket.Server | null = null;

export const broadcast = (data: any): void => {
  if (!GLOBAL_STATE.overlayClients || GLOBAL_STATE.isShuttingDown) return;
  const msg = JSON.stringify(data);
  GLOBAL_STATE.overlayClients.forEach((c) => {
    if (c.readyState === WebSocket.OPEN) c.send(msg);
  });
};

export const updateTrack = (data: TrackUpdate): void => {
  if (data && data.details && data.state) {
    let artistName = data.state;
    if (artistName.toLowerCase().startsWith("by ")) artistName = artistName.substring(3).trim();
    const escapedArtist = artistName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const separators = ["\\s*-\\s*", "\\s*–\\s*", "\\s*—\\s*", ":\\s*", "\\|\\s*", "\\s*~\\s*"];
    const artistRegex = new RegExp(`(?:^|\\s+)${escapedArtist}(?:${separators.join("|")})`, "i");
    data.details = data.details.replace(artistRegex, "").trim();
    const suffixRegex = new RegExp(`(?:${separators.join("|")})\\s*${escapedArtist}\\s*$`, "i");
    data.details = data.details.replace(suffixRegex, "").trim();
    const junkPatterns = [
      /\s*[\[\(\]]?(?:Copyright Free|Official Video|Lyrics|Audio|Video|Music Video|Official Audio)[^\]\)]*[\]\)]?\s*/gi,
      /\s*[\(\[].*?(?:Video|Audio|Lyrics|Version|Remix).*?[\)\]]\s*/gi,
      /\[Copyright Freel?\]?/gi,
    ];
    junkPatterns.forEach((p) => { data.details = data.details!.replace(p, " "); });
    data.details = data.details.replace(/\s\s+/g, " ").trim();
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
      try { res.end(getHTML(s.overlay.visualizer, GLOBAL_STATE.lastTrack)); } catch (e: any) { res.end(`Error: ${e.message}`); }
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

export const getHTML = (viz: VisualizerConfig, lastTrack: TrackUpdate | null): string => {
  const avatarUrl = "/assets/avatar.png";
  const track = lastTrack || {};
  const isPaused = !!track.paused;
  const isIdle = !track.details || (!track.startTimestamp && !isPaused);
  const thumb = (track.thumbnail && (!isIdle || isPaused)) ? track.thumbnail : "/assets/music.png";
  const title = track.details || "Not Playing";
  const artist = track.state || "T_Music_Bot";

  const randomHex = () => "#" + Math.floor(Math.random()*16777215).toString(16).padStart(6, '0');
  const colorTop = randomHex();
  const colorBot = randomHex();

  return `<!DOCTYPE html><html><head><style>
          html, body { background: transparent !important; margin: 0; padding: 0; overflow: hidden; width: 1360px; height: 440px; }
          body { font-family: 'Segoe UI', sans-serif; }
          #scale-wrapper { width: 1360px; height: 440px; position: relative; background: transparent !important; }
          #bg-blur { position: absolute; top: 0; left: 0; width: 100%; height: 100%; z-index: -1; filter: blur(60px); opacity: 0; background-size: cover; background-position: center; border-radius: 48px; pointer-events: none; transition: opacity 0.8s; background-color: transparent !important; }
          #bg-blur.visible { opacity: 0.5; }
          #widget { display: flex; align-items: center; background: transparent !important; color: white; padding: 60px; border-radius: 48px; width: 100%; height: 100%; box-sizing: border-box; position: relative; gap: 40px; overflow: hidden; border: none !important; }
          #widget.playing::before { content: ''; position: absolute; inset: 0; background: transparent !important; z-index: -1; border-radius: 48px; }
          #widget.shutdown { opacity: 0 !important; visibility: hidden !important; }
          #img-container { position: relative; width: 320px; height: 320px; flex-shrink: 0; background: transparent !important; }
          #avatar-img { position: absolute; top: -10px; left: -10px; width: 110px; height: 110px; z-index: 110; object-fit: cover; background: transparent !important; border: 4px solid rgba(255,255,255,0.2); border-radius: 50%; }
          #track-img-wrapper { width: 100%; height: 100%; border-radius: 32px; overflow: hidden; background: transparent !important; position: relative; z-index: 100; }
          #track-img { width: 100%; height: 100%; object-fit: cover; background: transparent !important; }
          #info { display: flex; flex-direction: column; justify-content: center; height: 320px; flex-grow: 1; min-width: 0; overflow: hidden; }
          #widget.playing #info { justify-content: space-between; }
          #widget.idle #info, #widget.offline #info { justify-content: center !important; }
          #text-wrapper { width: 100%; display: flex; flex-direction: column; overflow: hidden; text-align: left; }
          #widget.idle #text-wrapper { text-align: center; }
          #title { font-weight: bold; font-size: 3.8em; margin: 0; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; line-height: 1.2; text-shadow: 0 4px 12px rgba(0,0,0,0.9); }
          #artist { font-size: 2.8em; color: #ccc; margin: 0; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; }
          #progress-area { display: none; align-items: center; width: 100%; gap: 15px; margin-top: 10px; }
          #progress-container { flex-grow: 1; height: 14px; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.08); border-radius: 7px; overflow: hidden; position: relative; }
          #progress-bar { width: 0%; height: 100%; background: linear-gradient(90deg, ${colorBot}, ${colorTop}); box-shadow: 0 0 10px ${colorTop}80; border-radius: 6px; transition: width 0.1s linear; }
          #time-text { font-size: 2.8em; color: #fff; font-weight: 500; font-variant-numeric: tabular-nums; white-space: nowrap; text-shadow: 0 0 5px rgba(0,0,0,0.5); }
          #live-indicator { display: none; align-items: center; justify-content: flex-end; gap: 12px; color: #ff4444; font-weight: bold; font-size: 2.2em; text-transform: uppercase; letter-spacing: 2px; text-shadow: 0 0 15px rgba(255,68,68,0.4); margin: 10px 0; width: 100%; padding-right: 20px; box-sizing: border-box; }
          #live-indicator.visible { display: flex; }
          #live-dot { width: 18px; height: 18px; background-color: #ff4444; border-radius: 50%; box-shadow: 0 0 10px #ff4444; animation: pulse 1.5s infinite; }
          @keyframes pulse { 0% { transform: scale(1); opacity: 1; } 50% { transform: scale(1.3); opacity: 0.5; } 100% { transform: scale(1); opacity: 1; } }
          #viz-canvas { width: 880px; height: 100px; margin: 0; ${viz.enabled ? "" : "display: none;"} background: transparent !important; }
          #fps-text { position: absolute; bottom: 10px; right: 20px; font-size: 1.2em; color: rgba(255,255,255,0.4); font-family: monospace; z-index: 200; }
          
          /* ZOMBIE-PROOF HARD LOCK */
          #widget.idle #progress-area, #widget.idle #viz-canvas, #widget.idle #fps-text, #widget.idle #live-indicator,
          #widget.offline #progress-area, #widget.offline #viz-canvas, #widget.offline #fps-text, #widget.offline #live-indicator,
          #widget.offline #img-container { display: none !important; opacity: 0 !important; }
          #widget.paused #progress-area, #widget.paused #viz-canvas { display: none !important; }
          #widget.offline #text-wrapper { text-align: center !important; width: 100%; }
          #widget.offline #artist { color: #ff4444 !important; font-weight: bold; }
      </style></head><body><div id="scale-wrapper"><div id="bg-blur" class="${isIdle ? "" : "visible"}" style="background-image: url('${thumb}');"></div><div id="widget" class="${isIdle ? "idle visible" : (track.paused ? "paused playing visible" : "playing visible")}"><div id="img-container"><img id="avatar-img" src="${avatarUrl}" loading="eager" /><div id="track-img-wrapper"><img id="track-img" src="${thumb}" loading="eager" /></div></div><div id="info"><div id="text-wrapper"><div id="title">${title}</div><div id="artist">${artist}</div></div><div id="live-indicator"><div id="live-dot"></div><span>LIVE</span></div><div id="progress-area"><div id="progress-container"><div id="progress-bar"></div></div><div id="time-text">0:00 / 0:00</div></div><canvas id="viz-canvas"></canvas></div><div id="fps-text">0 FPS</div></div></div>
        <script>
            let track = ${JSON.stringify(track)}, isIdle = ${isIdle}, bins = new Uint8Array(${viz.bars}).fill(0), isOffline = false;
            const widget = document.getElementById("widget");
            const bgBlur = document.getElementById("bg-blur");
            const canvas = document.getElementById("viz-canvas"), ctx = canvas.getContext("2d", { alpha: true });

            function connectWS() {
                const socket = new WebSocket("ws://" + window.location.host);
                socket.onopen = () => { 
                    if (isOffline) { window.location.reload(); return; }
                    isOffline = false; 
                    widget.classList.remove("offline");
                };
                socket.onmessage = (e) => {
                    const m = JSON.parse(e.data);
                    if (m.type === "program_shutdown") { isOffline = true; widget.classList.add("shutdown"); resetDisplay(); return; }
                    if (m.type === "track_update") {
                        track = m.data; isIdle = !track || !track.details || (!track.startTimestamp && !track.paused);
                        const trackImg = document.getElementById('track-img');
                        widget.classList.toggle("idle", isIdle); 
                        widget.classList.toggle("playing", !isIdle);
                        widget.classList.toggle("paused", !!track?.paused);
                        if (track?.thumbnail && (!isIdle || track.paused)) { 
                            bgBlur.style.backgroundImage = 'url(' + track.thumbnail + ')'; 
                            bgBlur.classList.add("visible");
                            trackImg.src = track.thumbnail; 
                        } else if (isIdle) {
                            resetDisplay();
                        }
                        document.getElementById("title").innerText = track?.details || "Not Playing";
                        document.getElementById("artist").innerText = track?.state || "T_Music_Bot";
                    }
                    if (m.type === "fft_data") { bins.set(m.bins); }
                };
                socket.onclose = () => { if(!isOffline) setOffline("DISCONNECTED", "Retrying connection..."); setTimeout(connectWS, 2000); };
            }

            function setOffline(title, sub) {
                isOffline = true;
                widget.classList.add("offline");
                widget.classList.remove("playing");
                document.getElementById("title").innerText = title;
                document.getElementById("artist").innerText = sub;
                resetDisplay();
            }

            function resetDisplay() {
                bgBlur.style.backgroundImage = 'none';
                bgBlur.classList.remove("visible");
                const tImg = document.getElementById('track-img');
                if (tImg) tImg.src = "/assets/music.png";
                bins.fill(0);
                ctx.clearRect(0, 0, canvas.width, canvas.height);
            }

            connectWS();
          const BARS = ${viz.bars}, userW = ${viz.barWidth || 10}, userG = ${viz.barGap || 5};
          const cTop = "${colorTop}", cBot = "${colorBot}";
          const heights = new Float32Array(BARS).fill(0);
          canvas.width = 880; canvas.height = 100;
          const totalUnits = (userW * BARS) + (userG * (BARS - 1)), barW = (canvas.width / totalUnits) * userW, gap = (canvas.width / totalUnits) * userG;
          let lastFrameTime = performance.now(), frameCount = 0, lastFpsUpdate = lastFrameTime;
          const targetFps = ${viz.fps};
          const frameTarget = 1000 / targetFps;

          const barGrad = ctx.createLinearGradient(0, 0, 0, canvas.height);
          barGrad.addColorStop(0, cTop); barGrad.addColorStop(1, cBot);

          function draw() {
              requestAnimationFrame(draw);
              if (isOffline) return;
              const now = performance.now();
              const elapsed = now - lastFrameTime;
              
              // For high FPS (144+), bypass strict capping to avoid rAF jitter drops
              if (targetFps >= 144 || elapsed >= frameTarget - 0.5) {
                  lastFrameTime = now - (elapsed % frameTarget);
                  render();
                  frameCount++;
                  if (now - lastFpsUpdate >= 1000) {
                      document.getElementById("fps-text").innerText = frameCount + " FPS";
                      frameCount = 0;
                      lastFpsUpdate = now;
                  }
              }
          }

          function render() {
              const liveIndicator = document.getElementById("live-indicator");
              const progressArea = document.getElementById("progress-area");
              
              if (!isOffline && !isIdle) {
                  const tNow = Date.now(), start = track.startTimestamp < 10000000000 ? track.startTimestamp * 1000 : track.startTimestamp;
                  const end = track.endTimestamp < 10000000000 ? track.endTimestamp * 1000 : track.endTimestamp;
                  const elapsed = tNow - start, total = end - start;
                  
                  const isLive = !track.endTimestamp || total <= 0;
                  if (liveIndicator.classList.contains("visible") !== isLive) liveIndicator.classList.toggle("visible", isLive);

                  const shouldShowProgress = !isLive && !track.paused;
                  if (progressArea.style.display !== (shouldShowProgress ? "flex" : "none")) {
                      progressArea.style.display = (shouldShowProgress ? "flex" : "none");
                  }

                  if (shouldShowProgress) {
                      document.getElementById("progress-bar").style.width = Math.min(100, (elapsed/total)*100) + "%";
                      const fmt = (ms) => { 
                          const s = Math.floor(Math.max(0, ms)/1000), h = Math.floor(s/3600), m = Math.floor((s % 3600) / 60), sc = s % 60; 
                          const hPart = h > 0 ? h + ":" : "";
                          const mPart = h > 0 ? m.toString().padStart(2, "0") : m;
                          return hPart + mPart + ":" + sc.toString().padStart(2, "0"); 
                      };
                      document.getElementById("time-text").innerText = fmt(elapsed) + " / " + fmt(total);
                  }
              } else {
                  if (liveIndicator.classList.contains("visible")) liveIndicator.classList.remove("visible");
                  if (progressArea.style.display !== "none") progressArea.style.display = "none";
              }
              
              ctx.clearRect(0, 0, canvas.width, canvas.height);
              if (isIdle || track?.paused || !${viz.enabled}) return;

              const cHeight = canvas.height;
              for(let i=0; i<BARS; i++) {
                  const center = (bins[i] / 255) * cHeight;
                  const l = i > 0 ? (bins[i-1] / 255) * cHeight : 0;
                  const r = i < BARS - 1 ? (bins[i+1] / 255) * cHeight : 0;
                  
                  let target = center;
                  if (l * 0.65 > target) target = l * 0.65;
                  if (r * 0.65 > target) target = r * 0.65;

                  if (target > heights[i]) {
                      heights[i] = target;
                  } else {
                      const diff = heights[i] - target;
                      heights[i] -= (diff * 0.15 > 1) ? diff * 0.15 : 1;
                  }
              }
              
              ctx.fillStyle = barGrad;
              const showGlow = ${viz.glow} && BARS <= 128;
              const limitH = (h, pad) => Math.min(canvas.height - pad, Math.max(pad, h));

              switch("${viz.mode}") {
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
                  case "particles":
                      ctx.shadowBlur = showGlow ? 15 : 0; ctx.shadowColor = cTop;
                      const pRad = Math.max(2, Math.min(barW / 2, 8));
                      for(let i=0; i<BARS; i++) {
                          const h = limitH(heights[i], pRad + 2), x = i * (barW + gap) + barW/2;
                          ctx.beginPath(); ctx.arc(x, canvas.height - h, pRad, 0, Math.PI * 2); ctx.fill();
                      }
                      break;
                  case "neon-bars":
                      ctx.shadowBlur = showGlow ? 15 : 0; ctx.shadowColor = cTop;
                      const thinW = Math.max(2, barW * 0.3);
                      for(let i=0; i<BARS; i++) {
                          const h = limitH(heights[i], 4), x = i * (barW + gap) + (barW - thinW)/2;
                          ctx.fillRect(x, canvas.height - h, thinW, h);
                      }
                      break;
                  case "led":
                      ctx.shadowBlur = showGlow ? 10 : 0; ctx.shadowColor = cTop;
                      const blockH = 8; const blockGap = 4;
                      for(let i=0; i<BARS; i++) {
                          const h = limitH(heights[i], 4), x = i * (barW + gap);
                          const blocks = Math.floor(h / (blockH + blockGap));
                          for(let b=0; b<blocks; b++) {
                              const y = canvas.height - (b * (blockH + blockGap)) - blockH;
                              ctx.fillRect(x, y, barW, blockH);
                          }
                      }
                      break;
                  case "outline":
                      ctx.shadowBlur = showGlow ? 10 : 0; ctx.shadowColor = cTop;
                      ctx.lineWidth = 2;
                      ctx.strokeStyle = barGrad;
                      const rO = ${viz.rounded} ? Math.min(barW / 2, 6) : 0;
                      ctx.beginPath();
                      for(let i=0; i<BARS; i++) {
                          const h = limitH(heights[i], 4), x = i * (barW + gap), y = canvas.height - h;
                          if (rO > 0) ctx.roundRect(x, y, barW, h, [rO, rO, 0, 0]); else ctx.rect(x, y, barW, h);
                      }
                      ctx.stroke();
                      break;
                  case "center-bars":
                      const rC = ${viz.rounded} ? Math.min(barW / 2, 6) : 0;
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
                      const r = ${viz.rounded} ? Math.min(barW / 2, 6) : 0;
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
        </script></body></html>`;
};
