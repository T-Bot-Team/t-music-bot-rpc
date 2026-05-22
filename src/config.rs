use crate::models::Settings;
use std::fs;
use std::path::Path;

pub fn load_settings() -> Settings {
    let local_settings = Path::new("settings.json");
    if local_settings.exists() {
        let content = fs::read_to_string(local_settings).expect("Could not read settings file");
        match serde_json::from_str::<Settings>(&content) {
            Ok(settings) => settings,
            Err(_) => Settings::default()
        }
    } else {
        let settings = Settings::default();
        let content = serde_json::to_string_pretty(&settings).unwrap();
        let _ = fs::write(local_settings, content);
        settings
    }
}

pub fn save_settings(settings: &Settings) {
    if let Ok(content) = serde_json::to_string_pretty(settings) {
        let _ = fs::write("settings.json", content);
    }
}
