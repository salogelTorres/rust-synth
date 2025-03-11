use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::time::Duration;

pub fn soft_clip(x: f32) -> f32 {
    if x > 1.0 {
        1.0 - (-1.0 * (x - 1.0)).exp()
    } else if x < -1.0 {
        -1.0 + (-1.0 * (-x - 1.0)).exp()
    } else {
        x
    }
}

pub fn list_audio_hosts() -> Vec<cpal::HostId> {
    let available_hosts = cpal::available_hosts();
    for (idx, host_id) in available_hosts.iter().enumerate() {
        println!("{}. {}", idx, host_id.name());
    }
    available_hosts
}

pub fn create_audio_config(device: &cpal::Device) -> cpal::StreamConfig {
    let config = device.default_output_config().unwrap();
    cpal::StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(512),
    }
}