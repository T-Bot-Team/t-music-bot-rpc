import RPC from "@t_bot-team/discord-rpc";
import WebSocket from "ws";
import fs from "fs";
import os from "os";
import { WS_URL, PATHS, APP_VERSION, GLOBAL_STATE, TrackUpdate } from "../utils/constants";
import * as ui from "../lib/ui";

let rpcClient: any = null;
let wsClient: WebSocket | null = null;
let heartbeatInterval: NodeJS.Timeout | null = null;
let activityTimeout: NodeJS.Timeout | null = null;
let lastSuccessTime = 0;
let lastMessageTime = Date.now();
let queuedData: TrackUpdate | null = null;
let isPrompting = false;
let loginTimeout: NodeJS.Timeout | null = null;

const state = { ws: "Disconnected", rpc: "Disconnected" };

async function destroyRPC(): Promise<void> {
  if (!rpcClient) return;
  const client = rpcClient;
  rpcClient = null;
  try {
    await Promise.race([
      (async () => { try { await client.destroy(); } catch (e) {} })(),
      new Promise((r) => setTimeout(r, 2000)),
    ]);
  } catch (e) {}
}

export const connect = (tray: any, onUpdate: (data: TrackUpdate) => void): Promise<{ success: boolean }> => {
  if (wsClient) try { wsClient.terminate(); } catch (e) {}
  if (heartbeatInterval) clearInterval(heartbeatInterval);
  if (loginTimeout) clearTimeout(loginTimeout);

  const settings = GLOBAL_STATE.settings;

  return new Promise((resolve) => {
    let isResolved = false;
    const safeResolve = (val: { success: boolean }) => { if (isResolved) return; isResolved = true; resolve(val); };

    state.ws = "Connecting...";
    tray.updateStatus(state.ws, state.rpc);

    wsClient = new WebSocket(WS_URL, {
      headers: { "User-Agent": `T_Music_Bot-RPC/${APP_VERSION} (${os.platform()})`, Origin: WS_URL },
    });

    wsClient.on("open", () => {
      state.ws = "Connected";
      tray.updateStatus(state.ws, state.rpc);
      wsClient?.send(JSON.stringify({ type: "connect" }));
      heartbeatInterval = setInterval(() => {
        if (wsClient && wsClient.readyState === WebSocket.OPEN) wsClient.ping();
      }, 20000);
    });

    wsClient.on("message", async (raw) => {
      try {
        const m = JSON.parse(raw.toString());
        if (m.type === "rpc_update" || m.type === "authenticated") lastMessageTime = Date.now();

        if (m.type === "connect") {
          const setupWsAuth = async (): Promise<void> => {
            let currentCode = settings?.code;
            if (!currentCode || !/^\d{6}$/.test(currentCode)) {
              if (isPrompting) return;
              isPrompting = true;
              currentCode = await ui.getPairingCode();
              isPrompting = false;
              if (!currentCode || currentCode === "CANCELLED") return process.exit(0);
              if (settings) {
                settings.code = currentCode;
                fs.writeFileSync(PATHS.settings, JSON.stringify(settings, null, 2));
              }
            }
            const authId = settings?.userId || rpcClient?.user?.id;
            if (!authId) {
              setTimeout(setupWsAuth, 2000);
              return;
            }
            wsClient?.send(JSON.stringify({ type: "auth", userId: authId, code: currentCode }));
          };

          if (rpcClient && state.rpc === "Connected") setupWsAuth();
          else {
            if (rpcClient) await destroyRPC();
            rpcClient = new (RPC as any).Client({ transport: "ipc" });
            rpcClient.on("ready", () => {
              state.rpc = "Connected";
              tray.updateStatus(state.ws, state.rpc);
              setupWsAuth();
            });
            rpcClient.on("disconnected", () => {
              state.rpc = "Disconnected";
              tray.updateStatus(state.ws, state.rpc);
              if (wsClient) wsClient.terminate();
            });
            const tryLogin = () => {
              if (!rpcClient || state.ws !== "Connected") return;
              rpcClient.login({ clientId: m.clientId }).catch(() => {
                state.rpc = "Discord Not Found";
                tray.updateStatus(state.ws, state.rpc);
                loginTimeout = setTimeout(tryLogin, 15000);
              });
            };
            tryLogin();
          }
        }
        if (m.type === "authenticated") {
          const authId = settings?.userId || rpcClient?.user?.id;
          if (authId) wsClient?.send(JSON.stringify({ type: "request_update", userId: authId }));
          safeResolve({ success: true });
        }
        if (m.type === "rpc_update") {
            updateActivity(m.data, onUpdate);
        }
      } catch (e) {}
    });

    wsClient.on("close", () => { state.ws = "Disconnected"; tray.updateStatus(state.ws, state.rpc); safeResolve({ success: false }); });
    wsClient.on("error", () => safeResolve({ success: false }));
  });
};

export const updateActivity = async (data: TrackUpdate, onUpdate: (data: TrackUpdate) => void): Promise<void> => {
  if (!data) return;

  // 1. FORMATTING
  const formattedData = { ...data };
  if (formattedData.details && formattedData.state) {
    const artistName = formattedData.state;
    const escapedArtist = artistName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const artistRegex = new RegExp(`^${escapedArtist}\\s*[-:|~]\\s*`, "i");
    formattedData.details = formattedData.details.replace(artistRegex, "");
    const junkPatterns = [/\s*[\[\(\]]?(?:Copyright Free|Official Video|Lyrics|Audio|Video|Music Video|Official Audio)[^\]\)]*[\]\)]?\s*/gi];
    junkPatterns.forEach((p) => formattedData.details = formattedData.details!.replace(p, " "));
    formattedData.details = formattedData.details.replace(/\s\s+/g, " ").trim();
  }

  // 2. INSTANT OVERLAY BROADCAST
  if (onUpdate) onUpdate(formattedData);

  // 3. RPC RATE LIMIT LOGIC
  if (!rpcClient || state.rpc !== "Connected") return;
  
  const now = Date.now();
  const COOLDOWN_MS = 15500;
  
  if (now - lastSuccessTime < COOLDOWN_MS) {
    queuedData = data;
    if (activityTimeout) return;
    activityTimeout = setTimeout(() => {
        activityTimeout = null;
        const next = queuedData;
        queuedData = null;
        if (next) updateActivity(next, onUpdate);
    }, COOLDOWN_MS - (now - lastSuccessTime));
    return;
  }

  lastSuccessTime = now;
  const isClear = Object.keys(data).length === 0;
  const activityData = isClear ? null : { ...formattedData };
  
  if (activityData && activityData.paused) {
    delete activityData.startTimestamp;
    delete activityData.endTimestamp;
    delete activityData.paused;
  }
  
  const activity = isClear ? null : { ...activityData, type: 2, instance: false };
  try {
    if (isClear) await rpcClient.clearActivity();
    else await rpcClient.setActivity(activity);
  } catch (e) {}
};

export const waitForDisconnect = (): Promise<void> => {
  return new Promise((r) => {
    if (!wsClient || wsClient.readyState !== WebSocket.OPEN) return r();
    wsClient.once("close", r);
  });
};

export const shutdown = async (): Promise<void> => {
  if (heartbeatInterval) clearInterval(heartbeatInterval);
  if (rpcClient) { await rpcClient.clearActivity().catch(() => {}); await destroyRPC(); }
  if (wsClient) wsClient.terminate();
};
