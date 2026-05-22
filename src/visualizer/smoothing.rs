pub struct VisualFilter {
    bars: usize,
    audiolevel_smoothed: Vec<f32>,
    smoothed_final: Vec<f32>,
    history: Vec<std::collections::VecDeque<f32>>,
}

impl VisualFilter {
    pub fn new(bars: usize) -> Self {
        Self {
            bars,
            audiolevel_smoothed: vec![0.0; bars],
            smoothed_final: vec![0.0; bars],
            history: vec![std::collections::VecDeque::new(); bars],
        }
    }
    
    /// Reset filters on silence/pause
    pub fn reset(&mut self) {
        self.audiolevel_smoothed.fill(0.0);
        self.smoothed_final.fill(0.0);
        for h in self.history.iter_mut() { h.clear(); }
    }
    
    /// Converts raw frequency-band power values to 0-1 range using Sensitivity-based dB scaling.
    /// This replicates what the Rainmeter AudioLevel plugin does internally before outputting Band values.
    /// Formula: band_out = clamp( (10*log10(power) + Sensitivity) / Sensitivity, 0, 1 )
    pub fn scale_to_audiolevel(&self, raw_bars: &[f32], sensitivity: f32) -> Vec<f32> {
        let range_db = sensitivity.max(10.0);
        raw_bars.iter().map(|&x| {
            let y = x.max(1e-30);
            let db = 10.0 * y.log10();
            ((db + range_db) / range_db).clamp(0.0, 1.0)
        }).collect()
    }

    /// Temporal attack/decay smoothing (replicates Rainmeter FFTAttack/FFTDecay on Bands).
    /// Uses the k-coefficient formula: k = 0.01^(1 / (FPS * time_seconds)), alpha = 1 - k
    pub fn apply_temporal_smoothing(
        &mut self,
        targets: &[f32],
        attack: f32,
        decay: f32,
    ) {
        for i in 0..self.bars {
            let target = targets[i];
            if target > self.audiolevel_smoothed[i] {
                self.audiolevel_smoothed[i] += (target - self.audiolevel_smoothed[i]) * attack;
            } else {
                self.audiolevel_smoothed[i] -= (self.audiolevel_smoothed[i] - target) * decay;
            }
            self.audiolevel_smoothed[i] = self.audiolevel_smoothed[i].clamp(0.0, 1.0);
        }
    }

    pub fn get_smoothed_bars(&self) -> &[f32] {
        &self.audiolevel_smoothed
    }
    
    /// Spatial/horizontal smoothing that exactly replicates the Monstercat Rainmeter skin formula:
    ///   out[i] = MinBarValue + ((Raw[i-1] + Raw[i] + Raw[i+1]) / divisor) * log10(Multiplier)
    ///
    /// The taper divisors at the left and right edges are scaled proportionally relative to
    /// the current bar count, so they work correctly for any bar count (not just 100).
    /// The inner 9 bars from each edge get the taper; others use divisor 3.0 (or 2.8 for bars 6-8 from left).
    pub fn apply_spatial_smoothing(
        &self,
        bar_values: &[f32],
        horizontal_smoothing: bool,
        multiplier: f32,
    ) -> Vec<f32> {
        let scale_factor = multiplier / 25.0; // 25.0 = 1.0 by default (linear scaling)
        let min_bar_value = 0.01_f32;
        let mut out = vec![0.0f32; self.bars];

        if !horizontal_smoothing {
            for i in 0..self.bars {
                out[i] = (min_bar_value + bar_values[i] * scale_factor).clamp(0.0, 1.0);
            }
            return out;
        }

        // Left taper: first 9 bars (indices 0..8)
        // Right taper: last 9 bars.
        // Both taper from the edge inward.
        // These are RELATIVE to the bar count — index 0 from the left/right always gets div=12.
        let n = self.bars;

        for i in 0..n {
            let left_val  = if i == 0     { bar_values[0] }     else { bar_values[i - 1] };
            let center    = bar_values[i];
            let right_val = if i == n - 1 { bar_values[n - 1] } else { bar_values[i + 1] };

            // Mirror edges (same as Rainmeter's formula for bar 0 and bar 99)
            let sum = if i == 0 {
                center + center + right_val
            } else if i == n - 1 {
                left_val + center + center
            } else {
                left_val + center + right_val
            };

            // Distance from the nearest edge, scaled relative to bar count
            let dist_from_left  = i;
            let dist_from_right = n - 1 - i;
            let dist_from_edge  = dist_from_left.min(dist_from_right);

            let div_val: f32 = if dist_from_left < dist_from_right {
                // Left-side taper
                match dist_from_left {
                    0 => 12.0,
                    1 => 10.0,
                    2 => 8.0,
                    3 => 6.0,
                    4 => 4.0,
                    5 => 3.0,
                    6 | 7 | 8 => 2.8,
                    _ => 3.0,
                }
            } else if dist_from_right < dist_from_left {
                // Right-side taper
                match dist_from_right {
                    0 => 12.0,
                    1 => 11.0,
                    2 => 10.0,
                    3 => 9.0,
                    4 => 8.0,
                    5 => 7.0,
                    6 => 6.0,
                    7 => 5.0,
                    8 => 4.0,
                    _ => 3.0,
                }
            } else {
                // Exact middle (or beyond taper zone)
                if dist_from_edge < 9 {
                    // Prefer left-side logic when equidistant and within taper zone
                    match dist_from_edge {
                        0 => 12.0,
                        1 => 10.0,
                        2 => 8.0,
                        3 => 6.0,
                        4 => 4.0,
                        5 => 3.0,
                        6 | 7 | 8 => 2.8,
                        _ => 3.0,
                    }
                } else {
                    3.0
                }
            };

            out[i] = (min_bar_value + (sum / div_val) * scale_factor).clamp(0.0, 1.0);
        }
        out
    }
    
    /// Final temporal averaging across a rolling history window (AverageSize).
    pub fn generate_final_bins(
        &mut self,
        spatially_smoothed: &[f32],
        alpha: f32,
        avg_size: usize,
    ) -> Vec<u8> {
        let mut final_bins = vec![0u8; self.bars];
        let avg_size = avg_size.max(1);

        for i in 0..self.bars {
            // Alpha-blend into smoothed_final
            self.smoothed_final[i] = alpha * spatially_smoothed[i] + (1.0 - alpha) * self.smoothed_final[i];
            self.history[i].push_back(self.smoothed_final[i]);
            if self.history[i].len() > avg_size { self.history[i].pop_front(); }
            let avg: f32 = self.history[i].iter().sum::<f32>() / self.history[i].len() as f32;
            final_bins[i] = (avg.clamp(0.0, 1.0) * 255.0) as u8;
        }
        final_bins
    }
}
