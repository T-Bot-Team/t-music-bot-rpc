const form = document.getElementById('settingsForm'), status = document.getElementById('status');
const pickers = {};

const stripAlpha = (colorStr) => {
    if (!colorStr) return '#ffffff';
    colorStr = colorStr.trim();
    if (colorStr.toLowerCase() === 'transparent') return 'transparent';
    if (colorStr.startsWith('#')) {
        if (colorStr.length === 9) return colorStr.substring(0, 7);
        return colorStr;
    }
    if (colorStr.startsWith('rgba') || colorStr.startsWith('rgb')) {
        const parts = colorStr.match(/[\d.]+/g);
        if (parts && parts.length >= 3) {
            const r = parseInt(parts[0]).toString(16).padStart(2, '0');
            const g = parseInt(parts[1]).toString(16).padStart(2, '0');
            const b = parseInt(parts[2]).toString(16).padStart(2, '0');
            return `#${r}${g}${b}`;
        }
    }
    if (colorStr.startsWith('hsla') || colorStr.startsWith('hsl')) {
        const parts = colorStr.match(/[\d.]+/g);
        if (parts && parts.length >= 3) return `hsl(${parts[0]}, ${parts[1]}%, ${parts[2]}%)`;
    }
    return colorStr;
};

const createPicker = (id, defaultColor) => {
    const elPicker = el(id);
    if (!elPicker) return null;
    const strippedColor = stripAlpha(defaultColor);
    if (pickers[id]) {
        pickers[id].setColor(strippedColor || '#7cf6ff');
        pickers[id].applyColor(); // Force visual update in case silent fails
        return pickers[id];
    }
    const p = Pickr.create({
        el: `#${id}`, theme: 'nano', default: strippedColor || '#7cf6ff',
        components: { preview: true, opacity: false, hue: true, interaction: { input: true, hex: true, rgba: true, hsla: true, hsva: true, cmyk: true, save: true } }
    });
    p.on('change', (color) => { p.applyColor(true); checkActivePreset(); });
    p.on('save', (color) => { p.hide(); });
    
    // MutationObserver to remove "A" from format button since opacity is disabled
    setTimeout(() => {
        const typeBtn = p.getRoot().interaction.type;
        if (typeBtn) {
            const fixFormat = () => {
                let val = typeBtn.value;
                if (val && val.endsWith('A')) { typeBtn.value = val.slice(0, -1); }
            };
            fixFormat();
            new MutationObserver(fixFormat).observe(typeBtn, { attributes: true, attributeFilter: ['value'] });
        }
    }, 100);
    
    pickers[id] = p;
    return p;
};

const el = (id) => document.getElementById(id);

const setSelectValue = (id, val) => {
    const e = el(id);
    if (!e) return;
    e.value = val;
    if (e.tagName === 'SELECT') {
        Array.from(e.options).forEach(opt => {
            opt.selected = (opt.value === val);
        });
        e.dispatchEvent(new Event('change', { bubbles: true }));
        e.dispatchEvent(new Event('input', { bubbles: true }));
    }
};

let initialPort = 3000, g_audioDevice = "default", g_currentSettings = null, g_initialUIState = null, g_isLoadingSettings = false;

const PRESETS = {
    'balanced': { samples: 8192, smooth: 12 },
    'smooth':   { samples: 16384, smooth: 24 },
    'reactive': { samples: 4096, smooth: 6 }
};

function checkActivePreset() {
    const getV = (id) => parseInt(el(id)?.value);
    const current = {
        samples: getV('viz_samples'),
        smooth: getV('viz_smoothing')
    };

    document.querySelectorAll('.preset-btn').forEach(btn => {
        const type = btn.dataset.preset;
        const p = PRESETS[type];
        if (!p) return;
        const isMatch = p.samples === current.samples && p.smooth === current.smooth;
        btn.classList.toggle('active', isMatch);
    });
}

function deepMerge(target, source) {
    for (const key in source) {
        if (source[key] instanceof Object && key in target && !Array.isArray(source[key])) {
            Object.assign(source[key], deepMerge(target[key], source[key]));
        } else {
            target[key] = source[key];
        }
    }
    return target;
}

function updatePickerVisibility() {
    const rows = ['txt', 'brd', 'acc', 'viz'];
    rows.forEach(r => {
        const isStaticEl = el(`${r}_static`);
        if (!isStaticEl) return;
        const isStatic = isStaticEl.checked;
        const botContainer = el(`${r}_bot_container`);
        if (botContainer) botContainer.style.display = isStatic ? 'none' : 'block';
        const row = el(`row_${r}`);
        if (row) row.classList.toggle('is-solid', isStatic);
    });
}

function updateBorderVisibility() {
    const enabled = el('brd_enabled')?.checked;
    const staticToggle = el('brd_static');
    const topPicker = pickers['brd_top'];
    const botPicker = pickers['brd_bot'];
    const row = el('row_brd');
    
    if (staticToggle) {
        staticToggle.disabled = !enabled;
        const staticWrap = el('wrap_brd_static') || staticToggle.parentElement;
        if (staticWrap) {
            staticWrap.style.opacity = enabled ? '1' : '0.5';
            staticWrap.style.pointerEvents = enabled ? 'auto' : 'none';
        }
    }
    
    if (row) {
        row.classList.toggle('disabled', !enabled);
        const pickerWrap = row.querySelector('.picker-wrap');
        if (pickerWrap) {
            pickerWrap.style.opacity = enabled ? '1' : '0.5';
            pickerWrap.style.pointerEvents = enabled ? 'auto' : 'none';
        }
    }
    
    if (enabled) {
        if (topPicker) topPicker.enable();
        if (botPicker) botPicker.enable();
    } else {
        if (topPicker) topPicker.disable();
        if (botPicker) botPicker.disable();
    }
    checkDirtyState();
}

