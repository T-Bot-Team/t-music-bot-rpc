import { exec } from "child_process";
import fs from "fs";
import path from "path";
import os from "os";
import {
  PATHS,
  IS_WIN,
  IS_PKG,
  GLOBAL_STATE,
  APP_VERSION,
  Settings,
} from "./utils/constants";
import * as logger from "./lib/logger";
import { TrayController } from "./lib/tray";
import * as visualizer from "./handlers/visualizer";
import * as serverMod from "./handlers/server";
import * as rpcMod from "./handlers/rpc";

(process as any).noDeprecation = true;

const lockFile = path.join(os.tmpdir(), "t-music-bot-rpc.lock");

function checkLock(): void {
  if (fs.existsSync(lockFile)) {
    try {
      const pidStr = fs.readFileSync(lockFile, "utf8");
      const pid = parseInt(pidStr);
      if (!isNaN(pid)) {
        try {
          process.kill(pid, 0);
          logger.log(`Another instance is already running (PID: ${pid}). Exiting.`, true);
          process.exit(1);
        } catch (e) {
          // Process not found, we can continue
        }
      }
    } catch (e) {
      try { fs.unlinkSync(lockFile); } catch (e2) {}
    }
  }
  fs.writeFileSync(lockFile, process.pid.toString());
}

async function prepareFFmpeg(): Promise<void> {
  if (!IS_PKG) return;
  const targetDir = path.dirname(PATHS.extractedFFMPEG);
  try {
    if (!fs.existsSync(targetDir)) fs.mkdirSync(targetDir, { recursive: true });
    
    // Check bundled ffmpeg in multiple possible snapshot locations
    const possiblePaths = [
        path.join(PATHS.internal, IS_WIN ? "ffmpeg.exe" : "ffmpeg"),
        path.join(PATHS.internal, "dist", IS_WIN ? "ffmpeg.exe" : "ffmpeg"),
        path.join(process.cwd(), IS_WIN ? "ffmpeg.exe" : "ffmpeg")
    ];

    let src = "";
    for (const p of possiblePaths) {
        if (fs.existsSync(p)) { src = p; break; }
    }

    if (src) {
        if (!fs.existsSync(PATHS.extractedFFMPEG)) {
            logger.log(`Extracting bundled ffmpeg from ${src}...`, true);
            fs.writeFileSync(PATHS.extractedFFMPEG, fs.readFileSync(src));
            if (!IS_WIN) fs.chmodSync(PATHS.extractedFFMPEG, 0o755);
            logger.log("Extraction complete.", true);
        }
        (PATHS as any).ffmpeg = PATHS.extractedFFMPEG;
    } else {
        logger.log("Bundled ffmpeg not found in snapshot. Using system fallback.", true);
    }
  } catch (e: any) {
    logger.log(`FFmpeg Extraction failed: ${e.message}`, true);
  }
}

async function main(): Promise<void> {
  checkLock();
  await prepareFFmpeg();
  logger.log("================== T_Music_Bot RPC ==================", true);

  if (IS_WIN && IS_PKG) {
    exec(`powershell -NoProfile -Command "Add-Type -Name Win -Namespace Win -MemberDefinition '[DllImport(\\"kernel32.dll\\")] public static extern IntPtr GetConsoleWindow(); [DllImport(\\"user32.dll\\")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);'; [Win.Window]::ShowWindow([Win.Window]::GetConsoleWindow(), 0)"`, { windowsHide: true });
  }

  // 1. LOAD DEFAULTS FIRST
  let baseSettings: any = {};
  try {
    if (fs.existsSync(PATHS.defaultSettings)) {
      baseSettings = JSON.parse(fs.readFileSync(PATHS.defaultSettings, "utf8"));
    }
  } catch (e: any) { logger.log(`Default Settings Error: ${e.message}`); }

  // 2. OVERWRITE WITH USER SETTINGS (DEEP MERGE)
  try {
    if (fs.existsSync(PATHS.settings)) {
      const userSettings = JSON.parse(fs.readFileSync(PATHS.settings, "utf8"));
      
      const merge = (target: any, source: any): any => {
        for (const key in source) {
          if (source[key] instanceof Object && key in target) {
            merge(target[key], source[key]);
          } else {
            target[key] = source[key];
          }
        }
        return target;
      };
      
      GLOBAL_STATE.settings = merge(baseSettings, userSettings) as Settings;
    } else {
      GLOBAL_STATE.settings = baseSettings as Settings;
    }
  } catch (e: any) {
    logger.log(`User Settings Error: ${e.message}`);
    GLOBAL_STATE.settings = baseSettings as Settings;
  }

  // 3. FINAL VALIDATION - Ensure overlay and nested objects exist
  if (!GLOBAL_STATE.settings) GLOBAL_STATE.settings = {} as Settings;
  if (!GLOBAL_STATE.settings.overlay) GLOBAL_STATE.settings.overlay = { enabled: false, port: 3000, visualizer: {} as any };
  if (!GLOBAL_STATE.settings.overlay.visualizer) GLOBAL_STATE.settings.overlay.visualizer = { enabled: false } as any;

  if (process.argv.includes("--list")) {
    const d = await visualizer.listDevices();
    logger.log("================== AVAILABLE AUDIO DEVICES ==================", true);
    d.forEach((i) => logger.log(`- ${i}`, true));
    process.exit(0);
  }

  const tray = new TrayController({
    title: "T_Music_Bot",
    tooltip: "T_Music_Bot RPC",
    items: [
      { title: "WS: Disconnected", enabled: false, __id: 1 },
      { title: "RPC: Disconnected", enabled: false, __id: 2 },
      { title: "Open Logs", enabled: true, __id: 3 },
      { title: "Quit", enabled: true, __id: 4 },
      { title: `T_Music_Bot ${APP_VERSION}`, enabled: false, __id: 5 },
    ],
  });
  await tray.init(cleanup);

  if (GLOBAL_STATE.settings?.overlay.enabled) {
    visualizer.init();
    if (!visualizer.validate()) {
        logger.log("Fatal: Configuration validation failed. Please fix settings.json.", true);
        process.exit(1);
    }
    serverMod.start();
    if (GLOBAL_STATE.settings.overlay.visualizer.enabled)
      visualizer.startCapture(serverMod.broadcast);
    
    logger.log(`Overlay up! http://localhost:${GLOBAL_STATE.settings.overlay.port}`, true);
  }

  let retryDelay = 5000;
  while (true) {
    const res = await rpcMod.connect(tray, serverMod.updateTrack);
    if (!res.success) {
      if (GLOBAL_STATE.settings) GLOBAL_STATE.settings.code = null;
      await new Promise((r) => setTimeout(r, retryDelay));
      retryDelay = Math.min(retryDelay + 5000, 30000);
    } else {
      retryDelay = 5000;
      logger.log("Connection successful!");
      await rpcMod.waitForDisconnect();
    }
  }
}

async function cleanup(): Promise<void> {
  if (GLOBAL_STATE.isShuttingDown) return;
  GLOBAL_STATE.isShuttingDown = true;
  logger.log("Shutting down...", true);
  await rpcMod.shutdown();
  serverMod.stop();
  visualizer.kill();
  try {
    if (fs.existsSync(lockFile)) fs.unlinkSync(lockFile);
  } catch (e) {}
  process.exit(0);
}

process.on("SIGINT", cleanup);
process.on("SIGTERM", cleanup);

main().catch((e: any) => {
  logger.log(`Fatal Error: ${e.message}`, true);
  process.exit(1);
});
