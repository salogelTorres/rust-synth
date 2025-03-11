use super::envelope::Envelope;

pub struct Note {
    pub frequency: f32,
    pub envelope: Envelope,
    pub phase: f32,
}