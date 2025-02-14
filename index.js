import RPC from "discord-rpc";
import PWSL from '@performanc/pwsl';
import fs from 'fs'; // Using fs for file system operations
import { v4 as uuidv4 } from 'uuid'; // Import UUID generation

const rpc = new RPC.Client({ transport: "ipc" });
const ws = new PWSL('ws://localhost:8080/ws');
const scopes = ["rpc", "rpc.voice.read"];

// DO NOT CHANGE THESE VALUES OTHERWISE IT WILL NOT WORK
const clientId = "853333281155579975";
const targetUserId = "924669114138644500";

// Function to save tokens to file
function saveTokens(tokens) {
    fs.writeFileSync('tokens.json', JSON.stringify(tokens, null, 2));  // Save tokens to tokens.json
}

// Function to load tokens from file
function loadTokens() {
    if (fs.existsSync('tokens.json')) {
        const data = fs.readFileSync('tokens.json', 'utf8');
        return JSON.parse(data);
    }
    return null;
}

// 🎮 Update RPC Presence
async function updateRPC(data) {
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
    if (data.startTimestamp && data.endTimestamp) {
        setTimeout(async () => {
            const ch = await rpc.getSelectedVoiceChannel();
            ws.send(JSON.stringify({ type: "user_speaking_start", channelId: ch?.id, userId: targetUserId }));
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

    rpc.login({ clientId, scopes, accessToken, redirectUri: "http://localhost/" })
        .then(data => console.log("🔑 RPC Login Successful!", data))
        .catch(err => console.error("❌ RPC Login Failed:", err));
}

// 🎤 Manage Voice Events
async function subscribeToVoiceEvents(client, channelId) {
    const vc = await client.getChannel(channelId);

    await Promise.all([
        client.subscribe("SPEAKING_START", { channel_id: vc.id }),
        client.subscribe("SPEAKING_STOP", { channel_id: vc.id }),
        client.subscribe("VOICE_STATE_CREATE", { channel_id: vc.id })
    ]);

    console.log("🔍 Checking voice channel participants...");

    console.log(vc?.voice_states.map(x => x.user.id));

    if (vc?.voice_states.some(x => x.user.id === targetUserId)) {
        console.log("🎶 Target user found in channel!");
        updateRPC(client, {
            details: `Waiting for tracks`,
            state: `In a voice channel`,
        });
    }

    client.on("SPEAKING_START", (data) => handleSpeakingEvent(data, "start"));
    client.on("SPEAKING_STOP", (data) => handleSpeakingEvent(data, "stop"));

    client.on("VOICE_STATE_CREATE", (data) => {
        if (data.user_id === targetUserId) {
            console.log("🎶 Target user joined the channel!");
            updateRPC(client, {
                details: `Waiting for tracks`,
                state: `In a voice channel`,
            });
        }
    });
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
    // Generate a session ID on the client side
    const sessionId = uuidv4();
    ws.sessionId = sessionId; // Assign sessionId to the WebSocket connection for later use

    // Send an authentication request to the server along with the session ID
    ws.send(JSON.stringify({ type: "initiate_auth", sessionId, access_token: loadTokens().access_token }));
});

// 📩 Handle WebSocket messages
ws.on("message", (data) => {
    const message = JSON.parse(data);

    switch (message.type) {
        case "auth_request":
            console.log("🔑 Received auth request, please authorize using this link.");
            console.log(`🔗 ${message.authUrl}`);
            ws.sessionId = message.sessionId; // Store session ID
            break;

        case "existing_token":
            console.log("🟢 Authentication already valid!");
            initRPC(message.access_token);
            break;

        case "auth_success":
            console.log("🟢 Authentication successful!");
            saveTokens({
                access_token: message.access_token,
                refresh_token: message.refresh_token
            });
            initRPC(message.access_token);
            break;

        case "rpc_update":
            updateRPC(message.data);
            break

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