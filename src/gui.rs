use eframe::egui;
use egui_extras::RetainedImage;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use cpal::Device;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use midir::{MidiInput, MidiInputConnection};
use crate::midi::midi_note_to_freq;
use crate::structs::envelope::Envelope;
use crate::structs::note::Note;

#[derive(Clone, Copy, PartialEq)]
pub enum WaveType {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

impl WaveType {
    fn as_str(&self) -> &'static str {
        match self {
            WaveType::Sine => "Senoidal",
            WaveType::Square => "Cuadrada",
            WaveType::Triangle => "Triangular",
            WaveType::Sawtooth => "Sierra",
        }
    }
}

// Estructura para almacenar la configuración del sintetizador
pub struct SynthConfig {
    pub host_index: usize,
    pub device_index: usize,
    pub sample_rate_index: usize,
    pub available_hosts: Vec<String>,
    pub available_devices: Vec<String>,
    pub available_sample_rates: Vec<u32>,
    pub selected_config: Option<cpal::SupportedStreamConfig>,
    pub running: bool,
    pub volume: Arc<Mutex<f32>>,
    pub wave_type: Arc<Mutex<WaveType>>,
}

impl Default for SynthConfig {
    fn default() -> Self {
        Self {
            host_index: 0,
            device_index: 0,
            sample_rate_index: 0,
            available_hosts: Vec::new(),
            available_devices: Vec::new(),
            available_sample_rates: Vec::new(),
            selected_config: None,
            running: false,
            volume: Arc::new(Mutex::new(0.15)),
            wave_type: Arc::new(Mutex::new(WaveType::Sine)),
        }
    }
}

// Estructura principal de la aplicación
pub struct SynthApp {
    config: Arc<Mutex<SynthConfig>>,
    active_notes: Arc<Mutex<HashMap<u8, Note>>>,
    sample_rate: Arc<Mutex<f32>>,
    stream_handle: Option<cpal::Stream>,
    midi_connection: Option<MidiInputConnection<()>>,
}

impl SynthApp {
    pub fn new(
        config: Arc<Mutex<SynthConfig>>,
        active_notes: Arc<Mutex<HashMap<u8, Note>>>,
        sample_rate: Arc<Mutex<f32>>,
    ) -> Self {
        Self {
            config,
            active_notes,
            sample_rate,
            stream_handle: None,
            midi_connection: None,
        }
    }

    fn init_audio_hosts(&mut self) {
        // Obtener hosts de audio disponibles
        let available_hosts = cpal::available_hosts();
        let host_names: Vec<String> = available_hosts.iter()
            .map(|host_id| host_id.name().to_string())
            .collect();
        
        // Actualizar la configuración
        {
            let mut config = self.config.lock().unwrap();
            config.available_hosts = host_names;
        }
        
        // Inicializar dispositivos para el host seleccionado
        self.update_devices();
    }
    
    fn update_devices(&mut self) {
        // Obtener el host seleccionado
        let host_index;
        {
            let config = self.config.lock().unwrap();
            host_index = config.host_index;
        }
        
        let available_hosts = cpal::available_hosts();
        if host_index >= available_hosts.len() {
            let mut config = self.config.lock().unwrap();
            config.host_index = 0;
            return;
        }
        
        let host_id = available_hosts[host_index];
        let host = cpal::host_from_id(host_id).expect("Error al crear el host");
        
        // Obtener dispositivos de salida disponibles
        let output_devices = host.output_devices().expect("Error al obtener dispositivos de salida");
        let device_names: Vec<String> = output_devices
            .map(|device| device.name().unwrap_or_else(|_| "Dispositivo desconocido".into()))
            .collect();
        
        // Actualizar la configuración
        {
            let mut config = self.config.lock().unwrap();
            config.available_devices = device_names;
        }
        
        // Actualizar frecuencias de muestreo para el dispositivo seleccionado
        self.update_sample_rates();
    }
    
