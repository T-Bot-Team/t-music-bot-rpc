import RPC from "@t_bot-team/discord-rpc";
import PWSL from "@performanc/pwsl";
import config from "./config.json" assert { type: "json" };

const rpc = new RPC.Client({ transport: "ipc" });
let ws = null;
let reconnectDelay = 1000; // Start at 1 second
const maxDelay = 30000; // Max 30 seconds

async function updateRPC(data) {
  if (!data) return rpc.clearActivity();

  const currentTime = Date.now();
  const endTimestamp =
    data.endTimestamp !== undefined
      ? currentTime + (data.endTimestamp - data.startTimestamp)
      : undefined;

  rpc.setActivity({
    details: data.details,
    state: data.state,
    largeImageKey: data.largeImageKey || null,
    smallImageKey: data.smallImageKey || null,
    smallImageText: data.smallImageText || null,
    type: 2,
    startTimestamp: currentTime - data.startTimestamp,
    endTimestamp,
    instance: false,
  });
}

// Properly close existing WebSocket before creating a new one
function connect() {
  if (ws) {
    console.log("🔴 // Closing existing WebSocket connection...");
    ws.removeAllListeners(); // Remove old event listeners
    ws.close(); // Close previous WebSocket
    ws = null;
  }

  ws = new PWSL("ws://localhost:8080/ws");

  ws.on("open", () => {
    console.log("✅ >> Connected to WebSocket server.");
    reconnectDelay = 1000; // Reset reconnect delay on successful connection
    ws.send(JSON.stringify({ type: "connect" }));
  });

  ws.on("message", (data) => {
    if (config.debug_mode) console.log("📩 Received data", data);
    const message = JSON.parse(data);

    switch (message.type) {
      case "connect":
        console.log("🔗 >> WebSocket Connection Established.");
        break;

      case "rpc_update":
        updateRPC(message.data);
        break;

      case "error":
        console.error(`❌ // WebSocket Error: ${message.message}`);
        break;

      default:
        console.warn("⚠️ // Unhandled WebSocket message type:", message.type);
    }
  });

  ws.on("error", (err) => {
    console.error(`❌ // WebSocket error: ${config.debug_mode ? err : err.code}`);
  });

  ws.on("close", () => {
    console.log("🔴 // Disconnected from WebSocket server.");
    
    // Apply exponential backoff with jitter
    const jitter = Math.random() * 0.3 * reconnectDelay;
    const delay = Math.min(reconnectDelay + jitter, maxDelay);
    console.log(`⏳ // Attempting reconnection in ${Math.floor(delay)}ms...`);

    setTimeout(connect, delay);
    reconnectDelay = Math.min(reconnectDelay * 2, maxDelay);
  });
}

// Start WebSocket connection
connect();

// Monitor RAM Usage
setInterval(() => {
  console.log(`💻 Memory Usage: ${(process.memoryUsage().heapUsed / 1024 / 1024).toFixed(2)} MB`);
}, 15000);