function applyConditionalVisibility() {
    const overlayEnabled = el('sys_overlayEnabled').checked;
    const layout = el('overlay_layout')?.value || 'full';
    const isFullCinematic = layout === 'full';
    
    // Hide visualizer toggle option if not in full cinematic mode
    const wrapVizToggle = el('wrap_viz_toggle');
    if (wrapVizToggle) {
        wrapVizToggle.style.display = (overlayEnabled && isFullCinematic) ? 'flex' : 'none';
    }
    
    const vizEnabled = el('sys_vizEnabled').checked && isFullCinematic;
    
    // Hide entire cards if their main feature is disabled
    el('card_overlay').style.display = overlayEnabled ? 'block' : 'none';
    el('card_colors').style.display = overlayEnabled ? 'block' : 'none';
    el('card_viz_settings').style.display = (overlayEnabled && vizEnabled) ? 'block' : 'none';
    
    // Sync Quick Nav tab visibilities
    const navOverlay = document.querySelector('a[href="#card_overlay"]');
    if (navOverlay) navOverlay.style.display = overlayEnabled ? 'inline-block' : 'none';
    const navColors = el('nav_colors');
    if (navColors) navColors.style.display = overlayEnabled ? 'inline-block' : 'none';
    const navViz = el('nav_viz');
    if (navViz) navViz.style.display = (overlayEnabled && vizEnabled) ? 'inline-block' : 'none';
    
    // 🚀 OBS Guide with animation
    const guide = el('obs_guide');
    if (overlayEnabled) guide.classList.remove('hidden');
    else guide.classList.add('hidden');

    // 🚀 FPS toggle visibility (only for Full Cinematic mode)
    const fpsToggle = el('wrap_fps_toggle');
    if (fpsToggle) fpsToggle.style.display = (vizEnabled && layout === 'full') ? 'flex' : 'none';
    
    // 🚀 Global Sync (Cover Art) Logic
    const syncEnabled = el('overlay_globalSync').checked;
    const shuffleEnabled = el('bg_grad_random').checked;
    const colorsLocked = syncEnabled || shuffleEnabled;
    
    // Toggle Cover Art Sync lock badge
    const badge = el('cover_art_sync_badge');
    if (badge) {
        badge.style.display = colorsLocked ? 'inline-flex' : 'none';
        if (syncEnabled) {
            badge.innerHTML = 'Controlled by Cover Art';
        } else if (shuffleEnabled) {
            badge.innerHTML = 'Controlled by Shuffle';
        }
    }

    ['viz', 'acc', 'brd', 'txt'].forEach(prefix => {
        const card = el(`row_${prefix}`) || el(`card_${prefix}`);
        if (card) {
            card.style.opacity = colorsLocked ? '0.65' : '1';
            card.style.pointerEvents = colorsLocked ? 'none' : 'auto';
            card.style.transition = 'all 0.3s ease';
            card.classList.toggle('locked', colorsLocked);
            
            // Add a "Controlled by Sync" overlay if not present
            let overlay = card.querySelector('.sync-lock-overlay');
            if (colorsLocked && !overlay) {
                overlay = document.createElement('div');
                overlay.className = 'sync-lock-overlay';
                overlay.innerHTML = `<span>Controlled by ${syncEnabled ? 'Cover Art' : 'Shuffle'}</span>`;
                card.style.position = 'relative';
                card.appendChild(overlay);
            } else if (colorsLocked && overlay) {
                overlay.innerHTML = `<span>Controlled by ${syncEnabled ? 'Cover Art' : 'Shuffle'}</span>`;
            } else if (!colorsLocked && overlay) {
                overlay.remove();
            }
        }
        
        // Lock static checkboxes too
        const chk = el(`${prefix}_static`);
        if (chk) {
            if (shuffleEnabled) chk.checked = false; // Force uncheck so shuffle gives gradients
            chk.disabled = colorsLocked;
            if (chk.parentElement) {
                chk.parentElement.style.opacity = colorsLocked ? '0.5' : '1';
                chk.parentElement.style.pointerEvents = colorsLocked ? 'none' : 'auto';
            }
        }
        
        // Programmatically lock Pickr instances to prevent clicking/opening
        const topPicker = pickers[`${prefix}_top`];
        const botPicker = pickers[`${prefix}_bot`];
        const prefixLocked = colorsLocked || (prefix === 'brd' && !el('brd_enabled')?.checked);
        if (topPicker) prefixLocked ? topPicker.disable() : topPicker.enable();
        if (botPicker) prefixLocked ? botPicker.disable() : botPicker.enable();
    });
    
    // Disable random toggle if sync is on, and disable sync if random is on
    const randToggle = el('bg_grad_random');
    if (randToggle) {
        randToggle.disabled = syncEnabled;
        randToggle.parentElement.style.opacity = syncEnabled ? '0.5' : '1';
        randToggle.parentElement.style.pointerEvents = syncEnabled ? 'none' : 'auto';
    }
    
    const syncToggle = el('overlay_globalSync');
    if (syncToggle) {
        syncToggle.disabled = shuffleEnabled;
        syncToggle.parentElement.style.opacity = shuffleEnabled ? '0.5' : '1';
        syncToggle.parentElement.style.pointerEvents = shuffleEnabled ? 'none' : 'auto';
    }
}

function applyPreset(type, event) {
    const p = PRESETS[type];
    if (!p) return;

    const set = (id, val, textId) => {
        const target = el(id);
        if (target) {
            target.value = val;
            if (textId) {
                const disp = el('val_' + textId);
                if (disp) disp.innerText = val;
            }
        }
    };

    set('viz_samples', p.samples, 'samples');
    set('viz_smoothing', p.smooth, 'smooth');

    checkActivePreset();
    checkDirtyState();
}

function updateOBSGuide() {
    const layout = el('overlay_layout')?.value || 'full';
    const wEl = el('guide_w'), hEl = el('guide_h');
    if(wEl && hEl) {
        if(layout === 'compact-bar') { wEl.innerText = '800'; hEl.innerText = '200'; }
        else if(layout === 'compact-minimal') { wEl.innerText = '540'; hEl.innerText = '100'; }
        else { wEl.innerText = '1360'; hEl.innerText = '440'; }
    }
}

