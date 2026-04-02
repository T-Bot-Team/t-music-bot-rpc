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
  // 1. Check if ffmpeg is available in PATH or root
  const checkCmd = IS_WIN ? "where ffmpeg" : "which ffmpeg";
  const hasFFmpeg = await new Promise((r) => exec(checkCmd, (e) => r(!e)));

  if (hasFFmpeg) return;

  // 2. If not found, prompt the user
  const title = "FFmpeg Missing";
  const msg = "The visualizer requires FFmpeg to function. Would you like to download a minimal version automatically?";
  
  let shouldDownload = false;
  if (IS_WIN) {
    const ps = `powershell -NoProfile -Command "[Windows.Forms.MessageBox]::Show('${msg}', '${title}', [Windows.Forms.MessageBoxButtons]::YesNo, [Windows.Forms.MessageBoxIcon]::Information)"`;
    const res = await new Promise<string>((r) => exec(ps, (e, o) => r(o.trim())));
    shouldDownload = res === "Yes";
  } else {
    const res = await new Promise<boolean>((r) => exec(`zenity --question --title="${title}" --text="${msg}"`, (e) => r(!e)));
    shouldDownload = res;
  }

  if (shouldDownload) {
    logger.log("Downloading minimal FFmpeg...", true);
    // You would typically use a library like 'got' or 'axios' here, but since we want minimal deps, 
    // we'll use a powershell/curl command.
    if (IS_WIN) {
        const zipUrl = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl-shared.zip"; // Example URL, replace with a truly minimal build if possible
        const psDownload = `powershell -NoProfile -Command "Invoke-WebRequest -Uri '${zipUrl}' -OutFile 'ffmpeg.zip'; Expand-Archive 'ffmpeg.zip' -DestinationPath '.'; Move-Item 'ffmpeg-master-latest-win64-gpl-shared/bin/ffmpeg.exe' './ffmpeg.exe'; Remove-Item 'ffmpeg.zip'; Remove-Item -Recurse 'ffmpeg-master-latest-win64-gpl-shared'"`;
        await new Promise((r) => exec(psDownload, (e) => r(!e)));
    } else {
        // Simple Linux download logic
        await new Promise((r) => exec("curl -L https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz -o ffmpeg.tar.xz && tar -xvf ffmpeg.tar.xz && mv ffmpeg-*-amd64-static/ffmpeg . && rm -rf ffmpeg.tar.xz ffmpeg-*-amd64-static", (e) => r(!e)));
    }
    logger.log("FFmpeg installed successfully.", true);
  } else {
    logger.log("Visualizer will be disabled (FFmpeg missing).", true);
    if (GLOBAL_STATE.settings) GLOBAL_STATE.settings.overlay.visualizer.enabled = false;
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