    fn update_sample_rates(&mut self) {
        // Obtener el host y dispositivo seleccionados
        let host_index;
        let device_index;
        
        {
            let config = self.config.lock().unwrap();
            host_index = config.host_index;
            device_index = config.device_index;
        }
        
        let available_hosts = cpal::available_hosts();
        if host_index >= available_hosts.len() {
            return;
        }
        
        let host_id = available_hosts[host_index];
        let host = cpal::host_from_id(host_id).expect("Error al crear el host");
        
        // Obtener dispositivos de salida disponibles
        let output_devices = host.output_devices().expect("Error al obtener dispositivos de salida");
        let devices: Vec<Device> = output_devices.collect();
        
        if device_index >= devices.len() || devices.is_empty() {
            let mut config = self.config.lock().unwrap();
            config.device_index = 0;
            return;
        }
        
        // Obtener configuraciones soportadas para el dispositivo seleccionado
        let device = &devices[device_index];
        let supported_configs = match device.supported_output_configs() {
            Ok(configs) => configs,
            Err(_) => {
                return;
            }
        };

        // Manejar configuración específica para ASIO
        if host.id().name() == "ASIO" {
            let configs: Vec<_> = supported_configs.collect();
            
            // Encontrar una configuración compatible con al menos 44100 Hz
            if let Some(supported_config) = configs.iter()
                .find(|config| config.channels() == 2 && config.min_sample_rate().0 >= 44100)
                .or_else(|| configs.iter().find(|config| config.channels() == 2 && config.max_sample_rate().0 >= 44100))
            {
                let mut config = self.config.lock().unwrap();
                
                // Usar la frecuencia de muestreo más alta disponible para ASIO
                let sample_rate = if supported_config.min_sample_rate().0 >= 44100 {
                    supported_config.min_sample_rate().0
                } else {
                    44100
                };
                
                config.available_sample_rates = vec![sample_rate];
                config.sample_rate_index = 0;
                config.selected_config = Some(supported_config.clone().with_sample_rate(cpal::SampleRate(sample_rate)));
                *self.sample_rate.lock().unwrap() = sample_rate as f32;
            }
            return;
        }
        
        // Para otros hosts, mantener el comportamiento original
        let sample_rates: Vec<u32> = supported_configs
            .map(|config| {
                let min = config.min_sample_rate().0;
                let max = config.max_sample_rate().0;
                if min == max {
                    vec![min]
                } else {
                    // Incluir algunas frecuencias de muestreo comunes dentro del rango
                    let common_rates = [44100, 48000, 88200, 96000, 192000];
                    common_rates.iter()
                        .filter(|&&rate| rate >= min && rate <= max)
                        .cloned()
                        .collect()
                }
            })
            .flatten()
            .collect();
        
        // Actualizar la configuración
        let mut config = self.config.lock().unwrap();
        
        // Eliminar duplicados y ordenar
        config.available_sample_rates = sample_rates;
        config.available_sample_rates.sort();
        config.available_sample_rates.dedup();
        
        // Seleccionar una configuración por defecto
        if !config.available_sample_rates.is_empty() {
            if config.sample_rate_index >= config.available_sample_rates.len() {
                config.sample_rate_index = 0;
            }
            
            // Obtener la configuración seleccionada
            let sample_rate = config.available_sample_rates[config.sample_rate_index];
            
            // Intentar obtener la configuración por defecto
            if let Ok(supported_config) = device.default_output_config() {
                // Crear una configuración con la frecuencia de muestreo seleccionada
                config.selected_config = Some(supported_config);
                
                // Actualizar la frecuencia de muestreo compartida
                *self.sample_rate.lock().unwrap() = sample_rate as f32;
            }
        }
    }
    
