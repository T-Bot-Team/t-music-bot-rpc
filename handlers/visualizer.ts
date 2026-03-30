import { spawn, ChildProcess } from "child_process";
import fft from "fft-js";
import fs from "fs";
import os from "os";
import { PATHS, IS_WIN, IS_DEBUG_FFT, GLOBAL_STATE } from "../utils/constants";
import { log } from "../lib/logger";

let windowFunc: Float32Array, binHistory: Float32Array[], historyIndex = 0;
let ffmpegProcess: ChildProcess | null = null;

export const validate = (): boolean => {
  const viz = GLOBAL_STATE.settings?.overlay.visualizer;
  if (!viz) return false;

  const errors: string[] = [];
  viz.fps = Math.max(30, Math.min(240, parseInt(viz.fps as any) || 60));
  const s = parseInt(viz.samples as any);
  const isPowerOf2 = (n: number) => n && (n & (n - 1)) === 0;
  
  if (isNaN(s) || !isPowerOf2(s) || s < 512 || s > 16384)
      errors.push(`[Config Error] 'samples' must be a Power of 2 between 512 and 16384. (Got: ${viz.samples})`);
  
  viz.bars = Math.max(1, Math.min(2048, parseInt(viz.bars as any) || 96));
  viz.smoothing = Math.max(1, Math.min(20, parseInt(viz.smoothing as any) || 3));
  
  const isHex = (h: string) => /^#([0-9A-F]{3}){1,2}([0-9A-F]{2})?$/i.test(h);
  if (!isHex(viz.colorTop)) errors.push(`[Config Error] 'colorTop' invalid Hex. (Got: ${viz.colorTop})`);
  if (!isHex(viz.colorBottom)) errors.push(`[Config Error] 'colorBottom' invalid Hex. (Got: ${viz.colorBottom})`);
  
  const validModes = ["bars", "wave", "particles", "neon-bars", "led", "outline", "center-bars"];
  if (!validModes.includes(viz.mode)) errors.push(`[Config Error] 'mode' must be one of: ${validModes.join(", ")}. (Got: ${viz.mode})`);
  
  if (errors.length > 0) { 
    errors.forEach(err => log(err, true)); 
    return false; 
  }
  return true;
};

export const init = (): void => {
  const viz = GLOBAL_STATE.settings?.overlay.visualizer;
  if (!viz) return;

  const HISTORY_SIZE = Math.max(1, viz.smoothing);
  binHistory = Array.from({ length: viz.bars }, () => new Float32Array(HISTORY_SIZE).fill(0));
  windowFunc = new Float32Array(viz.samples);
  for (let i = 0; i < viz.samples; i++) {
    windowFunc[i] = 0.5 * (1 - Math.cos((2 * Math.PI * i) / (viz.samples - 1)));
  }
};

