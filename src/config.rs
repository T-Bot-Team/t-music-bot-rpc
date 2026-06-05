use crate::models::Settings;
use std::fs;

pub fn load_settings() -> Settings {
    let settings_path = crate::utils::get_app_dir().join("settings.json");
    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path).expect("Could not read settings file");
        match serde_json::from_str::<Settings>(&content) {
            Ok(settings) => settings,
            Err(_) => Settings::default()
        }
    } else {
        let settings = Settings::default();
        let content = serde_json::to_string_pretty(&settings).unwrap();
        let _ = fs::write(&settings_path, content);
        settings
    }
}

pub fn save_settings(settings: &Settings) {
    let settings_path = crate::utils::get_app_dir().join("settings.json");
    if let Ok(content) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(settings_path, content);
    }
}
