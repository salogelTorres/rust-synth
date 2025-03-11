#[derive(PartialEq, Clone, Copy)]
pub enum EnvelopeState {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

pub struct Envelope {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    current_amplitude: f32,
    state: EnvelopeState,
    sample_rate: f32,
    // Precalcular valores para evitar divisiones en cada muestra
    attack_increment: f32,
    decay_decrement: f32,
    release_decrement: f32,
}

impl Envelope {
    #[inline]
    pub fn new(sample_rate: f32) -> Self {
        let attack = 0.005;
        let decay = 0.05;
        let sustain = 0.7;
        let release = 0.05;
        
        // Precalcular incrementos/decrementos
        let attack_increment = 1.0 / (attack * sample_rate);
        let decay_decrement = (1.0 - sustain) / (decay * sample_rate);
        let release_decrement = sustain / (release * sample_rate);
        
        Envelope {
            attack,
            decay,
            sustain,
            release,
            current_amplitude: 0.0,
            state: EnvelopeState::Off,
            sample_rate,
            attack_increment,
            decay_decrement,
            release_decrement,
        }
    }

    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        match self.state {
            EnvelopeState::Attack => {
                self.current_amplitude += self.attack_increment;
                if self.current_amplitude >= 1.0 {
                    self.current_amplitude = 1.0;
                    self.state = EnvelopeState::Decay;
                }
            }
            EnvelopeState::Decay => {
                self.current_amplitude -= self.decay_decrement;
                if self.current_amplitude <= self.sustain {
                    self.current_amplitude = self.sustain;
                    self.state = EnvelopeState::Sustain;
                }
            }
            EnvelopeState::Sustain => {
                // Mantener el nivel de sustain
            }
            EnvelopeState::Release => {
                self.current_amplitude -= self.release_decrement;
                if self.current_amplitude <= 0.0 {
                    self.current_amplitude = 0.0;
                    self.state = EnvelopeState::Off;
                }
            }
            EnvelopeState::Off => {
                self.current_amplitude = 0.0;
            }
        }
        self.current_amplitude
    }

    #[inline]
    pub fn note_on(&mut self) {
        self.state = EnvelopeState::Attack;
    }

    #[inline]
    pub fn note_off(&mut self) {
        self.state = EnvelopeState::Release;
    }

    #[inline]
    pub fn is_finished(&self) -> bool {
        self.state == EnvelopeState::Off
    }
}