const RPC = require("discord-rpc");
const client = new RPC.Client({ transport: "ipc" });

client.on("ready", () => {
  console.log("✅ Connected to Discord!");

  // 🧪 MANUALLY TWEAK THESE VALUES TO TEST SETTINGS
  const activity = {
    details: "Sex Whales & Fraxo - Dead To Me",
    state: "Trap Nation",
    // On arRPC/WebCord, this field changes the "Listening to..." text:
    name: "Trap Nation",
    // Note: Standard Discord DOES NOT support 'detailsUrl' natively as text.
    // It only supports clickable links via buttons:
    buttons: [
      {
        label: "test",
        url: "https://google.com",
      },
    ],
    largeImageKey: "music_play",
    smallImageKey: "music_play",
    instance: false,
  };

  console.log("🚀 Sending Payload to Discord...");
  client.setActivity(activity).catch((err) => console.error("❌ Error:", err));
});

// Use your Client ID here
client.login({ clientId: "1045800378228281345" }).catch(console.error);