// Attach event listener to layout dropdown
document.addEventListener("DOMContentLoaded", () => {
    const layoutSelect = el('overlay_layout');
    if (layoutSelect) {
        layoutSelect.addEventListener('change', () => {
            updateOBSGuide();
            applyConditionalVisibility();
        });
    }
    
    // Trigger conditional visibility updates on user interaction
    ['overlay_globalSync', 'sys_overlayEnabled', 'sys_vizEnabled', 'bg_grad_random'].forEach(id => {
        const checkbox = el(id);
        if (checkbox) {
            checkbox.addEventListener('change', applyConditionalVisibility);
        }
    });
    
    // 🚀 Watch sliders for preset matches
    document.querySelectorAll('input[type="range"]').forEach(input => {
        input.addEventListener('input', checkActivePreset);
    });
    
    // ... existing DOMContentLoaded logic ...
    document.querySelectorAll('input[type="range"]').forEach(slider => {
        const label = slider.previousElementSibling;
        if (label && label.tagName === 'LABEL') {
            const resetBtn = document.createElement('span');
            resetBtn.className = 'title-reset-btn';
            resetBtn.innerHTML = ' ↺';
            resetBtn.title = 'Reset to Default';
            
            resetBtn.onclick = async (e) => {
                e.preventDefault();
                try {
                    const res = await fetch('/api/defaults', { cache: 'no-store' });
                    const defs = await res.json();
                    const o = defs.overlay, v = o.visualizer;
                    let defVal = null;
                    switch(slider.id) {
                        case 'overlay_backgroundOpacity': defVal = o.backgroundOpacity; break;
                        case 'overlay_thumbOpacity': defVal = o.thumbnailOpacity; break;
                        case 'overlay_textAnimationDuration': defVal = o.textAnimationDuration; break;
                        case 'viz_fps': defVal = v.fps; break;
                        case 'viz_samples': defVal = v.samples; break;
                        case 'viz_bars': defVal = v.bars; break;
                        case 'viz_smoothing': defVal = v.smoothing; break;
                        case 'viz_sensitivity': defVal = v.sensitivity; break;
                        case 'viz_multiplier': defVal = v.multiplier; break;
                        case 'viz_barWidth': defVal = v.barWidth; break;
                        case 'viz_barGap': defVal = v.barGap; break;
                    }
                    if (defVal !== null) {
                        slider.value = defVal;
                        slider.dispatchEvent(new Event('input')); // Update the UI label instantly
                        
                        // Give visual feedback
                        resetBtn.classList.add('spinning');
                        setTimeout(() => resetBtn.classList.remove('spinning'), 600);
                    }
                } catch (err) {}
            };
            
            // Fix: Place reset button exactly next to the text on the left
            const textNode = Array.from(label.childNodes).find(n => n.nodeType === Node.TEXT_NODE && n.textContent.trim().length > 0);
            if (textNode) {
                const textWrapper = document.createElement('span');
                textWrapper.style.display = 'inline-flex';
                textWrapper.style.alignItems = 'center';
                label.replaceChild(textWrapper, textNode);
                textWrapper.appendChild(textNode);
                textWrapper.appendChild(resetBtn);
            } else {
                label.appendChild(resetBtn);
            }
        }
    });

    // 🚀 COLOR PALETTE RESET BUTTONS
    document.querySelectorAll('label, .color-section-title').forEach(titleEl => {
        const text = titleEl.textContent ? titleEl.textContent.trim() : titleEl.innerText;
        if (text === 'Text Palette' || text === 'Border Style' || text === 'Accent Highlights' || text === 'Color Customization') {
            const resetBtn = document.createElement('span');
            resetBtn.className = 'title-reset-btn';
            resetBtn.innerHTML = ' ↺';
            resetBtn.title = 'Reset to Default';
            
            resetBtn.onclick = async (e) => {
                e.preventDefault();
                try {
                    const res = await fetch('/api/defaults', { cache: 'no-store' });
                    const defs = await res.json();
                    const o = defs.overlay, v = o.visualizer;
                    
                    if (text === 'Text Palette') {
                        if (pickers['txt_top']) { pickers['txt_top'].setColor(stripAlpha(o.textStyle.top)); pickers['txt_top'].applyColor(); }
                        if (pickers['txt_bot']) { pickers['txt_bot'].setColor(stripAlpha(o.textStyle.bottom)); pickers['txt_bot'].applyColor(); }
                        el('txt_static').checked = o.textStyle.isStatic;
                    } else if (text === 'Border Style') {
                        const brdTopColor = o.borderStyle.top && o.borderStyle.top !== 'transparent' ? o.borderStyle.top : '#38bdf8';
                        const brdBotColor = o.borderStyle.bottom && o.borderStyle.bottom !== 'transparent' ? o.borderStyle.bottom : '#38bdf8';
                        const borderEnabled = o.borderStyle.top !== 'transparent' && o.borderStyle.bottom !== 'transparent';
                        el('brd_enabled').checked = borderEnabled;
                        if (pickers['brd_top']) { pickers['brd_top'].setColor(stripAlpha(brdTopColor)); pickers['brd_top'].applyColor(); }
                        if (pickers['brd_bot']) { pickers['brd_bot'].setColor(stripAlpha(brdBotColor)); pickers['brd_bot'].applyColor(); }
                        el('brd_static').checked = o.borderStyle.isStatic;
                        updateBorderVisibility();
                    } else if (text === 'Accent Highlights') {
                        if (pickers['acc_top']) { pickers['acc_top'].setColor(stripAlpha(o.elementStyle.top)); pickers['acc_top'].applyColor(); }
                        if (pickers['acc_bot']) { pickers['acc_bot'].setColor(stripAlpha(o.elementStyle.bottom)); pickers['acc_bot'].applyColor(); }
                        el('acc_static').checked = o.elementStyle.isStatic;
                    } else if (text === 'Color Customization') {
                        if (pickers['viz_top']) { pickers['viz_top'].setColor(stripAlpha(v.gradient.top)); pickers['viz_top'].applyColor(); }
                        if (pickers['viz_bot']) { pickers['viz_bot'].setColor(stripAlpha(v.gradient.bottom)); pickers['viz_bot'].applyColor(); }
                        el('viz_static').checked = v.gradient.isStatic;
                    }
                    
                    updatePickerVisibility();
                    
                    resetBtn.classList.add('spinning');
                    setTimeout(() => resetBtn.classList.remove('spinning'), 600);
                } catch (err) {}
            };

            const textNode = Array.from(titleEl.childNodes).find(n => n.nodeType === Node.TEXT_NODE && n.textContent.trim().length > 0);
            if (textNode) {
                const textWrapper = document.createElement('span');
                textWrapper.style.display = 'inline-flex';
                textWrapper.style.alignItems = 'center';
                titleEl.replaceChild(textWrapper, textNode);
                textWrapper.appendChild(textNode);
                textWrapper.appendChild(resetBtn);
            } else {
                titleEl.appendChild(resetBtn);
            }
        }
    });

    // 🚀 QUICK NAV HIGHLIGHT OBSERVER
    const cards = ['card_general', 'card_overlay', 'card_colors', 'card_viz_settings'];
    const navItems = document.querySelectorAll('.nav-item');
    const observerOptions = {
        root: null,
        rootMargin: '-5% 0px -45% 0px',
        threshold: 0
    };
    
    let ignoreObserver = false;
    let observerTimeout = null;

    const navObserver = new IntersectionObserver((entries) => {
        if (ignoreObserver) return;
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                const id = entry.target.id;
                navItems.forEach(item => {
                    if (item.getAttribute('href') === `#${id}`) {
                        item.classList.add('active');
                    } else {
                        item.classList.remove('active');
                    }
                });
            }
        });
    }, observerOptions);
    cards.forEach(id => {
        const target = document.getElementById(id);
        if (target) navObserver.observe(target);
    });

    // Handle manual clicks to immediately highlight tabs
    navItems.forEach(item => {
        item.addEventListener('click', () => {
            navItems.forEach(i => i.classList.remove('active'));
            item.classList.add('active');
            
            ignoreObserver = true;
            clearTimeout(observerTimeout);
            observerTimeout = setTimeout(() => {
                ignoreObserver = false;
            }, 800);
        });
    });

    // Fallback when scrolled to the very bottom
    window.addEventListener('scroll', () => {
        if (ignoreObserver) return;
        if ((window.innerHeight + window.scrollY) >= document.body.offsetHeight - 50) {
            const visibleNavs = Array.from(navItems).filter(item => item.style.display !== 'none');
            if (visibleNavs.length > 0) {
                navItems.forEach(i => i.classList.remove('active'));
                visibleNavs[visibleNavs.length - 1].classList.add('active');
            }
        }
    });
});

