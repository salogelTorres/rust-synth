use cpal::traits::DeviceTrait;
use std::sync::Arc;
use std::sync::OnceLock;

// Tamaño de la tabla de ondas (potencia de 2 para optimizar)
pub const WAVETABLE_SIZE: usize = 4096;
// Máscara para operaciones de módulo eficientes (WAVETABLE_SIZE - 1)
const WAVETABLE_MASK: usize = WAVETABLE_SIZE - 1;

// Tabla de ondas compartida global
static SINE_TABLE: OnceLock<Arc<[f32; WAVETABLE_SIZE]>> = OnceLock::new();

// Inicializar la tabla de ondas
fn get_sine_table() -> Arc<[f32; WAVETABLE_SIZE]> {
    SINE_TABLE.get_or_init(|| {
        let mut table = [0.0; WAVETABLE_SIZE];
        for i in 0..WAVETABLE_SIZE {
            let phase = 2.0 * std::f32::consts::PI * (i as f32 / WAVETABLE_SIZE as f32);
            table[i] = phase.sin();
        }
        Arc::new(table)
    }).clone()
}

// Oscilador de tabla de ondas de alta calidad
pub struct WavetableOscillator {
    wavetable: Arc<[f32; WAVETABLE_SIZE]>,
    phase: f32,
    phase_increment: f32,
}

impl WavetableOscillator {
    pub fn new(sample_rate: f32, frequency: f32) -> Self {
        // Usar la tabla de ondas compartida
        let wavetable = get_sine_table();
        
        // Precalcular el incremento de fase
        let phase_increment = frequency * WAVETABLE_SIZE as f32 / sample_rate;
        
        WavetableOscillator {
            wavetable,
            phase: 0.0,
            phase_increment,
        }
    }
    
    #[inline]
    pub fn set_frequency(&mut self, frequency: f32, sample_rate: f32) {
        self.phase_increment = frequency * WAVETABLE_SIZE as f32 / sample_rate;
    }
    
    #[inline]
    pub fn get_sample(&mut self) -> f32 {
        // Interpolación lineal para un buen balance entre calidad y rendimiento
        let phase_floor = self.phase as usize;
        let phase_frac = self.phase - phase_floor as f32;
        
        // Usar operaciones de máscara para asegurar que los índices estén dentro del rango
        let idx1 = phase_floor & WAVETABLE_MASK;
        let idx2 = (phase_floor + 1) & WAVETABLE_MASK;
        
        let y1 = self.wavetable[idx1];
        let y2 = self.wavetable[idx2];
        
        // Interpolación lineal (más eficiente que la cúbica)
        let output = y1 + phase_frac * (y2 - y1);
        
        // Avanzar la fase
        self.phase += self.phase_increment;
        // Usar operación de máscara para mantener la fase dentro del rango
        if self.phase >= WAVETABLE_SIZE as f32 {
            self.phase -= WAVETABLE_SIZE as f32;
        }
        
        output
    }
}

// Estructura para un filtro notch (rechaza banda)
pub struct NotchFilter {
    frequency: f32,
    q: f32,
    sample_rate: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
    a0: f32,
    a1: f32,
    a2: f32,
    b0: f32,
    b1: f32,
    b2: f32,
}

impl NotchFilter {
    pub fn new(frequency: f32, q: f32, sample_rate: f32) -> Self {
        let mut filter = NotchFilter {
            frequency,
            q,
            sample_rate,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
        };
        filter.calculate_coefficients();
        filter
    }
    
    fn calculate_coefficients(&mut self) {
        let omega = 2.0 * std::f32::consts::PI * self.frequency / self.sample_rate;
        let alpha = (omega.sin()) / (2.0 * self.q);
        
        self.b0 = 1.0;
        self.b1 = -2.0 * omega.cos();
        self.b2 = 1.0;
        
        self.a0 = 1.0 + alpha;
        self.a1 = -2.0 * omega.cos();
        self.a2 = 1.0 - alpha;
        
        // Normalizar coeficientes
        self.b0 /= self.a0;
        self.b1 /= self.a0;
        self.b2 /= self.a0;
        self.a1 /= self.a0;
        self.a2 /= self.a0;
        self.a0 = 1.0;
    }
    
    pub fn process(&mut self, input: f32) -> f32 {
        // Implementación de un filtro biquad
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
                   - self.a1 * self.y1 - self.a2 * self.y2;
        
        // Actualizar estados
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        
        output
    }
    
    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
        self.calculate_coefficients();
    }
    
    pub fn set_q(&mut self, q: f32) {
        self.q = q;
        self.calculate_coefficients();
    }
    
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.calculate_coefficients();
    }
}

// Estructura para un filtro paso bajo simple
pub struct LowPassFilter {
    cutoff: f32,
    sample_rate: f32,
    alpha: f32,
    prev_output: f32,
}

impl LowPassFilter {
    #[inline]
    pub fn new(cutoff: f32, _resonance: f32, sample_rate: f32) -> Self {
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        let alpha = dt / (rc + dt);
        
        LowPassFilter {
            cutoff,
            sample_rate,
            alpha,
            prev_output: 0.0,
        }
    }
    
    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        // Implementación de un filtro paso bajo de primer orden optimizado
        self.prev_output += self.alpha * (input - self.prev_output);
        self.prev_output
    }
    
    #[inline]
    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
        let dt = 1.0 / self.sample_rate;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
        self.alpha = dt / (rc + dt);
    }
    
    #[inline]
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * self.cutoff);
        self.alpha = dt / (rc + dt);
    }
}

pub mod filters;
pub mod oscillator;
pub mod note;
pub mod wavetable;

// Re-export principales componentes
pub use filters::LowPassFilter;
pub use oscillator::Oscillator;
pub use note::Note;
pub use wavetable::WavetableOscillator;

// Funciones de utilidad para el sistema de audio
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    x.tanh()
}

pub fn list_audio_hosts() -> Vec<cpal::HostId> {
    let available_hosts = cpal::available_hosts();
    for (idx, host_id) in available_hosts.iter().enumerate() {
        println!("{}. {}", idx, host_id.name());
    }
    available_hosts
}

pub fn create_audio_config(device: &cpal::Device) -> cpal::StreamConfig {
    let config = device.default_output_config().unwrap();
    cpal::StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(512),
    }
}