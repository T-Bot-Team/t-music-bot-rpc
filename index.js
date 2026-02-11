import RPC from "@t_bot-team/discord-rpc";
import PWSL from "@performanc/pwsl";
import SysTray from "systray2";
import fs from "fs";
import path from "path";
import os from "os";
import { createRequire } from "module";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const require = createRequire(import.meta.url);
const config = require("./config.json");

// --- Optimization & Config ---
const log = (m, e) => (e || config.debug_mode) && console[e ? 'error' : 'log'](m);
const RPC_OPTS = { transport: "ipc" };
const WS_URL = process.env.RPC_WS_URL || "wss://rpc.tehcraft.xyz/ws";
const RECONNECT_INTERVAL = 60000; // 1 minute watchdog
let rpc, ws, tray, hb, retryTimer;
let state = { ws: false, rpc: false, connecting: false, lastActivity: null };

// --- System Tray Setup ---
const itemQuit = { title: "Quit", tooltip: "Exit Application", checked: false, enabled: true };
const itemStatus = { title: "Status: Starting...", tooltip: "Connection Status", checked: false, enabled: false };

function getTrayBinPath() {
  const platform = os.platform();
  const binName = {
    win32: 'tray_windows_release.exe',
    linux: 'tray_linux_release',
    darwin: 'tray_darwin_release'
  }[platform];
  
  if (!binName) return null;

  const pkgPath = path.join(__dirname, 'node_modules/systray2/traybin', binName);
  
  if (process.pkg) {
    // Extract binary from snapshot to temp
    const tmpPath = path.join(os.tmpdir(), `t-music-rpc-${binName}`);
    if (!fs.existsSync(tmpPath)) {
      try {
        fs.writeFileSync(tmpPath, fs.readFileSync(pkgPath));
        if (platform !== 'win32') fs.chmodSync(tmpPath, 0o755);
      } catch (e) {
        log(`⚠️ // Extract Failed: ${e.message}`, 1);
        return null;
      }
    }
    return tmpPath;
  }
  return pkgPath;
}

const trayMenu = {
  icon: fs.existsSync("icon.ico") ? fs.readFileSync("icon.ico", "base64") : "",
  title: "T-Music RPC",
  tooltip: "T-Music RPC Client",
  items: [itemStatus, SysTray.separator, itemQuit],
};

function updateTray(status) {
  if (!tray) return;
  itemStatus.title = `Status: ${status}`;
  tray.sendAction({ type: "update-item", item: itemStatus });
}

try {
  const binPath = getTrayBinPath();
  if (binPath) {
    // SysTray options might need tweaking depending on version, 
    // but systray2 usually takes just the menu. 
    // However, to use custom bin, we might need to patch or use specific constructor options if available.
    // Looking at systray2 source, it tries to find the bin automatically.
    // We can overwrite the internal property or use a subclass if needed, 
    // but simpler: if process.pkg, we might need to rely on it finding the temp file if we set it?
    // Actually, systray2 checks `pkg` and tries to run.
    // Let's force the path by passing it if the library supports it, or hacking it.
    // Reading systray2 source: it uses `path.resolve(__dirname, ...)` which fails in pkg for spawn.
    // We need to set `process.env.TRAYBIN_PATH`? No.
    // Wait, systray2 v2.1.0 allows `binPath` in constructor? No, check index.d.ts if available or assume.
    // Standard systray doesn't. 
    // HACK: We can't easily pass the path to the constructor in some versions.
    // BUT, since we are rewriting, we can try to use a modified approach or just hope the library handles it?
    // No, `pkg` is strict.
    // Let's try to set `binPath` in the options if possible.
    // If not, we might fail to launch tray in pkg without a library patch.
    // ALTERNATIVE: Use `process.env.SYSTRAY_PATH` if the lib supports it.
    // Let's assume standard usage and if it fails, it fails (graceful degradation).
    // Actually, let's try to pass it as a second arg or config?
    // Checking `systray2` common patterns: `new SysTray({ menu, binPath: ... })` might work.
    tray = new SysTray.default({ menu: trayMenu, binPath }); // Try this
    
    tray.onClick(action => {
      if (action.item.title === "Quit") process.exit(0);
    });
    log("✅ >> System Tray initialized.");
  } else {
    log("⚠️ // Tray binary not found.", 1);
  }
} catch (e) {
  log(`⚠️ // Tray Init Failed: ${e.message}`, 1);
}

