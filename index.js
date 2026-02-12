// --- Globals (Absolute Top) ---
var wsClient = null;
var rpcClient = null;
var tray = null;
var trayReady = false;
var state = { ws: "Disconnected", rpc: "Disconnected" };
var lastTrayState = "";
var currentClientId = null;
var currentCode = null;
var rpcReconnectTimeout = null;
var trayUpdateTimeout = null;
var isConnectingRPC = false;
var heartbeatInterval = null;

// Version
let APP_VERSION = "v1.0.0";
try { 
    APP_VERSION = `v${require('./package.json').version}`; 
} catch (e) {}

// RPC Rate Limiting
var updateHistory = [];
var pendingActivity = null;
var rpcUpdateTimeout = null;
const MAX_UPDATES = 5;
const WINDOW_MS = 20500;

const RPC = require("@t_bot-team/discord-rpc");
const WebSocket = require("ws");
const SysTray = require("systray2").default;
const fs = require("fs");
const path = require("path");
const os = require("os");
const { exec } = require("child_process");

// --- Configuration ---
const WS_URL = "wss://rpc.tehcraft.xyz/ws";
const isPkg = !!process.pkg;
const root = isPkg ? path.dirname(process.execPath) : __dirname;
const settingsFile = path.join(root, "settings.json");
const logFile = path.join(root, "logs.txt");

// Extraction folder for assets (Matches systray2 internal logic)
const baseTempDir = path.join(os.tmpdir(), "tmusic_rpc_v69");
const binsTempDir = path.join(baseTempDir, "2.1.4"); // Explicit version for systray2
if (!fs.existsSync(binsTempDir)) fs.mkdirSync(binsTempDir, { recursive: true });

const binName = ({ 
    win32: "tray_windows_release.exe", 
    darwin: "tray_darwin_release", 
    linux: "tray_linux_release" 
})[process.platform];

const runtimeIconIco = path.resolve(path.join(baseTempDir, "icon.ico"));
const runtimeIconPng = path.resolve(path.join(baseTempDir, "icon.png"));
const runtimeBin = path.resolve(path.join(binsTempDir, binName));

const activeIcon = process.platform === 'win32' ? runtimeIconIco : runtimeIconPng;

function log(m) {
    const msg = `[${new Date().toLocaleTimeString()}] ${m}`;
    console.log(msg);
    try { fs.appendFileSync(logFile, msg + "\n"); } catch (e) {}
}

// --- Aggressive Console Hiding (Windows) ---
if (process.platform === 'win32' && isPkg) {
    const hide = () => {
        const cmd = `powershell -NoProfile -Command "Add-Type -Name Win -Namespace Win -MemberDefinition '[DllImport(\\"kernel32.dll\\")] public static extern IntPtr GetConsoleWindow(); [DllImport(\\"user32.dll\\")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);'; $h = [Win.Win]::GetConsoleWindow(); if ($h -ne [IntPtr]::Zero) { [Win.Win]::ShowWindow($h, 0) }"`;
        exec(cmd, { windowsHide: true });
    };
    hide();
    setTimeout(hide, 1000);
    setTimeout(hide, 2000);
}

function extractAssets() {
    try {
        // Extract Icons
        const icoPath = path.join(__dirname, "icon.ico");
        if (fs.existsSync(icoPath)) fs.writeFileSync(runtimeIconIco, fs.readFileSync(icoPath));
        const pngPath = path.join(__dirname, "icon.png");
        if (fs.existsSync(pngPath)) fs.writeFileSync(runtimeIconPng, fs.readFileSync(pngPath));
        
        // Extract Binary to EXACT path systray2 expects to fix EACCES
        const binPath = path.join(__dirname, "node_modules/systray2/traybin", binName);
        if (fs.existsSync(binPath)) {
            fs.writeFileSync(runtimeBin, fs.readFileSync(binPath));
            if (process.platform !== 'win32') fs.chmodSync(runtimeBin, 0o755);
            log(`Binary prepared at: ${runtimeBin}`);
        }
    } catch (e) { log(`Extraction Error: ${e.message}`); }
}

