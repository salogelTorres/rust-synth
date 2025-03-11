mod audio;
mod midi;
mod structs;
mod gui;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::{Mutex, Condvar};
use std::env;
use egui::ViewportBuilder;

// Importaciones del módulo de audio
use crate::audio::{
    soft_clip,
    Note,
    create_audio_config,
    list_audio_hosts,
};

// Importaciones del módulo MIDI
use crate::midi::{midi_note_to_freq, connect_midi};

// Importaciones de GUI y estructuras
use crate::gui::{SynthApp, SynthConfig, WaveType};

fn main() {
    // Verificar si se debe usar la interfaz gráfica
    let args: Vec<String> = env::args().collect();
    let use_gui = args.len() > 1 && args[1] == "--gui";
    
    if use_gui {
        // Inicializar la configuración compartida
        let config = Arc::new(Mutex::new(SynthConfig::default()));
        
        // Inicializar las notas activas compartidas
        let active_notes = Arc::new(Mutex::new(HashMap::new()));
        
        // Inicializar la frecuencia de muestreo compartida
        let sample_rate_shared = Arc::new(Mutex::new(44100.0f32));
        
        // Crear la aplicación
        let app = SynthApp::new(
            config,
            active_notes,
            sample_rate_shared,
        );
        
        // Ejecutar la aplicación
        let native_options = eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_inner_size([800.0, 600.0]),
            ..Default::default()
        };
        
        eframe::run_native(
            "Rust Synth",
            native_options,
            Box::new(|_cc| Box::new(app)),
        ).unwrap();
    } else {
        // Versión de consola original
        run_console_version();
    }
}

