#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        $crate::utils::log_message(&msg);
    }};
}

pub mod models;
pub mod config;
pub mod audio_bus;
pub mod utils;
pub mod server;
pub mod rpc;
pub mod visualizer;
pub mod tray;

pub use models::*;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