function getDiff(original, current) {
    let diff = {};
    for (const key in current) {
        if (current[key] === undefined) continue;
        const oldVal = original ? original[key] : undefined;
        const newVal = current[key];
        
        if (newVal !== null && typeof newVal === 'object' && !Array.isArray(newVal)) {
            const subDiff = getDiff(oldVal || {}, newVal);
            if (Object.keys(subDiff).length > 0) diff[key] = subDiff;
        } else {
            // 🚀 Normalize for comparison: Numbers to strings, casing, etc.
            const nS = (v) => (v === null || v === undefined) ? "" : String(v).toLowerCase();
            if (nS(oldVal) !== nS(newVal)) {
                diff[key] = newVal;
            }
        }
    }
    return diff;
}

function buildUIState() {
    const getV = (id) => el(id)?.value;
    const getC = (id) => !!el(id)?.checked;
    const getN = (id, def) => { const v = parseInt(getV(id)); return isNaN(v) ? def : v; };
    const style = getV('overlay_bgStyle'), fxOp = parseFloat(getV('overlay_thumbOpacity')), newPort = getN('sys_port', 3000);
    const getStyle = (prefix) => {
        if (!pickers[`${prefix}_top`]) return undefined;
        if (prefix === 'brd' && !getC('brd_enabled')) {
            return {
                isStatic: getC('brd_static'),
                random: false,
                top: 'transparent',
                bottom: 'transparent'
            };
        }
        // 🚀 Normalize to lowercase hex for reliable diffing
        const top = stripAlpha(pickers[`${prefix}_top`].getColor().toHEXA().toString()).toLowerCase();
        const bot = stripAlpha(pickers[`${prefix}_bot`].getColor().toHEXA().toString()).toLowerCase();
        return { 
            isStatic: getC(`${prefix}_static`), 
            random: prefix === 'viz' ? getC('bg_grad_random') : false, 
            top: top, 
            bottom: bot 
        };
    };
    let code = getV('sys_code');
    if (code === "✓ LINKED" || code === "✓ LINKED SECURELY") code = "";
    const userId = getV('sys_userId');
    const savedCodes = (g_currentSettings && g_currentSettings.savedCodes) ? JSON.parse(JSON.stringify(g_currentSettings.savedCodes)) : {};
    if (userId && code && code.length === 6) savedCodes[userId] = code;

    return {
        code: code,
        userId: userId,
        savedCodes: savedCodes,
        allowFirewall: getC('sys_allowFirewall'),
        rpc: { swapRpcLines: getC('rpc_swapLines'), ipcPipe: getN('rpc_ipcPipe', -1) },
        overlay: {
            enabled: getC('sys_overlayEnabled'), port: newPort, layout: getV('overlay_layout'),
            showBackground: style === "blur", showThumbnailBackground: style === "cover",
            backgroundOpacity: parseFloat(getV('overlay_backgroundOpacity')),
            thumbnailOpacity: style === "blur" ? fxOp : undefined,
            thumbnailBackgroundOpacity: style === "cover" ? fxOp : undefined,
            centerText: getC('overlay_centerText'), globalSync: getC('overlay_globalSync'),
            enableTextAnimation: getC('overlay_textAnim'), 
            textAnimationMode: getV('overlay_textAnimationMode'),
            textAnimationDuration: getN('overlay_textAnimationDuration', 10),
            textStyle: getStyle('txt'), borderStyle: getStyle('brd'), elementStyle: getStyle('acc'),
            visualizer: {
                enabled: getC('sys_vizEnabled'), debugFps: getC('viz_debugFps'), audioDevice: getV('sys_audioDevice'),
                samples: getN('viz_samples', 4096), bars: getN('viz_bars', 64), smoothing: getN('viz_smoothing', 6),
                sensitivity: getN('viz_sensitivity', 50), multiplier: getN('viz_multiplier', 25),
                barWidth: getN('viz_barWidth', 10), barGap: getN('viz_barGap', 5), gradient: getStyle('viz'),
                mode: getV('viz_mode'), visualizerType: getV('viz_type'), 
                glow: getC('viz_glow'), rounded: getC('viz_rounded'), fps: getN('viz_fps', 60)
            }
        }
    };
}

