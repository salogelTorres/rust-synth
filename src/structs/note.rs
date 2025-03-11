use std::f32::consts::PI;
use crate::structs::envelope::Envelope;
use crate::gui::WaveType;

const OVERSAMPLING: usize = 4; // Reducido ya que usaremos PolyBLEP

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

pub struct Oscillator {
    pub wave_type: WaveType,
    pub phase: f32,
    pub detune: f32,  // En semitonos
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
    fn get_bandlimited_square(&mut self, phase_norm: f32, phase_inc: f32) -> f32 {
        let mut square = if phase_norm < 0.5 { 1.0 } else { -1.0 };
        square += self.poly_blep(phase_norm, phase_inc);
        square -= self.poly_blep((phase_norm + 0.5) % 1.0, phase_inc);
        square
    }

    #[inline(always)]
    fn get_bandlimited_saw(&mut self, phase_norm: f32, phase_inc: f32) -> f32 {
        let mut saw = 2.0 * phase_norm - 1.0;
        saw -= self.poly_blep(phase_norm, phase_inc);
        saw
    }

    #[inline(always)]
    fn get_bandlimited_triangle(&mut self, phase_norm: f32) -> f32 {
        // El triángulo tiene menos aliasing por naturaleza, usamos integración
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
        // Calcular la frecuencia con detune
        let frequency = base_frequency * (2.0f32.powf(self.detune / 12.0));
        let phase_inc = frequency / sample_rate;
        
        // Normalizar fase entre 0 y 1
        let phase_norm = self.phase / (2.0 * PI);
        
        // Generar forma de onda con antialiasing
        let sample = match self.wave_type {
            WaveType::Sine => (self.phase).sin(),
            WaveType::Square => self.get_bandlimited_square(phase_norm, phase_inc),
            WaveType::Triangle => self.get_bandlimited_triangle(phase_norm),
            WaveType::Sawtooth => self.get_bandlimited_saw(phase_norm, phase_inc),
        };

        // Actualizar fase
        self.phase += 2.0 * PI * phase_inc;
        if self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI;
        }

        // Aplicar suavizado adicional para frecuencias muy altas
        if frequency > sample_rate * 0.25 {
            let smoothing = 1.0 - ((frequency - sample_rate * 0.25) / (sample_rate * 0.25)).min(1.0);
            sample * smoothing * self.volume
        } else {
            sample * self.volume
        }
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