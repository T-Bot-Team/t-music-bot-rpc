use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_color_top() -> String { "#7cf6ff".to_string() }
fn default_color_bottom() -> String { "#1a69a8".to_string() }
fn default_animation_speed() -> f32 { 1.0 }
fn default_fps() -> u32 { 60 }
fn default_bg_color() -> String { "rgba(0,0,0,0.85)".to_string() }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GradientConfig {
    #[serde(default = "default_false")]
    pub is_static: bool, 
    #[serde(default = "default_true")]
    pub random: bool,
    #[serde(default = "default_color_top")]
    pub top: String,
    #[serde(default = "default_color_bottom")]
    pub bottom: String,
}

impl Default for GradientConfig {
    fn default() -> Self {
        Self {
            is_static: false,
            random: true,
            top: default_color_top(),
            bottom: default_color_bottom(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_bg_color")]
    pub color: String,
    #[serde(default = "default_false")]
    pub use_gradient: bool,
    #[serde(default)]
    pub gradient: GradientConfig,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            color: default_bg_color(),
            use_gradient: false,
            gradient: GradientConfig::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VisualizerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub debug_fps: bool,
    #[serde(default = "default_device")]
    pub audio_device: String,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    #[serde(default = "default_samples")]
    pub samples: u32,
    #[serde(default = "default_bars")]
    pub bars: u32,
    #[serde(default = "default_smoothing")]
    pub smoothing: u32,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: u32,
    #[serde(default = "default_multiplier")]
    pub multiplier: u32,
    #[serde(default = "default_reaction")]
    pub audio_reaction_delay: u32,
    #[serde(default = "default_fluidity")]
    pub visual_fluidity: f32,
    #[serde(default = "default_bar_width")]
    pub bar_width: u32,
    #[serde(default = "default_bar_gap")]
    pub bar_gap: u32,
    #[serde(default)]
    pub gradient: GradientConfig,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_false")]
    pub glow: bool,
    #[serde(default = "default_true")]
    pub rounded: bool,
    #[serde(default = "default_true")]
    pub toggle_animation: bool,
    #[serde(default = "default_true")]
    pub horizontal_smoothing: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f32,
    #[serde(default = "default_fps")]
    pub fps: u32,
}

fn default_smoothing() -> u32 { 6 }
fn default_device() -> String { "default".to_string() }
fn default_sample_rate() -> u32 { 48000 }
fn default_samples() -> u32 { 8192 }
fn default_bars() -> u32 { 96 }
fn default_sensitivity() -> u32 { 40 }
fn default_multiplier() -> u32 { 40 }
fn default_reaction() -> u32 { 3 }
fn default_fluidity() -> f32 { 0.25 }
fn default_bar_width() -> u32 { 10 }
fn default_bar_gap() -> u32 { 5 }
fn default_mode() -> String { "bars".to_string() }

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debug_fps: true,
            audio_device: default_device(),
            sample_rate: default_sample_rate(),
            samples: default_samples(),
            bars: default_bars(),
            smoothing: default_smoothing(),
            sensitivity: default_sensitivity(),
            multiplier: default_multiplier(),
            audio_reaction_delay: default_reaction(),
            visual_fluidity: default_fluidity(),
            bar_width: default_bar_width(),
            bar_gap: default_bar_gap(),
            gradient: GradientConfig::default(),
            mode: default_mode(),
            glow: false,
            rounded: true,
            toggle_animation: true,
            horizontal_smoothing: true,
            animation_speed: default_animation_speed(),
            fps: 60,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OverlayConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_layout")]
    pub layout: String,
    #[serde(default = "default_true")]
    pub show_background: bool,
    #[serde(default = "default_bg_opacity")]
    pub background_opacity: f32,
    #[serde(default = "default_thumb_opacity")]
    pub thumbnail_opacity: f32,
    #[serde(default = "default_false")]
    pub center_text: bool,
    #[serde(default = "default_false")]
    pub global_sync: bool,
    #[serde(default = "default_text_color")]
    pub text_color: String,
    #[serde(default = "default_border_color")]
    pub border_color: String, 
    #[serde(default = "default_element_color")]
    pub element_color: String, 
    #[serde(default)]
    pub background: BackgroundConfig,
    #[serde(default)]
    pub visualizer: VisualizerConfig,
    #[serde(default = "default_true")]
    pub show_thumbnail_background: bool,
    #[serde(default = "default_thumb_bg_opacity")]
    pub thumbnail_background_opacity: f32,
}

fn default_layout() -> String { "full".to_string() }
fn default_border_color() -> String { "transparent".to_string() }
fn default_element_color() -> String { "rgba(255,255,255,0.15)".to_string() }
fn default_thumb_bg_opacity() -> f32 { 0.3 }
fn default_port() -> u16 { 3000 }
fn default_bg_opacity() -> f32 { 0.85 }
fn default_thumb_opacity() -> f32 { 0.5 }
fn default_text_color() -> String { "#ffffff".to_string() }

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: default_port(),
            layout: default_layout(),
            show_background: true,
            background_opacity: default_bg_opacity(),
            thumbnail_opacity: default_thumb_opacity(),
            center_text: false,
            global_sync: false,
            text_color: default_text_color(),
            border_color: default_border_color(),
            element_color: default_element_color(),
            background: BackgroundConfig::default(),
            visualizer: VisualizerConfig::default(),
            show_thumbnail_background: false,
            thumbnail_background_opacity: default_thumb_bg_opacity(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub code: Option<String>,
    pub user_id: Option<String>,
    pub overlay: OverlayConfig,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            code: Some(String::new()),
            user_id: Some(String::new()),
            overlay: OverlayConfig::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TrackUpdate {
    pub details: Option<String>,
    pub state: Option<String>,
    #[serde(alias = "start_timestamp")]
    pub start_timestamp: Option<f64>,
    #[serde(alias = "end_timestamp")]
    pub end_timestamp: Option<f64>,
    pub thumbnail: Option<String>,
    pub paused: Option<bool>,
    pub position: Option<f64>,
    pub large_image_key: Option<String>,
    pub large_image_text: Option<String>,
    pub small_image_key: Option<String>,
    pub small_image_text: Option<String>,
}

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
