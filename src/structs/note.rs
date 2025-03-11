use std::f32::consts::PI;
use crate::structs::envelope::Envelope;
use crate::gui::WaveType;

const OVERSAMPLING: usize = 4; // Factor de sobremuestreo

pub struct LowPassFilter {
    prev_sample: f32,
    alpha: f32,
}

impl LowPassFilter {
    pub fn new(cutoff_freq: f32, sample_rate: f32) -> Self {
        let rc = 1.0 / (2.0 * PI * cutoff_freq);
        let dt = 1.0 / sample_rate;
        let alpha = dt / (rc + dt);
        Self {
            prev_sample: 0.0,
            alpha,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.prev_sample = self.prev_sample + self.alpha * (input - self.prev_sample);
        self.prev_sample
    }
}

pub struct Oscillator {
    pub wave_type: WaveType,
    pub phase: f32,
    pub detune: f32,  // En semitonos
    pub volume: f32,
    filter: LowPassFilter,
    oversample_buffer: [f32; OVERSAMPLING],
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
        }
    }

    pub fn get_sample(&mut self, base_frequency: f32, sample_rate: f32) -> f32 {
        // Calcular la frecuencia con detune
        let frequency = base_frequency * (2.0f32.powf(self.detune / 12.0));
        let oversample_rate = sample_rate * OVERSAMPLING as f32;
        
        // Generar muestras sobremuestreadas
        for i in 0..OVERSAMPLING {
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

            // Actualizar fase para cada muestra sobremuestreada
            self.phase += 2.0 * PI * frequency / oversample_rate;
            if self.phase >= 2.0 * PI {
                self.phase -= 2.0 * PI;
            }

            // Aplicar filtro antialiasing
            self.oversample_buffer[i] = self.filter.process(sample);
        }

        // Promediar las muestras sobremuestreadas
        let final_sample = self.oversample_buffer.iter().sum::<f32>() / OVERSAMPLING as f32;
        final_sample * self.volume
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
            osc1: Oscillator::new(wave_type1, sample_rate),
            osc2: Oscillator::new(wave_type2, sample_rate),
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