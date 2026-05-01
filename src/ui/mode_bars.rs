use crate::{config::Config, fft::FftOutput};
use eframe::egui;

pub fn render(ui: &mut egui::Ui, fft: &FftOutput, config: &Config) {
    if config.block_style {
        render_blocks(ui, fft, config);
        return;
    }

    let palette = config.palette();
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
    let rect = response.rect.shrink(18.0);
    let center_y = if config.show_mirror {
        rect.center().y
    } else {
        rect.bottom()
    };
    let count = fft.band_count.min(fft.bands.len()).max(1);
    let slot = rect.width() / count as f32;
    let bar_width = (slot * (1.0 - config.bar_gap.clamp(0.0, 0.8))).max(2.0);
    let max_height = if config.show_mirror {
        rect.height() * 0.46
    } else {
        rect.height() * 0.9
    };

    for i in 0..count {
        let value = fft.bands[i].clamp(0.0, 1.0);
        let height = (value * max_height * config.sensitivity).min(max_height);
        let x = rect.left() + i as f32 * slot + (slot - bar_width) * 0.5;
        let color = lerp_color(palette.secondary, palette.primary, value);
        let rounding = egui::Rounding::same(config.bar_rounding);

        let top_rect = egui::Rect::from_min_max(
            egui::pos2(x, center_y - height),
            egui::pos2(x + bar_width, center_y),
        );
        painter.rect_filled(top_rect, rounding, color);

        if config.show_mirror {
            let reflection_height = height * 0.52;
            let bottom_rect = egui::Rect::from_min_max(
                egui::pos2(x, center_y),
                egui::pos2(x + bar_width, center_y + reflection_height),
            );
            painter.rect_filled(bottom_rect, rounding, with_alpha(color, 21));
        }

        if config.show_peak_hold && i < fft.peaks.len() {
            let peak_y = center_y - fft.peaks[i] * max_height;
            let peak_rect =
                egui::Rect::from_min_size(egui::pos2(x, peak_y - 2.0), egui::vec2(bar_width, 2.0));
            painter.rect_filled(peak_rect, 0.0, with_alpha(color, 78));
        }
    }
}

fn render_blocks(ui: &mut egui::Ui, fft: &FftOutput, config: &Config) {
    let palette = config.palette();
    let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
    let rect = response.rect.shrink(18.0);
    let center_y = if config.show_mirror {
        rect.center().y
    } else {
        rect.bottom()
    };
    let count = fft.band_count.min(fft.bands.len()).max(1);
    let slot = rect.width() / count as f32;
    let bar_width = (slot * (1.0 - config.bar_gap.clamp(0.0, 0.8))).max(3.0);
    let block_gap = 2.0;
    let block_height = (bar_width * 0.58).clamp(4.0, 11.0);
    let step = block_height + block_gap;
    let max_height = if config.show_mirror {
        rect.height() * 0.46
    } else {
        rect.height() * 0.9
    };
    let max_blocks = (max_height / step).floor().max(1.0) as usize;
    let rounding = egui::Rounding::same(config.bar_rounding.min(block_height * 0.5));

    for i in 0..count {
        let value = fft.bands[i].clamp(0.0, 1.0);
        let active_blocks = (value * max_blocks as f32 * config.sensitivity)
            .ceil()
            .clamp(0.0, max_blocks as f32) as usize;
        let x = rect.left() + i as f32 * slot + (slot - bar_width) * 0.5;
        let color = lerp_color(palette.secondary, palette.primary, i as f32 / count as f32);

        for block in 0..active_blocks {
            let y_bottom = center_y - block as f32 * step;
            let block_rect = egui::Rect::from_min_max(
                egui::pos2(x, y_bottom - block_height),
                egui::pos2(x + bar_width, y_bottom),
            );
            painter.rect_filled(block_rect, rounding, color);
        }

        if config.show_mirror {
            let reflection_blocks = (active_blocks as f32 * 0.42).ceil() as usize;
            for block in 0..reflection_blocks {
                let y_top = center_y + block as f32 * step;
                let block_rect = egui::Rect::from_min_max(
                    egui::pos2(x, y_top),
                    egui::pos2(x + bar_width, y_top + block_height),
                );
                let fade = 1.0 - block as f32 / reflection_blocks.max(1) as f32;
                painter.rect_filled(block_rect, rounding, with_alpha(color, (24.0 * fade) as u8));
            }
        }

        if config.show_peak_hold && i < fft.peaks.len() {
            let peak_blocks = (fft.peaks[i] * max_blocks as f32)
                .ceil()
                .clamp(0.0, max_blocks as f32) as usize;
            let y = center_y - peak_blocks as f32 * step - block_gap;
            let peak_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(bar_width, 2.0));
            painter.rect_filled(peak_rect, 0.0, with_alpha(color, 90));
        }
    }
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

fn with_alpha(color: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}
