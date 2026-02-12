const fs = require('fs');
const path = require('path');

const exePath = path.join(__dirname, 'dist', 't-music-rpc-win.exe');

if (!fs.existsSync(exePath)) {
    console.error("Executable not found for patching!");
    process.exit(1);
}

const buffer = fs.readFileSync(exePath);

// The PE header location is found at offset 0x3C
const peOffset = buffer.readUInt32LE(0x3C);

// The Subsystem is at peOffset + 0x5C (for PE32+) or peOffset + 0x5C (for PE32)
// For most Node/Pkg builds, it's at peOffset + 92
const subsystemOffset = peOffset + 92;

// Subsystem values: 
// 2 = Windows GUI (No terminal)
// 3 = Windows Console (Terminal)
const currentSubsystem = buffer.readUInt16LE(subsystemOffset);

if (currentSubsystem === 3) {
    console.log("Found Console Subsystem. Patching to GUI Subsystem...");
    buffer.writeUInt16LE(2, subsystemOffset);
    fs.writeFileSync(exePath, buffer);
    console.log("✅ Executable patched successfully! It will now launch with NO terminal.");
} else if (currentSubsystem === 2) {
    console.log("Executable is already patched for GUI Subsystem.");
} else {
    console.log("Unknown subsystem: " + currentSubsystem);
}
