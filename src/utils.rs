use crate::config::Settings;
use std::process::Command;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use cpal::traits::{DeviceTrait, HostTrait};

pub fn get_log_path() -> PathBuf {
    std::env::current_exe()
        .map(|p| p.parent().unwrap().join("latest.log"))
        .unwrap_or_else(|_| PathBuf::from("latest.log"))
}

pub fn log_message(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_line = format!("[{}] {}\n", timestamp, msg);
    print!("{}", log_line);
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_log_path()) 
    {
        let _ = file.write_all(log_line.as_bytes());
        let _ = file.flush();
    }
}

pub fn get_audio_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut names = Vec::new();
    
    if let Ok(devices) = host.output_devices() {
        for d in devices {
            if let Ok(name) = d.name() {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    names
}

pub fn process_thumbnail(url: Option<&str>) -> String {
    match url {
        Some(u) if u.contains("ytimg.com") => {
            let re = regex::Regex::new(r"/(?:default|hqdefault|sddefault|mqdefault)\.jpg").unwrap();
            re.replace(u, "/maxresdefault.jpg").to_string()
        }
        Some(u) if !u.is_empty() && u != "null" => u.to_string(),
        _ => "/assets/music.png".to_string(),
    }
}

pub fn format_overlay_track(mut data: crate::config::TrackUpdate) -> crate::config::TrackUpdate {
    data.thumbnail = Some(process_thumbnail(data.thumbnail.as_deref()));

    if let (Some(d), Some(s)) = (data.details.as_mut(), data.state.as_mut()) {
        let mut artist_name = s.clone();
        if artist_name.to_lowercase().starts_with("by ") {
            artist_name = artist_name[3..].trim().to_string();
        }
        let escaped_artist = regex::escape(&artist_name);
        let separators = vec!["\\s*-\\s*", "\\s*–\\s*", "\\s*—\\s*", ":\\s*", "\\|\\s*", "\\s*~\\s*", "\\s*by\\s+"];
        let sep_join = separators.join("|");
        
        let artist_regex = regex::Regex::new(&format!(r"(?i)(?:^|\s+){}(?:{})", escaped_artist, sep_join)).unwrap();
        *d = artist_regex.replace(d, "").trim().to_string();
        
        let suffix_regex = regex::Regex::new(&format!(r"(?i)(?:{})\s*{}\s*$", sep_join, escaped_artist)).unwrap();
        *d = suffix_regex.replace(d, "").trim().to_string();
        
        let junk_patterns = vec![
            r"(?i)\s*[\(\[].*?Mashup.*?[\)\]]\s*",
            r"(?i)\s*[\[\(\]]?(?:Copyright[\s-]*Freel?|Official Music Video|Official Video|Official Audio|Music Video|Lyrics|Audio|Video)[^\\]\)]*[\\]\)]?\s*",
            r"(?i)\s*[\(\[].*?(?:Video|Audio|Lyrics|Version|Remix).*?[\)\]]\s*",
            r"(?i)\[Copyright[\s-]*Freel?\]?",
            r"(?i)\s*No\.\s*\d+\s*",
        ];
        for p in junk_patterns {
            let re = regex::Regex::new(p).unwrap();
            *d = re.replace_all(d, " ").to_string();
        }
        let re_space = regex::Regex::new(r"\s\s+").unwrap();
        *d = re_space.replace_all(d, " ").trim().to_string();
    }
    data
}

pub fn format_rpc_track(mut data: crate::config::TrackUpdate) -> crate::config::TrackUpdate {
    if let (Some(d), Some(s)) = (data.details.as_mut(), data.state.as_mut()) {
        let mut artist_name = s.clone();
        if artist_name.to_lowercase().starts_with("by ") {
            artist_name = artist_name[3..].trim().to_string();
        }
        let escaped_artist = regex::escape(&artist_name);
        let separators = vec!["\\s*-\\s*", "\\s*–\\s*", "\\s*—\\s*", ":\\s*", "\\|\\s*", "\\s*~\\s*", "\\s*by\\s+"];
        let sep_join = separators.join("|");

        let artist_regex = regex::Regex::new(&format!(r"(?i)^{}\s*[-:|~]\s*", escaped_artist)).unwrap();
        *d = artist_regex.replace(d, "").trim().to_string();

        let suffix_regex = regex::Regex::new(&format!(r"(?i)(?:{})\s*{}\s*$", sep_join, escaped_artist)).unwrap();
        *d = suffix_regex.replace(d, "").trim().to_string();
        
        let junk_patterns = vec![
            r"(?i)\s*[\[\(\]]?(?:Copyright[\s-]*Freel?|Official Music Video|Official Video|Official Audio|Music Video|Lyrics|Audio|Video)[^\]\)]*[\]\)]?\s*",
            r"(?i)\s*[\(\[].*?(?:Video|Audio|Lyrics|Version|Remix).*?[\)\]]\s*",
            r"(?i)\[Copyright[\s-]*Freel?\]?",
            r"(?i)\s*No\.\s*\d+\s*",
        ];
        for p in junk_patterns {
            let re = regex::Regex::new(p).unwrap();
            *d = re.replace_all(d, " ").to_string();
        }
        
        let re_space = regex::Regex::new(r"\s\s+").unwrap();
        *d = re_space.replace_all(d, " ").trim().to_string();
    }
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
        let output = Command::new("powershell").args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script]).output().expect("Failed to execute PowerShell");
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
            "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{}', 'T_Music_Bot RPC - Error', 'OK', 'Error')",
            msg.replace("'", "''")
        );
        let _ = Command::new("powershell").args(&["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_script]).output();
    } else {
        let _ = Command::new("zenity").args(&["--error", "--title", "T_Music_Bot RPC - Error", "--text", msg]).output();
    }
}

pub fn save_settings(settings: &Settings) {
    if let Ok(content) = serde_json::to_string_pretty(settings) {
        let _ = fs::write("settings.json", content);
    }
}

pub fn check_lock(port: u16) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::CreateMutexW;
        use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
        
        let mutex_name_str = format!("Global\\T_Music_Bot_RPC_Port_{}\0", port);
        let mutex_name: Vec<u16> = mutex_name_str.encode_utf16().collect();
        unsafe {
            let handle = CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr());
            if handle == 0 {
                let local_name: Vec<u16> = format!("Local\\T_Music_Bot_RPC_Port_{}\0", port).encode_utf16().collect();
                let local_handle = CreateMutexW(std::ptr::null(), 1, local_name.as_ptr());
                if local_handle == 0 {
                    eprintln!("[Core] Fatal: Could not create system mutex.");
                    std::process::exit(1);
                }
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    eprintln!("[Core] Port {} is already being used by another instance. Exiting.", port);
                    std::process::exit(1);
                }
            } else if GetLastError() == ERROR_ALREADY_EXISTS {
                eprintln!("[Core] Port {} is already being used by another instance. Exiting.", port);
                std::process::exit(1);
            }
        }
    }

    #[cfg(unix)]
    {
        let lock_file = std::env::temp_dir().join(format!("t-music-bot-rpc-port-{}.lock", port));
        if lock_file.exists() {
            if let Ok(pid_str) = fs::read_to_string(&lock_file) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if pid != std::process::id() && std::path::Path::new(&format!("/proc/{}", pid)).exists() {
                        eprintln!("[Core] Another instance is already running (PID: {}). Exiting.", pid);
                        std::process::exit(1);
                    }
                }
            }
        }
        let _ = fs::write(&lock_file, std::process::id().to_string());
    }
}