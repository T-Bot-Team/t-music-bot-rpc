const el = (id) => document.getElementById(id);
const overlayOpts = el('overlay_options');
const vizOpts = el('viz_options');

function toggleOverlayOptions() {
    if (el('sys_overlayEnabled').checked) {
        overlayOpts.classList.add('visible');
    } else {
        overlayOpts.classList.remove('visible');
        el('sys_vizEnabled').checked = false;
        toggleVizOptions();
    }
}

function toggleVizOptions() {
    if (el('sys_vizEnabled').checked) {
        vizOpts.classList.add('visible');
    } else {
        vizOpts.classList.remove('visible');
    }
}

async function fetchDevices() {
    try {
        const res = await fetch('/api/devices');
        const devices = await res.json();
        const select = el('sys_audioDevice');
        devices.forEach(d => {
            const opt = document.createElement('option');
            opt.value = d;
            opt.innerText = d;
            select.appendChild(opt);
        });
    } catch (e) {
        console.error("Failed to load devices", e);
    }
}

async function checkStatus() {
    try {
        const res = await fetch('/api/status');
        const status = await res.json();
        const userId = el('sys_userId').value.trim();
        
        if (status.arrpcDetected) {
            if (!userId) {
                el('auth_modal').style.display = 'flex';
            }
        }
    } catch (e) {}
}

el('modal_submit').onclick = () => {
    const val = el('modal_userId').value.trim();
    if (val) {
        el('sys_userId').value = val;
        el('auth_modal').style.display = 'none';
        // Auto-save the user ID if the user clicks Verify
        if (el('sys_code').value.trim().length === 6) {
            saveSettings(new Event('submit'));
        }
    } else {
        alert("User ID is required for arRPC connections.");
    }
};

async function saveSettings(e) {
    if (e && e.preventDefault) e.preventDefault();
    
    const codeInput = el('sys_code').value.trim();
    const userIdInput = el('sys_userId').value.trim();
    
    if (codeInput.length !== 6) {
        alert("Pairing code must be exactly 6 characters.");
        return;
    }

    try {
        const res1 = await fetch('/api/settings'); 
        const state = await res1.json();
        
        state.code = codeInput;
        if (userIdInput) {
            state.userId = userIdInput;
        }
        
        state.overlay.enabled = el('sys_overlayEnabled').checked;
        state.overlay.port = parseInt(el('sys_port').value) || 3000;
        state.overlay.visualizer.enabled = el('sys_vizEnabled').checked;
        
        if (!state.rpc) state.rpc = { swapRpcLines: false };
        state.rpc.swapRpcLines = el('rpc_swapLines').checked;

        if (el('sys_audioDevice').value !== 'default') {
            state.overlay.visualizer.audioDevice = el('sys_audioDevice').value;
        }

        const res2 = await fetch('/api/settings', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                settings: state,
                actions: {
                    desktop_shortcut: el('sys_desktopShortcut').checked,
                    start_menu_shortcut: el('sys_startMenuShortcut').checked,
                    allow_firewall: el('sys_allowFirewall') ? el('sys_allowFirewall').checked : false
                }
            })
        });
        
        if (!res2.ok) throw new Error(await res2.text());
        
        el('status').className = 'status-ok';
        const targetPort = el('sys_port').value || 3000;
        setTimeout(() => window.location.href = `http://127.0.0.1:${targetPort}/settings`, 1000);
    } catch(e) {
        alert("Save Error: " + e.message);
    }
}

fetchDevices();
setInterval(checkStatus, 3000);
checkStatus();
el('setupForm').onsubmit = saveSettings;
