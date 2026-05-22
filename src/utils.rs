pub mod firewall;
use crate::models::TrackUpdate;
use std::process::Command;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use cpal::traits::{DeviceTrait, HostTrait};

pub fn get_log_path() -> PathBuf {
    std::env::current_exe()
        .map(|p| p.parent().unwrap().join("latest.log"))
        .unwrap_or_else(|_| PathBuf::from("latest.log"))
}

use std::sync::Mutex;

static LOG_MUTEX: Mutex<()> = Mutex::new(());

pub fn log_message(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_line = format!("[{}] {}\n", timestamp, msg);
    print!("{}", log_line);
    
    let _lock = LOG_MUTEX.lock().unwrap();
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_log_path()) 
    {
        let _ = file.write_all(log_line.as_bytes());
        let _ = file.flush();
    }
}

use std::collections::HashSet;

pub fn get_audio_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    
    if let Ok(devices) = host.output_devices() {
        for d in devices {
            if let Ok(name) = d.name() {
                if seen.insert(name.clone()) {
                    names.push(name);
                }
            }
        }
    }
    names
}

pub fn process_thumbnail(url: Option<&str>) -> String {
    match url {
        Some(u) if !u.is_empty() && u != "null" => u.to_string(),
        _ => "/assets/music.png".to_string(),
    }
}

pub fn format_overlay_track(mut data: TrackUpdate) -> TrackUpdate {
    data.thumbnail = Some(process_thumbnail(data.thumbnail.as_deref()));
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

pub fn get_pairing_code() -> String {
    let title = "T_Music_Bot RPC Setup";
    if cfg!(target_os = "windows") {
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
        let output = {
            use std::os::windows::process::CommandExt;
            Command::new("powershell")
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script])
                .output()
                .expect("Failed to execute PowerShell")
        };
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        let output = Command::new("zenity").args(&["--entry", "--title", title, "--text", "Paste Pairing Code:"]).output().unwrap_or_else(|_| {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            let mut res = Command::new("echo").output().unwrap(); 
            res.stdout = input.into_bytes();
            res
        });
        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if result.is_empty() { "CANCELLED".to_string() } else { result }
    }
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
                .args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script])
                .output();
        }
        #[cfg(not(windows))]
        {
            let _ = Command::new("powershell").args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script]).output();
        }
    } else if cfg!(target_os = "macos") {
        let script = format!("display dialog \"{}\" buttons {{\"OK\"}} default button \"OK\" with title \"T_Music_Bot RPC\"", msg.replace("\"", "\\\""));
        let _ = Command::new("osascript").args(&["-e", &script]).output();
    } else {
        let _ = Command::new("zenity").args(&["--info", "--title", "T_Music_Bot RPC", "--text", msg]).output();
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
            .args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script])
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
            .args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script])
            .output();
    }
}

pub fn check_for_updates() {
    std::thread::spawn(|| {
        let url = "https://api.github.com/repos/T-Bot-Team/t-music-bot-rpc/releases/latest";
        if let Ok(resp) = minreq::get(url).with_header("User-Agent", "T_Music_Bot-RPC").send() {
            if let Ok(json) = resp.json::<serde_json::Value>() {
                let latest_ver = json["tag_name"].as_str().unwrap_or("").trim_start_matches('v');
                let current_ver = env!("CARGO_PKG_VERSION");

                if !latest_ver.is_empty() && latest_ver != current_ver {
                    log_message(&format!("[Updater] Update found: v{} (Current: v{}).", latest_ver, current_ver));
                    
                    let platform_name = if cfg!(windows) { "T_Music_Bot_RPC.exe" } else { "T_Music_Bot-RPC" };
                    let asset = json["assets"].as_array().and_then(|a| {
                        a.iter().find(|&asst| asst["name"].as_str().unwrap_or("").contains(platform_name))
                    });

                    if let Some(asset) = asset {
                        let download_url = asset["browser_download_url"].as_str().unwrap_or("");
                        if !download_url.is_empty() {
                            log_message("[Updater] Downloading new binary...");
                            if let Ok(bin_resp) = minreq::get(download_url).with_header("User-Agent", "T_Music_Bot-RPC").send() {
                                let bytes = bin_resp.as_bytes();
                                let tmp_path = std::env::temp_dir().join("t_music_bot_rpc_new");
                                if std::fs::write(&tmp_path, bytes).is_ok() {
                                    log_message("[Updater] Download complete. Restart to apply update.");
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
