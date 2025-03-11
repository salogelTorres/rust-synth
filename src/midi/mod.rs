use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use midir::{MidiInput, MidiInputConnection};
use crate::audio::Note;
use crate::gui::WaveType;
use crate::structs::envelope::Envelope;

pub fn midi_note_to_freq(note: u8) -> f32 {
    // La fórmula correcta para MIDI a frecuencia es:
    // f = 440 * 2^((n-69)/12)
    // donde n es el número de nota MIDI y 69 es la nota A4 (440Hz)
    
    // Implementación directa y explícita para evitar errores
    let note_diff = (note as f32) - 69.0;
    let power = note_diff / 12.0;
    let multiplier = 2.0f32.powf(power);
    let freq = 440.0 * multiplier;
    
    // Imprimir para depuración
    println!("Nota MIDI {} -> Frecuencia {} Hz", note, freq);
    
    freq
}

pub fn handle_midi_message(
    msg: &[u8], 
    active_notes: Arc<Mutex<HashMap<u8, Note>>>, 
    sample_rate: Arc<Mutex<f32>>, 
    wave_type: Arc<Mutex<WaveType>>
) {
    match msg[0] & 0xF0 {
        0x90 => { // Note On
            let note = msg[1];
            let velocity = msg[2] as f32 / 127.0;
            if velocity > 0.0 {
                let freq = midi_note_to_freq(note);
                let mut envelope = Envelope::new(*sample_rate.lock().unwrap());
                envelope.set_adsr(0.01, 0.1, 0.7, 0.3);
                envelope.set_velocity(velocity);
                let current_wave_type = *wave_type.lock().unwrap();
                let new_note = Note::new(freq, envelope, *sample_rate.lock().unwrap(), current_wave_type, current_wave_type);
                active_notes.lock().unwrap().insert(note, new_note);
            } else {
                if let Some(note) = active_notes.lock().unwrap().get_mut(&note) {
                    note.envelope.note_off();
                }
            }
        },
        0x80 => { // Note Off
            let note = msg[1];
            if let Some(note) = active_notes.lock().unwrap().get_mut(&note) {
                note.envelope.note_off();
            }
        },
        _ => (),
    }
}

pub fn connect_midi(
    active_notes: Arc<Mutex<HashMap<u8, Note>>>, 
    sample_rate: Arc<Mutex<f32>>, 
    wave_type: Arc<Mutex<WaveType>>
) -> Option<MidiInputConnection<()>> {
    let midi_in = MidiInput::new("rust-synth").ok()?;
    let ports = midi_in.ports();
    let port = ports.get(0)?;

    let notes = active_notes.clone();
    let sr = sample_rate.clone();
    let wt = wave_type.clone();
    
    midi_in.connect(
        port,
        "rust-synth",
        move |_stamp, message, _| {
            handle_midi_message(message, notes.clone(), sr.clone(), wt.clone());
        },
        (),
    ).ok()
}