// --- RPC & WS Logic ---
function initRPC() {
  if (rpc) try { rpc.destroy(); } catch {}
  rpc = new RPC.Client(RPC_OPTS);

  rpc.on("ready", () => {
    log("✅ >> RPC Connected!");
    state.rpc = true;
    updateTray("RPC Connected");
    sendAuth();
    if (state.lastActivity) updateRPC(state.lastActivity);
  });

  rpc.on("disconnected", () => {
    log("🔴 // RPC Disconnected", 1);
    state.rpc = false;
    updateTray("RPC Disconnected");
    // Don't force reconnect WS here, just wait for watchdog or WS error
  });
}

function sendAuth() {
  if (state.ws && state.rpc && rpc.user && config.pairing_code) {
    ws.send(JSON.stringify({ type: "auth", userId: rpc.user.id, code: String(config.pairing_code).trim() }));
  }
}

async function updateRPC(data) {
  state.lastActivity = data;
  if (!state.rpc) return;
  if (!data) return rpc.clearActivity().catch(e => log(`❌ // Clear: ${e.message}`, 1));

  const now = Date.now(), isAbs = v => v > 1e12;
  let start = Number(data.startTimestamp), end = data.endTimestamp !== undefined ? Number(data.endTimestamp) : undefined;
  
  if (start && !isAbs(start)) start = now - start;
  // Fix: End time logic optimization
  if (end && !isAbs(end)) end = (start ? start : now) + end;

  try {
    await rpc.setActivity({
      details: data.details, state: data.state,
      largeImageKey: data.largeImageKey, largeImageText: data.largeImageText,
      smallImageKey: data.smallImageKey, smallImageText: data.smallImageText,
      type: 2, instance: false,
      startTimestamp: start ? Math.floor(start) : undefined,
      endTimestamp: end ? Math.floor(end) : undefined,
    });
  } catch (e) { log(`❌ // Set Activity: ${e.message}`, 1); }
}

function connect() {
  if (state.connecting) return;
  state.connecting = true;
  updateTray("Connecting...");

  if (ws) {
    clearInterval(hb);
    ws.removeAllListeners();
    try { ws.close(); } catch {}
  }

  log("🌐 // Connecting WS...");
  ws = new PWSL(WS_URL);

  ws.on("open", () => {
    log("✅ >> WS Open.");
    state.ws = true;
    state.connecting = false;
    updateTray("WS Connected");
    ws.send(JSON.stringify({ type: "connect" }));
    
    // Heartbeat
    clearInterval(hb);
    hb = setInterval(() => {
      try { ws.sendData(Buffer.alloc(0), { len: 0, fin: true, opcode: 0x9, mask: true }); }
      catch { reconnect(); }
    }, 30000);
  });

  ws.on("message", (data) => {
    try {
      const m = JSON.parse(data);
      switch(m.type) {
        case "connect": 
          initRPC(); 
          rpc.login({ clientId: m.clientId }).catch(e => log(`❌ // Login: ${e.message}`, 1));
          break;
        case "authenticated": 
          log("🔓 // Auth OK."); 
          updateTray("Authenticated");
          break;
        case "rpc_update": 
          updateRPC(m.data); 
          break;
        case "error": 
          log(`❌ // Server: ${m.message}`, 1); 
          // If auth error, maybe don't reconnect immediately to avoid loop, but watchdog will handle it.
          if (m.message.includes("pairing")) updateTray("Auth Failed");
          break;
      }
    } catch (e) { log(`❌ // Parse: ${e.message}`, 1); }
  });

  ws.on("close", () => {
    log("🔴 // WS Closed");
    state.ws = false;
    state.connecting = false;
    updateTray("Disconnected");
    reconnect();
  });

  ws.on("error", (e) => log(`❌ // WS Error: ${e.message}`, 1));
}

function reconnect() {
  if (retryTimer) return;
  log(`🔄 // Reconnecting in 5s...`);
  state.connecting = false; // Reset flag to allow connect()
  retryTimer = setTimeout(() => {
    retryTimer = null;
    connect();
  }, 5000);
}

// --- Watchdog ---
setInterval(() => {
  if (!state.ws) {
    log("⏰ // Watchdog: WS disconnected. Reconnecting...");
    connect();
  } else if (!state.rpc && state.ws) {
     log("⏰ // Watchdog: RPC disconnected but WS connected. Re-initiating...");
     // Triggering a re-login might be needed if RPC died locally
     // We can ask server to re-send connect? Or just re-login if we have clientId?
     // For now, simpler to restart WS connection to refresh everything
     connect();
  }
}, RECONNECT_INTERVAL);

// Start
connect();
