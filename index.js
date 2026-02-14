const RPC = require("@t_bot-team/discord-rpc");
const WebSocket = require("ws");
const fs = require("fs");
const path = require("path");
const os = require("os");
const { spawn, exec } = require("child_process");
const readline = require("readline");

// --- Globals ---
var wsClient = null;
var rpcClient = null;
var tray = null;
var state = { ws: "Disconnected", rpc: "Disconnected" };
var lastActivityData = null;
var activityTimeout = null;
var lastSuccessTime = 0;
var lastMessageTime = Date.now();
var lastConnectLogTime = 0;
var lastReuseLogTime = 0;
var queuedData = null;
var isPrompting = false;
var heartbeatInterval = null;
var isQuiet = process.argv.includes("--quiet") || process.argv.includes("-q");

// Version
let APP_VERSION = "v1.0.0";
try { APP_VERSION = `v${require("./package.json").version}`; } catch (e) {}

// Config
const isPkg = !!process.pkg;
const root = isPkg ? path.dirname(process.execPath) : __dirname;
const settingsFile = path.join(root, "settings.json");
const logFile = path.join(root, "logs.txt");

// --- Single Instance Lock ---
const lockFile = path.join(os.tmpdir(), "t-music-bot-rpc.lock");
function checkLock() {
  if (fs.existsSync(lockFile)) {
    try {
      const pid = parseInt(fs.readFileSync(lockFile, "utf8"));
      if (!isNaN(pid)) {
        process.kill(pid, 0); // Throws error if process doesn't exist
        log(`Another instance is already running (PID: ${pid}). Exiting.`, true);
        process.exit(1);
      }
    } catch (e) {
      try { fs.unlinkSync(lockFile); } catch (e2) {}
    }
  }
  fs.writeFileSync(lockFile, process.pid.toString());
}
if (isPkg) checkLock();

function log(m, force = false) {
  if (isQuiet && !force) return;
  const msg = `[${new Date().toLocaleTimeString()}] ${m}`;
  console.log(msg);
  try {
    // Only write to file if not quiet, or if it's a critical forced message
    if (!isQuiet || force) {
      fs.appendFileSync(logFile, msg + os.EOL);
    }
  } catch (e) {}
}

async function updateActivity(data) {
  if (!rpcClient || state.rpc !== "Connected" || isShuttingDown) return;

  const isClear = !data || Object.keys(data).length === 0;
  
  // Helper to compare activities while ignoring minor timestamp jitters (< 2s)
  const isDuplicate = (a, b) => {
    if (!a || !b) return a === b;
    const fields = ["details", "state", "largeImageKey", "largeImageText", "smallImageKey", "smallImageText"];
    for (const f of fields) {
      if (a[f] !== b[f]) return false;
    }
    if (Math.abs((a.startTimestamp || 0) - (b.startTimestamp || 0)) > 2000) return false;
    if (Math.abs((a.endTimestamp || 0) - (b.endTimestamp || 0)) > 2000) return false;
    return true;
  };

  const activity = isClear ? null : { ...data, type: 2, instance: false };
  
  // Check against last sent
  const lastActivity = lastActivityData ? (lastActivityData === "CLEARED" ? null : JSON.parse(lastActivityData)) : undefined;
  if (isDuplicate(activity, lastActivity)) {
    // If we are back to the current state, cancel any queued changes
    if (queuedData) {
      queuedData = null;
      if (activityTimeout) {
        clearTimeout(activityTimeout);
        activityTimeout = null;
      }
    }
    return;
  }

  // Check against queued
  if (queuedData && isDuplicate(activity, { ...queuedData, type: 2, instance: false })) return;

  const now = Date.now();
  const elapsed = now - lastSuccessTime;
  const COOLDOWN_MS = 15500;

  if (elapsed < COOLDOWN_MS) {
    queuedData = data;
    if (activityTimeout) return;

    const waitTime = COOLDOWN_MS - elapsed;
    log(`Rate limiting (${isClear ? "Clear" : (data.details || "Song")}) - Waiting ${Math.ceil(waitTime / 1000)}s...`);
    activityTimeout = setTimeout(() => {
      activityTimeout = null;
      const next = queuedData;
      queuedData = null;
      updateActivity(next);
    }, waitTime);
    return;
  }

  if (activityTimeout) {
    clearTimeout(activityTimeout);
    activityTimeout = null;
  }
  queuedData = null;

  // Update state before await
  lastActivityData = isClear ? "CLEARED" : JSON.stringify(activity);
  lastSuccessTime = Date.now();

  try {
    if (isClear) {
      if (rpcClient.clearActivity) await rpcClient.clearActivity();
      else await rpcClient.setActivity({});
      log("RPC Cleared.");
    } else {
      await rpcClient.setActivity(activity);
      log(`RPC Updated: ${data.details || "Untitled"}`);
      // log(`[DEBUG] Payload: ${JSON.stringify(activity)}`); 
    }
  } catch (err) {
    const errM = err.message.toLowerCase();
    log(`RPC Error: ${err.message}`);
    lastActivityData = null; 
    
    if (errM.includes("rate limit") || errM.includes("cooldown")) {
      lastSuccessTime = Date.now();
      updateActivity(data); 
    } else if (errM.includes("connection lost") || errM.includes("not connected")) {
      state.rpc = "Disconnected";
      tray.updateStatus(state.ws, state.rpc);
    }
  }
}

