mod audio;
mod midi;
mod structs;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use midir::MidiInput;
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::{Mutex, Condvar};

use crate::audio::soft_clip;
use crate::midi::midi_note_to_freq;
use crate::structs::envelope::Envelope;
use crate::structs::note::Note;

fn main() {
    // Reemplazar el HashSet por un HashMap
    let active_notes = Arc::new(Mutex::new(HashMap::new()));
    
    // Configurar entrada MIDI
    let midi_in = MidiInput::new("rust-synth").unwrap();
    let ports = midi_in.ports();
    
    if ports.is_empty() {
        println!("No hay dispositivos MIDI disponibles");
        return;
    }

    let notes_for_audio = active_notes.clone();

    // Listar hosts de audio disponibles
    println!("\nHosts de audio disponibles:");
    let available_hosts = cpal::available_hosts();
    for (idx, host_id) in available_hosts.iter().enumerate() {
        println!("{}. {}", idx, host_id.name());
    }

    println!("\nSelecciona un host (0-{}): ", available_hosts.len() - 1);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let host_index: usize = input.trim().parse().unwrap_or(0);

    let host = if host_index < available_hosts.len() {
        cpal::host_from_id(available_hosts[host_index])
            .expect("Error al crear el host")
    } else {
        println!("Índice inválido, usando host por defecto");
        cpal::default_host()
    };

    println!("Usando host de audio: {}", host.id().name());

    // Get sample rate before MIDI callback
    let device = host.default_output_device().expect("No se encontró dispositivo de audio");
    let config = device.default_output_config().unwrap();
    let sample_rate = config.sample_rate().0 as f32;
    
    // Callback para mensajes MIDI
    let _midi_connection = midi_in.connect(&ports[0], "midi-read", move |_timestamp, message, _| {
        if message.len() == 3 {
            let mut notes = active_notes.lock().unwrap();
            match message[0] {
                0x90 => { // Note On
                    let note = message[1];
                    let velocity = message[2];
                    if velocity > 0 {
                        let freq = midi_note_to_freq(note);
                        let mut envelope = Envelope::new(sample_rate);
                        envelope.note_on();
                        notes.insert(note, Note {
                            frequency: freq,
                            envelope,
                            phase: 0.0,  // Inicializar fase
                        });
                    } else {
                        if let Some(note_data) = notes.get_mut(&note) {
                            note_data.envelope.note_off();
                        }
                    }
                },
                0x80 => { // Note Off
                    let note = message[1];
                    if let Some(note_data) = notes.get_mut(&note) {
                        note_data.envelope.note_off();
                    }
                },
                _ => (),
            }
        }
    }, ()).unwrap();

    // Configurar salida de audio
    println!("Usando host de audio: {}", host.id().name());
    
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
    
    // Crear una configuración personalizada con buffer más pequeño
    let config = cpal::StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(512),  // Volvemos a 512 para estabilidad
    };
    
    println!("Configuración optimizada: {:?}", config);
    
    
    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut notes = notes_for_audio.lock().unwrap();
            
            for sample in data.iter_mut() {
                *sample = 0.0;
                
                // Process each note directly
                for note in notes.values_mut() {
                    let envelope_amp = note.envelope.next_sample();
                    
                    note.phase += 2.0 * std::f32::consts::PI * note.frequency / sample_rate;
                    while note.phase >= 2.0 * std::f32::consts::PI {
                        note.phase -= 2.0 * std::f32::consts::PI;
                    }
                    
                    *sample += note.phase.sin() * envelope_amp * 0.15;
                }
                
                *sample = soft_clip(*sample);
            }
            
            // Clean up finished notes after processing using the public method
            notes.retain(|_, note| !note.envelope.is_finished());
        },
        |err| eprintln!("Error en el stream: {}", err),
        Some(Duration::from_millis(100))
    ).unwrap();

    stream.play().unwrap();

    // Mantener el programa corriendo de forma más eficiente
    let running = Arc::new((Mutex::new(true), Condvar::new()));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        let (lock, cvar) = &*r;
        let mut running = lock.lock().unwrap();
        *running = false;
        cvar.notify_one();
    }).expect("Error setting Ctrl-C handler");

    let (lock, cvar) = &*running;
    let mut running = lock.lock().unwrap();
    while *running {
        running = cvar.wait(running).unwrap();
    }
}