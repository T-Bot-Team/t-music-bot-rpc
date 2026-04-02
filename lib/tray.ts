import { spawn, ChildProcess, exec } from "child_process";
import fs from "fs";
import path from "path";
import os from "os";
import readline from "readline";
import { PATHS, IS_WIN, IS_LINUX } from "../utils/constants";

export interface TrayItem {
  title: string;
  tooltip?: string;
  enabled: boolean;
  checked?: boolean;
  __id: number;
}

export interface TrayMenu {
  title: string;
  tooltip: string;
  icon?: string;
  items: TrayItem[];
}

export class TrayController {
  private menu: TrayMenu;
  private process: ChildProcess | null = null;
  public ready: boolean = false;
  private _readyPromise: Promise<boolean> | null = null;
  private _resolveReady: ((val: boolean) => void) | null = null;

  constructor(menu: TrayMenu) {
    this.menu = menu;
  }

  async init(onQuit: () => void): Promise<boolean> {
    this._readyPromise = new Promise((resolve) => {
      this._resolveReady = resolve;
    });

    const binName = IS_WIN ? "tray_windows_release.exe" : "tray_linux_release";
    const dstName = IS_WIN ? "T_Music_Bot-RPC.exe" : "T_Music_Bot RPC";
    const tempDir = path.join(os.tmpdir(), "t-music-bot-rpc");

    try {
      if (!fs.existsSync(tempDir)) fs.mkdirSync(tempDir, { recursive: true });
      const binPath = path.resolve(path.join(tempDir, dstName));
      const iconName = IS_WIN ? "icon.ico" : "icon.png";
      const iconPath = [
        path.join(PATHS.assets, iconName),
        path.join(PATHS.internalAssets, iconName),
      ].find((p) => fs.existsSync(p));

      const binSrc = path.join(PATHS.internal, "node_modules", "systray2", "traybin", binName);

      if (fs.existsSync(binSrc) && !fs.existsSync(binPath)) {
        fs.writeFileSync(binPath, fs.readFileSync(binSrc));
        if (IS_LINUX) fs.chmodSync(binPath, 0o755);
      }

      if (iconPath) {
        this.menu.icon = fs.readFileSync(iconPath).toString("base64");
      }


      this.process = spawn(binPath, [], { windowsHide: true });
      const rl = readline.createInterface({ input: this.process.stdout! });

      rl.on("line", (line) => {
        try {
          const action = JSON.parse(line);
          if (action.type === "ready") {
            this.ready = true;
            this.sendAction({ type: "initial", ...this.menu });
            this._resolveReady?.(true);
          } else if (action.type === "clicked") {
            if (action.item.title === "Quit") onQuit();
            if (action.item.title === "Open Logs") {
              exec(IS_WIN ? `start "" "${PATHS.logs}"` : `xdg-open "${PATHS.logs}"`);
            }
          }
        } catch (e) {}
      });

      this.process.on("exit", () => {
        this.ready = false;
        this._resolveReady?.(false);
      });
    } catch (e) {
      this._resolveReady?.(false);
    }

    return this._readyPromise!;
  }

  sendAction(action: any): void {
    if (this.process && this.process.stdin?.writable) {
      this.process.stdin.write(JSON.stringify(action) + "\n");
    }
  }

  updateStatus(ws: string, rpc: string): void {
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

  kill(): void {
    if (this.process) this.process.kill();
  }
}