// --- Cleanup & Exit ---
let isShuttingDown = false;
async function cleanup() {
  if (isShuttingDown) return;
  isShuttingDown = true;
  log("Shutting down...");
  
  // Force exit fallback if cleanup hangs
  const forceExit = setTimeout(() => {
    log("Cleanup timeout - forcing exit.", true);
    process.exit(1);
  }, 10000); // Increased to 10s to allow for rate limit wait if needed

  if (activityTimeout) clearTimeout(activityTimeout);
  if (heartbeatInterval) { clearInterval(heartbeatInterval); heartbeatInterval = null; }
  
  if (rpcClient) {
    try {
      // Check if we need to wait for rate limit to ensure clearActivity works
      const now = Date.now();
      const elapsed = now - lastSuccessTime;
      const COOLDOWN_MS = 15500;
      
      if (elapsed < COOLDOWN_MS) {
        const waitTime = COOLDOWN_MS - elapsed;
        log(`Rate limit active. Waiting ${Math.ceil(waitTime/1000)}s to clear RPC...`);
        await new Promise(r => setTimeout(r, waitTime));
      }

      log("Clearing RPC activity...");
      // clearActivity() is the correct way to remove the presence entirely
      await rpcClient.clearActivity().catch(() => {});
      
      // Wait for Discord to process the clear packet
      await new Promise(r => setTimeout(r, 1500));
      
      log("Destroying RPC Client...");
      await rpcClient.destroy().catch(() => {});
      
      // Fallback: Manually close transport if still alive
      if (rpcClient.transport && rpcClient.transport.close) {
        try { rpcClient.transport.close(); } catch(e){}
      }
    } catch (e) {
      log(`Cleanup RPC Error: ${e.message}`);
    }
  }
  
  if (wsClient) {
    try { wsClient.terminate(); } catch (e) {}
  }
  
  if (tray) {
    try { tray.kill(); } catch (e) {}
  }

  try { if (fs.existsSync(lockFile)) fs.unlinkSync(lockFile); } catch(e){}
  
  log("Exit complete.", true);
  clearTimeout(forceExit);
  process.exit(0);
}

process.on("SIGINT", cleanup);
process.on("SIGTERM", cleanup);
process.on("SIGHUP", cleanup);
process.on("uncaughtException", (e) => { log(`Uncaught Error: ${e.message}`); cleanup(); });
process.on("unhandledRejection", (e) => { log(`Unhandled Rejection: ${e.message}`); cleanup(); });

// --- Tray Controller ---
class TrayController {
  constructor(menu) {
    this.menu = menu;
    this.process = null;
    this.ready = false;
    this._readyPromise = null;
    this._resolveReady = null;
  }

