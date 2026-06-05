# Audio Routing & Isolation Guide

There are many methods and programs available to capture audio. One of the easiest and most common methods is using a virtual cable. This guide includes step-by-step setup instructions for three major platforms: **Windows**, **Linux**, and **macOS**.

---

# Table of Contents
1. [Using Voicemeeter Mixer (Windows-only)](#1-using-voicemeeter-mixer-windows-only)
2. [Using VB-Audio Virtual Cable (Windows / macOS - Mixerless Setup)](#2-using-vb-audio-virtual-cable-windows--macos---mixerless-setup)
3. [Linux: Using PipeWire / PulseAudio Virtual Sinks](#3-linux-using-pipewire--pulseaudio-virtual-sinks)
4. [⚠️ Crucial Warning for Discord voice calls](#️-crucial-warning-for-discord-voice-calls)

---

## 1. Using Voicemeeter Mixer (Windows-only)

Voicemeeter is a virtual audio mixer that routes different applications to different audio buses. There are 3 versions of Voicemeeter depending on the amount of mixing inputs and outputs you require:
*   [Voicemeeter (Regular)](https://vb-audio.com/Voicemeeter/index.htm) (A physical outputs, B virtual outputs)
*   [Voicemeeter Banana](https://vb-audio.com/Voicemeeter/banana.htm) (A1/A2/A3 physical outputs, B1/B2 virtual outputs)
*   [Voicemeeter Potato](https://vb-audio.com/Voicemeeter/potato.htm) (A1-A5 physical outputs, B1/B2/B3 virtual outputs)

### How Routing Works: Voicemeeter UI vs. Windows Applications
To configure routing, it is important to distinguish between what you see inside the **Voicemeeter Program** and what you select inside your **Windows Application settings**:

1.  **Inside the Voicemeeter Program**:
    *   **"A" Buttons (A1/A2/A3...)**: Route the sound of that strip to a physical hardware output bus (like your headphones).
    *   **"B" Buttons (B1/B2/B3...)**: Route the sound of that strip to a virtual output bus (independent of physical outputs, meant for recording/streaming).
2.  **Inside Windows / Application Settings**:
    *   **To send audio IN to Voicemeeter**: Applications select a playback device like `Voicemeeter Input` (or `Voicemeeter In 1` through `Voicemeeter In 5`).
    *   **To capture audio OUT from Voicemeeter**: Applications (like OBS or our RPC Visualizer) select a recording/capture device like `Voicemeeter Out A1` through `Voicemeeter Out B3`.

### Bus and Device Mappings
| Voicemeeter Edition | Hardware Output Buses (To Hear) | Virtual Output Buses (To Capture) | Recording Device Name in Windows |
| :--- | :--- | :--- | :--- |
| **Voicemeeter (Regular)** | **A** | **B** | `VoiceMeeter Output (VB-Audio VoiceMeeter VAIO)` |
| **Voicemeeter Banana** | **A1**, **A2**, **A3** | **B1** | `Voicemeeter Out B1 (VB-Audio VoiceMeeter VAIO)` |
| | | **B2** | `Voicemeeter Out B2 (VB-Audio VoiceMeeter VAIO)` |
| **Voicemeeter Potato** | **A1** through **A5** | **B1** | `Voicemeeter Out B1 (VB-Audio VoiceMeeter VAIO)` |
| | | **B2** | `Voicemeeter Out B2 (VB-Audio VoiceMeeter VAIO)` |
| | | **B3** | `Voicemeeter Out B3 (VB-Audio VoiceMeeter VAIO)` |

### Installing the software
Click on the "Download" button once you get to the software's website. Keep in mind that this will download a compressed (ZIP) file with all the files required for the software to function.

![[Pasted image 20260604195825.png|341]]
###### Example of the Download button in the software's website.

### Configuration Steps
1. Set your **Windows Default Output Device** to **Voicemeeter Input (VB-Audio Voicemeeter VAIO)** through Windows' sound settings.
2. In Voicemeeter, select your physical headphones/speakers under the **A1** output hardware button (top-right).
3. Ensure **A** (or A1 or A2) is turned **ON** for the **Virtual Input** strip. This sends your default system sounds and game audio to your physical headphones.

> [!TIP]
> Always prefer WDM outputs over MME to have the lowest latency.

### Routing Logic (Separating Music from System Sounds)
To capture only music in your visualizer while hearing everything together in your headphones, route your audio using one of the two methods below:

#### Method A: Direct Virtual Routing (For Banana / Potato — No Cables Needed)
Banana and Potato have multiple virtual inputs, allowing you to isolate the music completely inside Voicemeeter without installing extra virtual cables:

1. **Route System Sounds/Games/Discord**:
   * Set your Windows default playback device to **Voicemeeter Input (VB-Audio Voicemeeter VAIO)**.
   * In the Voicemeeter program, go to the first virtual input strip (**VAIO**). Turn **A1 ON** (so you can hear it), but keep **B1 and B2 OFF** (so the visualizer blocks it).
2. **Route Music**:
   * Open your Windows **Sound settings -> Volume Mixer**. Set your music player app (e.g., Spotify, Chrome) to output to **Voicemeeter AUX Input (VB-Audio Voicemeeter VAIO)**.
   * In the Voicemeeter program, go to the second virtual input strip (**AUX**). Turn **A1 ON** (so you can hear the music) and turn **B2 ON** (to send it to the virtual output).
3. **Bind the RPC Visualizer**:
   * In the RPC settings dashboard under **Visualizer Settings**, set the **Audio Device** to **Voicemeeter Out B2 (VB-Audio Voicemeeter VAIO)**.

---

#### Method B: Cable Routing (For Regular Voicemeeter / Alternative)
Since regular Voicemeeter only has a single virtual input channel, you must use a virtual audio cable (downloadable from [VB-Cable Website](https://vb-audio.com/Cable/)) to separate the sounds:

1. **Route System Sounds/Games/Discord**:
   * Set your Windows default playback device to **Voicemeeter Input (VB-Audio Voicemeeter VAIO)**.
   * In the Voicemeeter program, go to the virtual input strip. Turn **A ON** (so you can hear it), but keep **B OFF** (so the visualizer blocks it).
2. **Route Music**:
   * Open your Windows **Sound settings -> Volume Mixer**. Set your music player app (e.g., Spotify, Chrome) to output to **CABLE Input (VB-Audio Virtual Cable)**.
   * In the Voicemeeter program, set **Hardware Input 1** (top-left dropdown) to **CABLE Output (VB-Audio Virtual Cable)**.
   * On the **Hardware Input 1** strip, turn **A ON** (so you can hear the music) and turn **B ON** (to send it to the virtual output).
3. **Bind the RPC Visualizer**:
   * In the RPC settings dashboard under **Visualizer Settings**, set the **Audio Device** to **VoiceMeeter Output (VB-Audio VoiceMeeter VAIO)** (which captures the **B** virtual bus).

---

## 2. Using VB-Audio Virtual Cable (Windows / macOS - Mixerless Setup)

This section is for users who want to isolate their music using **only** a virtual audio cable, without installing the full Voicemeeter mixer program. You can download the driver from the [VB-Cable Website](https://vb-audio.com/Cable/).

Alternatively, macOS users can also download and use [BlackHole 2ch](https://existential.audio/blackhole/) for the exact same purpose.

### Windows Configuration
1. Open **Sound Settings -> Volume Mixer**, locate your music player app, and set its Output Device to **CABLE Input (VB-Audio Virtual Cable)**.
2. Open your Windows **Sound Control Panel**, head to the **Recording** tab, and double-click **CABLE Output**. Under the **Listen** tab, check the box for **"Listen to this device"** and select your physical headphones (so you can hear the music).
3. In the RPC settings dashboard under **Visualizer Settings**, set the **Audio Device** to **CABLE Output (VB-Audio Virtual Cable)**.

### macOS Configuration
1. Route your music player app to **CABLE Input** (or **BlackHole 2ch**).
2. Open the **Audio MIDI Setup** app (found in Applications > Utilities).
3. Click the **+** button in the bottom-left corner and select **Create Multi-Output Device**.
4. Check the box next to your **Built-in Output** (or headphones) and **BlackHole 2ch** (or **CABLE Input**). Set the **Master Device** to your headphones.
5. Set your macOS system sound output to the newly created **Multi-Output Device**.
6. In your RPC client dashboard under **Visualizer Settings**, set the **Audio Device** to **BlackHole 2ch** (or **CABLE Output**).

---

## 3. Linux: Using PipeWire / PulseAudio Virtual Sinks

Linux users can create virtual loopback sinks dynamically using standard terminal tools.

### Configuration Steps
1. Create a virtual null-sink named "VisualizerSink" by running:
   ```bash
   pactl load-module module-null-sink sink_name=visualizer_sink sink_properties=device.description="VisualizerSink"
   ```
2. Open **pavucontrol** (PulseAudio Volume Control), head to the **Playback** tab, locate your music app, and change its output routing to **VisualizerSink**.
3. To listen to the music in your own headphones at the same time, create a loopback link:
   ```bash
   pactl load-module module-loopback source=visualizer_sink.monitor sink=YOUR_PHYSICAL_HEADPHONES_SINK_NAME
   ```
4. Set the RPC visualizer **Audio Device** option to **VisualizerSink** (or `visualizer_sink.monitor`).

---

## ⚠️ Crucial Warning for Discord voice calls

> [!WARNING]
> The visualizer captures **all** audio passing through your selected capture device. 
> If you route your default Discord client's output to the Virtual Cable while chatting, the visualizer will react and bounce to your friends' voices when they speak, which ruins the music visualizer.

### How to Isolate the Bot's Music from Friends' Voices

If you want to chat with your friends on Discord *and* keep the visualizer reacting only to the music bot, use one of the following methods:

#### Method A: Multi-Instance Separation (Recommended)
You can connect to the voice channel twice to split the audio:
1. Join the voice channel in your **Discord Desktop App** as your main account (to talk and listen to your friends normally). Ensure your desktop app output goes directly to your physical headphones.
2. Open a **Web Browser Tab** (or a secondary Discord client), log into a secondary alt-account (or use a browser session), and join the same voice channel.
3. Route your **Browser Output** to the **Virtual Cable** (via Windows Volume Mixer).
4. In the browser voice channel, right-click and **Mute (0% volume)** all human participants, leaving **only the Music Bot unmuted**. 
5. Set your visualizer to capture the **Virtual Cable**.

#### Method B: Local Volume Adjustments
If you do not want to use a second account, you can route your main Discord app to the Virtual Cable, but you must manually right-click and **mute or turn down to 0%** every human friend in the voice channel, leaving only the bot active. *Note: This means you will not be able to hear your friends talk at all.*

---

> [!NOTE]
> If you find something that could be outdated or not working while following the required steps, please open an Issue or a Pull Request to help update the guide.
