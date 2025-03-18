use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use vst3_com::sys::GUID;
use vst3_plugin::{
    audio_processor::{AudioProcessor, Process, ProcessConfig, ProcessData, ProcessStatus},
    base::{Plugin, ThreadingModel},
    prelude::*,
    util::ProcessSetup,
};

mod audio;
mod midi;
mod structs;
mod gui;

use crate::audio::{Note, soft_clip};
use crate::gui::WaveType;
use crate::structs::envelope::Envelope;

#[derive(Default)]
struct RustSynthController {
    wave_type: Arc<Mutex<WaveType>>,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

#[derive(Default)]
struct RustSynth {
    active_notes: Arc<Mutex<HashMap<u8, Note>>>,
    sample_rate: f32,
    wave_type: Arc<Mutex<WaveType>>,
    process_config: ProcessConfig,
    controller: RustSynthController,
}

impl Plugin for RustSynth {
    fn get_info(&self) -> PluginInfo {
        PluginInfo {
            name: "Rust Synth".into(),
            vendor: "Your Name".into(),
            url: "".into(),
            email: "".into(),
            version: "1.0.0".into(),
            unique_id: GUID {
                data: [0x12345678, 0x1234, 0x1234, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0],
            },
            category: vst3_plugin::base::PlugCategory::Instrument,
            ..Default::default()
        }
    }

    fn initialize(&mut self) -> bool {
        true
    }

    fn terminate(&mut self) {}
}

impl AudioProcessor for RustSynth {
    fn set_process_config(&mut self, config: ProcessConfig) {
        self.process_config = config;
        self.sample_rate = config.sample_rate;
    }

    fn process(&mut self, data: ProcessData<'_>) -> ProcessStatus {
        // Procesar eventos MIDI
        if let Some(events) = data.inputs.events {
            for event in events.events() {
                if let Some(midi_event) = event.try_as_midi() {
                    self.handle_midi_event(midi_event);
                }
            }
        }

        // Procesar audio
        if let Some(mut output) = data.outputs.first_mut() {
            let num_samples = output.samples_per_channel() as usize;
            let mut notes = self.active_notes.lock().unwrap();

            for frame_idx in 0..num_samples {
                let mut mix = 0.0;

                for note in notes.values_mut() {
                    let envelope_amp = note.envelope.next_sample();
                    let sine_value = note.get_sample();
                    mix += sine_value * envelope_amp * 0.15;
                }

                // Aplicar soft clip
                let processed = soft_clip(mix);

                // Escribir a todos los canales de salida
                for channel in output.channels_mut() {
                    channel[frame_idx] = processed;
                }
            }

            // Eliminar notas terminadas
            notes.retain(|_, note| !note.envelope.is_finished());
        }

        ProcessStatus::Normal
    }

    fn get_tail_samples(&self) -> u32 {
        0
    }
}

impl RustSynth {
    fn handle_midi_event(&mut self, event: MidiEvent<'_>) {
        let status = event.data[0] & 0xF0;
        match status {
            0x90 => { // Note On
                let note = event.data[1];
                let velocity = event.data[2] as f32 / 127.0;
                if velocity > 0.0 {
                    let freq = midi::midi_note_to_freq(note);
                    let mut envelope = Envelope::new(self.sample_rate);
                    envelope.set_adsr(
                        self.controller.attack,
                        self.controller.decay,
                        self.controller.sustain,
                        self.controller.release
                    );
                    envelope.set_velocity(velocity);
                    let current_wave_type = *self.wave_type.lock().unwrap();
                    let new_note = Note::new(
                        freq,
                        envelope,
                        self.sample_rate,
                        current_wave_type,
                        current_wave_type,
                    );
                    self.active_notes.lock().unwrap().insert(note, new_note);
                } else {
                    if let Some(note) = self.active_notes.lock().unwrap().get_mut(&note) {
                        note.envelope.note_off();
                    }
                }
            },
            0x80 => { // Note Off
                let note = event.data[1];
                if let Some(note) = self.active_notes.lock().unwrap().get_mut(&note) {
                    note.envelope.note_off();
                }
            },
            _ => (),
        }
    }
}

impl EditController for RustSynth {
    fn set_component_state(&mut self, _state: &str) -> tresult {
        kResultOk
    }

