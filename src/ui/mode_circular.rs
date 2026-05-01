use crate::{
    config::{Config, RingStyle},
    fft::FftOutput,
};
use eframe::egui;

pub fn render(ui: &mut egui::Ui, fft: &FftOutput, config: &Config, t: f32) {
    match config.ring_style {
        RingStyle::Wave => render_wave_ring(ui, fft, config, t),
        RingStyle::Bars => render_radial_bars(ui, fft, config, t),
    }
}

fn render_wave_ring(ui: &mut egui::Ui, fft: &FftOutput, config: &Config, t: f32) {
    let palette = config.palette();
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
    let rect = response.rect.shrink(34.0);
    let center = rect.center();
    let size = rect.width().min(rect.height());
    let base_radius = size * 0.25;
    let wave_amp = size * 0.16 * config.sensitivity.clamp(0.5, 2.4);
    let points = circular_wave_points(&fft.waveform, center, base_radius, wave_amp, t);

    painter.circle_stroke(
        center,
        base_radius,
        egui::Stroke::new(1.0, with_alpha(egui::Color32::WHITE, 34)),
    );

    if points.len() > 2 {
        painter.add(egui::Shape::closed_line(
            points.clone(),
            egui::Stroke::new(11.0, with_alpha(palette.secondary, 24)),
        ));
        painter.add(egui::Shape::closed_line(
            points.clone(),
            egui::Stroke::new(5.0, with_alpha(egui::Color32::from_rgb(55, 145, 255), 145)),
        ));
        painter.add(egui::Shape::closed_line(
            points.clone(),
            egui::Stroke::new(2.6, palette.primary),
        ));
        painter.add(egui::Shape::closed_line(
            points,
            egui::Stroke::new(1.0, with_alpha(egui::Color32::from_rgb(255, 115, 255), 230)),
        ));
    }

    let pulse = ((fft.peak_db + 80.0) / 80.0).clamp(0.0, 1.0);
    painter.circle_filled(center, 12.0 + pulse * 14.0, with_alpha(palette.primary, 42));
    painter.circle_filled(center, 5.0 + pulse * 5.0, palette.primary);
}

fn render_radial_bars(ui: &mut egui::Ui, fft: &FftOutput, config: &Config, t: f32) {
    if config.block_style {
        render_radial_blocks(ui, fft, config, t);
        return;
    }

    let palette = config.palette();
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
    let rect = response.rect.shrink(34.0);
    let center = rect.center();
    let size = rect.width().min(rect.height());
    let radius = size * 0.25;
    let spike = size * 0.22 * config.sensitivity.clamp(0.5, 2.6);
    let count = config.bar_count.min(fft.bands.len()).max(1);
    let rotation = t * 0.16;

    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(1.0, with_alpha(egui::Color32::WHITE, 32)),
    );

    for i in 0..count {
        let value = fft.bands[i].clamp(0.0, 1.0);
        let angle = std::f32::consts::TAU * i as f32 / count as f32 + rotation;
        let dir = egui::vec2(angle.cos(), angle.sin());
        let perp = egui::vec2(-angle.sin(), angle.cos());
        let inner = center + dir * radius;
        let height = value * spike;
        let outer = center + dir * (radius + height);
        let color = lerp_color(palette.secondary, palette.primary, i as f32 / count as f32);
        let gap = config.bar_gap.clamp(0.0, 0.82);
        let mid_radius = radius + spike * 0.45;
        let bar_width = (std::f32::consts::TAU * mid_radius / count as f32 * (1.0 - gap))
            .clamp(2.0, size * 0.045);
        let half_width = bar_width * 0.5;

        let bar = vec![
            inner - perp * half_width,
            inner + perp * half_width,
            outer + perp * half_width,
            outer - perp * half_width,
        ];
        painter.add(egui::Shape::convex_polygon(bar, color, egui::Stroke::NONE));

        let round = (config.bar_rounding / 18.0).clamp(0.0, 1.0);
        if round > 0.0 && height > half_width {
            let cap_radius = half_width * round;
            painter.circle_filled(inner, cap_radius, color);
            painter.circle_filled(outer, cap_radius, color);
        }

        if config.show_peak_hold && i < fft.peaks.len() {
            let peak = center + dir * (radius + fft.peaks[i].clamp(0.0, 1.0) * spike + 4.0);
            painter.circle_filled(peak, half_width.min(3.0), with_alpha(color, 120));
        }
    }

    let pulse = ((fft.peak_db + 80.0) / 80.0).clamp(0.0, 1.0);
    painter.circle_filled(center, 18.0 + pulse * 22.0, with_alpha(palette.primary, 34));
    painter.circle_filled(center, 7.0 + pulse * 7.0, palette.primary);
}

