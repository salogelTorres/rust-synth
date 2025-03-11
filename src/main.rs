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
        println!("{}. {} {}", idx, host_id.name(), 
            if host_id.name() == "ASIO" { "(Recomendado para menor latencia)" } else { "" });
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

    if host.id().name() == "ASIO" {
        println!("Usando ASIO - Asegúrate de tener abierto el panel de control de ASIO4ALL");
        println!("y haber configurado correctamente tu dispositivo de audio");
    }

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
    
    // Crear una configuración compatible con ASIO
    let config = if host.id().name() == "ASIO" {
        // ASIO generalmente requiere configuraciones específicas
        let supported_configs = device.supported_output_configs()
            .expect("Error al obtener configuraciones soportadas");

        // Imprimir configuraciones soportadas para debug
        println!("\nConfiguraciones soportadas por ASIO:");
        let configs: Vec<_> = supported_configs.collect();
        for config in &configs {
            println!("Canales: {}, Formato: {:?}, Sample Rate: {:?}-{:?}", 
                config.channels(), 
                config.sample_format(),
                config.min_sample_rate(),
                config.max_sample_rate());
        }

        // Encontrar una configuración compatible
        let supported_config = configs.iter()
            .find(|config| config.channels() == 2)
            .expect("No se encontró una configuración compatible con ASIO");

        cpal::StreamConfig {
            channels: 2,
            sample_rate: supported_config.min_sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        }
    } else {
        cpal::StreamConfig {
            channels: config.channels(),
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Fixed(512),
        }
    };
    
    // Get the sample format before creating the stream
    let sample_format = device.default_output_config()
        .expect("Failed to get default output config")
        .sample_format();

    println!("Configuración optimizada: {:?}", config);
    
    let stream = match sample_format {
        cpal::SampleFormat::I32 => device.build_output_stream(
            &config,
            move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                let mut notes = notes_for_audio.lock().unwrap();
                
                let channels = config.channels as usize;
                for frame in data.chunks_mut(channels) {
                    let sample = {
                        let mut mix = 0.0;
                        
                        for note in notes.values_mut() {
                            let envelope_amp = note.envelope.next_sample();
                            note.phase += 2.0 * std::f32::consts::PI * note.frequency / sample_rate;
                            while note.phase >= 2.0 * std::f32::consts::PI {
                                note.phase -= 2.0 * std::f32::consts::PI;
                            }
                            mix += note.phase.sin() * envelope_amp * 0.15;
                        }
                        
                        // Convertir de f32 a i32
                        (soft_clip(mix) * i32::MAX as f32) as i32
                    };

                    for channel in frame.iter_mut() {
                        *channel = sample;
                    }
                }
                
                notes.retain(|_, note| !note.envelope.is_finished());
            },
            |err| eprintln!("Error en el stream: {}", err),
            Some(Duration::from_millis(100))
        ),
        _ => device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut notes = notes_for_audio.lock().unwrap();
                
                // Procesar el audio en grupos de channels
                let channels = config.channels as usize;
                for frame in data.chunks_mut(channels) {
                    let sample = {
                        let mut mix = 0.0;
                        
                        for note in notes.values_mut() {
                            let envelope_amp = note.envelope.next_sample();
                            
                            note.phase += 2.0 * std::f32::consts::PI * note.frequency / sample_rate;
                            while note.phase >= 2.0 * std::f32::consts::PI {
                                note.phase -= 2.0 * std::f32::consts::PI;
                            }
                            
                            mix += note.phase.sin() * envelope_amp * 0.15;
                        }
                        
                        soft_clip(mix)
                    };

                    // Copiar el mismo valor a todos los canales
                    for channel in frame.iter_mut() {
                        *channel = sample;
                    }
                }
                
                notes.retain(|_, note| !note.envelope.is_finished());
            },
            |err| eprintln!("Error en el stream: {}", err),
            Some(Duration::from_millis(100))
        )
    }.unwrap();

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