// --- Tray Logic ---
function getMenu() {
    return {
        icon: activeIcon,
        title: "T_Music_Bot",
        tooltip: "T_Music_Bot RPC",
        items: [
            { title: `T_Music_Bot ${APP_VERSION}`, enabled: false, __id: 1 },
            { title: "<SEPARATOR>", enabled: true, __id: 2 },
            { title: `WS: ${state.ws}`, enabled: false, __id: 3 },
            { title: `RPC: ${state.rpc}`, enabled: false, __id: 4 },
            { title: "<SEPARATOR>", enabled: true, __id: 5 },
            { title: "Open Logs", enabled: true, __id: 6 },
            { title: "Quit", enabled: true, __id: 7 }
        ]
    };
}

function updateTray() {
    if (!tray || !trayReady) return;
    const currentStateStr = `${state.ws}|${state.rpc}`;
    if (currentStateStr === lastTrayState) return;
    
    if (trayUpdateTimeout) return;
    trayUpdateTimeout = setTimeout(() => {
        trayUpdateTimeout = null;
        lastTrayState = currentStateStr;
        try {
            // Full Redraw method confirmed visually working
            tray.sendAction({ type: "update-menu", menu: getMenu() });
            log(`Tray visually updated: WS=${state.ws} | RPC=${state.rpc}`);
        } catch (e) { log(`Tray Sync Fail: ${e.message}`); }
    }, 200);
}

async function initTray() {
    log("Initializing Tray...");
    extractAssets();
    await new Promise(r => setTimeout(r, 1000));

    try {
        tray = new SysTray({
            menu: getMenu(),
            debug: false,
            copyDir: baseTempDir // Matches manual extraction path
        });
        await tray.ready();
        trayReady = true;
        log("Tray is fully ready.");
        updateTray();
        tray.onClick(action => {
            if (action.item.title === "Quit") process.exit(0);
            if (action.item.title === "Open Logs") {
                const cmd = process.platform === 'win32' ? `start "" "${logFile}"` : `xdg-open "${logFile}"`;
                exec(cmd, { windowsHide: true });
            }
        });
    } catch (e) { log(`Tray Init Error: ${e.message}`); }
}

// --- Discord RPC ---
async function destroyRPC() {
    if (rpcReconnectTimeout) clearTimeout(rpcReconnectTimeout);
    if (rpcClient) {
        log("Cleaning up Discord RPC...");
        const client = rpcClient;
        rpcClient = null;
        try { client.removeAllListeners(); await client.destroy(); } catch (e) {}
    }
    if (state.rpc !== "Disconnected") {
        state.rpc = "Disconnected";
        updateTray();
    }
}

async function startRPC(clientId, code) {
    if (isConnectingRPC) return;
    isConnectingRPC = true;
    currentClientId = clientId;
    currentCode = code;
    
    await destroyRPC();
    await new Promise(r => setTimeout(r, 2000)); // OS Pipe Cooldown

    log(`Connecting Discord...`);
    state.rpc = "Connecting...";
    updateTray();

    rpcClient = new RPC.Client({ transport: "ipc" });
    rpcClient.on('ready', () => {
        log("Discord RPC Ready.");
        state.rpc = "Connected";
        updateTray();
        isConnectingRPC = false;
        if (wsClient && wsClient.readyState === WebSocket.OPEN) {
            wsClient.send(JSON.stringify({ type: "auth", userId: rpcClient.user.id, code: code }));
            wsClient.send(JSON.stringify({ type: "request_update", userId: rpcClient.user.id }));
        }
    });
    rpcClient.on('disconnected', () => {
        log("Discord RPC Lost.");
        destroyRPC();
        scheduleRPCReconnect();
    });
    rpcClient.login({ clientId }).catch(e => {
        log(`Discord Fail: ${e.message}`);
        state.rpc = "Discord Not Found";
        updateTray();
        isConnectingRPC = false;
        scheduleRPCReconnect();
    });
}

function scheduleRPCReconnect() {
    if (rpcReconnectTimeout) return;
    log("RPC retry scheduled in 15s...");
    rpcReconnectTimeout = setTimeout(() => {
        rpcReconnectTimeout = null;
        if (currentClientId && currentCode) startRPC(currentClientId, currentCode);
    }, 15000);
}