async function fetchDiscordClients(targetPipe) {
    try {
        const res = await fetch('/api/discord_clients', { cache: 'no-store' });
        const clients = await res.json();
        const select = el('rpc_ipcPipe');
        if (!select) return;
        let currentVal = (targetPipe !== undefined) ? targetPipe : parseInt(select.value);
        if (isNaN(currentVal)) currentVal = -1;
        select.innerHTML = '<option value="-1">Auto-Detect</option>';
        clients.forEach(c => {
            const opt = document.createElement('option');
            opt.value = c.pipe; opt.innerText = `${c.display_name} (Pipe ${c.pipe})`; opt.dataset.id = c.id;
            if (c.pipe === currentVal) opt.selected = true;
            select.appendChild(opt);
        });
    } catch (e) {}
}

const ipcSelect = el('rpc_ipcPipe');
if (ipcSelect) {
    ipcSelect.addEventListener('change', (e) => {
        const opt = e.target.selectedOptions[0];
        if (opt && opt.dataset.id) {
            const idInput = el('sys_userId');
            if (idInput) {
                if (opt.dataset.id === "1045800378228281345") {
                    idInput.value = "";
                    showAuthModal();
                } else {
                    idInput.value = opt.dataset.id;
                    saveSettings();
                }
            }
        } else {
            saveSettings();
        }
    });
}

async function loadSettings(settingsToLoad) {
    g_isLoadingSettings = true;
    try {
        const isReset = !!settingsToLoad;
        const state = settingsToLoad || window.__SETTINGS__ || await (await fetch('/api/settings', { cache: 'no-store' })).json();
        window.__SETTINGS__ = null;
        if (!isReset) g_currentSettings = state;
        const o = state.overlay || {}, v = o.visualizer || {}, r = state.rpc || {};
        const setS = (id, val) => { 
            const e = el(id); if(!e) return;
            if (e.tagName === 'SELECT') {
                setSelectValue(id, val);
            } else {
                e.value = val;
            }
            if(e.type==='range') { 
                const displayId = id.replace('viz_smoothing','smooth').replace('viz_sensitivity','sens').replace('viz_multiplier','mult').replace('viz_','').replace('overlay_','').replace('backgroundOpacity','bgOp').replace('thumbOpacity','thumbOp').replace('textAnimationDuration','textDur');
                const disp = el('val_'+displayId); if(disp) disp.innerText = val; 
            } 
        };
        const setC = (id, val) => { const e = el(id); if(e) e.checked = !!val; };

        const sysCode = el('sys_code');
        if (state.sessionToken) {
            if (sysCode) {
                sysCode.value = "✓ LINKED";
                sysCode.disabled = true;
                sysCode.style.background = "rgba(16, 185, 129, 0.05)";
                sysCode.style.color = "var(--success)";
                sysCode.style.borderColor = "var(--success)";
                sysCode.style.textAlign = "center";
                sysCode.style.fontWeight = "bold";
            }
        } else {
            if (sysCode) {
                sysCode.value = state.code || "";
                sysCode.placeholder = "000000";
                sysCode.disabled = false;
                sysCode.style = "";
            }
        }
        setS('sys_userId', state.userId); setS('sys_port', o.port); initialPort = o.port;
        const pipeVal = r.ipcPipe; setS('rpc_ipcPipe', pipeVal);
        const urls = document.querySelectorAll('.guide_url'); urls.forEach(u => u.innerText = `http://127.0.0.1:${o.port}/`);
        g_audioDevice = v.audioDevice; setC('sys_overlayEnabled', o.enabled); setC('sys_vizEnabled', v.enabled); setC('rpc_swapLines', r.swapRpcLines); setC('sys_allowFirewall', state.allowFirewall);
        setS('overlay_layout', o.layout); setS('overlay_bgStyle', o.showBackground ? "blur" : (o.showThumbnailBackground ? "cover" : "none"));
        setS('overlay_backgroundOpacity', o.backgroundOpacity); setS('overlay_thumbOpacity', o.thumbnailOpacity || o.thumbnailBackgroundOpacity || 0.5);
        setC('overlay_centerText', o.centerText); setC('overlay_globalSync', o.globalSync); setC('overlay_textAnim', o.enableTextAnimation);
        setS('overlay_textAnimationMode', o.textAnimationMode); setS('overlay_textAnimationDuration', o.textAnimationDuration);
        const textDurWrap = el('wrap_text_speed'); if (textDurWrap) textDurWrap.style.display = o.textAnimationMode === 'static' ? 'block' : 'none';
        setS('viz_mode', v.mode); setS('viz_type', v.visualizerType); setS('viz_fps', v.fps); setS('viz_samples', v.samples); setS('viz_bars', v.bars);
        setS('viz_smoothing', v.smoothing); setS('viz_sensitivity', v.sensitivity); setS('viz_multiplier', v.multiplier);
        setS('viz_barWidth', v.barWidth); setS('viz_barGap', v.barGap); setC('viz_glow', v.glow); setC('viz_rounded', v.rounded); setC('viz_debugFps', v.debugFps);

        createPicker('txt_top', stripAlpha(o.textStyle?.top)); createPicker('txt_bot', stripAlpha(o.textStyle?.bottom)); setC('txt_static', o.textStyle?.isStatic);
        
        const borderStyle = o.borderStyle || {};
        const borderEnabled = borderStyle.top !== 'transparent' && borderStyle.bottom !== 'transparent';
        setC('brd_enabled', borderEnabled);
        const brdTopColor = borderStyle.top && borderStyle.top !== 'transparent' ? borderStyle.top : '#38bdf8';
        const brdBotColor = borderStyle.bottom && borderStyle.bottom !== 'transparent' ? borderStyle.bottom : '#38bdf8';
        createPicker('brd_top', stripAlpha(brdTopColor)); createPicker('brd_bot', stripAlpha(brdBotColor)); setC('brd_static', borderStyle.isStatic);
        
        createPicker('acc_top', stripAlpha(o.elementStyle?.top)); createPicker('acc_bot', stripAlpha(o.elementStyle?.bottom)); setC('acc_static', o.elementStyle?.isStatic);
        createPicker('viz_top', stripAlpha(v.gradient?.top)); createPicker('viz_bot', stripAlpha(v.gradient?.bottom)); setC('viz_static', v.gradient?.isStatic); setC('bg_grad_random', v.gradient?.random);

        updatePickerVisibility(); updateBorderVisibility(); applyConditionalVisibility(); fetchDevices(); fetchDiscordClients(pipeVal);
        checkActivePreset();
        
        if (!isReset) {
            setTimeout(() => { g_initialUIState = buildUIState(); }, 500);
        }
    } catch (e) { console.error("Load failed", e); }
    finally {
        g_isLoadingSettings = false;
    }
}