    fn start_synth(&mut self) {
        println!("Iniciando sintetizador...");
        
        // Obtener la configuración actual
        let config_clone;
        let host_index;
        let device_index;
        let volume;
        
        {
            let config = self.config.lock().unwrap();
            if config.selected_config.is_none() {
                println!("Error: No hay configuración de audio seleccionada");
                return;
            }
            config_clone = config.selected_config.clone();
            host_index = config.host_index;
            device_index = config.device_index;
            volume = config.volume.clone();
        }
        
        // Obtener el host seleccionado
        let available_hosts = cpal::available_hosts();
        if host_index >= available_hosts.len() {
            println!("Error: Índice de host inválido");
            return;
        }
        
        let host_id = available_hosts[host_index];
        let host = cpal::host_from_id(host_id).expect("Error al crear el host");
        println!("Host de audio: {}", host.id().name());
        
        // Obtener dispositivos de salida disponibles
        let output_devices = host.output_devices().expect("Error al obtener dispositivos de salida");
        let devices: Vec<Device> = output_devices.collect();
        
        if device_index >= devices.len() || devices.is_empty() {
            println!("Error: No hay dispositivos de salida disponibles");
            return;
        }
        
        // Obtener el dispositivo seleccionado
        let device = &devices[device_index];
        println!("Dispositivo de salida: {}", device.name().unwrap_or_else(|_| "Desconocido".into()));
        
        // Obtener la configuración seleccionada
        let supported_config = config_clone.unwrap();
        let sample_format = supported_config.sample_format();
        println!("Formato de muestra: {:?}", sample_format);
        
        // Crear configuración de stream
        let stream_config = cpal::StreamConfig {
            channels: supported_config.channels(),
            sample_rate: supported_config.sample_rate(),
            buffer_size: if host.id().name() == "ASIO" {
                cpal::BufferSize::Default // ASIO maneja su propio tamaño de buffer
            } else {
                cpal::BufferSize::Fixed(512)
            },
        };
        println!("Configuración del stream:");
        println!("  Canales: {}", stream_config.channels);
        println!("  Frecuencia de muestreo: {} Hz", stream_config.sample_rate.0);
        println!("  Tamaño del buffer: {:?}", stream_config.buffer_size);
        
        // Clonar referencias para el callback
        let active_notes = self.active_notes.clone();
        let sample_rate_shared = self.sample_rate.clone();
        
        // Tamaño del buffer de audio para reducir las operaciones de bloqueo
        const BUFFER_SIZE: usize = 64;
        
        // Crear stream de audio
        let stream = match sample_format {
            cpal::SampleFormat::I32 => device.build_output_stream(
                &stream_config,
                move |data: &mut [i32], _: &cpal::OutputCallbackInfo| {
                    // Adquirir el bloqueo una vez por buffer en lugar de por muestra
                    let mut notes_guard = active_notes.lock().unwrap();
                    let current_sample_rate = *sample_rate_shared.lock().unwrap();
                    let current_volume = *volume.lock().unwrap();
                    
                    // Actualizar las frecuencias de muestreo si es necesario
                    for note in notes_guard.values_mut() {
                        if note.sample_rate != current_sample_rate {
                            note.sample_rate = current_sample_rate;
                            note.update_frequency(note.frequency);
                        }
                    }
                    
                    let channels = stream_config.channels as usize;
                    
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
                                        mix += sine_value * envelope_amp * current_volume;
                                    }
                                    
                                    // Aplicar soft clip y convertir a i32
                                    temp_buffer[i] = crate::audio::soft_clip(mix);
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
                None,
            ),
            _ => device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Adquirir el bloqueo una vez por buffer en lugar de por muestra
                    let mut notes_guard = active_notes.lock().unwrap();
                    let current_sample_rate = *sample_rate_shared.lock().unwrap();
                    let current_volume = *volume.lock().unwrap();
                    
                    // Actualizar las frecuencias de muestreo si es necesario
                    for note in notes_guard.values_mut() {
                        if note.sample_rate != current_sample_rate {
                            note.sample_rate = current_sample_rate;
                            note.update_frequency(note.frequency);
                        }
                    }
                    
                    let channels = stream_config.channels as usize;
                    
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
                                        mix += sine_value * envelope_amp * current_volume;
                                    }
                                    
                                    // Aplicar soft clip
                                    temp_buffer[i] = crate::audio::soft_clip(mix);
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
                None,
            ),
        }.unwrap();
        
        // Iniciar el stream
        stream.play().unwrap();
        
        // Guardar el stream
        self.stream_handle = Some(stream);
        
        // Actualizar estado
        self.config.lock().unwrap().running = true;
    }
    
    fn stop_synth(&mut self) {
        // Detener el stream
        self.stream_handle = None;
        
        // Actualizar estado
        self.config.lock().unwrap().running = false;
    }
    
    fn connect_midi(&mut self) {
        // Verificar si ya hay una conexión MIDI
        if self.midi_connection.is_some() {
            return;
        }
        
        // Crear entrada MIDI
        let midi_in = MidiInput::new("rust-synth").unwrap();
        let ports = midi_in.ports();
        
        if ports.is_empty() {
            println!("No se encontraron puertos MIDI disponibles");
            return;
        }
        
        println!("Puertos MIDI disponibles:");
        for (i, port) in ports.iter().enumerate() {
            println!("  {}: {}", i, midi_in.port_name(port).unwrap_or_else(|_| "Puerto desconocido".into()));
        }
        
        // Clonar referencias para el callback
        let active_notes = self.active_notes.clone();
        let sample_rate_for_midi = self.sample_rate.clone();
        let wave_type = self.config.lock().unwrap().wave_type.clone();
        
        // Conectar al primer puerto MIDI disponible
        let midi_connection = midi_in.connect(&ports[0], "midi-read", move |_timestamp, message, _| {
            if message.len() == 3 {
                let mut notes = active_notes.lock().unwrap();
                let current_sample_rate = *sample_rate_for_midi.lock().unwrap();
                let current_wave_type = *wave_type.lock().unwrap();
                
                match message[0] {
                    0x90 => { // Note On
                        let note = message[1];
                        let velocity = message[2] as f32 / 127.0;
                        if velocity > 0.0 {
                            let freq = midi_note_to_freq(note);
                            println!("Nota ON - Número: {}, Frecuencia: {:.2} Hz, Velocidad: {:.2}", note, freq, velocity);
                            let mut envelope = Envelope::new(current_sample_rate);
                            envelope.set_adsr(0.01, 0.1, 0.7, 0.3);
                            envelope.set_velocity(velocity);
                            envelope.note_on(); // Asegurarse de que la envolvente se inicie
                            notes.insert(note, Note::new(freq, envelope, current_sample_rate, current_wave_type));
                        } else {
                            println!("Nota OFF (velocity 0) - Número: {}", note);
                            if let Some(note_data) = notes.get_mut(&note) {
                                note_data.envelope.note_off();
                            }
                        }
                    },
                    0x80 => { // Note Off
                        let note = message[1];
                        println!("Nota OFF - Número: {}", note);
                        if let Some(note_data) = notes.get_mut(&note) {
                            note_data.envelope.note_off();
                        }
                    },
                    _ => (),
                }
            }
        }, ()).unwrap();
        
        println!("Conexión MIDI establecida");
        
        // Guardar la conexión MIDI
        self.midi_connection = Some(midi_connection);
    }
    
    fn disconnect_midi(&mut self) {
        // Cerrar la conexión MIDI
        self.midi_connection = None;
    }
}