    fn set_state(&mut self, _state: &str) -> tresult {
        kResultOk
    }

    fn get_state(&mut self) -> String {
        String::new()
    }

    fn get_parameter_count(&self) -> i32 {
        5
    }

    fn get_parameter_info(&self, param_index: i32) -> ParameterInfo {
        match param_index {
            0 => ParameterInfo {
                id: 0,
                title: String::from("Wave Type"),
                short_title: String::from("Wave"),
                units: String::new(),
                step_count: 3,
                default_normalized_value: 0.0,
                unit_id: 0,
                parameter_flags: ParameterFlags::empty(),
            },
            1 => ParameterInfo {
                id: 1,
                title: String::from("Attack"),
                short_title: String::from("Atk"),
                units: String::from("s"),
                step_count: 0,
                default_normalized_value: 0.01,
                unit_id: 0,
                parameter_flags: ParameterFlags::empty(),
            },
            2 => ParameterInfo {
                id: 2,
                title: String::from("Decay"),
                short_title: String::from("Dec"),
                units: String::from("s"),
                step_count: 0,
                default_normalized_value: 0.1,
                unit_id: 0,
                parameter_flags: ParameterFlags::empty(),
            },
            3 => ParameterInfo {
                id: 3,
                title: String::from("Sustain"),
                short_title: String::from("Sus"),
                units: String::new(),
                step_count: 0,
                default_normalized_value: 0.7,
                unit_id: 0,
                parameter_flags: ParameterFlags::empty(),
            },
            4 => ParameterInfo {
                id: 4,
                title: String::from("Release"),
                short_title: String::from("Rel"),
                units: String::from("s"),
                step_count: 0,
                default_normalized_value: 0.3,
                unit_id: 0,
                parameter_flags: ParameterFlags::empty(),
            },
            _ => Default::default(),
        }
    }

    fn get_parameter_normalized(&self, id: u32) -> f64 {
        match id {
            0 => *self.wave_type.lock().unwrap() as u8 as f64 / 3.0,
            1 => self.controller.attack as f64,
            2 => self.controller.decay as f64,
            3 => self.controller.sustain as f64,
            4 => self.controller.release as f64,
            _ => 0.0,
        }
    }

    fn set_parameter_normalized(&mut self, id: u32, value: f64) {
        match id {
            0 => {
                let wave_type = match (value * 3.0).round() as u8 {
                    0 => WaveType::Sine,
                    1 => WaveType::Square,
                    2 => WaveType::Triangle,
                    _ => WaveType::Sawtooth,
                };
                *self.wave_type.lock().unwrap() = wave_type;
            }
            1 => self.controller.attack = value as f32,
            2 => self.controller.decay = value as f32,
            3 => self.controller.sustain = value as f32,
            4 => self.controller.release = value as f32,
            _ => (),
        }
    }
}

impl ThreadingModel for RustSynth {
    type ThreadingModel = SingleThread;
}

impl IPluginFactory for RustSynth {
    fn get_factory_info(&self) -> FactoryInfo {
        FactoryInfo {
            vendor: "Your Name".into(),
            url: "".into(),
            email: "".into(),
            flags: FactoryFlags::empty(),
        }
    }

    fn get_class_info(&self, index: i32) -> ClassInfo {
        match index {
            0 => ClassInfo {
                cid: GUID {
                    data: [0x12345678, 0x1234, 0x1234, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0],
                },
                cardinality: 1,
                category: "Audio Module Class".into(),
                name: "Rust Synth".into(),
                vendor: "Your Name".into(),
                version: "1.0.0".into(),
                sdk_version: "VST 3.7.0".into(),
                class_flags: ClassFlags::empty(),
                subcategories: "Instrument|Synth".into(),
            },
            _ => Default::default(),
        }
    }

    fn create_instance(&self, _cid: &GUID) -> Option<Box<dyn IComponent>> {
        Some(Box::new(RustSynth::default()))
    }
}

plugin_factory!(RustSynth); 