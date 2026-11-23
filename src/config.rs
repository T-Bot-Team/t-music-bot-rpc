use crate::models::Settings;
use std::fs;

pub fn load_settings() -> Settings {
    let settings_path = crate::utils::get_app_dir().join("settings.json");
    if settings_path.exists() {
        match fs::read_to_string(&settings_path) {
            Ok(content) => serde_json::from_str::<Settings>(&content).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    } else {
        let settings = Settings::default();
        if let Ok(content) = serde_json::to_string_pretty(&settings) {
            let _ = fs::write(&settings_path, content);
        }
        settings
    }
}

pub fn save_settings(settings: &Settings) {
    let settings_path = crate::utils::get_app_dir().join("settings.json");
    if let Ok(content) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(settings_path, content);
    }
}
