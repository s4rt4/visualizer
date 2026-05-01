use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use ringbuf::{traits::*, HeapCons, HeapRb};
use std::sync::Arc;

const BUFFER_CAPACITY: usize = 48_000 * 4;

#[derive(Debug)]
pub struct AudioState {
    pub sample_rate: u32,
    pub channels: u16,
    pub device_name: String,
    pub is_active: bool,
    pub last_error: Option<String>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
            device_name: "No device".to_owned(),
            is_active: false,
            last_error: None,
        }
    }
}

pub type SharedAudioState = Arc<Mutex<AudioState>>;

pub struct AudioEngine {
    state: SharedAudioState,
    consumer: Option<HeapCons<f32>>,
    stream: Option<cpal::Stream>,
}

impl AudioEngine {
    pub fn start(selected_device: Option<&str>) -> Self {
        let state = Arc::new(Mutex::new(AudioState::default()));
        let (stream, consumer) = start_stream(state.clone(), selected_device);
        Self {
            state,
            consumer,
            stream,
        }
    }

    pub fn restart(&mut self, selected_device: Option<&str>) {
        self.stream = None;
        self.consumer = None;
        {
            let mut state = self.state.lock();
            state.is_active = false;
            state.last_error = None;
        }
        let (stream, consumer) = start_stream(self.state.clone(), selected_device);
        self.stream = stream;
        self.consumer = consumer;
    }

    pub fn drain_samples(&mut self, max: usize) -> Vec<f32> {
        let Some(consumer) = &mut self.consumer else {
            return Vec::new();
        };

        let available = consumer.occupied_len();
        if available > max {
            consumer.skip(available - max);
        }

        let count = consumer.occupied_len().min(max);
        let mut samples = vec![0.0; count];
        let popped = consumer.pop_slice(&mut samples);
        samples.truncate(popped);
        samples
    }

    pub fn state(&self) -> SharedAudioState {
        self.state.clone()
    }

    pub fn devices() -> Vec<String> {
        let host = audio_host();
        host.output_devices()
            .map(|devices| {
                devices
                    .filter_map(|device| device.name().ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }
}

fn start_stream(
    state: SharedAudioState,
    selected_device: Option<&str>,
) -> (Option<cpal::Stream>, Option<HeapCons<f32>>) {
    let rb = HeapRb::<f32>::new(BUFFER_CAPACITY);
    let (producer, consumer) = rb.split();
    let host = audio_host();

    match start_loopback_stream(&host, state.clone(), selected_device, producer) {
        Ok(stream) => {
            state.lock().is_active = true;
            (Some(stream), Some(consumer))
        }
        Err(err) => {
            {
                let mut state = state.lock();
                state.is_active = false;
                state.last_error = Some(format!("Loopback stream failed: {err}"));
            }
            let rb = HeapRb::<f32>::new(BUFFER_CAPACITY);
            let (producer, consumer) = rb.split();
            match start_stereo_mix_stream(&host, state.clone(), producer) {
                Ok(stream) => {
                    state.lock().is_active = true;
                    (Some(stream), Some(consumer))
                }
                Err(fallback_err) => {
                    let mut state = state.lock();
                    state.is_active = false;
                    state.last_error = Some(format!(
                        "Loopback failed: {err}; Stereo Mix fallback failed: {fallback_err}"
                    ));
                    (None, None)
                }
            }
        }
    }
}

fn start_loopback_stream(
    host: &cpal::Host,
    state: SharedAudioState,
    selected_device: Option<&str>,
    producer: ringbuf::HeapProd<f32>,
) -> Result<cpal::Stream, String> {
    let device = select_output_device(host, selected_device)
        .ok_or_else(|| "No output device available".to_owned())?;
    let name = device
        .name()
        .unwrap_or_else(|_| "Unknown output".to_owned());
    let supported_config = device
        .default_output_config()
        .map_err(|err| format!("Default output config failed: {err}"))?;
    build_stream_for_device(device, name, supported_config, state, producer)
}

fn start_stereo_mix_stream(
    host: &cpal::Host,
    state: SharedAudioState,
    producer: ringbuf::HeapProd<f32>,
) -> Result<cpal::Stream, String> {
    let mut devices = host
        .input_devices()
        .map_err(|err| format!("Input device enumeration failed: {err}"))?;
    let device = devices
        .find(|device| {
            device
                .name()
                .map(|name| name.to_ascii_lowercase().contains("stereo mix"))
                .unwrap_or(false)
        })
        .ok_or_else(|| "Stereo Mix input device not found".to_owned())?;
    let name = device.name().unwrap_or_else(|_| "Stereo Mix".to_owned());
    let supported_config = device
        .default_input_config()
        .map_err(|err| format!("Stereo Mix config failed: {err}"))?;
    build_stream_for_device(device, name, supported_config, state, producer)
}

fn build_stream_for_device(
    device: cpal::Device,
    name: String,
    supported_config: cpal::SupportedStreamConfig,
    state: SharedAudioState,
    producer: ringbuf::HeapProd<f32>,
) -> Result<cpal::Stream, String> {
    let sample_rate = supported_config.sample_rate().0;
    let channels = supported_config.channels();
    {
        let mut state = state.lock();
        state.sample_rate = sample_rate;
        state.channels = channels;
        state.device_name = name;
        state.last_error = None;
    }

    let config: cpal::StreamConfig = supported_config.clone().into();
    let err_state = state.clone();
    let error_callback = move |err: cpal::StreamError| {
        let mut state = err_state.lock();
        state.is_active = false;
        state.last_error = Some(err.to_string());
    };

    let stream = match supported_config.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config, producer, error_callback),
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config, producer, error_callback),
        cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config, producer, error_callback),
        other => return Err(format!("Unsupported sample format: {other:?}")),
    }
    .map_err(|err| err.to_string())?;

    stream.play().map_err(|err| err.to_string())?;
    Ok(stream)
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut producer: ringbuf::HeapProd<f32>,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::Sample + cpal::SizedSample + Send + 'static,
    f32: FromSample<T>,
{
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            for sample in data {
                if producer.try_push(f32::from_sample(*sample)).is_err() {
                    break;
                }
            }
        },
        error_callback,
        None,
    )
}

fn select_output_device(host: &cpal::Host, selected_device: Option<&str>) -> Option<cpal::Device> {
    if let Some(name) = selected_device {
        if let Ok(mut devices) = host.output_devices() {
            if let Some(device) = devices.find(|device| device.name().ok().as_deref() == Some(name))
            {
                return Some(device);
            }
        }
    }
    host.default_output_device()
}

fn audio_host() -> cpal::Host {
    #[cfg(windows)]
    {
        cpal::host_from_id(cpal::HostId::Wasapi).unwrap_or_else(|_| cpal::default_host())
    }

    #[cfg(not(windows))]
    {
        cpal::default_host()
    }
}

pub trait FromSample<T> {
    fn from_sample(sample: T) -> Self;
}

impl FromSample<f32> for f32 {
    fn from_sample(sample: f32) -> Self {
        sample
    }
}

impl FromSample<i16> for f32 {
    fn from_sample(sample: i16) -> Self {
        sample as f32 / i16::MAX as f32
    }
}

impl FromSample<u16> for f32 {
    fn from_sample(sample: u16) -> Self {
        (sample as f32 / u16::MAX as f32) * 2.0 - 1.0
    }
}
