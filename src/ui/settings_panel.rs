use crate::{
    audio::AudioState,
    config::{Config, RingStyle, VisualMode, THEME_NAMES},
};
use eframe::egui;
use std::ops::RangeInclusive;

pub fn render(
    ctx: &egui::Context,
    config: &mut Config,
    devices: &[String],
    audio: &AudioState,
    fps: f32,
    restart_audio: &mut bool,
) -> bool {
    let mut changed = false;

    egui::SidePanel::right("settings_panel")
        .exact_width(310.0)
        .frame(
            egui::Frame::side_top_panel(&ctx.style())
                .inner_margin(egui::Margin::symmetric(18.0, 16.0)),
        )
        .resizable(false)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.heading(egui::RichText::new("Settings").size(20.0));
                    ui.add_space(10.0);

                    section(ui, "Audio", |ui| {
                        let before = config.selected_device.clone();
                        egui::ComboBox::from_id_salt("device")
                            .width(ui.available_width())
                            .selected_text(
                                config
                                    .selected_device
                                    .as_deref()
                                    .unwrap_or("Default output"),
                            )
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut config.selected_device,
                                    None,
                                    "Default output",
                                );
                                for device in devices {
                                    ui.selectable_value(
                                        &mut config.selected_device,
                                        Some(device.clone()),
                                        device,
                                    );
                                }
                            });
                        if config.selected_device != before {
                            changed = true;
                            *restart_audio = true;
                        }
                        let sensitivity_text = format!("{:.2}", config.sensitivity);
                        changed |= value_slider(
                            ui,
                            "Sensitivity",
                            &mut config.sensitivity,
                            0.5..=3.0,
                            sensitivity_text,
                        );
                        let decay_text = format!("{:.3}", config.smoothing_decay);
                        changed |= value_slider(
                            ui,
                            "Decay",
                            &mut config.smoothing_decay,
                            0.05..=0.4,
                            decay_text,
                        );
                    });

                    section(ui, "Visual", |ui| {
                        ui.horizontal(|ui| {
                            for mode in VisualMode::ALL {
                                changed |= ui
                                    .selectable_value(&mut config.visual_mode, mode, mode.label())
                                    .changed();
                            }
                        });

                        if matches!(config.visual_mode, VisualMode::Bars | VisualMode::Circular) {
                            egui::ComboBox::from_id_salt("bar_count")
                                .width(ui.available_width())
                                .selected_text(format!("{} bands", config.bar_count))
                                .show_ui(ui, |ui| {
                                    for count in [32, 64, 96, 128] {
                                        changed |= ui
                                            .selectable_value(
                                                &mut config.bar_count,
                                                count,
                                                format!("{count} bands"),
                                            )
                                            .changed();
                                    }
                                });
                            let gap_text = format!("{:.2}", config.bar_gap);
                            changed |=
                                value_slider(ui, "Gap", &mut config.bar_gap, 0.0..=0.5, gap_text);
                            let radius_text = format!("{:.0}", config.bar_rounding);
                            changed |= value_slider(
                                ui,
                                "Radius",
                                &mut config.bar_rounding,
                                0.0..=18.0,
                                radius_text,
                            );
                        }

                        if matches!(config.visual_mode, VisualMode::Bars) {
                            changed |= ui
                                .checkbox(&mut config.block_style, "Block style")
                                .changed();
                            changed |= ui.checkbox(&mut config.show_mirror, "Mirror").changed();
                        }

                        if matches!(config.visual_mode, VisualMode::Circular) {
                            egui::ComboBox::from_id_salt("ring_style")
                                .width(ui.available_width())
                                .selected_text(config.ring_style.label())
                                .show_ui(ui, |ui| {
                                    for style in RingStyle::ALL {
                                        changed |= ui
                                            .selectable_value(
                                                &mut config.ring_style,
                                                style,
                                                style.label(),
                                            )
                                            .changed();
                                    }
                                });
                            if matches!(config.ring_style, RingStyle::Bars) {
                                changed |= ui
                                    .checkbox(&mut config.block_style, "Block style")
                                    .changed();
                            }
                        }

                        changed |= ui
                            .checkbox(&mut config.show_peak_hold, "Peak hold")
                            .changed();
                    });

                    section(ui, "Colors", |ui| {
                        egui::ComboBox::from_id_salt("theme")
                            .width(ui.available_width())
                            .selected_text(config.palette().name)
                            .show_ui(ui, |ui| {
                                for (index, name) in THEME_NAMES.iter().enumerate() {
                                    changed |= ui
                                        .selectable_value(&mut config.color_theme, index, *name)
                                        .changed();
                                }
                            });
                    });

                    section(ui, "Window", |ui| {
                        changed |= ui
                            .checkbox(&mut config.window_always_on_top, "Pin and lock position")
                            .changed();
                        let opacity_text = format!("{:.0}%", config.opacity * 100.0);
                        changed |= value_slider(
                            ui,
                            "Opacity",
                            &mut config.opacity,
                            0.3..=1.0,
                            opacity_text,
                        );
                    });

                    section(ui, "Info", |ui| {
                        info_row(ui, "FPS", format!("{fps:.0}"));
                        info_row(ui, "Bands", audio_band_count(config).to_string());
                        info_row(ui, "Sample rate", format!("{} Hz", audio.sample_rate));
                        info_row(ui, "Device", audio.device_name.as_str());
                        info_row(
                            ui,
                            "Audio",
                            if audio.is_active {
                                "active"
                            } else {
                                "inactive"
                            },
                        );
                        if let Some(error) = &audio.last_error {
                            ui.add_space(4.0);
                            ui.colored_label(egui::Color32::from_rgb(255, 126, 126), error);
                        }
                    });
                });
        });

    changed
}

fn section(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(title)
            .size(12.0)
            .color(egui::Color32::from_gray(170))
            .strong(),
    );
    ui.add_space(2.0);

    egui::Frame::default()
        .inner_margin(egui::Margin::symmetric(0.0, 2.0))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            add_contents(ui);
        });

    ui.add_space(6.0);
    ui.separator();
}

fn value_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    value_text: String,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.monospace(value_text);
        });
    });
    changed |= ui
        .add_sized(
            [ui.available_width(), 18.0],
            egui::Slider::new(value, range).show_value(false),
        )
        .changed();
    changed
}

fn info_row(ui: &mut egui::Ui, label: &str, value: impl ToString) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(egui::Color32::from_gray(145)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.monospace(value.to_string());
        });
    });
}

fn audio_band_count(config: &Config) -> usize {
    config.bar_count
}