  async init() {
    this._readyPromise = new Promise((resolve) => {
      this._resolveReady = resolve;
    });
    const binName = process.platform === "win32" ? "tray_windows_release.exe" : "tray_linux_release";
    const dstName = process.platform === "win32" ? "T_Music_Bot-RPC.exe" : "T_Music_Bot RPC";
    const tempDir = path.join(os.tmpdir(), "t-music-bot-rpc");

    try {
      if (!fs.existsSync(tempDir)) fs.mkdirSync(tempDir, { recursive: true });

      const binPath = path.resolve(path.join(tempDir, dstName));
      const iconPath = path.join(__dirname, process.platform === "win32" ? "icon.ico" : "icon.png");

      const binSrc = path.join(__dirname, "node_modules", "systray2", "traybin", binName);

      if (fs.existsSync(binSrc) && !fs.existsSync(binPath)) {
        fs.writeFileSync(binPath, fs.readFileSync(binSrc));
        if (process.platform !== "win32") fs.chmodSync(binPath, 0o755);
      }

      if (!fs.existsSync(binPath)) {
        this._resolveReady(false);
        return;
      }

      if (fs.existsSync(iconPath)) {
        this.menu.icon = fs.readFileSync(iconPath).toString("base64");
      }

      this.process = spawn(binPath, [], { windowsHide: true });

      this.process.on("error", () => this._resolveReady(false));

      const rl = readline.createInterface({ input: this.process.stdout });
      rl.on("line", (line) => {
        try {
          const action = JSON.parse(line);
          if (action.type === "ready") {
            this.ready = true;
            this.sendAction({ type: "initial", ...this.menu });
            this._resolveReady(true);
          } else if (action.type === "clicked") {
            this.handleBoxClick(action.item);
          }
        } catch (e) {}
      });

      this.process.stderr.on("data", (d) => {
        if (d.toString().includes("libgtk")) log("Missing Linux tray dependencies: libgtk-3-0", true);
      });
      
      this.process.on("exit", () => {
        this.ready = false;
        this._resolveReady(false);
        rl.close();
      });
    } catch (e) {
      this._resolveReady(false);
    }
    return this._readyPromise;
  }

  sendAction(action) {
    if (this.process && this.process.stdin.writable) {
      this.process.stdin.write(JSON.stringify(action) + "\n");
    }
  }

  updateStatus(ws, rpc) {
    if (!this.ready) return;
    this.sendAction({
      type: "update-item",
      item: { title: `WS: ${ws} `, enabled: false, __id: 1 },
      seq_id: 0,
    });
    this.sendAction({
      type: "update-item",
      item: { title: `RPC: ${rpc} `, enabled: false, __id: 2 },
      seq_id: 1,
    });
  }

  handleBoxClick(item) {
    if (item.title === "Quit") cleanup();
    if (item.title === "Open Logs") {
      const cmd =
        process.platform === "win32"
          ? `start "" "${logFile}"`
          : `xdg-open "${logFile}"`;
      exec(cmd);
    }
  }

  kill() {
    if (this.process) this.process.kill();
  }
}

