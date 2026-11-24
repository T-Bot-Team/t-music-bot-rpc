pub mod firewall;
use crate::models::TrackUpdate;
use std::process::Command;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub fn get_app_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let mut path = PathBuf::from(appdata);
            path.push("T_Music_Bot_RPC");
            let _ = std::fs::create_dir_all(&path);
            return path;
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let mut path = PathBuf::from(home);
            path.push("Library");
            path.push("Application Support");
            path.push("T_Music_Bot_RPC");
            let _ = std::fs::create_dir_all(&path);
            return path;
        }
    }
    
    // Linux and fallback
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let mut path = PathBuf::from(xdg);
        path.push("t-music-bot-rpc");
        let _ = std::fs::create_dir_all(&path);
        return path;
    }
    if let Ok(home) = std::env::var("HOME") {
        let mut path = PathBuf::from(home);
        path.push(".config");
        path.push("t-music-bot-rpc");
        let _ = std::fs::create_dir_all(&path);
        return path;
    }
    
    PathBuf::from(".")
}

pub fn get_log_path() -> PathBuf {
    get_app_dir().join("latest.log")
}

use std::sync::Mutex;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub fn log_message(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_line = format!("[{}] {}\n", timestamp, msg);
    print!("{}", log_line);
    
    let _lock = LOG_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_log_path()) 
    {
        let _ = file.write_all(log_line.as_bytes());
        let _ = file.flush();
    }
}

pub fn process_thumbnail(url: Option<&str>) -> Option<String> {
    match url {
        Some(u) if !u.is_empty() && u != "null" && u != "/assets/icon.png" && u != "/assets/music.png" => Some(u.to_string()),
        _ => None,
    }
}

pub fn format_overlay_track(mut data: TrackUpdate) -> TrackUpdate {
    data.thumbnail = process_thumbnail(data.thumbnail.as_deref());
    // 🚀 UNIFIED IDLE STRINGS: Ensure the UI always has consistent text for idle states
    if data.status == "idle" {
        if data.details.is_none() { data.details = Some("Resting...".to_string()); }
        if data.state.is_none() { data.state = Some("Browsing for music".to_string()); }
    }
    data
}

pub fn format_rpc_track(data: TrackUpdate) -> TrackUpdate {
    data
}

#[cfg(windows)]
pub fn get_pairing_code() -> String {
    let title = "T_Music_Bot RPC Setup";
    let ps_script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; [Windows.Forms.Application]::EnableVisualStyles(); \
         $f=New-Object Windows.Forms.Form; $f.Text='{0}'; $f.Size=New-Object Drawing.Size(460,340); $f.StartPosition='CenterScreen'; $f.FormBorderStyle='FixedDialog'; $f.Topmost=$true; $f.Font=New-Object Drawing.Font('Segoe UI', 11); \
         $l1=New-Object Windows.Forms.Label; $l1.Text='Instructions:'; $l1.Font=New-Object Drawing.Font('Segoe UI', 11, [Drawing.FontStyle]::Bold); $l1.Location=New-Object Drawing.Point(25,25); $l1.AutoSize=$true; \
         $l2=New-Object Windows.Forms.Label; $l2.Text='1. Run [/rpc connect] in a Discord channel.' + [char]13 + [char]10 + '2. Paste the code given below.'; $l2.Size=New-Object Drawing.Size(400,60); $l2.Location=New-Object Drawing.Point(25,55); \
         $l3=New-Object Windows.Forms.Label; $l3.Text='Enter Code:'; $l3.Font=New-Object Drawing.Font('Segoe UI', 11, [Drawing.FontStyle]::Bold); $l3.Location=New-Object Drawing.Point(25,120); $l3.AutoSize=$true; \
         $t=New-Object Windows.Forms.TextBox; $t.Location=New-Object Drawing.Point(27,150); $t.Size=New-Object Drawing.Size(390,30); \
         $btnOk=New-Object Windows.Forms.Button; $btnOk.Text='Connect'; $btnOk.Location=New-Object Drawing.Point(170,220); $btnOk.Size=New-Object Drawing.Size(120,45); $btnOk.DialogResult=1; \
         $btnCan=New-Object Windows.Forms.Button; $btnCan.Text='Cancel'; $btnCan.Location=New-Object Drawing.Point(300,220); $btnCan.Size=New-Object Drawing.Size(120,45); $btnCan.DialogResult=2; \
         $f.Controls.AddRange(@($l1,$l2,$l3,$t,$btnOk,$btnCan)); $f.Activate(); if($f.ShowDialog()-eq1){{$t.Text}}else{{'CANCELLED'}}",
        title
    );
    use std::os::windows::process::CommandExt;
    let output = Command::new("powershell")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script])
        .output();
    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "CANCELLED".to_string(),
    }
}

