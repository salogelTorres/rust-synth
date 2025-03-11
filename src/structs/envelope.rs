#[derive(Clone, Copy, PartialEq)]
pub enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct Envelope {
    pub sample_rate: f32,
    pub state: EnvelopeState,
    pub current_level: f32,
    pub attack_time: f32,
    pub decay_time: f32,
    pub sustain_level: f32,
    pub release_time: f32,
    pub velocity: f32,
    attack_increment: f32,
    decay_increment: f32,
    release_increment: f32,
}

impl Envelope {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            state: EnvelopeState::Idle,
            current_level: 0.0,
            attack_time: 0.01,
            decay_time: 0.1,
            sustain_level: 0.7,
            release_time: 0.3,
            velocity: 1.0,
            attack_increment: 0.0,
            decay_increment: 0.0,
            release_increment: 0.0,
        }
    }

    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.attack_time = attack;
        self.decay_time = decay;
        self.sustain_level = sustain;
        self.release_time = release;
        self.recalculate_increments();
    }

    pub fn set_velocity(&mut self, velocity: f32) {
        self.velocity = velocity;
        self.recalculate_increments();
    }

    fn recalculate_increments(&mut self) {
        self.attack_increment = 1.0 / (self.attack_time * self.sample_rate);
        self.decay_increment = (1.0 - self.sustain_level) / (self.decay_time * self.sample_rate);
        self.release_increment = self.sustain_level / (self.release_time * self.sample_rate);
    }

    pub fn note_on(&mut self) {
        self.state = EnvelopeState::Attack;
        self.recalculate_increments();
    }

    pub fn note_off(&mut self) {
        if self.state != EnvelopeState::Idle {
            self.state = EnvelopeState::Release;
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        match self.state {
            EnvelopeState::Idle => 0.0,
            EnvelopeState::Attack => {
                self.current_level += self.attack_increment;
                if self.current_level >= 1.0 {
                    self.current_level = 1.0;
                    self.state = EnvelopeState::Decay;
                }
                self.current_level * self.velocity
            }
            EnvelopeState::Decay => {
                self.current_level -= self.decay_increment;
                if self.current_level <= self.sustain_level {
                    self.current_level = self.sustain_level;
                    self.state = EnvelopeState::Sustain;
                }
                self.current_level * self.velocity
            }
            EnvelopeState::Sustain => {
                self.current_level * self.velocity
            }
            EnvelopeState::Release => {
                self.current_level -= self.release_increment;
                if self.current_level <= 0.0 {
                    self.current_level = 0.0;
                    self.state = EnvelopeState::Idle;
                }
                self.current_level * self.velocity
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.state == EnvelopeState::Idle
    }
}