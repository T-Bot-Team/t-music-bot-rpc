import RPC from "@t_bot-team/discord-rpc";
import PWSL from '@performanc/pwsl';
import { readFileSync, writeFileSync, existsSync } from 'fs'; // Using fs for file system operations
import { v4 as uuidv4 } from 'uuid'; // Import UUID generation

const rpc = new RPC.Client({ transport: "ipc" });
//const ws = new PWSL('ws://192.168.1.24:8080/ws');
const ws = new PWSL('ws://localhost:8080/ws');
const scopes = ["rpc", "rpc.voice.read"];

const state = new Map();

// DO NOT CHANGE THESE VALUES OTHERWISE IT WILL NOT WORK
let targetUserId = "924669114138644500";

// Save tokens to file
function saveTokens(tokens) {
    writeFileSync('tokens.json', JSON.stringify(tokens, null, 2));
}

// Load tokens from file
function loadTokens() {
    if (existsSync('tokens.json')) {
        const data = readFileSync('tokens.json', 'utf8');
        return JSON.parse(data);
    }
    return null;
}

// Update RPC Presence
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
        startTimestamp: currentTime - data.startTimestamp || null,
        endTimestamp: currentTime + (data.endTimestamp - data.startTimestamp) || null,
        instance: false
    });

    // Send a request to the websocket server to make sure that the track is updated
    if (data.startTimestamp && data.endTimestamp) {
        setTimeout(async () => {
            const ch = await rpc.getSelectedChannel();
            ws.send(JSON.stringify({ type: "user_speaking_start", channelId: ch?.id, userId: targetUserId }));
        }, data.endTimestamp - data.startTimestamp);
    }
}

// Initialize Discord RPC
async function initRPC(clientId, accessToken) {
    rpc.on("ready", async () => {
        console.log("✅ >> RPC Connected!");

        const [, vc] = await Promise.all([
            rpc.subscribe("VOICE_CHANNEL_SELECT"),
            rpc.getSelectedChannel()
        ]);
        // Check if already in a voice channel
        if (vc) subscribeToVoiceEvents(vc.id);
    });

    rpc.on("VOICE_CHANNEL_SELECT", async (channel) => {
        if (!channel.channel_id) {
            unsubscribeFromVoiceEvents();
            return console.log("🔴 // Left voice channel.");
        }

        console.log(`🟢 >> Joined voice channel ${channel.channel_id}`);
        subscribeToVoiceEvents(channel.channel_id);
    });

    rpc.login({ clientId: "421978090823090186", scopes, accessToken, redirectUri: "http://192.168.1.24:8080/callback" })
        .catch(err => {
            if(err.code == 4009) return console.error("❌ // Authentication was performed using a different Discord account than the one used in the Desktop application.")
            console.error("❌ // RPC Login Failed:", err)
        });
}

// Manage Voice Events
async function subscribeToVoiceEvents(channelId) {
    const vc = await rpc.getChannel(channelId);

    console.log("Subscribing to voice events")

    await Promise.all([
        rpc.subscribe("SPEAKING_START", { channel_id: vc.id }),
        rpc.subscribe("SPEAKING_STOP", { channel_id: vc.id }),
        rpc.subscribe("VOICE_STATE_CREATE", { channel_id: vc.id })
    ]);

    console.log("🔍 || Checking voice channel participants...");
    console.log(vc?.voice_states.map(x => x.user.id));

    if (vc?.voice_states.some(x => x.user.id === targetUserId)) {
        console.log("🎶 || Target user found in channel!");
        handleSpeakingEvent({ channel_id: vc.id, user_id: targetUserId }, "start");
    }

    rpc.on("SPEAKING_START", (data) => handleSpeakingEvent(data, "start"));
    rpc.on("SPEAKING_STOP", (data) => handleSpeakingEvent(data, "stop"));

    rpc.on("VOICE_STATE_CREATE", (data) => {
        if (data.user.id === targetUserId) {
            console.log("🎶 || Target user joined the channel!");
            updateRPC({
                details: `Waiting for tracks`,
                state: `In a voice channel`,
            });
        }
    });
}

// Handle Speaking Events
async function handleSpeakingEvent(data, type) {
    if (data.user_id === targetUserId) {
        if(type === "stop" && state.get("playing") == false) return;
        if(type === "start" && state.get("playing") == true) return;

        if(type === "stop") state.set("playing", false);
        if(type === "start") state.set("playing", true);

        console.log(`Track is ${type == "start" ? "playing" : "NOT playing"}`)

        ws.send(JSON.stringify({
            type: `user_speaking_${type}`,
            channelId: data.channel_id,
            userId: targetUserId
        }));
    }
}

// Unsubscribe from Voice Events
async function unsubscribeFromVoiceEvents() {
    rpc.removeAllListeners("SPEAKING_START");
    rpc.removeAllListeners("SPEAKING_STOP");
    rpc.removeAllListeners("VOICE_STATE_CREATE")
}

// WebSocket connection opens
ws.on("open", () => {
    console.log("✅ >> Connected to WebSocket server.");
    
    const sessionId = uuidv4();
    ws.sessionId = sessionId;

    ws.send(JSON.stringify({ type: "initiate_auth", sessionId, access_token: loadTokens()?.access_token }));
});

// Handle WebSocket messages
ws.on("message", (data) => {
    const message = JSON.parse(data);

    switch (message.type) {
        case "auth_request":
            console.log("🔑 || Received auth request, please authorize using this link.");
            console.log(`🔗 >> ${message.authUrl}`);
            ws.sessionId = message.sessionId;
            break;

        case "existing_token":
            console.log("🟢 >> Authentication already valid!");
            //targetUserId = message.clientId;
            initRPC(message.access_token);
            break;

        case "auth_success":
            console.log("🟢 >> Authentication successful!");
            saveTokens({
                access_token: message.access_token,
                refresh_token: message.refresh_token
            });
            //targetUserId = message.clientId;
            initRPC(message.access_token);
            break;

        case "rpc_update":
            updateRPC(message.data);
            break

        case "auth_error":
        case "refresh_error":
        case "error":
            console.error(`❌ // ${message.type}:`, message.message);
            break;

        default:
            console.warn("⚠️ // Unhandled WebSocket message type:", message.type);
    }
});

ws.on("error", (err) => console.error("❌ // WebSocket error:", err));
ws.on("close", () => console.log("🔴 // Disconnected from WebSocket server."));