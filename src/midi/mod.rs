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