async function getPairingCode() {
  const title = "T_Music_Bot RPC Setup";
  const msg = "1. Go to Discord and run /rpc connect\n2. Copy the pairing code from the bot";

  if (process.platform === "win32") {
    const psMsgParts = msg.split("\n").map(line => `'${line}'`).join(" + [char]13 + [char]10 + ");
    // One-liner PowerShell command with added labels and adjusted positioning
    const psCmd = `powershell -NoProfile -Command "Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; [Windows.Forms.Application]::EnableVisualStyles(); $f=New-Object Windows.Forms.Form; $f.Text='${title}'; $f.Size=New-Object Drawing.Size(420,300); $f.StartPosition='CenterScreen'; $f.FormBorderStyle='FixedDialog'; $f.Topmost=$true; $f.MaximizeBox=$false; $f.MinimizeBox=$false; $f.Font=New-Object Drawing.Font('Segoe UI', 10); $l1=New-Object Windows.Forms.Label; $l1.Text='Instructions:'; $l1.Font=New-Object Drawing.Font('Segoe UI', 10, [Drawing.FontStyle]::Bold); $l1.Size=New-Object Drawing.Size(380,20); $l1.Location=New-Object Drawing.Point(20,20); $l2=New-Object Windows.Forms.Label; $l2.Text=(${psMsgParts}); $l2.Size=New-Object Drawing.Size(380,50); $l2.Location=New-Object Drawing.Point(20,45); $l3=New-Object Windows.Forms.Label; $l3.Text='Enter Code:'; $l3.Font=New-Object Drawing.Font('Segoe UI', 10, [Drawing.FontStyle]::Bold); $l3.Size=New-Object Drawing.Size(380,20); $l3.Location=New-Object Drawing.Point(20,105); $t=New-Object Windows.Forms.TextBox; $t.Location=New-Object Drawing.Point(22,130); $t.Size=New-Object Drawing.Size(360,25); $btnOk=New-Object Windows.Forms.Button; $btnOk.Text='Connect'; $btnOk.Size=New-Object Drawing.Size(95,32); $btnOk.Location=New-Object Drawing.Point(195,190); $btnOk.DialogResult=1; $btnOk.FlatStyle='System'; $btnCan=New-Object Windows.Forms.Button; $btnCan.Text='Cancel'; $btnCan.Size=New-Object Drawing.Size(95,32); $btnCan.Location=New-Object Drawing.Point(300,190); $btnCan.DialogResult=2; $btnCan.FlatStyle='System'; $f.AcceptButton=$btnOk; $f.CancelButton=$btnCan; $f.Controls.AddRange(@($l1,$l2,$l3,$t,$btnOk,$btnCan)); $f.Activate(); if($f.ShowDialog()-eq1){$t.Text}else{'CANCELLED'}"`;
    
    return await new Promise((r) => {
      exec(psCmd, { windowsHide: true }, (err, out) => {
        if (err || !out) {
          const fallbackMsg = `Instructions: ${msg.replace(/\n/g, " ")} | Enter Code:`;
          const fallback = `powershell -NoProfile -Command "Add-Type -AssemblyName Microsoft.VisualBasic; [Microsoft.VisualBasic.Interaction]::InputBox('${fallbackMsg}', '${title}')"`;
          exec(fallback, { windowsHide: true }, (err2, out2) => r(out2 ? out2.trim() : "CANCELLED"));
        } else {
          r(out.trim());
        }
      });
    });
  } else {
    // Linux: Try zenity first, then fallback to terminal
    return await new Promise((r) => {
      const zenityMsg = `Instructions:\\n${msg.replace(/\n/g, "\\n")}\\n\\nEnter Code:`;
      exec(`zenity --entry --title="${title}" --text="${zenityMsg}" --width=400`, (err, out) => {
        if (!err && out) return r(out.trim());
        console.log(`\n=== ${title} ===\nInstructions:\n${msg}\n\nEnter Code:`);
        const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
        rl.question("> ", (ans) => { rl.close(); r(ans.trim()); });
      });
    });
  }
}

