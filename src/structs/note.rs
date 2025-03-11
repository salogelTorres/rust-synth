use super::envelope::Envelope;
use crate::audio::WavetableOscillator;
use crate::audio::LowPassFilter;

pub struct Note {
    pub frequency: f32,
    pub envelope: Envelope,
    pub sample_rate: f32,
    pub oscillator: WavetableOscillator,
    pub lowpass_filter: LowPassFilter,
}

impl Note {
    #[inline]
    pub fn new(frequency: f32, envelope: Envelope, sample_rate: f32) -> Self {
        // Crear un oscilador de tabla de ondas de alta calidad
        let oscillator = WavetableOscillator::new(sample_rate, frequency);
        
        // Crear un filtro paso bajo para eliminar frecuencias altas no deseadas
        // La frecuencia de corte es 10 veces la frecuencia fundamental, pero limitada a Nyquist/2
        let nyquist = sample_rate / 2.0;
        let cutoff = (frequency * 10.0).min(nyquist * 0.45);
        let lowpass_filter = LowPassFilter::new(cutoff, 0.7, sample_rate);
        
        Note {
            frequency,
            envelope,
            sample_rate,
            oscillator,
            lowpass_filter,
        }
    }
    
    #[inline]
    pub fn get_sample(&mut self) -> f32 {
        // Usar el oscilador de tabla de ondas para generar una onda sinusoidal pura
        let raw_sample = self.oscillator.get_sample();
        
        // Aplicar el filtro paso bajo para eliminar frecuencias altas no deseadas
        self.lowpass_filter.process(raw_sample)
    }
    
    #[inline]
    pub fn update_frequency(&mut self, new_freq: f32, _sample_rate: f32) {
        self.frequency = new_freq;
        
        // Actualizar la frecuencia del oscilador
        self.oscillator.set_frequency(new_freq, self.sample_rate);
        
        // Actualizar la frecuencia de corte del filtro paso bajo
        let nyquist = self.sample_rate / 2.0;
        let cutoff = (new_freq * 10.0).min(nyquist * 0.45);
        self.lowpass_filter.set_cutoff(cutoff);
    }
}