use std::f32::consts::PI;
use crate::structs::envelope::Envelope;
use crate::gui::WaveType;

pub struct Note {
    pub frequency: f32,
    pub sample_rate: f32,
    pub phase: f32,
    pub envelope: Envelope,
    pub wave_type: WaveType,
}

impl Note {
    pub fn new(frequency: f32, envelope: Envelope, sample_rate: f32, wave_type: WaveType) -> Self {
        Self {
            frequency,
            sample_rate,
            phase: 0.0,
            envelope,
            wave_type,
        }
    }

    pub fn get_sample(&mut self) -> f32 {
        let sample = match self.wave_type {
            WaveType::Sine => self.phase.sin(),
            WaveType::Square => if self.phase.sin() >= 0.0 { 1.0 } else { -1.0 },
            WaveType::Triangle => {
                let phase_norm = (self.phase / PI) % 2.0;
                if phase_norm < 0.5 {
                    phase_norm * 4.0 - 1.0
                } else if phase_norm < 1.5 {
                    1.0 - (phase_norm - 0.5) * 4.0
                } else {
                    (phase_norm - 2.0) * 4.0 + 1.0
                }
            },
            WaveType::Sawtooth => {
                let phase_norm = (self.phase / PI) % 2.0;
                2.0 * (phase_norm - 1.0)
            },
        };

        self.phase += 2.0 * PI * self.frequency / self.sample_rate;
        if self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI;
        }

        sample
    }

    pub fn update_frequency(&mut self, new_frequency: f32) {
        self.frequency = new_frequency;
    }
}