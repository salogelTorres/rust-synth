use std::f32::consts::PI;
use crate::structs::envelope::Envelope;
use crate::gui::WaveType;

pub struct Oscillator {
    pub wave_type: WaveType,
    pub phase: f32,
    pub detune: f32,  // En semitonos
    pub volume: f32,
}

impl Oscillator {
    pub fn new(wave_type: WaveType) -> Self {
        Self {
            wave_type,
            phase: 0.0,
            detune: 0.0,
            volume: 1.0,
        }
    }

    pub fn get_sample(&mut self, base_frequency: f32, sample_rate: f32) -> f32 {
        // Calcular la frecuencia con detune
        let frequency = base_frequency * (2.0f32.powf(self.detune / 12.0));
        
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

        self.phase += 2.0 * PI * frequency / sample_rate;
        if self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI;
        }

        sample * self.volume
    }
}

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
            osc1: Oscillator::new(wave_type1),
            osc2: Oscillator::new(wave_type2),
        }
    }

    pub fn get_sample(&mut self) -> f32 {
        let osc1_sample = self.osc1.get_sample(self.frequency, self.sample_rate);
        let osc2_sample = self.osc2.get_sample(self.frequency, self.sample_rate);
        
        // Mezclar las salidas de ambos osciladores
        (osc1_sample + osc2_sample) * 0.5
    }

    pub fn update_frequency(&mut self, new_frequency: f32) {
        self.frequency = new_frequency;
    }
}