impl eframe::App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Inicializar hosts de audio si es necesario
        if self.config.lock().unwrap().available_hosts.is_empty() {
            self.init_audio_hosts();
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Rust Synth");
            ui.add_space(10.0);
            
            // Configuración de audio
            ui.group(|ui| {
                ui.heading("Configuración de Audio");
                
                // Obtener datos para mostrar en la UI
                let host_text;
                let device_text;
                let rate_text;
                let volume;
                let wave_type;
                let available_hosts;
                let available_devices;
                let available_sample_rates;
                let host_index;
                let device_index;
                let sample_rate_index;
                
                {
                    let config = self.config.lock().unwrap();
                    host_text = config.available_hosts.get(config.host_index)
                        .cloned().unwrap_or_else(|| "Ninguno".to_string());
                    device_text = config.available_devices.get(config.device_index)
                        .cloned().unwrap_or_else(|| "Ninguno".to_string());
                    rate_text = config.available_sample_rates.get(config.sample_rate_index)
                        .map(|rate| format!("{} Hz", rate))
                        .unwrap_or_else(|| "Ninguna".to_string());
                    volume = config.volume.clone();
                    wave_type = config.wave_type.clone();
                    
                    // Clonar las colecciones para evitar problemas de préstamo
                    available_hosts = config.available_hosts.clone();
                    available_devices = config.available_devices.clone();
                    available_sample_rates = config.available_sample_rates.clone();
                    host_index = config.host_index;
                    device_index = config.device_index;
                    sample_rate_index = config.sample_rate_index;
                }
                
                // Selección de host
                let mut host_changed = false;
                let mut new_host_index = host_index;
                
                egui::ComboBox::from_label("Host de Audio")
                    .selected_text(host_text)
                    .show_ui(ui, |ui| {
                        for (i, host) in available_hosts.iter().enumerate() {
                            let is_selected = i == host_index;
                            if ui.selectable_label(is_selected, host).clicked() {
                                new_host_index = i;
                                host_changed = true;
                            }
                        }
                    });
                
                if host_changed {
                    let mut config = self.config.lock().unwrap();
                    config.host_index = new_host_index;
                    drop(config);
                    self.update_devices();
                }
                
                // Selección de dispositivo
                let mut device_changed = false;
                let mut new_device_index = device_index;
                
                egui::ComboBox::from_label("Dispositivo de Salida")
                    .selected_text(device_text)
                    .show_ui(ui, |ui| {
                        for (i, device) in available_devices.iter().enumerate() {
                            let is_selected = i == device_index;
                            if ui.selectable_label(is_selected, device).clicked() {
                                new_device_index = i;
                                device_changed = true;
                            }
                        }
                    });
                
                if device_changed {
                    let mut config = self.config.lock().unwrap();
                    config.device_index = new_device_index;
                    drop(config);
                    self.update_sample_rates();
                }
                
                // Selección de frecuencia de muestreo
                let mut rate_changed = false;
                let mut new_rate_index = sample_rate_index;
                let mut new_rate = 0u32;
                
                egui::ComboBox::from_label("Frecuencia de Muestreo")
                    .selected_text(rate_text)
                    .show_ui(ui, |ui| {
                        for (i, rate) in available_sample_rates.iter().enumerate() {
                            let is_selected = i == sample_rate_index;
                            if ui.selectable_label(is_selected, format!("{} Hz", rate)).clicked() {
                                new_rate_index = i;
                                new_rate = *rate;
                                rate_changed = true;
                            }
                        }
                    });
                
                if rate_changed {
                    let mut config = self.config.lock().unwrap();
                    config.sample_rate_index = new_rate_index;
                    drop(config);
                    *self.sample_rate.lock().unwrap() = new_rate as f32;
                }
            });
            
            ui.add_space(10.0);
            
            // Control de volumen y forma de onda en una sección separada
            ui.group(|ui| {
                ui.heading("Controles de Sonido");
                ui.horizontal(|ui| {
                    // Control de volumen
                    ui.vertical(|ui| {
                        ui.label("Volumen");
                        let mut volume_value = *self.config.lock().unwrap().volume.lock().unwrap();
                        let volume_response = ui.add(egui::widgets::Slider::new(&mut volume_value, 0.0..=1.0)
                            .show_value(true)
                            .text(""));
                        if volume_response.changed() {
                            *self.config.lock().unwrap().volume.lock().unwrap() = volume_value;
                            println!("Volumen: {}", volume_value);
                        }
                    });

                    ui.add_space(20.0);

                    // Control de tipo de onda con botones visuales
                    ui.vertical(|ui| {
                        ui.label("Tipo de Onda");
                        ui.horizontal(|ui| {
                            let button_size = egui::vec2(50.0, 50.0);
                            let mut current_wave = *self.config.lock().unwrap().wave_type.lock().unwrap();
                            
                            // Botón Sine
                            let sine_response = ui.add(egui::Button::new("")
                                .min_size(button_size)
                                .selected(current_wave == WaveType::Sine));
                            if sine_response.clicked() {
                                *self.config.lock().unwrap().wave_type.lock().unwrap() = WaveType::Sine;
                            }
                            draw_sine_wave(ui.painter(), sine_response.rect, current_wave == WaveType::Sine);

                            // Botón Square
                            let square_response = ui.add(egui::Button::new("")
                                .min_size(button_size)
                                .selected(current_wave == WaveType::Square));
                            if square_response.clicked() {
                                *self.config.lock().unwrap().wave_type.lock().unwrap() = WaveType::Square;
                            }
                            draw_square_wave(ui.painter(), square_response.rect, current_wave == WaveType::Square);

                            // Botón Triangle
                            let triangle_response = ui.add(egui::Button::new("")
                                .min_size(button_size)
                                .selected(current_wave == WaveType::Triangle));
                            if triangle_response.clicked() {
                                *self.config.lock().unwrap().wave_type.lock().unwrap() = WaveType::Triangle;
                            }
                            draw_triangle_wave(ui.painter(), triangle_response.rect, current_wave == WaveType::Triangle);

                            // Botón Sawtooth
                            let saw_response = ui.add(egui::Button::new("")
                                .min_size(button_size)
                                .selected(current_wave == WaveType::Sawtooth));
                            if saw_response.clicked() {
                                *self.config.lock().unwrap().wave_type.lock().unwrap() = WaveType::Sawtooth;
                            }
                            draw_sawtooth_wave(ui.painter(), saw_response.rect, current_wave == WaveType::Sawtooth);
                        });
                    });
                });
            });
            
            ui.add_space(10.0);
            
            // Controles del sintetizador
            ui.group(|ui| {
                ui.heading("Controles");
                
                let is_running;
                let is_midi_connected;
                
                {
                    let config = self.config.lock().unwrap();
                    is_running = config.running;
                    is_midi_connected = self.midi_connection.is_some();
                }
                
                ui.horizontal(|ui| {
                    if is_running {
                        if ui.button("Detener Sintetizador").clicked() {
                            self.stop_synth();
                        }
                    } else {
                        if ui.button("Iniciar Sintetizador").clicked() {
                            self.start_synth();
                        }
                    }
                    
                    if is_midi_connected {
                        if ui.button("Desconectar MIDI").clicked() {
                            self.disconnect_midi();
                        }
                    } else {
                        if ui.button("Conectar MIDI").clicked() {
                            self.connect_midi();
                        }
                    }
                });
            });
            
            ui.add_space(10.0);
            
            // Estado actual
            ui.group(|ui| {
                ui.heading("Estado");
                
                let is_running;
                let is_midi_connected;
                let sample_rate;
                let active_note_count;
                
                {
                    let config = self.config.lock().unwrap();
                    is_running = config.running;
                    is_midi_connected = self.midi_connection.is_some();
                    sample_rate = *self.sample_rate.lock().unwrap();
                    active_note_count = self.active_notes.lock().unwrap().len();
                }
                
                ui.label(format!("Estado del sintetizador: {}", if is_running { "Ejecutando" } else { "Detenido" }));
                ui.label(format!("Conexión MIDI: {}", if is_midi_connected { "Conectado" } else { "Desconectado" }));
                ui.label(format!("Frecuencia de muestreo actual: {:.1} Hz", sample_rate));
                ui.label(format!("Notas activas: {}", active_note_count));
            });
            
            // Información
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.hyperlink_to("Rust Synth", "https://github.com/tu-usuario/rust-synth");
                ui.label("Presiona las teclas en tu controlador MIDI para tocar notas");
                ui.label("Desarrollado con Rust y egui");
            });
        });
        
        // Solicitar repintado continuo para actualizar el estado
        ctx.request_repaint();
    }
}

