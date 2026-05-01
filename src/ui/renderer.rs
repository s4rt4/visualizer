use super::{mode_bars, mode_circular, mode_waveform};
use crate::{
    config::{Config, VisualMode},
    fft::FftOutput,
};
use eframe::egui;

pub fn render(
    mode: VisualMode,
    ui: &mut egui::Ui,
    fft: &FftOutput,
    config: &Config,
    t: f32,
    _dt: f32,
) {
    match mode {
        VisualMode::Bars | VisualMode::LegacyParticles => mode_bars::render(ui, fft, config),
        VisualMode::Waveform => mode_waveform::render(ui, fft, config),
        VisualMode::Circular => mode_circular::render(ui, fft, config, t),
    }
}
