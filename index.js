import RPC from "discord-rpc";
const rpc = new RPC.Client({ transport: "ipc" });

import PWSL from '@performanc/pwsl';
import { readFileSync, existsSync, writeFileSync } from 'node:fs';

const ws = new PWSL('ws://localhost:8080');
const scopes = ["rpc", "rpc.voice.read"];

// DO NOT CHANGE THESE VALUES OTHERWISE IT WILL NOT WORK
const clientId = "853333281155579975";
const targetUserId = "924669114138644500";

// 🟢 Load tokens from file
function loadTokens() {
    if (existsSync("tokens.json")) return JSON.parse(readFileSync("tokens.json", "utf8"));
    return {};
}

// 💾 Save tokens to file
function saveTokens(tokens) {
    writeFileSync("tokens.json", JSON.stringify(tokens, null, 2));
}

// 🎮 Update RPC Presence
async function updateRPC(client, data) {
    if (!data) return rpc.clearActivity();

    const currentTime = Date.now();
    rpc.setActivity({
        details: data.details,
        state: data.state,
        largeImageKey: data.largeImageKey || null,
        smallImageKey: data.smallImageKey || null,
        smallImageText: data.smallImageText || null,
        type: 2,
        startTimestamp: currentTime - data.startTimestamp || null, // Current time minus the playback position
        endTimestamp: currentTime + (data.endTimestamp - data.startTimestamp) || null, // Current time plus remaining track duration
        instance: false
    });

    // Send a request to the websocket server to make sure that the track is updated
    if(data.startTimestamp && data.endTimestamp){
        setTimeout(async () => {
            const ch = await client.getSelectedVoiceChannel();
            ws.send(JSON.stringify({ type: "user_speaking_start", channelId: ch?.id, userId: targetUserId }))
        }, data.endTimestamp - data.startTimestamp + 50);
    }
}

// 🔌 Initialize Discord RPC
async function initRPC(accessToken) {
    rpc.on("ready", async () => {
        console.log("✅ RPC Connected!");

        const [, vc] = await Promise.all([
            rpc.subscribe("VOICE_CHANNEL_SELECT"),
            rpc.getSelectedVoiceChannel()
        ]);
        // Check if already in a voice channel
        if (vc) subscribeToVoiceEvents(rpc, vc.id);
    });

    rpc.on("VOICE_CHANNEL_SELECT", async (channel) => {
        if (!channel.channel_id) {
            unsubscribeFromVoiceEvents();
            return console.log("🔴 Left voice channel.");
        }

        console.log(`🟢 Joined voice channel ${channel.channel_id}`);
        subscribeToVoiceEvents(rpc, channel.channel_id);
    });

    rpc.login({ clientId, scopes, accessToken })
        .catch(err => console.error("❌ RPC Login Failed:", err));
}

// 🎤 Manage Voice Events
async function subscribeToVoiceEvents(client, channelId) {
    const vc = await client.getChannel(channelId);

    await Promise.all([
        client.subscribe("SPEAKING_START", { channel_id: vc.id }),
        client.subscribe("SPEAKING_STOP", { channel_id: vc.id })
    ]);

    console.log("🔍 Checking voice channel participants...");

    if (vc?.voice_states.some(x => x.user.id === targetUserId)) {
        console.log("🎶 Target user found in channel!");
        updateRPC(client, {
            details: `Waiting for tracks`,
            state: `In a voice channel`,
        });
    }

    client.on("SPEAKING_START", (data) => handleSpeakingEvent(data, "start"));
    client.on("SPEAKING_STOP", (data) => handleSpeakingEvent(data, "stop"));
}

// 🗣️ Handle Speaking Events
async function handleSpeakingEvent(data, type) {
    if (data.user_id === targetUserId) {
        ws.send(JSON.stringify({
            type: `user_speaking_${type}`,
            channelId: data.channel_id,
            userId: targetUserId
        }));
    }
}

// ❌ Unsubscribe from Voice Events
async function unsubscribeFromVoiceEvents() {
    rpc.removeAllListeners("SPEAKING_START");
    rpc.removeAllListeners("SPEAKING_STOP");
}

// 🔌 WebSocket connection opens
ws.on("open", () => {
    console.log("✅ Connected to WebSocket server.");
    ws.send(JSON.stringify({ type: "auth" }));
});

// 📩 Handle WebSocket messages
ws.on("message", (data) => {
    const message = JSON.parse(data);

    switch (message.type) {
        case "new_access_token":
            console.log("🔑 Received new access token");
            saveTokens({ access_token: message.token });
            initRPC(message.token);
            break;

        case "rpc_update":
            console.log("🔄 Received RPC update data");
            updateRPC(message.rpc_data);
            break;

        case "auth_success":
            console.log(`🟢 Authentication successful!`);
            initRPC(loadTokens().access_token);
            break;

        case "auth_error":
        case "refresh_error":
        case "error":
            console.error(`❌ ${message.type}:`, message.message);
            break;

        default:
            console.warn("⚠️ Unhandled WebSocket message type:", message.type);
    }
});

ws.on("error", (err) => console.error("❌ WebSocket error:", err));
ws.on("close", () => console.log("🔴 Disconnected from WebSocket server."));

setInterval(async () => {
    console.log(`Memory Usage: ${(process.memoryUsage().heapUsed / 1024 / 1024).toFixed(2)} MB`)
}, 60000)