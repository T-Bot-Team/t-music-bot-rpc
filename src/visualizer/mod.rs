pub mod capture;
pub mod processor;

use crate::{AppState, info};
use cpal::traits::{DeviceTrait, HostTrait};
use tokio::task::JoinHandle;

pub async fn start_visualizer(state: AppState) -> Option<JoinHandle<()>> {
    let config = {
        let s = state.read().await;
        s.settings.as_ref().unwrap().overlay.visualizer.clone()
    };

    if !config.enabled { return None; }

    let host = cpal::default_host();
    let device_name = &config.audio_device;
    
    let mut selected_device = None;
    let mut is_loopback = false;

    if device_name == "default" || device_name.is_empty() {
        selected_device = host.default_output_device();
        is_loopback = true;
    } else {
        if let Ok(devices) = host.output_devices() {
            for d in devices {
                if let Ok(name) = d.name() {
                    if name.contains(device_name) {
                        selected_device = Some(d);
                        is_loopback = true;
                        break;
                    }
                }
            }
        }
        
        if selected_device.is_none() {
            if let Ok(devices) = host.input_devices() {
                for d in devices {
                    if let Ok(name) = d.name() {
                        if name.contains(device_name) {
                            selected_device = Some(d);
                            is_loopback = false;
                            break;
                        }
                    }
                }
            }
        }
    }

    let device = selected_device.or_else(|| {
        info!("[Visualizer] Device '{}' not found. Fallback to Default.", device_name);
        is_loopback = true;
        host.default_output_device()
    });

    if let Some(device) = device {
        let actual_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        info!("[Visualizer] Bound to: {}", actual_name);

        let supported_config = if is_loopback {
            device.default_output_config().expect("Output config failed")
        } else {
            device.default_input_config().expect("Input config failed")
        };

        let sample_format = supported_config.sample_format();
        let stream_config: cpal::StreamConfig = supported_config.into();
        let sr = stream_config.sample_rate.0 as f32;
        let channels = stream_config.channels as usize;

        let tx = { state.read().await.overlay_tx.clone() };
        let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(128);
        
        // 🚀 THE FIX: Use a channel to keep the thread alive and kill it on command
        let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel::<()>(1);

        std::thread::spawn(move || {
            let _stream = capture::start_capture(&device, &stream_config, sample_format, audio_tx);
            let cfg_clone = config.clone();
            
            // Start the audio processor
            std::thread::spawn(move || {
                processor::run_processor(cfg_clone, audio_rx, tx, sr, channels);
            });

            // Block this thread until stop_tx is dropped
            let _ = stop_rx.blocking_recv();
            info!("[Visualizer] Capture thread stopped.");
        });

        // Return a task that holds the Sender. When aborted, Sender drops, Thread stops.
        Some(tokio::spawn(async move {
            let _keep_alive = stop_tx;
            std::future::pending::<()>().await;
        }))
    } else {
        info!("[Visualizer] FATAL: No audio device available.");
        None
    }
}
