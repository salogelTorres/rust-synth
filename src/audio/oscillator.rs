use std::f32::consts::PI;
use crate::gui::WaveType;
use super::filters::LowPassFilter;

const OVERSAMPLING: usize = 4;

pub struct Oscillator {
    pub wave_type: WaveType,
    pub phase: f32,
    pub detune: f32,
    pub volume: f32,
    filter: LowPassFilter,
    oversample_buffer: [f32; OVERSAMPLING],
    prev_frequency: f32,
    prev_cutoff: f32,
}

impl Oscillator {
    pub fn new(wave_type: WaveType, sample_rate: f32) -> Self {
        Self {
            wave_type,
            phase: 0.0,
            detune: 0.0,
            volume: 1.0,
            filter: LowPassFilter::new(20000.0, sample_rate * OVERSAMPLING as f32),
            oversample_buffer: [0.0; OVERSAMPLING],
            prev_frequency: 0.0,
            prev_cutoff: 20000.0,
        }
    }

    #[inline(always)]
    fn poly_blep(&self, t: f32, dt: f32) -> f32 {
        if t < dt {
            let t = t / dt;
            return 2.0 * t - t * t - 1.0;
        } else if t > 1.0 - dt {
            let t = (t - 1.0) / dt;
            return t * t + 2.0 * t + 1.0;
        }
        0.0
    }

    #[inline(always)]
    fn get_bandlimited_square(&self, phase_norm: f32, phase_inc: f32) -> f32 {
        let mut square = if phase_norm < 0.5 { 1.0 } else { -1.0 };
        square += self.poly_blep(phase_norm, phase_inc);
        square -= self.poly_blep((phase_norm + 0.5) % 1.0, phase_inc);
        square
    }

    #[inline(always)]
    fn get_bandlimited_saw(&self, phase_norm: f32, phase_inc: f32) -> f32 {
        let mut saw = 2.0 * phase_norm - 1.0;
        saw -= self.poly_blep(phase_norm, phase_inc);
        saw
    }

    #[inline(always)]
    fn get_bandlimited_triangle(&self, phase_norm: f32) -> f32 {
        let phase_quad = phase_norm * 4.0;
        if phase_quad < 1.0 {
            phase_quad
        } else if phase_quad < 2.0 {
            2.0 - phase_quad
        } else if phase_quad < 3.0 {
            phase_quad - 4.0
        } else {
            -4.0 + phase_quad
        }
    }

    #[inline(always)]
    pub fn get_sample(&mut self, base_frequency: f32, sample_rate: f32) -> f32 {
        let frequency = base_frequency * (2.0f32.powf(self.detune / 12.0));
        let phase_inc = frequency / sample_rate;
        
        let cutoff = if frequency > sample_rate * 0.125 {
            frequency * 1.5
        } else {
            frequency * 2.5
        };

        if (cutoff - self.prev_cutoff).abs() > 1.0 {
            self.filter.set_cutoff(cutoff.min(sample_rate * 0.45), sample_rate);
            self.prev_cutoff = cutoff;
        }
        
        let phase_norm = self.phase / (2.0 * PI);
        
        let raw_sample = match self.wave_type {
            WaveType::Sine => (self.phase).sin(),
            WaveType::Square => self.get_bandlimited_square(phase_norm, phase_inc),
            WaveType::Triangle => self.get_bandlimited_triangle(phase_norm),
            WaveType::Sawtooth => self.get_bandlimited_saw(phase_norm, phase_inc),
        };

        self.phase += 2.0 * PI * phase_inc;
        if self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI;
        }

        if frequency > sample_rate * 0.25 {
            let smoothing = 1.0 - ((frequency - sample_rate * 0.25) / (sample_rate * 0.25)).min(1.0);
            self.filter.process(raw_sample) * smoothing * self.volume
        } else {
            raw_sample * self.volume
        }
    }
} 