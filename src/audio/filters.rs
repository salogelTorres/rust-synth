use std::f32::consts::PI;

pub struct LowPassFilter {
    prev_sample: f32,
    alpha: f32,
}

impl LowPassFilter {
    #[inline(always)]
    pub fn new(cutoff_freq: f32, sample_rate: f32) -> Self {
        let rc = 1.0 / (2.0 * PI * cutoff_freq);
        let dt = 1.0 / sample_rate;
        let alpha = dt / (rc + dt);
        Self {
            prev_sample: 0.0,
            alpha,
        }
    }

    #[inline(always)]
    pub fn process(&mut self, input: f32) -> f32 {
        self.prev_sample += self.alpha * (input - self.prev_sample);
        self.prev_sample
    }

    #[inline(always)]
    pub fn set_cutoff(&mut self, cutoff_freq: f32, sample_rate: f32) {
        let rc = 1.0 / (2.0 * PI * cutoff_freq);
        let dt = 1.0 / sample_rate;
        self.alpha = dt / (rc + dt);
    }
}