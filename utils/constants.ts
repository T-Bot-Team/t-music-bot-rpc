import path from "path";
import os from "os";
import fs from "fs";

// --- TYPES FOR RUST PORTING ---
export interface VisualizerConfig {
  enabled: boolean;
  audioDevice: string;
  sampleRate: number;
  samples: number;
  bars: number;
  minFreq: number;
  maxFreq: number;
  sensitivity: number;
  multiplier: number;
  smoothing: number;
  barWidth: number;
  barGap: number;
  colorTop: string;
  colorBottom: string;
  mode: "bars" | "wave" | "particles" | "neon-bars" | "led" | "outline" | "center-bars";
  glow: boolean;
  rounded: boolean;
  fps: number;
}

export interface OverlayConfig {
  enabled: boolean;
  port: number;
  visualizer: VisualizerConfig;
}

export interface Settings {
  code: string | null;
  userId: string | null;
  overlay: OverlayConfig;
}

export interface TrackUpdate {
  details?: string;
  state?: string;
  startTimestamp?: number;
  endTimestamp?: number;
  thumbnail?: string;
  paused?: boolean;
}

export interface GlobalState {
  isShuttingDown: boolean;
  lastTrack: TrackUpdate | null;
  overlayClients: any[]; // Using any[] for now, will refine as we convert the server
  settings: Settings | null;
}

const isPkg = !!(process as any).pkg;
const root = isPkg ? path.dirname(process.execPath) : process.cwd();
// When packaged, __dirname is inside /snapshot/t-music-rpc/dist/utils/
// We need to go up two levels to reach the snapshot root where ffmpeg.exe is bundled.
const internal = isPkg ? path.join(__dirname, "..", "..") : path.join(__dirname, "..");

export const APP_VERSION = "v1.1.0";
export const WS_URL = Buffer.from(
  "d3NzOi8vcnBjLnRlaGNyYWZ0Lnh5ei93cw==",
  "base64",
).toString();

export const PATHS = {
  root,
  internal,
  settings: path.join(root, "settings.json"),
  defaultSettings: path.join(internal, "settings.default.json"),
  logs: path.join(root, "logs.txt"),
  debugFFT: path.join(root, "debug_fft.txt"),
  ffmpeg: (() => {
    const local = path.join(root, process.platform === "win32" ? "ffmpeg.exe" : "ffmpeg");
    if (isPkg && fs.existsSync(local)) return local;
    return "ffmpeg"; // Fallback to PATH
  })(),
  extractedFFMPEG: path.join(os.tmpdir(), "t-music-bot-rpc", process.platform === "win32" ? "ffmpeg.exe" : "ffmpeg"),
  assets: path.join(root, "assets"),
  internalAssets: path.join(internal, "assets"),
};

export const GLOBAL_STATE: GlobalState = {
  isShuttingDown: false,
  lastTrack: null,
  overlayClients: [],
  settings: null,
};

export const IS_WIN = process.platform === "win32";
export const IS_LINUX = process.platform === "linux";
export const IS_PKG = isPkg;
export const IS_QUIET = process.argv.includes("--quiet") || process.argv.includes("-q");
export const IS_DEBUG_FFT = process.argv.includes("--debug-fft");
