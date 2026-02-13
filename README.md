# T_Music_Bot RPC

A lightweight, cross-platform Discord Rich Presence (RPC) client for the T_Music_Bot. This client displays your current music activity from the bot directly on your Discord profile.

## Features

- **Cross-Platform:** Native support for Windows and Linux.
- **Lightweight:** Minimal CPU and memory usage.
- **Auto-Setup:** Easy pairing process via Discord.
- **GUI & Tray:** Integrated system tray with a clean setup interface.

## Installation

### For Users
Download the latest version for your operating system from the [Releases](https://github.com/TehPig/t-music-bot-rpc/releases) page.

#### Windows
1. Run `T_Music_Bot-win.exe`.
2. Follow the GUI instructions to pair with Discord.
3. The app will minimize to your system tray.

#### Linux
1. Give the binary execution permissions: `chmod +x T_Music_Bot-linux`.
2. Run the binary.
3. **Note:** On some Linux environments, you may need to manually enter your Discord User ID in `settings.json` if automatic detection fails.

---

## Building from Source

If you want to build the project yourself, follow these steps:

### Prerequisites
- [Node.js](https://nodejs.org/) (v18 or higher recommended)
- `npm` (comes with Node.js)

### Steps
1. **Clone the repository:**
   ```bash
   git clone https://github.com/TehPig/t-music-bot-rpc.git
   cd t-music-bot-rpc
   ```

2. **Install dependencies:**
   ```bash
   npm install
   ```

3. **Build the binaries:**
   ```bash
   npm run build
   ```
   This will use the `build.js` script to generate optimized, Brotli-compressed binaries in the `dist/` folder. It also automatically patches the Windows binary to run in GUI mode (hiding the terminal).

---

## Configuration

Settings are stored in `settings.json` in the same directory as the executable:
- `code`: Your 6-digit pairing code.
- `userId`: (Optional) Your Discord User ID, primarily for Linux fallbacks.

## License
[MIT](LICENSE)