#[cfg(not(windows))]
pub fn get_pairing_code() -> String {
    let title = "T_Music_Bot RPC Setup";
    let output = Command::new("zenity").args(["--entry", "--title", title, "--text", "Paste Pairing Code:"]).output();
    let result = match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => {
            let mut input = String::new();
            let _ = std::io::stdin().read_line(&mut input);
            input.trim().to_string()
        }
    };
    if result.is_empty() { "CANCELLED".to_string() } else { result }
}

pub fn show_error_popup(msg: &str) {
    if cfg!(target_os = "windows") {
        let ps_script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{}', 'T_Music_Bot RPC - Info', 'OK', 'Information')",
            msg.replace("'", "''")
        );
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let _ = Command::new("powershell")
                .creation_flags(0x08000000)
                .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script])
                .output();
        }
        #[cfg(not(windows))]
        {
            let _ = Command::new("powershell").args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script]).output();
        }
    } else if cfg!(target_os = "macos") {
        let script = format!("display dialog \"{}\" buttons {{\"OK\"}} default button \"OK\" with title \"T_Music_Bot RPC\"", msg.replace("\"", "\\\""));
        let _ = Command::new("osascript").args(["-e", &script]).output();
    } else {
        let _ = Command::new("zenity").args(["--info", "--title", "T_Music_Bot RPC", "--text", msg]).output();
    }
}

pub fn create_desktop_shortcut() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let exe_path = std::env::current_exe().unwrap_or_default();
        if exe_path.as_os_str().is_empty() { return; }
        let ps_script = format!(
            "$s=New-Object -ComObject WScript.Shell; $d=[System.Environment]::GetFolderPath('Desktop'); $l=$s.CreateShortcut(\"$d\\T_Music_Bot RPC.lnk\"); $l.TargetPath='{}'; $l.Save();",
            exe_path.to_str().unwrap_or_default().replace("\\", "\\\\")
        );
        let _ = Command::new("powershell")
            .creation_flags(0x08000000)
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script])
            .output();
    }
}

pub fn create_start_menu_shortcut() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let exe_path = std::env::current_exe().unwrap_or_default();
        if exe_path.as_os_str().is_empty() { return; }
        let ps_script = format!(
            "$s=New-Object -ComObject WScript.Shell; $m=[System.Environment]::GetFolderPath('StartMenu'); $l=$s.CreateShortcut(\"$m\\T_Music_Bot RPC.lnk\"); $l.TargetPath='{}'; $l.Save();",
            exe_path.to_str().unwrap_or_default().replace("\\", "\\\\")
        );
        let _ = Command::new("powershell")
            .creation_flags(0x08000000)
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script])
            .output();
    }
}

/// `true` when a published release version is strictly newer than ours.
///
/// Components are compared numerically (punctuation and non-numeric
/// pre-release parts are ignored). A plain `!=` comparison made every
/// local/dev build re-download the older published binary on startup.
pub(crate) fn is_newer_release(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.split(['.', '-', '_'])
            .filter_map(|p| p.trim().parse::<u64>().ok())
            .collect()
    };
    parse(latest) > parse(current)
}

