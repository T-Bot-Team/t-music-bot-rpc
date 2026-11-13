//! Runtime smoke test for the Linux PulseAudio/PipeWire capture backend.
//! Run: cargo run --example pulse_capture_test [device-substring]
//!
//! Captures 2 seconds of audio and prints peak/RMS levels per 250ms window.

use std::sync::mpsc::sync_channel;

fn main() {
    let device = std::env::args().nth(1).unwrap_or_else(|| "default".to_string());

    println!("Listing devices:");
    for d in t_music_bot_rpc::visualizer::backend::list_devices() {
        println!("  [{:?}] {}", d.kind, d.name);
    }

    let (tx, rx) = sync_channel::<Vec<f32>>(128);
    println!("\nOpening capture for '{}'...", device);
    let opened = t_music_bot_rpc::visualizer::backend::open_capture(&device, tx);

    let mut opened = match opened {
        Ok(o) => {
            println!("Bound to: {} ({} Hz, {} ch)", o.bound_name, o.sample_rate, o.channels);
            o
        }
        Err(e) => {
            eprintln!("FAILED: {}", e);
            std::process::exit(1);
        }
    };

    let start = std::time::Instant::now();
    let mut window_peak = 0.0f32;
    let mut window_sum_sq = 0.0f32;
    let mut window_n = 0usize;
    let mut windows_printed = 0;

    while start.elapsed() < std::time::Duration::from_secs(2) {
        if let Ok(data) = rx.recv_timeout(std::time::Duration::from_millis(200)) {
            for &s in &data {
                window_peak = window_peak.max(s.abs());
                window_sum_sq += (s as f64 * s as f64) as f32;
            }
            window_n += data.len();
        }
        if start.elapsed().as_millis() > (windows_printed + 1) as u128 * 250 {
            let rms = if window_n > 0 { (window_sum_sq / window_n as f32).sqrt() } else { 0.0 };
            println!("  {:>4}ms  peak={:.4}  rms={:.4}  samples={}", start.elapsed().as_millis(), window_peak, rms, window_n);
            windows_printed += 1;
            window_peak = 0.0;
            window_sum_sq = 0.0;
            window_n = 0;
        }
    }

    // Exercise pause/resume round-trip.
    opened.capture.pause();
    opened.capture.resume();
    drop(opened);
    println!("OK");
}
