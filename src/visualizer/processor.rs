use crate::{VisualizerConfig, AppState};
use std::sync::mpsc::Receiver;
use tokio::sync::broadcast::Sender;
use super::fft::FftProcessor;
use super::smoothing::VisualFilter;

pub fn run_processor(
    config: VisualizerConfig,
    audio_rx: Receiver<Vec<f32>>,
    tx_proc: Sender<String>,
    sr: f32,
    channels: usize,
    _state: AppState,
) {
    let win_size = config.samples as usize; 
    let bars = config.bars as usize;
    let target_len = win_size * channels;

    // Instantiate modular components
    let mut fft_processor = FftProcessor::new(win_size);
    let mut visual_filter = VisualFilter::new(bars);

    // Pre-allocate staging buffer
    let mut staging_buffer: Vec<f32> = Vec::with_capacity(target_len * 2);
    let mut silence_count = 0;
    
    // Temporal coefficients — derived from smoothing preset
    let smooth_val = (config.smoothing as f32).max(0.1);
    let target_fps = config.fps.max(15).min(240) as f32;
    
    // Exact Rainmeter k-constant formula: k = 0.01 ^ (1 / (FPS * TimeInSeconds))
    let calc_alpha = |ms: f32| -> f32 {
        if ms <= 0.0 { return 1.0; }
        let k = 0.01f32.powf(1.0 / (target_fps * (ms / 1000.0)));
        1.0 - k
    };

    let smooth_clamped = smooth_val.max(0.0).min(40.0);
    let attack_ms = smooth_clamped * 2.5;
    let decay_ms = 50.0 + smooth_clamped * 18.75;
    let alpha_ms = smooth_clamped * 4.0;
    let avg_size = 1 + (smooth_clamped * 0.75) as usize;

    let attack = calc_alpha(attack_ms);
    let decay = calc_alpha(decay_ms);
    let alpha = calc_alpha(alpha_ms);

    let sensitivity = config.sensitivity as f32;
    let multiplier = config.multiplier as f32;

    let frame_duration = std::time::Duration::from_secs_f32(1.0 / target_fps);
    let mut last_frame_time = std::time::Instant::now();

    while let Ok(data) = audio_rx.recv() {
        staging_buffer.extend(data);
        while let Ok(more_data) = audio_rx.try_recv() {
            staging_buffer.extend(more_data);
        }

        if staging_buffer.len() > target_len {
            let excess = staging_buffer.len() - target_len;
            staging_buffer.drain(0..excess);
        }

        if last_frame_time.elapsed() >= frame_duration {
            if staging_buffer.len() >= target_len {
                let current_samples = &staging_buffer[0..target_len];

                let mut max_amp = 0.0f32;
                for &s in current_samples {
                    if s.abs() > max_amp { max_amp = s.abs(); }
                }

                if max_amp < 0.001 { 
                    if silence_count < 5 { 
                        let _ = tx_proc.send(serde_json::json!({ "type": "fft_data", "bins": vec![0u8; bars] }).to_string());
                        silence_count += 1;
                    }
                    visual_filter.reset();
                    last_frame_time = std::time::Instant::now();
                    continue;
                }
                silence_count = 0;
                last_frame_time = std::time::Instant::now();

                // 1. Process mono samples and perform forward FFT
                fft_processor.process_samples(current_samples, channels);

                // 2. Get raw magnitudes and group into bars FIRST
                let raw_magnitudes = fft_processor.get_magnitudes();
                let raw_bars = fft_processor.group_frequencies(bars, sr, &config.visualizer_type, &raw_magnitudes);

                // 3. Convert raw power to 0-1 using Sensitivity-based dB scaling (mirrors Rainmeter AudioLevel Band output)
                //    Formula: (10*log10(power) + Sensitivity) / Sensitivity, clamped 0-1
                let scaled_bars = visual_filter.scale_to_audiolevel(&raw_bars, sensitivity);

                // 4. Temporal attack/decay smoothing (mirrors Rainmeter FFTAttack/FFTDecay)
                visual_filter.apply_temporal_smoothing(&scaled_bars, attack, decay);

                // 4. Spatial horizontal smoothing and final pass
                let bar_values = visual_filter.get_smoothed_bars();
                let spatially_smoothed = visual_filter.apply_spatial_smoothing(bar_values, config.horizontal_smoothing, multiplier);
                let final_bins = visual_filter.generate_final_bins(&spatially_smoothed, alpha, avg_size);

                let _ = tx_proc.send(serde_json::json!({ "type": "fft_data", "bins": final_bins }).to_string());
            }
        }
    }
}
