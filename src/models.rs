use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use serde_json::Value;

// --- CONFIGURATION MODELS ---

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_color_top() -> String { "#7cf6ff".to_string() }
fn default_color_bottom() -> String { "#1a69a8".to_string() }
fn default_animation_speed() -> f32 { 1.0 }
fn default_fps() -> u32 { 60 }
fn default_bg_color() -> String { "rgba(0,0,0,0.85)".to_string() }
fn default_status() -> String { "idle".to_string() }
fn default_viz_type() -> String { "logarithmic".to_string() }
fn default_smoothing() -> u32 { 6 }
fn default_device() -> String { "default".to_string() }
fn default_samples() -> u32 { 4096 }
fn default_bars() -> u32 { 64 }
fn default_sensitivity() -> u32 { 45 }
fn default_multiplier() -> u32 { 25 }
fn default_reaction() -> u32 { 3 }
fn default_fluidity() -> f32 { 0.15 }
fn default_bar_width() -> u32 { 10 }
fn default_bar_gap() -> u32 { 5 }
fn default_mode() -> String { "bars".to_string() }
fn default_layout() -> String { "full".to_string() }
fn default_thumb_bg_opacity() -> f32 { 0.3 }
fn default_port() -> u16 { 3000 }
fn default_bg_opacity() -> f32 { 0.85 }
fn default_thumb_opacity() -> f32 { 0.5 }
fn default_ipc_pipe() -> i32 { -1 }
fn default_text_anim_mode() -> String { "dynamic".to_string() }
fn default_text_anim_duration() -> u32 { 10 }

fn default_text_style() -> GradientConfig {
    GradientConfig {
        is_static: true,
        random: false,
        top: "#ffffff".to_string(),
        bottom: "#ffffff".to_string(),
    }
}

fn default_border_style() -> GradientConfig {
    GradientConfig {
        is_static: true,
        random: false,
        top: "rgba(255,255,255,0.15)".to_string(),
        bottom: "rgba(255,255,255,0.15)".to_string(),
    }
}

fn default_element_style() -> GradientConfig {
    GradientConfig {
        is_static: true,
        random: false,
        top: "rgba(255,255,255,0.15)".to_string(),
        bottom: "rgba(255,255,255,0.15)".to_string(),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GradientConfig {
    #[serde(default = "default_false")]
    pub is_static: bool, 
    #[serde(default = "default_false")]
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
            random: false,
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
    #[serde(default = "default_false")]
    pub debug_fps: bool,
    #[serde(default = "default_device")]
    pub audio_device: String,
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
    #[serde(default = "default_viz_type")]
    pub visualizer_type: String,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debug_fps: false,
            audio_device: default_device(),
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
            visualizer_type: default_viz_type(),
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
    #[serde(default = "default_true")]
    pub enable_text_animation: bool,
    #[serde(default = "default_text_anim_mode")]
    pub text_animation_mode: String,
    #[serde(default = "default_text_anim_duration")]
    pub text_animation_duration: u32,
    #[serde(default = "default_text_style")]
    pub text_style: GradientConfig,
    #[serde(default = "default_border_style")]
    pub border_style: GradientConfig,
    #[serde(default = "default_element_style")]
    pub element_style: GradientConfig,
    #[serde(default)]
    pub background: BackgroundConfig,
    #[serde(default)]
    pub visualizer: VisualizerConfig,
    #[serde(default = "default_true")]
    pub show_thumbnail_background: bool,
    #[serde(default = "default_thumb_bg_opacity")]
    pub thumbnail_background_opacity: f32,
}

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
            enable_text_animation: true,
            text_animation_mode: default_text_anim_mode(),
            text_animation_duration: default_text_anim_duration(),
            text_style: GradientConfig {
                is_static: true,
                random: false,
                top: "#ffffff".to_string(),
                bottom: "#ffffff".to_string(),
            },
            border_style: GradientConfig {
                is_static: true,
                random: false,
                top: "rgba(255,255,255,0.15)".to_string(),
                bottom: "rgba(255,255,255,0.15)".to_string(),
            },
            element_style: GradientConfig {
                is_static: true,
                random: false,
                top: "rgba(255,255,255,0.15)".to_string(),
                bottom: "rgba(255,255,255,0.15)".to_string(),
            },
            background: BackgroundConfig::default(),
            visualizer: VisualizerConfig::default(),
            show_thumbnail_background: false,
            thumbnail_background_opacity: default_thumb_bg_opacity(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RpcConfig {
    #[serde(default = "default_false")]
    pub swap_rpc_lines: bool,
    #[serde(default = "default_ipc_pipe")]
    pub ipc_pipe: i32,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            swap_rpc_lines: false,
            ipc_pipe: -1,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub code: Option<String>,
    pub user_id: Option<String>,
    pub session_token: Option<String>,
    #[serde(default)]
    pub saved_codes: std::collections::HashMap<String, String>,
    pub overlay: OverlayConfig,
    #[serde(default)]
    pub rpc: RpcConfig,
    #[serde(default)]
    pub allow_firewall: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            code: Some(String::new()),
            user_id: Some(String::new()),
            session_token: None,
            saved_codes: std::collections::HashMap::new(),
            overlay: OverlayConfig::default(),
            rpc: RpcConfig::default(),
            allow_firewall: false,
        }
    }
}

// --- DATA TRANSFER MODELS ---

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackUpdate {
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(alias = "details")]
    pub details: Option<String>,
    #[serde(alias = "state")]
    pub state: Option<String>,
    #[serde(alias = "start_timestamp", alias = "start")]
    pub start_timestamp: Option<f64>,
    #[serde(alias = "end_timestamp", alias = "end")]
    pub end_timestamp: Option<f64>,
    #[serde(alias = "thumbnail", alias = "image")]
    pub thumbnail: Option<String>,
    #[serde(alias = "detailsUrl", alias = "details_url")]
    pub details_url: Option<String>,
    #[serde(alias = "paused")]
    pub paused: Option<bool>,
    #[serde(alias = "position")]
    pub position: Option<f64>,
    #[serde(alias = "large_image_key")]
    pub large_image_key: Option<String>,
    #[serde(alias = "large_image_text")]
    pub large_image_text: Option<String>,
    #[serde(alias = "small_image_key")]
    pub small_image_key: Option<String>,
    #[serde(alias = "small_image_text")]
    pub small_image_text: Option<String>,
    #[serde(alias = "playback_speed")]
    pub playback_speed: Option<f32>,
}

pub enum RpcCommand {
    Update { track: TrackUpdate, rpc_raw: Value, is_transition: bool },
    Connect(String),
    Refresh,
}

// --- APPLICATION STATE ---

pub struct GlobalState {
    pub is_shutting_down: bool,
    pub last_track: Option<TrackUpdate>,
    pub last_rpc_raw: Option<Value>,
    pub settings: Option<Settings>,
    pub overlay_tx: broadcast::Sender<String>,
    pub viz_tx: broadcast::Sender<String>,
    pub rpc_tx: Option<std::sync::mpsc::Sender<RpcCommand>>,
    pub ws_status: String,
    pub rpc_status: String,
    pub arrpc_detected: bool,
    pub active_pipe: Option<i32>,
    pub discovery_cache: Option<(std::time::Instant, Vec<Value>)>,
}

pub type AppState = Arc<RwLock<GlobalState>>;