async function resetSection(section) {
    try {
        const res = await fetch('/api/defaults', { cache: 'no-store' });
        const defaults = await res.json();
        
        if (section === 'factory') {
            if (!confirm("Are you sure? This will wipe ALL layout and visualizer settings.")) return;
            defaults.code = el('sys_code').value;
            defaults.userId = el('sys_userId').value;
            await loadSettings(defaults); 
            status.innerText = "Factory Reset Applied! Click Deploy to save.";
            status.className = 'status-ok';
            return;
        }

        const o = defaults.overlay;
        const v = o.visualizer;
        
        if (section === 'general') { 
            el('sys_overlayEnabled').checked = o.enabled;
            el('sys_vizEnabled').checked = v.enabled;
            el('sys_port').value = o.port;
            el('rpc_swapLines').checked = defaults.rpc.swapRpcLines;
            applyConditionalVisibility();
        }
        else if (section === 'overlay') { 
            setSelectValue('overlay_layout', o.layout);
            setSelectValue('overlay_bgStyle', 'blur');
            el('overlay_backgroundOpacity').value = o.backgroundOpacity;
            el('overlay_thumbOpacity').value = o.thumbnailOpacity;
            el('overlay_centerText').checked = o.centerText;
            el('overlay_globalSync').checked = o.globalSync;
            el('overlay_textAnim').checked = o.enableTextAnimation;
            setSelectValue('overlay_textAnimationMode', o.textAnimationMode);
            el('overlay_textAnimationDuration').value = o.textAnimationDuration;
            
            if (el('val_bgOp')) el('val_bgOp').innerText = o.backgroundOpacity;
            if (el('val_thumbOp')) el('val_thumbOp').innerText = o.thumbnailOpacity;
            if (el('val_textDur')) el('val_textDur').innerText = o.textAnimationDuration;
        }
        else if (section === 'colors') { 
            if (pickers['txt_top']) { pickers['txt_top'].setColor(stripAlpha(o.textStyle.top)); pickers['txt_top'].applyColor(); }
            if (pickers['txt_bot']) { pickers['txt_bot'].setColor(stripAlpha(o.textStyle.bottom)); pickers['txt_bot'].applyColor(); }
            el('txt_static').checked = o.textStyle.isStatic;

            const brdTopColor = o.borderStyle.top && o.borderStyle.top !== 'transparent' ? o.borderStyle.top : '#38bdf8';
            const brdBotColor = o.borderStyle.bottom && o.borderStyle.bottom !== 'transparent' ? o.borderStyle.bottom : '#38bdf8';
            const borderEnabled = o.borderStyle.top !== 'transparent' && o.borderStyle.bottom !== 'transparent';
            el('brd_enabled').checked = borderEnabled;
            if (pickers['brd_top']) { pickers['brd_top'].setColor(stripAlpha(brdTopColor)); pickers['brd_top'].applyColor(); }
            if (pickers['brd_bot']) { pickers['brd_bot'].setColor(stripAlpha(brdBotColor)); pickers['brd_bot'].applyColor(); }
            el('brd_static').checked = o.borderStyle.isStatic;

            if (pickers['acc_top']) { pickers['acc_top'].setColor(stripAlpha(o.elementStyle.top)); pickers['acc_top'].applyColor(); }
            if (pickers['acc_bot']) { pickers['acc_bot'].setColor(stripAlpha(o.elementStyle.bottom)); pickers['acc_bot'].applyColor(); }
            el('acc_static').checked = o.elementStyle.isStatic;
            
            updatePickerVisibility();
            updateBorderVisibility();
        }
        else if (section === 'visualizer') { 
            setSelectValue('viz_mode', v.mode);
            setSelectValue('viz_type', v.visualizerType);
            el('viz_rounded').checked = v.rounded;
            el('viz_glow').checked = v.glow;
            
            el('viz_fps').value = v.fps;
            if (el('val_fps')) el('val_fps').innerText = v.fps;
            
            el('viz_bars').value = v.bars;
            if (el('val_bars')) el('val_bars').innerText = v.bars;
            
            el('viz_samples').value = v.samples;
            if (el('val_samples')) el('val_samples').innerText = v.samples;
            
            el('viz_smoothing').value = v.smoothing;
            if (el('val_smooth')) el('val_smooth').innerText = v.smoothing;
            
            el('viz_sensitivity').value = v.sensitivity;
            if (el('val_sens')) el('val_sens').innerText = v.sensitivity;
            
            el('viz_multiplier').value = v.multiplier;
            if (el('val_mult')) el('val_mult').innerText = v.multiplier;
            
            el('viz_barWidth').value = v.barWidth;
            if (el('val_barWidth')) el('val_barWidth').innerText = v.barWidth;
            
            el('viz_barGap').value = v.barGap;
            if (el('val_barGap')) el('val_barGap').innerText = v.barGap;
            
            if (pickers['viz_top']) { pickers['viz_top'].setColor(stripAlpha(v.gradient.top)); pickers['viz_top'].applyColor(); }
            if (pickers['viz_bot']) { pickers['viz_bot'].setColor(stripAlpha(v.gradient.bottom)); pickers['viz_bot'].applyColor(); }
            el('viz_static').checked = v.gradient.isStatic;
            el('bg_grad_random').checked = v.gradient.random;
            
            updatePickerVisibility();
        }
        
        status.innerText = `${section.charAt(0).toUpperCase() + section.slice(1)} Reset! Click Deploy to save.`;
        status.className = 'status-ok';
        checkDirtyState();
        setTimeout(() => status.className = '', 4000);
    } catch (e) { alert("Reset failed: " + e.message); console.error(e); }
}

