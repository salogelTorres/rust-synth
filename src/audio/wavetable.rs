use std::sync::Arc;
use std::sync::OnceLock;

// Tamaño de la tabla de ondas (potencia de 2 para optimizar)
pub const WAVETABLE_SIZE: usize = 4096;
// Máscara para operaciones de módulo eficientes
const WAVETABLE_MASK: usize = WAVETABLE_SIZE - 1;

// Tabla de ondas compartida global
static SINE_TABLE: OnceLock<Arc<[f32; WAVETABLE_SIZE]>> = OnceLock::new();

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

pub struct WavetableOscillator {
    wavetable: Arc<[f32; WAVETABLE_SIZE]>,
    phase: f32,
    phase_increment: f32,
}

impl WavetableOscillator {
    pub fn new(sample_rate: f32, frequency: f32) -> Self {
        let wavetable = get_sine_table();
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
        let phase_floor = self.phase as usize;
        let phase_frac = self.phase - phase_floor as f32;
        
        let idx1 = phase_floor & WAVETABLE_MASK;
        let idx2 = (phase_floor + 1) & WAVETABLE_MASK;
        
        let y1 = self.wavetable[idx1];
        let y2 = self.wavetable[idx2];
        
        let output = y1 + phase_frac * (y2 - y1);
        
        self.phase += self.phase_increment;
        if self.phase >= WAVETABLE_SIZE as f32 {
            self.phase -= WAVETABLE_SIZE as f32;
        }
        
        output
    }
} 