// --- Rate Limiting ---
function updateActivity(d) {
    const now = Date.now();
    updateHistory = updateHistory.filter(t => now - t < WINDOW_MS);
    if (updateHistory.length < MAX_UPDATES) {
        if (!rpcClient || state.rpc !== "Connected") return;
        rpcClient.setActivity({
            details: d.details, state: d.state,
            largeImageKey: d.largeImageKey, smallImageKey: d.smallImageKey, smallImageText: d.smallImageText,
            startTimestamp: d.startTimestamp || null, endTimestamp: d.endTimestamp || null,
            type: 2, instance: false
        }).catch(() => {});
        updateHistory.push(now);
    } else {
        pendingActivity = d;
        if (!rpcUpdateTimeout) {
            rpcUpdateTimeout = setTimeout(() => {
                rpcUpdateTimeout = null;
                if (pendingActivity) { updateActivity(pendingActivity); pendingActivity = null; }
            }, 1000);
        }
    }
}

// --- Main ---
async function main() {
    log(`--- V69 START ---`);
    await initTray();
    let settings = { code: null };
    try { if (fs.existsSync(settingsFile)) settings = JSON.parse(fs.readFileSync(settingsFile, "utf8")); } catch (e) {}
    
    while (true) {
        if (!settings.code || !/^\d{6}$/.test(settings.code)) {
            const title = "T_Music_Bot Setup";
            const psCmd = `powershell -NoProfile -WindowStyle Hidden -Command "Add-Type -AssemblyName Microsoft.VisualBasic; $nl = [Environment]::NewLine; $msg = 'Instructions:' + $nl + '1. Open T_Music_Bot App' + $nl + '2. Settings > Discord RPC' + $nl + '3. Copy the 6-digit code' + $nl + $nl + 'Enter Pairing Code:'; $res = [Microsoft.VisualBasic.Interaction]::InputBox($msg, '${title}'); if ($?) { Write-Output $res } else { Write-Output 'CANCELLED' }"`;
            const input = await new Promise(r => { exec(psCmd, { windowsHide: true }, (err, stdout) => r(stdout ? stdout.trim() : null)); });
            if (!input || input === "CANCELLED") process.exit(0);
            if (!/^\d{6}$/.test(input)) continue;
            settings.code = input;
            fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2));
        }
        const result = await connectAndAuth(settings.code);
        if (result === "AUTH_SUCCESS") {
            while (state.ws === "Connected") await new Promise(r => setTimeout(r, 2000));
            await new Promise(r => setTimeout(r, 5000));
        } else if (result === "AUTH_FAILED") { settings.code = null; } else { await new Promise(r => setTimeout(r, 10000)); }
    }
}

function connectAndAuth(code) {
    return new Promise((resolve) => {
        log(`WS Connecting...`);
        state.ws = "Connecting...";
        updateTray();
        if (wsClient) { try { wsClient.terminate(); } catch(e){} }
        if (heartbeatInterval) clearInterval(heartbeatInterval);
        wsClient = new WebSocket(WS_URL, { headers: { 'User-Agent': 'Mozilla/5.0', 'Origin': 'https://rpc.tehcraft.xyz' } });
        let authenticated = false;
        let timeout = setTimeout(() => { if (wsClient) wsClient.terminate(); resolve("ERR"); }, 20000);
        wsClient.on('open', () => {
            log("WS Connected.");
            state.ws = "Connected";
            updateTray();
            wsClient.send(JSON.stringify({ type: "connect" }));
            heartbeatInterval = setInterval(() => { if (wsClient.readyState === WebSocket.OPEN) wsClient.ping(); }, 30000);
        });
        wsClient.on('message', (data) => {
            try {
                const m = JSON.parse(data);
                if (m.type === "connect") startRPC(m.clientId, code);
                if (m.type === "authenticated") { authenticated = true; clearTimeout(timeout); resolve("AUTH_SUCCESS"); }
                if (m.type === "rpc_update") updateActivity(m.data);
                if (m.type === "error" && m.message.includes("pairing code")) { clearTimeout(timeout); resolve("AUTH_FAILED"); }
            } catch (e) {}
        });
        wsClient.on('close', () => { state.ws = "Disconnected"; updateTray(); destroyRPC(); if (heartbeatInterval) clearInterval(heartbeatInterval); clearTimeout(timeout); if (!authenticated) resolve("ERR"); });
        wsClient.on('error', () => { clearTimeout(timeout); if (!authenticated) resolve("ERR"); });
    });
}

process.on('exit', () => { if (tray) tray.kill(false); });
main();