fn draw_sine_wave(painter: &egui::Painter, rect: egui::Rect, selected: bool) {
    let color = if selected { egui::Color32::WHITE } else { egui::Color32::GRAY };
    let stroke = egui::Stroke::new(2.0, color);
    let mut points = Vec::new();
    let width = rect.width();
    let height = rect.height();
    let center_y = rect.center().y;
    
    for i in 0..=20 {
        let x = rect.left() + (i as f32 * width / 20.0);
        let y = center_y + (i as f32 * std::f32::consts::TAU / 20.0).sin() * height * 0.3;
        points.push(egui::pos2(x, y));
    }
    painter.add(egui::Shape::line(points, stroke));
}

fn draw_square_wave(painter: &egui::Painter, rect: egui::Rect, selected: bool) {
    let color = if selected { egui::Color32::WHITE } else { egui::Color32::GRAY };
    let stroke = egui::Stroke::new(2.0, color);
    let points = vec![
        egui::pos2(rect.left(), rect.center().y + rect.height() * 0.3),
        egui::pos2(rect.left() + rect.width() * 0.25, rect.center().y + rect.height() * 0.3),
        egui::pos2(rect.left() + rect.width() * 0.25, rect.center().y - rect.height() * 0.3),
        egui::pos2(rect.left() + rect.width() * 0.75, rect.center().y - rect.height() * 0.3),
        egui::pos2(rect.left() + rect.width() * 0.75, rect.center().y + rect.height() * 0.3),
        egui::pos2(rect.right(), rect.center().y + rect.height() * 0.3),
    ];
    painter.add(egui::Shape::line(points, stroke));
}

