use crate::config::VisualizerConfig;
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::mpsc::Receiver;
use tokio::sync::broadcast::Sender;

pub fn run_processor(
    config: VisualizerConfig,
    audio_rx: Receiver<Vec<f32>>,
    tx_proc: Sender<String>,
    sr: f32,
    channels: usize,
) {
    let win_size = config.samples as usize; 
    let bars = config.bars as usize;
    let target_len = win_size * channels;

    let mut planner = FftPlanner::new();
    let fft_proc = planner.plan_fft_forward(win_size);

    let window: Vec<f32> = (0..win_size)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (win_size - 1) as f32).cos()))
        .collect();

    // 🎯 LOGARITHMIC FREQUENCY MAPPING (FreqMin=20, FreqMax=16000)
    let bin_width = sr / win_size as f32;
    let mut bin_ranges = Vec::with_capacity(bars);
    let min_f = 20.0;
    let max_f = 16000.0;

    for i in 0..bars {
        let f_start = min_f * (max_f / min_f as f32).powf(i as f32 / bars as f32);
        let f_end = min_f * (max_f / min_f as f32).powf((i + 1) as f32 / bars as f32);
    
        // 🎯 FIX: Never use Bin 0 (DC Offset) for the lowest bass frequencies.
        // DC offset is mathematically silent in audio, which breaks the left-most visualizer bars.
        let b_start = ((f_start / bin_width).round() as usize).max(1);
        let b_end = ((f_end / bin_width).round() as usize).max(b_start + 1);

        bin_ranges.push(b_start..b_end.min(win_size / 2));
    }

    let mut staging_buffer: Vec<f32> = Vec::with_capacity(target_len * 2);
    let mut mono: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); win_size];

    // AudioLevel Temporal State
    let mut audiolevel_smoothed = vec![0.0f32; bars];
    
    // Calc Measure AverageSize State (Ring Buffer)
    let average_size = config.smoothing.clamp(1, 20) as usize;
    let mut history = vec![vec![0.0f32; bars]; average_size];
    let mut hist_idx = 0;

    // Physics parameters
    let attack = 0.85; 
    let decay = config.visual_fluidity.clamp(0.01, 1.0); 
    let sensitivity = config.sensitivity as f32;
    let multiplier = config.multiplier as f32;

    while let Ok(data) = audio_rx.recv() {
        staging_buffer.extend(data);
        while let Ok(more_data) = audio_rx.try_recv() {
            staging_buffer.extend(more_data);
        }

        if staging_buffer.len() > target_len {
            let excess = staging_buffer.len() - target_len;
            staging_buffer.drain(0..excess);
        }

        if staging_buffer.len() >= target_len {
            let current_samples = &staging_buffer[0..target_len];
            for (i, chunk) in current_samples.chunks_exact(channels).take(win_size).enumerate() {
                let avg: f32 = chunk.iter().sum::<f32>() / channels as f32;
                mono[i] = Complex::new(avg * window[i], 0.0);
            }
            fft_proc.process(&mut mono);

            let mut magnitudes = vec![0.0f32; win_size / 2];
            for i in 0..win_size / 2 {
                magnitudes[i] = (mono[i].norm() / win_size as f32) * 2.0;
            }

            let mut raw_scaled = vec![0.0f32; bars];

            // 🎯 STEP 1: AUDIOLEVEL PLUGIN (Fractional Bin Integration -> dB -> Normalize)
            // Rainmeter's exact math: sums proportional bin energy, scales by sqrt(N), uses 10*log10
            let df = sr / win_size as f32;
            let scalar_fft = 1.0 / (win_size as f32).sqrt(); 
            let scalar_int = 2.0 / sr;

            let mut i_bin = (min_f / df).round() as usize;
            let mut i_band = 0;
            let mut f0 = min_f;

            while i_bin <= (win_size / 2) && i_band < bars {
                let f_lin1 = (i_bin as f32 + 0.5) * df;
                // Rainmeter m_bandFreq[iBand] is the upper frequency of the band
                let f_log1 = min_f * (max_f / min_f).powf((i_band + 1) as f32 / bars as f32);
                
                let c = mono[i_bin];
                let r_sq_i_sq = c.re * c.re + c.im * c.im;
                let x = r_sq_i_sq * scalar_fft; // Rainmeter's squared magnitude scaled by 1/sqrt(N)

                if f_lin1 <= f_log1 {
                    raw_scaled[i_band] += (f_lin1 - f0) * x * scalar_int;
                    f0 = f_lin1;
                    i_bin += 1;
                } else {
                    raw_scaled[i_band] += (f_log1 - f0) * x * scalar_int;
                    f0 = f_log1;
                    i_band += 1;
                }
            }

            for i in 0..bars {
                let y = raw_scaled[i].clamp(0.0, 1.0);
                // Rainmeter: 10.0 / sensitivity * log10(y) + 1.0
                let db_scaled = (10.0 / sensitivity) * y.max(1e-10).log10() + 1.0;
                raw_scaled[i] = db_scaled.clamp(0.0, 1.0);
            }

            // 🎯 STEP 2: AUDIOLEVEL PLUGIN (Temporal Attack/Decay)
            for i in 0..bars {
                let target = raw_scaled[i].clamp(0.0, 1.0);
                if target > audiolevel_smoothed[i] {
                    audiolevel_smoothed[i] += (target - audiolevel_smoothed[i]) * attack;
                } else {
                    audiolevel_smoothed[i] -= (audiolevel_smoothed[i] - target) * decay;
                }
            }

            // 🎯 STEP 3: MARCOPIXEL SPATIAL SCULPTING (MeasureBandsSmoothed)
            let scale_factor = multiplier / 50.0; 
            let min_bar_value = 0.01;
            let mut spatially_smoothed = vec![0.0f32; bars];

            if config.horizontal_smoothing {
                for i in 0..bars {
                    // Duplicates edges per the .inc file logic
                    let left_val = if i == 0 { audiolevel_smoothed[0] } else { audiolevel_smoothed[i - 1] };
                    let center = audiolevel_smoothed[i];
                    let right_val = if i == bars - 1 { audiolevel_smoothed[bars - 1] } else { audiolevel_smoothed[i + 1] };
                    
                    let sum = left_val + center + right_val;
                    
                    let divisor = if i <= 8 {
                        match i {
                            0 => 12.0, 1 => 10.0, 2 => 8.0, 3 => 6.0, 
                            4 => 4.0, 5 => 3.0, 6 => 2.8, 7 => 2.8, 8 => 2.8, _ => 3.0,
                        }
                    } else if i >= bars - 9 {
                        let dist_to_end = bars - 1 - i;
                        12.0 - (dist_to_end as f32)
                    } else {
                        3.0
                    };
                    
                    let scaled = min_bar_value + ((sum / divisor) * scale_factor);
                    spatially_smoothed[i] = scaled.clamp(0.0, 1.0);
                }
            } else {
                for i in 0..bars {
                    let scaled = min_bar_value + (audiolevel_smoothed[i] * scale_factor);
                    spatially_smoothed[i] = scaled.clamp(0.0, 1.0);
                }
            }

            // 🎯 STEP 4: CALC MEASURE (Temporal AverageSize)
            history[hist_idx].copy_from_slice(&spatially_smoothed);
            hist_idx = (hist_idx + 1) % average_size;

            let mut final_bins = vec![0u8; bars];
            for i in 0..bars {
                let mut sum = 0.0;
                for h in &history {
                    sum += h[i];
                }
                let avg = sum / average_size as f32;
                final_bins[i] = (avg.clamp(0.0, 1.0) * 255.0) as u8;
            }

            if tx_proc.send(serde_json::json!({ "type": "fft_data", "bins": final_bins }).to_string()).is_err() {
                return; 
            }
        }
    }
}