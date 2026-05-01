use crate::config::Config;
use rustfft::{num_complex::Complex32, Fft, FftPlanner};
use std::{collections::VecDeque, sync::Arc};

const MIN_DB: f32 = -80.0;
const MAX_DB: f32 = 0.0;

#[derive(Clone, Debug)]
pub struct FftOutput {
    pub bands: Vec<f32>,
    pub peaks: Vec<f32>,
    pub waveform: Vec<f32>,
    pub peak_db: f32,
    pub band_count: usize,
}

impl FftOutput {
    pub fn empty(band_count: usize, waveform_size: usize) -> Self {
        Self {
            bands: vec![0.0; band_count],
            peaks: vec![0.0; band_count],
            waveform: vec![0.0; waveform_size],
            peak_db: MIN_DB,
            band_count,
        }
    }
}

pub struct FftProcessor {
    window_size: usize,
    band_count: usize,
    fft: Arc<dyn Fft<f32>>,
    history: VecDeque<f32>,
    smoothed: Vec<f32>,
    peaks: Vec<f32>,
    hann: Vec<f32>,
    buffer: Vec<Complex32>,
    magnitudes: Vec<f32>,
    bands: Vec<f32>,
}

impl FftProcessor {
    pub fn new(window_size: usize, band_count: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(window_size);
        let hann = (0..window_size)
            .map(|i| {
                let phase = std::f32::consts::TAU * i as f32 / (window_size - 1) as f32;
                0.5 * (1.0 - phase.cos())
            })
            .collect();

        Self {
            window_size,
            band_count,
            fft,
            history: VecDeque::with_capacity(window_size * 2),
            smoothed: vec![0.0; band_count],
            peaks: vec![0.0; band_count],
            hann,
            buffer: vec![Complex32::new(0.0, 0.0); window_size],
            magnitudes: Vec::with_capacity(window_size / 2),
            bands: vec![0.0; band_count],
        }
    }

    pub fn set_band_count(&mut self, band_count: usize) {
        if self.band_count == band_count {
            return;
        }
        self.band_count = band_count;
        self.smoothed = vec![0.0; band_count];
        self.peaks = vec![0.0; band_count];
        self.bands = vec![0.0; band_count];
    }

    pub fn process(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        config: &Config,
    ) -> FftOutput {
        self.set_band_count(config.bar_count);
        let channels = channels.max(1) as usize;
        if samples.is_empty() {
            let silence_frames = (sample_rate as usize / 60).clamp(1, self.window_size);
            for _ in 0..silence_frames {
                self.push_history_sample(0.0);
            }
        } else {
            for frame in samples.chunks(channels) {
                let sample = frame.iter().copied().sum::<f32>() / frame.len() as f32;
                self.push_history_sample(sample);
            }
        }

        let mut waveform = self.history.iter().copied().collect::<Vec<_>>();
        if waveform.len() < self.window_size {
            waveform.resize(self.window_size, 0.0);
        }

        self.buffer.fill(Complex32::new(0.0, 0.0));
        let offset = self.window_size.saturating_sub(self.history.len());
        for (i, sample) in self.history.iter().copied().enumerate() {
            self.buffer[offset + i] = Complex32::new(sample * self.hann[offset + i], 0.0);
        }

        self.fft.process(&mut self.buffer);

        self.magnitudes.clear();
        self.magnitudes.extend(
            self.buffer
                .iter()
                .take(self.window_size / 2)
                .map(|c| (c.re * c.re + c.im * c.im).sqrt() / self.window_size as f32),
        );

        group_bands_into(
            self.window_size,
            self.band_count,
            &self.magnitudes,
            sample_rate,
            &mut self.bands,
        );
        let mut peak_db = MIN_DB;
        for (i, value) in self.bands.iter().copied().enumerate() {
            let db = linear_to_db(value * config.sensitivity);
            peak_db = peak_db.max(db);
            let normalized = ((db - MIN_DB) / (MAX_DB - MIN_DB)).clamp(0.0, 1.0);
            let current = self.smoothed[i];
            let alpha = if normalized > current {
                0.8
            } else {
                config.smoothing_decay.clamp(0.05, 0.4)
            };
            self.smoothed[i] = current * (1.0 - alpha) + normalized * alpha;
            self.peaks[i] = self.peaks[i].max(self.smoothed[i]);
            self.peaks[i] = (self.peaks[i] - 0.008).max(self.smoothed[i]);
        }

        FftOutput {
            bands: self.smoothed.clone(),
            peaks: self.peaks.clone(),
            waveform,
            peak_db,
            band_count: self.band_count,
        }
    }

    fn push_history_sample(&mut self, sample: f32) {
        if self.history.len() >= self.window_size {
            self.history.pop_front();
        }
        self.history.push_back(sample);
    }
}

fn group_bands_into(
    window_size: usize,
    band_count: usize,
    magnitudes: &[f32],
    sample_rate: u32,
    bands: &mut [f32],
) {
    let min_hz = 20.0_f32;
    let max_hz = (20_000.0_f32).min(sample_rate as f32 * 0.48);
    let min_log = min_hz.log10();
    let max_log = max_hz.log10();
    let bin_hz = sample_rate as f32 / window_size as f32;

    for (band, value) in bands.iter_mut().enumerate().take(band_count) {
        let t0 = band as f32 / band_count as f32;
        let t1 = (band + 1) as f32 / band_count as f32;
        let low = 10.0_f32.powf(min_log + (max_log - min_log) * t0);
        let high = 10.0_f32.powf(min_log + (max_log - min_log) * t1);
        let start = (low / bin_hz).floor().max(1.0) as usize;
        let end = (high / bin_hz).ceil().min(magnitudes.len() as f32 - 1.0) as usize;
        let mut total = 0.0;
        let mut count = 0;
        for magnitude in magnitudes.iter().take(end.max(start + 1)).skip(start) {
            total += *magnitude;
            count += 1;
        }
        *value = if count == 0 {
            0.0
        } else {
            total / count as f32
        };
    }
}

fn linear_to_db(value: f32) -> f32 {
    20.0 * (value.abs() + 1.0e-9).log10()
}
