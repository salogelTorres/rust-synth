#[derive(PartialEq)]
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
}

impl Envelope {
    pub fn new(sample_rate: f32) -> Self {
        Envelope {
            attack: 0.005,
            decay: 0.05,
            sustain: 0.7,
            release: 0.05,
            current_amplitude: 0.0,
            state: EnvelopeState::Off,
            sample_rate,
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        let attack_samples = self.attack * self.sample_rate;
        let decay_samples = self.decay * self.sample_rate;
        let release_samples = self.release * self.sample_rate;

        match self.state {
            EnvelopeState::Attack => {
                self.current_amplitude += 1.0 / attack_samples;
                if self.current_amplitude >= 1.0 {
                    self.current_amplitude = 1.0;
                    self.state = EnvelopeState::Decay;
                }
            }
            EnvelopeState::Decay => {
                self.current_amplitude -= (1.0 - self.sustain) / decay_samples;
                if self.current_amplitude <= self.sustain {
                    self.current_amplitude = self.sustain;
                    self.state = EnvelopeState::Sustain;
                }
            }
            EnvelopeState::Sustain => {
                // Mantener el nivel de sustain
            }
            EnvelopeState::Release => {
                self.current_amplitude -= self.sustain / release_samples;
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

    pub fn note_on(&mut self) {
        self.state = EnvelopeState::Attack;
    }

    pub fn note_off(&mut self) {
        self.state = EnvelopeState::Release;
    }

    pub fn is_finished(&self) -> bool {
        self.state == EnvelopeState::Off
    }
}