async function saveSettings() {
    if (g_isLoadingSettings) return;
    try {
        if (!g_initialUIState) return;
        const currentUI = buildUIState();
        const diff = getDiff(g_initialUIState, currentUI);
        
        status.innerText = "Deploying Changes..."; status.className = 'status-ok';
        
        // Always send a payload so the server triggers a reload and the user sees success
        const allowFirewall = el('sys_allowFirewall') ? el('sys_allowFirewall').checked : false;
        
        let payload = {};
        if (allowFirewall) {
            payload = { 
                settings: Object.keys(diff).length > 0 ? diff : { overlay: { port: currentUI.overlay.port } },
                actions: { allow_firewall: true }
            };
        } else {
            payload = Object.keys(diff).length > 0 ? diff : { overlay: { port: currentUI.overlay.port } };
        }
        
        const res = await fetch('/api/settings', { 
            method: 'POST', 
            headers: { 'Content-Type': 'application/json' }, 
            body: JSON.stringify(payload) 
        });
        
        if (!res.ok) throw new Error(await res.text());
        
        g_initialUIState = currentUI;
        const freshRes = await fetch('/api/settings', { cache: 'no-store' });
        g_currentSettings = await freshRes.json();
        
        checkDirtyState();
        
        status.innerText = "Deployed Successfully!";
        setTimeout(() => status.className = '', 5000);
        
        // Keep the allowFirewall checked state saved and persistent
        
        if (diff.overlay && diff.overlay.port !== undefined && diff.overlay.port !== initialPort) {
            alert(`Restart required for port change!`);
            initialPort = diff.overlay.port;
        }
    } catch (e) { alert("Save Error: " + e.message); }
}

async function fetchDevices() {
    try {
        const res = await fetch('/api/devices', { cache: 'no-store' });
        const devices = await res.json();
        const select = el('sys_audioDevice'); if (!select) return;
        select.innerHTML = '';
        devices.forEach(d => {
            const opt = document.createElement('option'); opt.value = d; opt.innerText = d;
            if (d === g_audioDevice) opt.selected = true;
            select.appendChild(opt);
        });
    } catch (e) {}
}

async function checkStatus() {
    try {
        const res = await fetch('/api/status', { cache: 'no-store' });
        const status = await res.json();
        
        const sysCode = el('sys_code');
        if (sysCode) {
            const isUIVisuallyLinked = sysCode.disabled && sysCode.value === "✓ LINKED";
            if (isUIVisuallyLinked && status.isLinked === false) {
                sysCode.value = "";
                sysCode.placeholder = "000000";
                sysCode.disabled = false;
                sysCode.style = "";
                
                const statusLabel = el('status');
                if (statusLabel) {
                    statusLabel.innerText = "Stale connection. Please enter a new pairing code.";
                    statusLabel.className = 'status-err';
                }
            } else if (!isUIVisuallyLinked && status.isLinked === true) {
                loadSettings();
            }
        }

        const userId = el('sys_userId').value.trim();
        if (status.arrpcDetected) {
            if (!userId) {
                showAuthModal();
            }
        }
    } catch (e) {}
}

function showAuthModal() {
    let modal = el('auth_modal');
    if (!modal) {
        modal = document.createElement('div');
        modal.id = 'auth_modal';
        modal.className = 'modal-overlay';
        modal.innerHTML = `
            <div class="modal-content">
                <h2>Discord ID Required</h2>
                <p>arRPC / WebCord detected. Your 18-digit Discord User ID is required to authenticate with the bridge server.</p>
                <div class="field">
                    <label>User ID</label>
                    <input type="text" id="modal_userId" placeholder="Enter User ID">
                </div>
                <button id="modal_submit">Apply & Connect</button>
            </div>
        `;
        document.body.appendChild(modal);
        el('modal_submit').onclick = () => {
            const val = el('modal_userId').value.trim();
            if (val) {
                el('sys_userId').value = val;
                modal.style.display = 'none';
                saveSettings();
            } else {
                alert("User ID is required for arRPC connections.");
            }
        };
    }
    modal.style.display = 'flex';
}

form.onsubmit = (e) => { e.preventDefault(); saveSettings(); }; 
loadSettings();

setInterval(checkStatus, 3000);
checkStatus();