// --- App Logic ---
async function main() {
  const isLinux = process.platform === "linux";
  const isWin = process.platform === "win32";

  // Linux backgrounding: Double-fork and disconnect from TTY
  // This must be at the very top to prevent any stdout from locking the terminal
  if (isLinux && isPkg && !process.argv.includes("--foreground")) {
    try {
      const { spawn } = require("child_process");
      const child = spawn(process.execPath, [...process.argv.slice(1), "--foreground"], {
        detached: true,
        stdio: 'ignore',
        cwd: root
      });

      child.unref();
      process.exit(0);
    } catch (e) {}
  }

  // Ensure stdin is fully closed and doesn't hold the TTY
  if (isPkg && isLinux) {
    try {
      process.stdin.unref();
      if (process.stdin.close) process.stdin.close();
    } catch(e){}
  }

  log("=== V77 START ===");

  if (isWin && isPkg) {
    exec(
      `powershell -NoProfile -Command "Add-Type -Name Win -Namespace Win -MemberDefinition '[DllImport(\\"kernel32.dll\\")] public static extern IntPtr GetConsoleWindow(); [DllImport(\\"user32.dll\\")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);'; [Win.Window]::ShowWindow([Win.Window]::GetConsoleWindow(), 0)"`,
      { windowsHide: true },
    );
  }

  const titleText = isLinux ? "T__Music__Bot" : "T_Music_Bot";

    tray = new TrayController({
    title: titleText,
    tooltip: isLinux ? "T__Music__Bot RPC" : "T_Music_Bot RPC",
    items: [
      { title: "WS: Disconnected", enabled: false, __id: 1 },
      { title: "RPC: Disconnected", enabled: false, __id: 2 },
      { title: "Open Logs", enabled: true, __id: 3 },
      { title: "Quit", enabled: true, __id: 4 },
      { title: `${titleText} ${APP_VERSION}`, enabled: false, __id: 5 },
    ],
  });
  await tray.init();

  let settings = { code: null, userId: "" };
  let retryDelay = 5000;

  try {
    if (fs.existsSync(settingsFile)) {
      const data = JSON.parse(fs.readFileSync(settingsFile, "utf8"));
      settings = { ...settings, ...data };
      fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2));
    } else {
      fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2));
    }
  } catch (e) {
    log(`Settings Load Error: ${e.message}`);
  }

  while (true) {
    try {
      if (fs.existsSync(settingsFile)) {
        settings = { ...settings, ...JSON.parse(fs.readFileSync(settingsFile, "utf8")) };
      }
    } catch (e) {}

    const result = await connect(settings.code, settings);
    if (!result.success) {
      if (result.clearCode) {
        settings.code = null;
        try { fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2)); } catch (e) {}
      }
      state.ws = "Retrying...";
      tray.updateStatus(state.ws, state.rpc);
      
      // Gradually increase retry delay up to 30s during persistent outages (like 530 errors)
      await new Promise((r) => setTimeout(r, retryDelay));
      retryDelay = Math.min(retryDelay + 5000, 30000);
    } else {
      retryDelay = 5000; // Reset on success
      // Wait for disconnection of the CURRENT client
      const activeClient = wsClient;
      await new Promise((r) => {
        if (!activeClient || activeClient.readyState !== WebSocket.OPEN) return r();
        activeClient.once("close", () => {
          log("Reconnection loop triggered by close.");
          r();
        });
        activeClient.once("error", () => {
          log("Reconnection loop triggered by error.");
          r();
        });
      });
    }
  }
}

