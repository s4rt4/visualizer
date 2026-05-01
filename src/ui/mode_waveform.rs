use crate::{config::Config, fft::FftOutput};
use eframe::egui;

pub fn render(ui: &mut egui::Ui, fft: &FftOutput, config: &Config) {
    let palette = config.palette();
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
    let rect = response.rect.shrink2(egui::vec2(30.0, 52.0));
    let center_y = rect.center().y;

    painter.line_segment(
        [
            egui::pos2(rect.left(), center_y),
            egui::pos2(rect.right(), center_y),
        ],
        egui::Stroke::new(
            1.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 42),
        ),
    );

    let points = smooth_oscilloscope_points(&fft.waveform, rect, config.sensitivity);
    if points.len() < 2 {
        return;
    }

    painter.add(egui::Shape::line(
        points.clone(),
        egui::Stroke::new(
            8.0,
            egui::Color32::from_rgba_unmultiplied(
                palette.secondary.r(),
                palette.secondary.g(),
                palette.secondary.b(),
                28,
            ),
        ),
    ));
    painter.add(egui::Shape::line(
        points.clone(),
        egui::Stroke::new(
            4.5,
            egui::Color32::from_rgba_unmultiplied(55, 145, 255, 150),
        ),
    ));
    painter.add(egui::Shape::line(
        points.clone(),
        egui::Stroke::new(2.4, palette.primary),
    ));
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(
            1.0,
            egui::Color32::from_rgba_unmultiplied(255, 120, 255, 230),
        ),
    ));
}

fn smooth_oscilloscope_points(
    samples: &[f32],
    rect: egui::Rect,
    sensitivity: f32,
) -> Vec<egui::Pos2> {
    let width = rect.width().round().max(240.0) as usize;
    let center_y = rect.center().y;
    let amplitude = rect.height() * 0.46 * sensitivity.clamp(0.5, 3.0);
    let source = if samples.len() > 32 { samples } else { &[] };

    if source.is_empty() {
        return Vec::new();
    }

    let mut values = Vec::with_capacity(width);
    for x in 0..width {
        let t = x as f32 / (width - 1) as f32;
        let sample = cubic_sample(source, t * (source.len() - 1) as f32);
        let envelope = raised_cosine_envelope(t);
        values.push((sample * envelope).clamp(-1.0, 1.0));
    }

    smooth_values(&mut values, 3);

    values
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            let x = rect.left() + rect.width() * i as f32 / (width - 1) as f32;
            egui::pos2(x, center_y - value * amplitude)
        })
        .collect()
}

fn cubic_sample(samples: &[f32], index: f32) -> f32 {
    let len = samples.len() as isize;
    let i = index.floor() as isize;
    let frac = index - i as f32;
    let p0 = samples[(i - 1).clamp(0, len - 1) as usize];
    let p1 = samples[i.clamp(0, len - 1) as usize];
    let p2 = samples[(i + 1).clamp(0, len - 1) as usize];
    let p3 = samples[(i + 2).clamp(0, len - 1) as usize];
    let a = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
    let b = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c = -0.5 * p0 + 0.5 * p2;
    let d = p1;
    ((a * frac + b) * frac + c) * frac + d
}

fn raised_cosine_envelope(t: f32) -> f32 {
    let edge = 0.5 - 0.5 * (std::f32::consts::TAU * t).cos();
    0.18 + 0.82 * edge.powf(0.55)
}

fn smooth_values(values: &mut [f32], passes: usize) {
    if values.len() < 3 {
        return;
    }

    let mut scratch = values.to_vec();
    for _ in 0..passes {
        scratch[0] = values[0];
        scratch[values.len() - 1] = values[values.len() - 1];
        for i in 1..values.len() - 1 {
            scratch[i] = values[i - 1] * 0.24 + values[i] * 0.52 + values[i + 1] * 0.24;
        }
        values.copy_from_slice(&scratch);
    }
}