fn render_radial_blocks(ui: &mut egui::Ui, fft: &FftOutput, config: &Config, t: f32) {
    let palette = config.palette();
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
    let rect = response.rect.shrink(34.0);
    let center = rect.center();
    let size = rect.width().min(rect.height());
    let radius = size * 0.22;
    let spike = size * 0.25 * config.sensitivity.clamp(0.5, 2.6);
    let count = config.bar_count.min(fft.bands.len()).max(1);
    let rotation = t * 0.16;
    let gap = config.bar_gap.clamp(0.0, 0.82);
    let mid_radius = radius + spike * 0.45;
    let bar_width =
        (std::f32::consts::TAU * mid_radius / count as f32 * (1.0 - gap)).clamp(2.5, size * 0.045);
    let half_width = bar_width * 0.5;
    let block_depth = (bar_width * 0.85).clamp(5.0, 13.0);
    let block_gap = (block_depth * 0.28).clamp(2.0, 4.0);
    let step = block_depth + block_gap;
    let max_blocks = (spike / step).floor().max(1.0) as usize;
    let round = (config.bar_rounding / 18.0).clamp(0.0, 1.0);

    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(1.0, with_alpha(egui::Color32::WHITE, 28)),
    );

    for i in 0..count {
        let value = fft.bands[i].clamp(0.0, 1.0);
        let active_blocks = (value * max_blocks as f32)
            .ceil()
            .clamp(0.0, max_blocks as f32) as usize;
        let angle = std::f32::consts::TAU * i as f32 / count as f32 + rotation;
        let dir = egui::vec2(angle.cos(), angle.sin());
        let perp = egui::vec2(-angle.sin(), angle.cos());
        let color = lerp_color(palette.secondary, palette.primary, i as f32 / count as f32);

        for block in 0..active_blocks {
            let r0 = radius + block as f32 * step;
            let r1 = r0 + block_depth;
            let inner = center + dir * r0;
            let outer = center + dir * r1;
            let segment = vec![
                inner - perp * half_width,
                inner + perp * half_width,
                outer + perp * half_width,
                outer - perp * half_width,
            ];
            painter.add(egui::Shape::convex_polygon(
                segment,
                color,
                egui::Stroke::NONE,
            ));

            if round > 0.0 {
                let cap_radius = half_width.min(block_depth * 0.5) * round;
                painter.circle_filled(inner, cap_radius, color);
                painter.circle_filled(outer, cap_radius, color);
            }
        }

        if config.show_peak_hold && i < fft.peaks.len() {
            let peak_blocks = (fft.peaks[i].clamp(0.0, 1.0) * max_blocks as f32)
                .ceil()
                .clamp(0.0, max_blocks as f32) as usize;
            let peak = center + dir * (radius + peak_blocks as f32 * step + 3.0);
            painter.circle_filled(peak, half_width.min(3.0), with_alpha(color, 120));
        }
    }

    let pulse = ((fft.peak_db + 80.0) / 80.0).clamp(0.0, 1.0);
    painter.circle_filled(center, 16.0 + pulse * 20.0, with_alpha(palette.primary, 34));
    painter.circle_filled(center, 6.0 + pulse * 7.0, palette.primary);
}

fn circular_wave_points(
    samples: &[f32],
    center: egui::Pos2,
    base_radius: f32,
    wave_amp: f32,
    t: f32,
) -> Vec<egui::Pos2> {
    if samples.len() < 32 {
        return Vec::new();
    }

    let count = 360;
    let rotation = t * 0.18;
    let mut values = Vec::with_capacity(count);
    for i in 0..count {
        let phase = i as f32 / count as f32;
        let sample = cubic_sample(samples, phase * (samples.len() - 1) as f32);
        values.push(sample.clamp(-1.0, 1.0));
    }
    smooth_cyclic(&mut values, 4);

    values
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            let phase = i as f32 / count as f32;
            let angle = std::f32::consts::TAU * phase + rotation;
            let radius = (base_radius + value * wave_amp).max(base_radius * 0.34);
            center + egui::vec2(angle.cos(), angle.sin()) * radius
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

fn smooth_cyclic(values: &mut [f32], passes: usize) {
    if values.len() < 3 {
        return;
    }

    let len = values.len();
    let mut scratch = values.to_vec();
    for _ in 0..passes {
        for i in 0..len {
            let prev = values[(i + len - 1) % len];
            let current = values[i];
            let next = values[(i + 1) % len];
            scratch[i] = prev * 0.24 + current * 0.52 + next * 0.24;
        }
        values.copy_from_slice(&scratch);
    }
}

fn with_alpha(color: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| x as f32 + (y as f32 - x as f32) * t;
    egui::Color32::from_rgb(
        lerp(a.r(), b.r()) as u8,
        lerp(a.g(), b.g()) as u8,
        lerp(a.b(), b.b()) as u8,
    )
}
