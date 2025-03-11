use crate::gui::WaveType;
use crate::structs::envelope::Envelope;
use super::oscillator::Oscillator;

pub struct Note {
    pub frequency: f32,
    pub sample_rate: f32,
    pub envelope: Envelope,
    pub osc1: Oscillator,
    pub osc2: Oscillator,
}

impl Note {
    pub fn new(frequency: f32, envelope: Envelope, sample_rate: f32, wave_type1: WaveType, wave_type2: WaveType) -> Self {
        Self {
            frequency,
            sample_rate,
            envelope,
            osc1: Oscillator::new(wave_type1, sample_rate),
            osc2: Oscillator::new(wave_type2, sample_rate),
        }
    }

    pub fn get_sample(&mut self) -> f32 {
        let osc1_sample = self.osc1.get_sample(self.frequency, self.sample_rate);
        let osc2_sample = self.osc2.get_sample(self.frequency, self.sample_rate);
        (osc1_sample + osc2_sample) * 0.5
    }

    pub fn update_frequency(&mut self, new_frequency: f32) {
        self.frequency = new_frequency;
    }
} 