pub fn check_for_updates() {
    std::thread::spawn(|| {
        let url = "https://api.github.com/repos/T-Bot-Team/t-music-bot-rpc/releases/latest";
        if let Ok(resp) = minreq::get(url).with_header("User-Agent", "T_Music_Bot-RPC").send() {
            if let Ok(json) = resp.json::<serde_json::Value>() {
                let latest_ver = json["tag_name"].as_str().unwrap_or("").trim_start_matches('v');
                let current_ver = env!("CARGO_PKG_VERSION");

                if !latest_ver.is_empty() && is_newer_release(latest_ver, current_ver) {
                    log_message(&format!("[Updater] Update found: v{} (Current: v{}).", latest_ver, current_ver));
                    
                    let platform_name = if cfg!(windows) {
                        "T_Music_Bot_RPC.exe"
                    } else if cfg!(target_os = "macos") {
                        if cfg!(target_arch = "aarch64") {
                            "T_Music_Bot-RPC_Apple_Silicon"
                        } else {
                            "T_Music_Bot-RPC_Intel"
                        }
                    } else {
                        "T_Music_Bot-RPC"
                    };

                    let asset = json["assets"].as_array().and_then(|a| {
                        a.iter().find(|&asst| {
                            let name = asst["name"].as_str().unwrap_or("");
                            name == platform_name || (name.contains(platform_name) && !name.ends_with(".zip") && !name.ends_with(".tar.gz") && !name.ends_with(".dmg"))
                        })
                    });

                    if let Some(asset) = asset {
                        let download_url = asset["browser_download_url"].as_str().unwrap_or("");
                        if !download_url.is_empty() {
                            log_message("[Updater] Downloading new binary...");
                            if let Ok(bin_resp) = minreq::get(download_url).with_header("User-Agent", "T_Music_Bot-RPC").send() {
                                let bytes = bin_resp.as_bytes();
                                if bin_resp.status_code == 200 && !bytes.is_empty() {
                                    let tmp_path = std::env::temp_dir().join(format!("t_music_bot_rpc_update_{}", std::process::id()));
                                    if std::fs::write(&tmp_path, bytes).is_ok() {
                                        #[cfg(unix)]
                                        {
                                            use std::os::unix::fs::PermissionsExt;
                                            let _ = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755));
                                        }

                                        match self_replace::self_replace(&tmp_path) {
                                            Ok(_) => {
                                                let _ = std::fs::remove_file(&tmp_path);
                                                log_message("[Updater] Download complete and verified. Restart to apply update.");
                                            }
                                            Err(e) => {
                                                let _ = std::fs::remove_file(&tmp_path);
                                                log_message(&format!("[Updater] Failed to apply update: {}. Keeping current binary.", e));
                                            }
                                        }
                                    }
                                } else {
                                    log_message(&format!("[Updater] Download failed with status {}. Keeping current binary.", bin_resp.status_code));
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

pub fn check_lock(port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    if std::net::TcpStream::connect(&addr).is_ok() {
        let msg = format!("T_Music_Bot RPC is already running on port {}. Please close the other instance.", port);
        show_error_popup(&msg);
        std::process::exit(1);
    }
}

/// Turns a raw ALSA device string into something a human can read.
///
/// Only Linux has ALSA-style identifiers; everywhere else the name is passed
/// through unchanged. The original id is kept separate by the visualizer so
/// the stored setting stays stable — this is purely for display.
pub fn format_alsa_device_name(name: &str) -> String {
    #[cfg(target_os = "linux")]
    {
        if name == "default" {
            return "Default".to_string();
        }
        if name == "pipewire" {
            return "PipeWire".to_string();
        }
        if name == "pulse" {
            return "PulseAudio".to_string();
        }
        if name == "jack" {
            return "JACK".to_string();
        }

        if name.contains(':') {
            let parts: Vec<&str> = name.split(':').collect();
            let dev_type = parts[0];
            let rest = parts[1];

            let mut card_name = "Unknown";
            let mut dev_num = None;

            for param in rest.split(',') {
                if let Some(rest) = param.strip_prefix("CARD=") {
                    card_name = rest;
                } else if let Some(rest) = param.strip_prefix("DEV=") {
                    dev_num = Some(rest);
                }
            }

            let type_label = match dev_type {
                "sysdefault" => "System Default",
                "front" => "Front Speakers",
                "surround40" => "Surround 4.0",
                "surround51" => "Surround 5.1",
                "surround71" => "Surround 7.1",
                "hdmi" => "HDMI Output",
                "iec958" => "S/PDIF Digital Output",
                "dmix" => "Direct Mixer",
                other => other,
            };

            let formatted_card = card_name.replace('_', " ");

            let formatted = if let Some(dev) = dev_num {
                format!("{} ({}, Device {})", type_label, formatted_card, dev)
            } else {
                format!("{} ({})", type_label, formatted_card)
            };
            return formatted;
        }
    }
    name.to_string()
}

#[cfg(test)]
mod device_name_tests {
    use super::format_alsa_device_name;

    #[cfg(target_os = "linux")]
    #[test]
    fn known_aliases_stay_short() {
        assert_eq!(format_alsa_device_name("default"), "Default");
        assert_eq!(format_alsa_device_name("pulse"), "PulseAudio");
        assert_eq!(format_alsa_device_name("pipewire"), "PipeWire");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn alsa_pcm_strings_become_readable() {
        let f = format_alsa_device_name(
            "front:CARD=Generic,DEV=0",
        );
        assert_eq!(f, "Front Speakers (Generic, Device 0)");
        let hdmi = format_alsa_device_name("hdmi:CARD=HDMI,DEV=0");
        assert_eq!(hdmi, "HDMI Output (HDMI, Device 0)");
    }

    #[test]
    fn passthrough_for_plain_names() {
        assert_eq!(format_alsa_device_name("USB Microphone"), "USB Microphone");
    }

    #[test]
    fn updater_never_downgrades() {
        use super::is_newer_release;
        // Published 2.0.3 vs local 2.0.4: not an update.
        assert!(!is_newer_release("2.0.3", "2.0.4"));
        assert!(!is_newer_release("2.0.4", "2.0.4"));
        // Genuinely newer releases are picked up.
        assert!(is_newer_release("2.0.5", "2.0.4"));
        assert!(is_newer_release("2.1.0", "2.0.9"));
        assert!(is_newer_release("10.0.0", "9.9.9"));
        // Malformed / pre-release-ish inputs don't trigger a downgrade.
        assert!(!is_newer_release("2.0", "2.0.4"));
        assert!(!is_newer_release("nightly", "2.0.4"));
    }
}