const WS_URL = "wss://rpc.tehcraft.xyz/ws";
async function connect(code, settings) {
  if (wsClient) { try { wsClient.terminate(); } catch(e){} }
  if (heartbeatInterval) { clearInterval(heartbeatInterval); heartbeatInterval = null; }
  
  // Only destroy RPC if it's truly dead. If it's still "Connected", keep it to preserve activity.
  if (rpcClient && state.rpc !== "Connected") { 
    try { await rpcClient.destroy().catch(() => {}); } catch(e){} 
  }
  
  return new Promise((resolve) => {
    let isResolved = false;
    const safeResolve = (val) => {
      if (isResolved) return;
      isResolved = true;
      if (heartbeatInterval) { clearInterval(heartbeatInterval); heartbeatInterval = null; }
      resolve(val);
    };

    const now = Date.now();
    if (now - lastConnectLogTime > 30000) {
      log("Connecting to server...");
      lastConnectLogTime = now;
    }
    state.ws = "Connecting...";
    tray.updateStatus(state.ws, state.rpc);

    try {
      wsClient = new WebSocket(WS_URL, { 
        headers: { 
          "User-Agent": `T-Music-RPC/${APP_VERSION} (${os.platform()})`,
          "Origin": "https://rpc.tehcraft.xyz"
        } 
      });
    } catch (e) {
      safeResolve({ success: false, clearCode: false });
      return;
    }

    let timeout = setTimeout(() => {
      log("Connection timed out.");
      if (wsClient) {
        try { wsClient.terminate(); } catch(e){}
      }
      safeResolve({ success: false, clearCode: false });
    }, 15000);

    wsClient.on("open", () => {
      state.ws = "Connected";
      tray.updateStatus(state.ws, state.rpc);
      wsClient.send(JSON.stringify({ type: "connect" }));

      // Setup Heartbeat
      if (heartbeatInterval) clearInterval(heartbeatInterval);
      let lastActivity = Date.now();
      
      wsClient.on("pong", () => { lastActivity = Date.now(); });
      wsClient.on("message", (raw) => { 
        lastActivity = Date.now(); 
        try {
          const m = JSON.parse(raw.toString());
          if (m.type === "rpc_update" || m.type === "authenticated") {
            lastMessageTime = Date.now();
          }
        } catch(e) {
          // Log malformed messages as they might indicate server-side issues
          log(`Malformed server message: ${raw.toString().substring(0, 60)}...`);
        }
      });

      heartbeatInterval = setInterval(() => {
        if (wsClient && wsClient.readyState === WebSocket.OPEN) {
          wsClient.ping();
          
          // If no activity (pong or message) for 90s, the connection is a "zombie"
          if (Date.now() - lastActivity > 90000) {
            log("Connection idle for 90s - terminating zombie session.");
            wsClient.terminate();
          }

          // If we haven't received a real RPC update or auth in 20 minutes, 
          // the server might have forgotten about us or the socket is stale.
          if (Date.now() - lastMessageTime > 1200000) {
            log("No data received for 20m - refreshing connection.");
            wsClient.terminate();
          }
        }
      }, 20000); // Check every 20s
    });

    wsClient.on("message", (raw) => {
      try {
        const m = JSON.parse(raw.toString());

        if (m.type === "connect") {
          const setupWsAuth = async () => {
            let currentCode = settings.code;
            if (!currentCode || !/^\d{6}$/.test(currentCode)) {
              if (isPrompting) return;
              isPrompting = true;
              currentCode = await getPairingCode();
              isPrompting = false;
              if (!currentCode || currentCode === "CANCELLED") { await cleanup(); return; }
              settings.code = currentCode;
              try { fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2)); } catch(e){}
            }
            const authId = settings.userId || rpcClient.user.id;
            wsClient.send(JSON.stringify({ type: "auth", userId: authId, code: currentCode }));
            
            log("Sending update request.");
            wsClient.send(JSON.stringify({ type: "request_update", userId: authId }));
          };

          if (rpcClient && state.rpc === "Connected") {
            if (Date.now() - lastReuseLogTime > 5000) {
               log("Reusing existing Discord RPC connection.");
               lastReuseLogTime = Date.now();
            }
            setupWsAuth();
          } else {
            if (rpcClient) { try { rpcClient.destroy(); } catch(e){} }
            lastActivityData = null; 
            rpcClient = new RPC.Client({ transport: "ipc" });
            rpcClient.on("ready", () => {
              log(`Discord Connected: ${rpcClient.user.username}`);
              state.rpc = "Connected";
              tray.updateStatus(state.ws, state.rpc);
              setupWsAuth();
            });

            const tryLogin = () => {
              if (!rpcClient || state.ws !== "Connected" || isShuttingDown) return;
              rpcClient.login({ clientId: m.clientId }).catch(() => {
                state.rpc = "Discord Not Found";
                tray.updateStatus(state.ws, state.rpc);
                setTimeout(tryLogin, 15000);
              });
            };
            tryLogin();
          }
        }
        if (m.type === "authenticated") {
          log("Authenticated.");
          clearTimeout(timeout);
          safeResolve({ success: true });
        }
        if (m.type === "rpc_update") {
          if (rpcClient && state.rpc === "Connected") {
            updateActivity(m.data);
          } else {
            const details = m.data?.details || "Clear Request";
            log(`Ignored Update: ${details} (RPC: ${state.rpc})`);
          }
        }
        if (m.type === "error") {
          log(`Error: ${m.message}`);
          if (m.message.toLowerCase().includes("code") || m.message.toLowerCase().includes("pairing")) {
            clearTimeout(timeout);
            safeResolve({ success: false, clearCode: true, message: m.message });
          }
        }
      } catch (e) {} // Errors handled in the open listener's message handler
    });

    wsClient.on("close", (code, reason) => {
      const reasonStr = reason ? ` (${reason})` : "";
      log(`WS Closed. Code: ${code}${reasonStr}`);
      state.ws = "Disconnected";
      tray.updateStatus(state.ws, state.rpc);
      clearTimeout(timeout);
      if (heartbeatInterval) { clearInterval(heartbeatInterval); heartbeatInterval = null; }
      safeResolve({ success: false, clearCode: false });
    });

    wsClient.on("error", (err) => {
      if (err.message.includes("530")) {
        log("WS Error: Server response 530 (Origin Unreachable). The Fastify server likely crashed.");
      } else {
        log(`WS Error: ${err.message}`);
      }
      clearTimeout(timeout);
      if (heartbeatInterval) { clearInterval(heartbeatInterval); heartbeatInterval = null; }
      safeResolve({ success: false, clearCode: false });
    });
  });
}

main();
