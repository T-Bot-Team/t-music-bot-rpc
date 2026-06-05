pub mod capture;
pub mod processor;
pub mod fft;
pub mod smoothing;

use crate::AppState;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
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

        let tx = { state.read().await.viz_tx.clone() };
        let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(128);
        
        let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel::<()>(1);
        let (done_tx, mut done_rx) = tokio::sync::mpsc::channel::<()>(1);

        let state_clone = state.clone();
        std::thread::spawn(move || {
            let stream = capture::start_capture(&device, &stream_config, sample_format, audio_tx);
            let cfg_clone = config.clone();
            let tx_proc = tx.clone();
            
            let s_proc = state_clone.clone();
            let processor_handle = std::thread::spawn(move || {
                processor::run_processor(cfg_clone, audio_rx, tx_proc, sr, channels, s_proc);
            });

            let mut is_paused = false;
            loop {
                match stop_rx.try_recv() {
                    Ok(_) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
                }

                // 🚀 SMART CHECK: Wake up if music is playing OR if anyone is watching the overlay
                let (listeners, is_playing) = {
                    let s = state_clone.blocking_read();
                    let is_p = s.last_track.as_ref().map(|t| t.status == "playing").unwrap_or(false);
                    (s.viz_tx.receiver_count(), is_p)
                };

                let should_pause = listeners == 0 || !is_playing;

                if should_pause && !is_paused {
                    let _ = stream.pause();
                    is_paused = true;
                } else if !should_pause && is_paused {
                    let _ = stream.play();
                    is_paused = false;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            
            drop(stream);
            let _ = processor_handle.join();
            let _ = done_tx.blocking_send(());
        });

        Some(tokio::spawn(async move {
            let _keep_alive = stop_tx;
            tokio::select! {
                _ = done_rx.recv() => {},
                _ = tokio::signal::ctrl_c() => {},
            }
        }))
    } else {
        info!("[Visualizer] FATAL: No audio device available.");
        None
    }
}
