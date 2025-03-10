use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use midir::MidiInput;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::Duration;
use std::io;

fn main() {
    // Frecuencia actual y estado de nota
    let current_freq = Arc::new(AtomicU64::new(440_0000));
    let note_on = Arc::new(AtomicBool::new(false));
    
    // Configurar entrada MIDI
    let midi_in = MidiInput::new("rust-synth").unwrap();
    let ports = midi_in.ports();
    
    if ports.is_empty() {
        println!("No hay dispositivos MIDI disponibles");
        return;
    }

    let freq_for_audio = current_freq.clone();
    let note_on_for_audio = note_on.clone();
    
    // Callback para mensajes MIDI
    let _midi_connection = midi_in.connect(&ports[0], "midi-read", move |_timestamp, message, _| {
        if message.len() == 3 {
            match message[0] {
                0x90 => { // Note On
                    let note = message[1];
                    let velocity = message[2];
                    if velocity > 0 {
                        let freq = midi_note_to_freq(note);
                        current_freq.store((freq * 10000.0) as u64, Ordering::Relaxed);
                        note_on.store(true, Ordering::Relaxed);
                    } else {
                        // Note On con velocidad 0 es equivalente a Note Off
                        note_on.store(false, Ordering::Relaxed);
                    }
                },
                0x80 => { // Note Off
                    note_on.store(false, Ordering::Relaxed);
                },
                _ => (),
            }
        }
    }, ()).unwrap();

    // Configurar salida de audio
    let host = cpal::default_host();
    
    // Listar dispositivos de salida disponibles
    println!("\nDispositivos de salida disponibles:");
    let output_devices = host.output_devices()
        .expect("Error al obtener dispositivos de salida");
    
    let mut devices_vec = Vec::new();
    for (idx, device) in output_devices.enumerate() {
        println!("{}. {}", idx, device.name().unwrap_or_else(|_| "Nombre desconocido".into()));
        devices_vec.push(device);
    }

    println!("\nSelecciona un dispositivo (0-{}): ", devices_vec.len() - 1);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let device_index: usize = input.trim().parse().unwrap_or(0);
    
    let device = devices_vec.get(device_index).cloned().unwrap_or_else(|| {
        println!("Índice inválido, usando dispositivo por defecto");
        host.default_output_device()
            .expect("No se encontró dispositivo de audio")
    });

    println!("Usando host de audio: {}", host.id().name());
    println!("Dispositivo de salida: {}", device.name().unwrap());

    let config = device.default_output_config().unwrap();
    println!("Configuración por defecto: {:?}", config);
    
    let sample_rate = config.sample_rate().0 as f32;
    let mut sample_clock = 0f32;
    
    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for sample in data.iter_mut() {
                let freq = freq_for_audio.load(Ordering::Relaxed) as f32 / 10000.0;
                *sample = if note_on_for_audio.load(Ordering::Relaxed) {
                    (2.0 * std::f32::consts::PI * freq * sample_clock / sample_rate).sin() * 0.5
                } else {
                    0.0
                };
                sample_clock += 1.0;
            }
        },
        |err| eprintln!("Error en el stream: {}", err),
        Some(Duration::from_secs(1))
    ).unwrap();

    stream.play().unwrap();

    // Mantener el programa corriendo
    std::thread::sleep(std::time::Duration::from_secs(3600));
}

fn midi_note_to_freq(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}