fn run_console_version() {
    // Reemplazar el HashSet por un HashMap
    let active_notes = Arc::new(Mutex::new(HashMap::new()));
    
    // Usar un Arc<Mutex<f32>> para la frecuencia de muestreo
    let sample_rate_shared = Arc::new(Mutex::new(44100.0f32));
    
    // Tipo de onda compartido
    let wave_type_shared = Arc::new(Mutex::new(WaveType::Sine));
    
    // Configurar entrada MIDI
    let midi_in = connect_midi(
        active_notes.clone(),
        sample_rate_shared.clone(),
        wave_type_shared.clone(),
    ).expect("No se pudo conectar al dispositivo MIDI");

    // Listar hosts de audio disponibles
    println!("\nHosts de audio disponibles:");
    let available_hosts = list_audio_hosts();

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
    let default_config = device.default_output_config().unwrap();
    *sample_rate_shared.lock().unwrap() = default_config.sample_rate().0 as f32;
    
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

        // Encontrar una configuración compatible con al menos 44100 Hz
        let supported_config = configs.iter()
            .find(|config| config.channels() == 2 && config.min_sample_rate().0 >= 44100)
            .or_else(|| configs.iter().find(|config| config.channels() == 2 && config.max_sample_rate().0 >= 44100))
            .expect("No se encontró una configuración compatible con ASIO");

        // Usar directamente la frecuencia de muestreo de la configuración seleccionada
        // o forzar a 44100 Hz si es menor
        let asio_sample_rate = if supported_config.min_sample_rate().0 >= 44100 {
            supported_config.min_sample_rate()
        } else {
            // Forzar a 44100 Hz como mínimo
            cpal::SampleRate(44100)
        };
        
        println!("Configuración ASIO seleccionada: Canales: {}, Formato: {:?}, Sample Rate: {:?}", 
            supported_config.channels(), 
            supported_config.sample_format(),
            asio_sample_rate);
        
        // Actualizar la variable sample_rate para que coincida con la configuración de ASIO
        *sample_rate_shared.lock().unwrap() = asio_sample_rate.0 as f32;
        let current_sample_rate = *sample_rate_shared.lock().unwrap();
        
        println!("Usando frecuencia de muestreo para ASIO: {} Hz", current_sample_rate);
        println!("Nota A4 (MIDI 69) = {} Hz", midi_note_to_freq(69));
        println!("Nota C4 (MIDI 60) = {} Hz", midi_note_to_freq(60));
        println!("Nota C7 (MIDI 96) = {} Hz", midi_note_to_freq(96));
        println!("Frecuencia de Nyquist: {} Hz", current_sample_rate / 2.0);
        println!("Límite seguro para evitar aliasing: {} Hz", current_sample_rate / 4.0);

        cpal::StreamConfig {
            channels: 2,
            sample_rate: asio_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        }
    } else {
        create_audio_config(&device)
    };
    
    // Get the sample format before creating the stream
    let sample_format = device.default_output_config()
        .expect("Failed to get default output config")
        .sample_format();

    println!("Configuración optimizada: {:?}", config);
    
    let sample_rate_for_audio = sample_rate_shared.clone();
    let notes_for_audio = active_notes.clone();
    
    // Tamaño del buffer de audio para reducir las operaciones de bloqueo
    const BUFFER_SIZE: usize = 64;
    
    let stream = match sample_format {
        cpal::SampleFormat::I32 => device.build_output_stream(
            &config,
            move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                // Adquirir el bloqueo una vez por buffer en lugar de por muestra
                let mut notes_guard = notes_for_audio.lock().unwrap();
                let current_sample_rate = *sample_rate_for_audio.lock().unwrap();
                
                // Actualizar las frecuencias de muestreo si es necesario
                for note in notes_guard.values_mut() {
                    if note.sample_rate != current_sample_rate {
                        note.sample_rate = current_sample_rate;
                        note.update_frequency(note.frequency);
                    }
                }
                
                let channels = config.channels as usize;
                
                // Procesar el audio en bloques para mejorar la eficiencia
                for chunk in data.chunks_mut(channels * BUFFER_SIZE).filter(|c| !c.is_empty()) {
                    // Generar un buffer temporal de muestras
                    let mut temp_buffer = [0.0f32; BUFFER_SIZE];
                    
                    // Generar todas las muestras para este bloque
                    for (i, frame) in chunk.chunks_mut(channels).enumerate() {
                        if i < BUFFER_SIZE {
                            let sample = {
                                let mut mix = 0.0;
                                
                                for note in notes_guard.values_mut() {
                                    let envelope_amp = note.envelope.next_sample();
                                    let sine_value = note.get_sample();
                                    mix += sine_value * envelope_amp * 0.15;
                                }
                                
                                // Aplicar soft clip y convertir a i32
                                temp_buffer[i] = soft_clip(mix);
                                (temp_buffer[i] * i32::MAX as f32) as i32
                            };
                            
                            // Copiar la muestra a todos los canales
                            for channel in frame.iter_mut() {
                                *channel = sample;
                            }
                        }
                    }
                }
                
                // Eliminar las notas terminadas
                notes_guard.retain(|_, note| !note.envelope.is_finished());
            },
            |err| eprintln!("Error en el stream: {}", err),
            Some(Duration::from_millis(100))
        ),
        _ => device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Adquirir el bloqueo una vez por buffer en lugar de por muestra
                let mut notes_guard = notes_for_audio.lock().unwrap();
                let current_sample_rate = *sample_rate_for_audio.lock().unwrap();
                
                // Actualizar las frecuencias de muestreo si es necesario
                for note in notes_guard.values_mut() {
                    if note.sample_rate != current_sample_rate {
                        note.sample_rate = current_sample_rate;
                        note.update_frequency(note.frequency);
                    }
                }
                
                let channels = config.channels as usize;
                
                // Procesar el audio en bloques para mejorar la eficiencia
                for chunk in data.chunks_mut(channels * BUFFER_SIZE).filter(|c| !c.is_empty()) {
                    // Generar un buffer temporal de muestras
                    let mut temp_buffer = [0.0f32; BUFFER_SIZE];
                    
                    // Generar todas las muestras para este bloque
                    for (i, frame) in chunk.chunks_mut(channels).enumerate() {
                        if i < BUFFER_SIZE {
                            let sample = {
                                let mut mix = 0.0;
                                
                                for note in notes_guard.values_mut() {
                                    let envelope_amp = note.envelope.next_sample();
                                    let sine_value = note.get_sample();
                                    mix += sine_value * envelope_amp * 0.15;
                                }
                                
                                // Aplicar soft clip
                                temp_buffer[i] = soft_clip(mix);
                                temp_buffer[i]
                            };
                            
                            // Copiar la muestra a todos los canales
                            for channel in frame.iter_mut() {
                                *channel = sample;
                            }
                        }
                    }
                }
                
                // Eliminar las notas terminadas
                notes_guard.retain(|_, note| !note.envelope.is_finished());
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
    }).expect("Error al configurar el manejador de Ctrl+C");

    // Esperar a que el usuario presione Ctrl+C
    let (lock, cvar) = &*running;
    let mut running = lock.lock().unwrap();
    while *running {
        running = cvar.wait(running).unwrap();
    }
    
    println!("Saliendo...");
}