fn draw_triangle_wave(painter: &egui::Painter, rect: egui::Rect, selected: bool) {
    let color = if selected { egui::Color32::WHITE } else { egui::Color32::GRAY };
    let stroke = egui::Stroke::new(2.0, color);
    let points = vec![
        egui::pos2(rect.left(), rect.center().y),
        egui::pos2(rect.left() + rect.width() * 0.25, rect.center().y - rect.height() * 0.3),
        egui::pos2(rect.left() + rect.width() * 0.75, rect.center().y + rect.height() * 0.3),
        egui::pos2(rect.right(), rect.center().y),
    ];
    painter.add(egui::Shape::line(points, stroke));
}

fn draw_sawtooth_wave(painter: &egui::Painter, rect: egui::Rect, selected: bool) {
    let color = if selected { egui::Color32::WHITE } else { egui::Color32::GRAY };
    let stroke = egui::Stroke::new(2.0, color);
    let points = vec![
        egui::pos2(rect.left(), rect.center().y + rect.height() * 0.3),
        egui::pos2(rect.left() + rect.width() * 0.5, rect.center().y - rect.height() * 0.3),
        egui::pos2(rect.left() + rect.width() * 0.5, rect.center().y + rect.height() * 0.3),
        egui::pos2(rect.right(), rect.center().y - rect.height() * 0.3),
    ];
    painter.add(egui::Shape::line(points, stroke));
} 