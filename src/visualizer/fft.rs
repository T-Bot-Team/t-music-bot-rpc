use rustfft::{num_complex::Complex, FftPlanner, Fft};
use std::sync::Arc;

pub struct FftProcessor {
    win_size: usize,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    mono: Vec<Complex<f32>>,
}

impl FftProcessor {
    pub fn new(win_size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(win_size);
        
        // Precompute Hann window
        let window: Vec<f32> = (0..win_size)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (win_size - 1) as f32).cos()))
            .collect();
            
        let mono = vec![Complex::new(0.0, 0.0); win_size];
        
        Self {
            win_size,
            fft,
            window,
            mono,
        }
    }
    
    /// Converts multi-channel input to monophonic and applies Hann window
    pub fn process_samples(&mut self, samples: &[f32], channels: usize) -> &[Complex<f32>] {
        for (i, chunk) in samples.chunks_exact(channels).take(self.win_size).enumerate() {
            let avg: f32 = chunk.iter().sum::<f32>() / channels as f32;
            self.mono[i] = Complex::new(avg * self.window[i], 0.0);
        }
        
        // Execute FFT in place
        let _ = self.fft.process(&mut self.mono);
        
        &self.mono
    }
    
    pub fn get_magnitudes(&self) -> Vec<f32> {
        let scalar_fft = 1.0 / (self.win_size as f32).sqrt(); 
        self.mono.iter().take(self.win_size / 2 + 1)
            .map(|c| (c.re * c.re + c.im * c.im) * scalar_fft)
            .collect()
    }

    /// Group pre-processed frequency bins into a target number of bars
    pub fn group_frequencies(
        &self,
        bars: usize,
        sr: f32,
        visualizer_type: &str,
        magnitudes: &[f32],
    ) -> Vec<f32> {
        let mut raw_scaled = vec![0.0f32; bars];
        let df = sr / self.win_size as f32;
        let scalar_int = 2.0 / sr;
        
        let min_f = 20.0;
        let max_f = 16000.0;
        
        let mut i_bin = (min_f / df).round() as usize;
        let mut i_band = 0;
        let mut f0 = min_f;
        
        let use_linear = visualizer_type == "linear";
        
        while i_bin < magnitudes.len() && i_band < bars {
            let f_lin1 = (i_bin as f32 + 0.5) * df;
            let f_target = if use_linear {
                min_f + (max_f - min_f) * (i_band + 1) as f32 / bars as f32
            } else {
                min_f * (max_f / min_f).powf((i_band + 1) as f32 / bars as f32)
            };
            
            let x = magnitudes[i_bin];
            
            if f_lin1 <= f_target {
                let weight = f_lin1 - f0;
                raw_scaled[i_band] += weight * x * scalar_int;
                f0 = f_lin1;
                i_bin += 1;
            } else {
                let weight = f_target - f0;
                raw_scaled[i_band] += weight * x * scalar_int;
                f0 = f_target;
                i_band += 1;
            }
        }
        
        raw_scaled
    }
}