// Diff-based Dirty State Logic
function checkDirtyState() {
    if (!g_initialUIState) return;
    const currentUI = buildUIState();
    const diff = getDiff(g_initialUIState, currentUI);
    
    // Flatten diff keys for easy lookup
    const diffKeys = new Set();
    function flattenKeys(obj, prefix = '') {
        for (const k in obj) {
            const path = prefix ? `${prefix}.${k}` : k;
            if (typeof obj[k] === 'object' && obj[k] !== null && !Array.isArray(obj[k])) flattenKeys(obj[k], path);
            else diffKeys.add(path);
        }
    }
    flattenKeys(diff);

    document.querySelectorAll('input, select, .pcr-button, .toggle-item, .val-display').forEach(el => {
        el.classList.remove('is-dirty');
    });

    // Map DOM IDs to state paths
    const map = {
        'sys_code': 'code', 'sys_port': 'overlay.port',
        'rpc_swapLines': 'rpc.swapRpcLines', 'sys_allowFirewall': 'allowFirewall',
        'sys_overlayEnabled': 'overlay.enabled', 'sys_vizEnabled': 'overlay.visualizer.enabled',
        'viz_debugFps': 'overlay.visualizer.debugFps', 'overlay_layout': 'overlay.layout',
        'overlay_backgroundOpacity': 'overlay.backgroundOpacity', 'overlay_centerText': 'overlay.centerText',
        'overlay_globalSync': 'overlay.globalSync', 'overlay_textAnim': 'overlay.enableTextAnimation',
        'overlay_textAnimationMode': 'overlay.textAnimationMode', 'overlay_textAnimationDuration': 'overlay.textAnimationDuration',
        'viz_mode': 'overlay.visualizer.mode', 'viz_type': 'overlay.visualizer.visualizerType',
        'viz_rounded': 'overlay.visualizer.rounded', 'viz_glow': 'overlay.visualizer.glow',
        'viz_fps': 'overlay.visualizer.fps', 'viz_bars': 'overlay.visualizer.bars',
        'viz_samples': 'overlay.visualizer.samples', 'viz_smoothing': 'overlay.visualizer.smoothing',
        'viz_sensitivity': 'overlay.visualizer.sensitivity', 'viz_multiplier': 'overlay.visualizer.multiplier',
        'viz_barWidth': 'overlay.visualizer.barWidth', 'viz_barGap': 'overlay.visualizer.barGap',
        'bg_grad_random': 'overlay.visualizer.gradient.random',
        'brd_enabled': 'overlay.borderStyle.top'
    };

    for (const id in map) {
        if (diffKeys.has(map[id])) {
            const elNode = document.getElementById(id);
            if (elNode) {
                if (elNode.type === 'checkbox') elNode.parentElement.classList.add('is-dirty');
                else elNode.classList.add('is-dirty');
            }
        }
    }
    
    // Handle sliders
    document.querySelectorAll('input[type="range"]').forEach(slider => {
        if (slider.classList.contains('is-dirty')) {
            const idParts = slider.id.split('_');
            if (idParts.length > 1) {
                const valDisplay = document.getElementById('val_' + idParts[1]) || document.getElementById('val_' + slider.id.replace('viz_smoothing','smooth').replace('viz_sensitivity','sens').replace('viz_multiplier','mult').replace('viz_','').replace('overlay_','').replace('backgroundOpacity','bgOp').replace('thumbOpacity','thumbOp').replace('textAnimationDuration','textDur'));
                if (valDisplay) valDisplay.classList.add('is-dirty');
            }
        } else {
            const idParts = slider.id.split('_');
            if (idParts.length > 1) {
                const valDisplay = document.getElementById('val_' + idParts[1]) || document.getElementById('val_' + slider.id.replace('viz_smoothing','smooth').replace('viz_sensitivity','sens').replace('viz_multiplier','mult').replace('viz_','').replace('overlay_','').replace('backgroundOpacity','bgOp').replace('thumbOpacity','thumbOp').replace('textAnimationDuration','textDur'));
                if (valDisplay) valDisplay.classList.remove('is-dirty');
            }
        }
    });

    // Handle Pickr Buttons
    const pickrMap = {
        'txt_top': 'overlay.textStyle.top', 'txt_bot': 'overlay.textStyle.bottom', 'txt_static': 'overlay.textStyle.isStatic',
        'brd_top': 'overlay.borderStyle.top', 'brd_bot': 'overlay.borderStyle.bottom', 'brd_static': 'overlay.borderStyle.isStatic',
        'acc_top': 'overlay.elementStyle.top', 'acc_bot': 'overlay.elementStyle.bottom', 'acc_static': 'overlay.elementStyle.isStatic',
        'viz_top': 'overlay.visualizer.gradient.top', 'viz_bot': 'overlay.visualizer.gradient.bottom', 'viz_static': 'overlay.visualizer.gradient.isStatic'
    };
    for (const id in pickrMap) {
        if (diffKeys.has(pickrMap[id])) {
            const pcrBtn = document.querySelector(`#${id} .pcr-button`);
            if (pcrBtn) pcrBtn.classList.add('is-dirty');
            const chkNode = document.getElementById(id);
            if (chkNode && chkNode.type === 'checkbox') chkNode.parentElement.classList.add('is-dirty');
        }
    }
}

document.querySelectorAll('input, select').forEach(input => {
    input.addEventListener('change', checkDirtyState);
    if (input.type !== 'range') {
        input.addEventListener('input', checkDirtyState);
    }
});

// Load Logo
const logo = document.querySelector('.header-left img');
if (logo) {
    if (logo.complete) logo.classList.add('loaded');
    else logo.onload = () => logo.classList.add('loaded');
}

// 🚀 PREMIUM CUSTOM SELECT DROPDOWN LOGIC
const initCustomSelects = () => {
    document.querySelectorAll('select').forEach(select => {
        if (select.closest('.custom-select-wrapper')) return; // Already wrapped
        
        const wrapper = document.createElement('div');
        wrapper.className = 'custom-select-wrapper';
        
        // Insert wrapper before select in the DOM
        select.parentNode.insertBefore(wrapper, select);
        
        const trigger = document.createElement('div');
        trigger.className = 'custom-select-trigger';
        
        const optionsContainer = document.createElement('div');
        optionsContainer.className = 'custom-select-options';
        
        wrapper.appendChild(select); // Move select inside wrapper
        wrapper.appendChild(trigger);
        wrapper.appendChild(optionsContainer);
        
        const updateTriggerText = () => {
            const selectedOpt = select.querySelector('option:checked') || select.options[0];
            trigger.innerText = selectedOpt ? selectedOpt.innerText : 'Select...';
        };
        
        const syncOptions = () => {
            optionsContainer.innerHTML = '';
            Array.from(select.options).forEach(opt => {
                const item = document.createElement('div');
                item.className = 'custom-select-option';
                if (opt.selected) item.classList.add('selected');
                item.innerText = opt.innerText;
                item.dataset.value = opt.value;
                
                item.addEventListener('click', (e) => {
                    e.stopPropagation();
                    select.value = opt.value;
                    
                    // Dispatch events so regular change listeners work!
                    select.dispatchEvent(new Event('change', { bubbles: true }));
                    select.dispatchEvent(new Event('input', { bubbles: true }));
                    
                    wrapper.classList.remove('open');
                });
                optionsContainer.appendChild(item);
            });
            updateTriggerText();
        };
        
        // Listen to standard selection changes to update trigger text
        select.addEventListener('change', () => {
            updateTriggerText();
            // Sync selected class on items
            Array.from(optionsContainer.children).forEach(child => {
                child.classList.toggle('selected', child.dataset.value === select.value);
            });
        });
        
        trigger.addEventListener('click', (e) => {
            e.stopPropagation();
            const isOpen = wrapper.classList.contains('open');
            // Close all other dropdowns
            document.querySelectorAll('.custom-select-wrapper').forEach(w => w.classList.remove('open'));
            if (!isOpen) wrapper.classList.add('open');
        });
        
        // MutationObserver to automatically rebuild options if they change dynamically
        const observer = new MutationObserver(() => {
            syncOptions();
        });
        observer.observe(select, { childList: true, subtree: true, attributes: true });
        
        // Initial build
        syncOptions();
    });
};

// Close all custom select dropdowns when clicking outside
document.addEventListener('click', (e) => {
    document.querySelectorAll('.custom-select-wrapper').forEach(w => w.classList.remove('open'));
});

// Run initialization
initCustomSelects();