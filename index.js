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
var isQuiet = process.argv.includes("--quiet") || process.argv.includes("-q");

// Version
let APP_VERSION = "v1.0.0";
try { APP_VERSION = `v${require("./package.json").version}`; } catch (e) {}

// Config
const isPkg = !!process.pkg;
const root = isPkg ? path.dirname(process.execPath) : __dirname;
const settingsFile = path.join(root, "settings.json");
const logFile = path.join(root, "logs.txt");

function log(m, force = false) {
  if (isQuiet && !force) return;
  const msg = `[${new Date().toLocaleTimeString()}] ${m}`;
  console.log(msg);
  try {
    fs.appendFileSync(logFile, msg + os.EOL);
  } catch (e) {}
}

async function updateActivity(data) {
  if (!rpcClient || state.rpc !== "Connected") return;
  
  const activity = { ...data, type: 2, instance: false };
  const activityStr = JSON.stringify(activity);
  if (lastActivityData === activityStr) return;

  if (activityTimeout) clearTimeout(activityTimeout);
  
  try {
    await rpcClient.setActivity(activity);
    lastActivityData = activityStr;
  } catch (err) {
    const errM = err.message.toLowerCase();
    if (errM.includes("rate limit") || errM.includes("cooldown")) {
      log("RPC Cooldown - Retrying in 15s...");
      activityTimeout = setTimeout(() => updateActivity(data), 15000);
    } else {
      log(`RPC Update Error: ${err.message}`);
    }
  }
}

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
    if (item.title === "Quit") process.exit(0);
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
  log("=== V77 START ===");

  if (process.platform === "win32" && isPkg) {
    exec(
      `powershell -NoProfile -Command "Add-Type -Name Win -Namespace Win -MemberDefinition '[DllImport(\\"kernel32.dll\\")] public static extern IntPtr GetConsoleWindow(); [DllImport(\\"user32.dll\\")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);'; [Win.Window]::ShowWindow([Win.Window]::GetConsoleWindow(), 0)"`,
      { windowsHide: true },
    );
  }

  const isLinux = process.platform === "linux";
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
        const data = JSON.parse(fs.readFileSync(settingsFile, "utf8"));
        if (data && typeof data === "object") settings = { ...settings, ...data };
      }
    } catch (e) {}

    if (!settings.code || !/^\d{6}$/.test(settings.code)) {
      const input = await getPairingCode();
      if (!input || input === "CANCELLED") process.exit(0);
      if (!/^\d{6}$/.test(input)) { 
        log("Invalid code."); 
        continue; 
      }
      settings.code = input;
      fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2));
    }

    const result = await connect(settings.code, settings);
    if (!result.success) {
      if (result.clearCode) {
        settings.code = null;
        try { fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2)); } catch (e) {}
        await new Promise((r) => setTimeout(r, 500));
      } else {
        await new Promise((r) => setTimeout(r, 5000));
      }
    } else {
      // Wait for disconnection without polling
      await new Promise((r) => {
        if (state.ws !== "Connected") return r();
        wsClient.once("close", r);
      });
    }
  }
}

const WS_URL = "wss://rpc.tehcraft.xyz/ws";
function connect(code, settings) {
  return new Promise((resolve) => {
    if (wsClient) { try { wsClient.terminate(); } catch(e){} }
    if (rpcClient) { try { rpcClient.destroy(); } catch(e){} }

    state.ws = "Connecting...";
    tray.updateStatus(state.ws, state.rpc);

    try {
      wsClient = new WebSocket(WS_URL, { headers: { "User-Agent": "Mozilla/5.0" } });
    } catch (e) {
      resolve({ success: false, clearCode: false });
      return;
    }

    let timeout = setTimeout(() => {
      if (wsClient) wsClient.terminate();
      resolve({ success: false, clearCode: false });
    }, 10000);

    wsClient.on("open", () => {
      state.ws = "Connected";
      tray.updateStatus(state.ws, state.rpc);
      wsClient.send(JSON.stringify({ type: "connect" }));
    });

    wsClient.on("message", (data) => {
      try {
        const m = JSON.parse(data);
        if (m.type === "connect") {
          rpcClient = new RPC.Client({ transport: "ipc" });
          rpcClient.on("ready", () => {
            log(`Discord Connected: ${rpcClient.user.username}`);
            state.rpc = "Connected";
            tray.updateStatus(state.ws, state.rpc);
            const authId = settings.userId || rpcClient.user.id;
            wsClient.send(JSON.stringify({ type: "auth", userId: authId, code: code }));
            wsClient.send(JSON.stringify({ type: "request_update", userId: authId }));
          });
          rpcClient.login({ clientId: m.clientId }).catch(() => {
            state.rpc = "Discord Not Found";
            tray.updateStatus(state.ws, state.rpc);
          });
        }
        if (m.type === "authenticated") {
          log("Authenticated.");
          clearTimeout(timeout);
          resolve({ success: true });
        }
        if (m.type === "rpc_update" && rpcClient && state.rpc === "Connected") {
          updateActivity(m.data);
        }
        if (m.type === "error") {
          log(`Error: ${m.message}`);
          if (m.message.toLowerCase().includes("code") || m.message.toLowerCase().includes("pairing")) {
            clearTimeout(timeout);
            resolve({ success: false, clearCode: true, message: m.message });
          }
        }
      } catch (e) {}
    });

    wsClient.on("close", () => {
      state.ws = "Disconnected";
      state.rpc = "Disconnected";
      tray.updateStatus(state.ws, state.rpc);
      clearTimeout(timeout);
      resolve({ success: false, clearCode: false });
    });

    wsClient.on("error", () => resolve({ success: false, clearCode: false }));
  });
}

process.on("exit", () => {
  if (tray) tray.kill();
});
main();