export const startCapture = (broadcaster: (data: any) => void): void => {
  const s = GLOBAL_STATE.settings;
  if (!s) return;

  const viz = s.overlay.visualizer;
  const target = viz.audioDevice || (IS_WIN ? "default" : "default");
  const sr = viz.sampleRate || 44100;
  const binWidth = sr / viz.samples;

  const bandRanges: { start: number; end: number }[] = [];
  const MIN_FREQ = 20, MAX_FREQ = 16000;
  for (let i = 0; i < viz.bars; i++) {
    let fStart = MIN_FREQ * Math.pow(MAX_FREQ / MIN_FREQ, i / viz.bars);
    let fEnd = MIN_FREQ * Math.pow(MAX_FREQ / MIN_FREQ, (i + 1) / viz.bars);
    let bStart = Math.floor(fStart / binWidth);
    let bEnd = Math.ceil(fEnd / binWidth);
    if (bEnd <= bStart) bEnd = bStart + 1;
    bandRanges.push({ start: bStart, end: bEnd });
  }

  const args = IS_WIN
    ? ["-loglevel", "quiet", "-f", "dshow", "-audio_buffer_size", "10", "-rtbufsize", "64k", "-threads", "1", "-i", `audio=${target}`, "-ac", "1", "-ar", sr.toString(), "-f", "s16le", "-fflags", "nobuffer+discardcorrupt", "-probesize", "32", "-analyzeduration", "0", "-flags", "low_delay", "pipe:1"]
    : ["-f", "pulse", "-i", target, "-ac", "1", "-ar", sr.toString(), "-f", "s16le", "pipe:1"];

  log(`Starting Visualizer: ${target} (Using: ${PATHS.ffmpeg})`);
  ffmpegProcess = spawn(PATHS.ffmpeg, args);

  let stderr = "";
  ffmpegProcess.stderr?.on("data", (d) => stderr += d.toString());

  ffmpegProcess.on("error", (err) => {
    log(`FFmpeg Spawn Error: ${err.message}`, true);
  });

  let audioBuffer = Buffer.alloc(0);
  const requiredBytes = viz.samples * 2;
  const hopSize = Math.floor(sr / 60) * 2;

  ffmpegProcess.stdout?.on("data", (chunk: Buffer) => {
    audioBuffer = Buffer.concat([audioBuffer, chunk]);
    if (audioBuffer.length > requiredBytes * 2) {
        audioBuffer = audioBuffer.slice(audioBuffer.length - requiredBytes);
    }

    while (audioBuffer.length >= requiredBytes) {
      const windowData = audioBuffer.slice(0, requiredBytes);
      audioBuffer = audioBuffer.slice(hopSize);

      const floatData = new Float32Array(viz.samples);
      for (let i = 0; i < viz.samples; i++) floatData[i] = (windowData.readInt16LE(i * 2) / 32768.0) * windowFunc[i];

      setImmediate(() => {
          try {
              const phasors = (fft as any).fft(floatData);
              const magnitudes = new Float32Array(viz.samples / 2);
              for (let i = 0; i < viz.samples / 2; i++) {
                  magnitudes[i] = (Math.sqrt(phasors[i][0] ** 2 + phasors[i][1] ** 2) / viz.samples) * 2.0;
              }

              for (let i = 0; i < viz.bars; i++) {
                let { start, end } = bandRanges[i];
                let maxPeak = 0, sumSq = 0;
                for (let b = start; b < end && b < magnitudes.length; b++) {
                  if (magnitudes[b] > maxPeak) maxPeak = magnitudes[b];
                  sumSq += magnitudes[b] * magnitudes[b];
                }
                let energy = (maxPeak * 0.7) + (Math.sqrt(sumSq / (end - start)) * 0.3);
                let db = 20 * Math.log10(Math.max(energy, 1e-10));
                db += (i / viz.bars) * 25.0; 
                let normalized = (db + viz.sensitivity) / viz.sensitivity;
                binHistory[i][historyIndex] = Math.max(0, Math.min(1.0, normalized * (viz.multiplier / 25.0)));
              }
              historyIndex = (historyIndex + 1) % Math.max(1, viz.smoothing);

              const barData = new Uint8Array(viz.bars);
              for (let i = 0; i < viz.bars; i++) {
                let avg = 0;
                for (let h = 0; h < binHistory[i].length; h++) avg += binHistory[i][h];
                barData[i] = Math.min(255, (avg / binHistory[i].length) * 255);
              }
              broadcaster({ type: "fft_data", bins: Array.from(barData) });
          } catch (e) {}
      });
    }
  });

  const startTime = Date.now();
  ffmpegProcess.on("close", (code) => {
    if (GLOBAL_STATE.isShuttingDown) return;
    
    const duration = Date.now() - startTime;
    if (duration < 1500) {
        log(`FFmpeg exited too quickly (code ${code}). Check if the audio device is correct.`, true);
        if (stderr) log(`FFmpeg Stderr: ${stderr.trim()}`, true);
        return; // Stop the loop if it's crashing instantly
    }
    
    setTimeout(() => startCapture(broadcaster), 2000);
  });
};

export const kill = (): void => { if (ffmpegProcess) ffmpegProcess.kill(); };

export const listDevices = (): Promise<string[]> => {
  return new Promise((resolve) => {
    if (!IS_WIN) return resolve(["default"]);
    const child = spawn(PATHS.ffmpeg, ["-list_devices", "true", "-f", "dshow", "-i", "dummy"]);
    let out = "";
    child.stderr.on("data", (d) => (out += d.toString()));
    child.on("close", () => {
      const devices: string[] = [];
      out.split("\n").forEach((line) => {
        if (line.includes('(audio)') && !line.includes('Alternative name')) {
          const match = line.match(/\"(.*)\"/);
          if (match) devices.push(match[1]);
        }
      });
      resolve(devices);
    });
  });
};
