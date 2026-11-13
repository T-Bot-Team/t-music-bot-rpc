pub mod backend;
pub mod processor;
pub mod fft;
pub mod smoothing;

use crate::{AppState, VisualizerConfig};
use tokio::task::JoinHandle;

/// Starts the visualizer pipeline: capture -> FFT processor -> WebSocket broadcast.
///
/// The returned task owns the capture. Aborting it (or it returning) tears
/// everything down: dropping the capture releases the device, and closing the
/// sample channel ends the processor thread.
pub async fn start_visualizer(state: AppState) -> Option<JoinHandle<()>> {
    let config: VisualizerConfig = {
        let s = state.read().await;
        s.settings.as_ref()?.overlay.visualizer.clone()
    };

    if !config.enabled {
        return None;
    }

    let viz_tx = state.read().await.viz_tx.clone();
    let audio = state
        .read()
        .await
        .settings
        .as_ref()
        .map(|s| s.audio.clone())
        .unwrap_or_default();
    let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel::<Vec<f32>>(128);

    // When the client manages the virtual bus, the visualizer records the
    // bus monitor (kept at full scale) instead of the system default output,
    // so turning the music down never flattens the bars.
    let device_name = config.audio_device.clone();
    #[cfg(target_os = "linux")]
    let device_name = if audio.manage_bus && backend::is_default_request(&device_name) {
        crate::audio_bus::monitor_device()
    } else {
        device_name
    };
    let manage_bus = audio.manage_bus;
    let listen_volume = audio.listen_volume.min(150) as u8;

    let proc_config = config.clone();
    let proc_state = state.clone();

    Some(tokio::spawn(async move {
        let opened = tokio::task::spawn_blocking(move || {
            #[cfg(target_os = "linux")]
            if manage_bus {
                if let Err(e) = crate::audio_bus::ensure(listen_volume) {
                    info!("[AudioBus] Could not prepare the audio bus before capture: {}", e);
                }
            }
            #[cfg(not(target_os = "linux"))]
            let _ = (manage_bus, listen_volume);
            backend::open_capture(&device_name, audio_tx)
        })
        .await;

        let (mut capture, sr, channels, bound_name, bound_id) = match opened {
            Ok(Ok(o)) => (o.capture, o.sample_rate, o.channels, o.bound_name, o.bound_id),
            Ok(Err(e)) => {
                info!("[Visualizer] Capture failed: {}", e);
                return;
            }
            Err(e) => {
                info!("[Visualizer] Capture task failed: {}", e);
                return;
            }
        };
        info!(
            "[Visualizer] Bound to: {} ({} Hz, {} ch)",
            crate::visualizer::backend::format_label(&bound_name, &bound_id),
            sr,
            channels
        );

        // Detached on purpose: the thread exits when audio_tx closes.
        let _processor_handle = std::thread::Builder::new()
            .name("viz-processor".into())
            .spawn(move || {
                processor::run_processor(proc_config, audio_rx, viz_tx, sr as f32, channels as usize, proc_state);
            });

        // Smart suspend: pause capture when nobody is listening or nothing is playing.
        let mut paused = false;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let (listeners, is_playing) = {
                let s = state.read().await;
                let playing = s.last_track.as_ref().map(|t| t.status == "playing").unwrap_or(false);
                (s.viz_tx.receiver_count(), playing)
            };

            let should_pause = listeners == 0 || !is_playing;
            if should_pause && !paused {
                capture.pause();
                paused = true;
            } else if !should_pause && paused {
                capture.resume();
                paused = false;
            }
        }
    }))
}
