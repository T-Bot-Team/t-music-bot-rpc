const { execSync } = require('child_process');
const fs = require('fs');
const path = require("path");

async function build() {
    console.log("🚀 Starting Build V77...");
    if (fs.existsSync('dist')) {
        fs.rmSync('dist', { recursive: true, force: true });
    }
    fs.mkdirSync('dist');

    console.log("📦 Packaging binaries (Brotli compression)...");
    // Standard pkg build with Brotli and icon
    execSync('npx pkg . --targets node18-win-x64,node18-linux-x64 --compress Brotli --icon icon.ico --out-path dist', { stdio: 'inherit' });

    const files = fs.readdirSync('dist');
    
    // Process Windows
    let winFile = files.find(f => f.includes('win') && f.endsWith('.exe'));
    if (winFile) {
        const target = path.join('dist', 'T_Music_Bot-win.exe');
        fs.renameSync(path.join('dist', winFile), target);
        
        try {
            console.log("🛠️ Hiding Windows Console...");
            const buffer = fs.readFileSync(target);
            const peOffset = buffer.readUInt32LE(0x3C);
            // Verify PE signature
            if (buffer.readUInt32BE(peOffset) === 0x50450000) {
                const subsystemOffset = peOffset + 92;
                // Only write if it's currently 3 (Console)
                if (buffer[subsystemOffset] === 3) {
                    buffer[subsystemOffset] = 2; // 2 = GUI
                    fs.writeFileSync(target, buffer);
                    console.log("✅ Console hidden.");
                } else {
                    console.log("ℹ️ Subsystem already patched.");
                }
            }
        } catch (e) {
            console.error("❌ Subsystem patch failed: " + e.message);
        }
        console.log("✅ Windows binary ready.");
    }

    // Process Linux
    let linuxFile = files.find(f => f.includes('linux'));
    if (linuxFile) {
        fs.renameSync(path.join('dist', linuxFile), path.join('dist', 'T_Music_Bot-linux'));
        console.log("✅ Linux binary ready.");
    }
    console.log("\n✨ Build Complete!");
}

build().catch(console.error);
