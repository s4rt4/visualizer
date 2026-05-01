mod mode_bars;
mod mode_circular;
mod mode_waveform;
mod renderer;
mod settings_panel;

use crate::{
    audio::{AudioEngine, SharedAudioState},
    config::{Config, DebouncedConfigSave, VisualMode},
    fft::{FftOutput, FftProcessor},
};
use eframe::egui;
use std::time::{Duration, Instant};

pub struct VisualizerApp {
    audio_engine: AudioEngine,
    audio_state: SharedAudioState,
    fft_processor: FftProcessor,
    fft_output: FftOutput,
    config: Config,
    save: DebouncedConfigSave,
    devices: Vec<String>,
    start_time: Instant,
    last_frame_time: Instant,
    applied_visual_fill: Option<egui::Color32>,
    applied_window_top: Option<bool>,
    locked_window_position: Option<egui::Pos2>,
    fps: f32,
}

impl VisualizerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgba_premultiplied(8, 10, 14, 235);
        visuals.window_fill = egui::Color32::from_rgba_premultiplied(8, 10, 14, 245);
        cc.egui_ctx.set_visuals(visuals);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let config = Config::load();
        let devices = AudioEngine::devices();
        let audio_engine = AudioEngine::start(config.selected_device.as_deref());
        let audio_state = audio_engine.state();
        let band_count = config.bar_count;

        Self {
            audio_engine,
            audio_state,
            fft_processor: FftProcessor::new(2048, band_count),
            fft_output: FftOutput::empty(band_count, 2048),
            config,
            save: DebouncedConfigSave::new(),
            devices,
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
            applied_visual_fill: None,
            applied_window_top: None,
            locked_window_position: None,
            fps: 0.0,
        }
    }

    fn top_bar(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = false;
        let palette = self.config.palette();
        egui::TopBottomPanel::top("top_bar")
            .exact_height(48.0)
            .frame(
                egui::Frame::default()
                    .fill(color_with_opacity(palette.background, self.config.opacity))
                    .inner_margin(egui::Margin::symmetric(16.0, 0.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if icon_button(
                        ui,
                        egui::include_image!("../../assets/icons/bars.svg"),
                        self.config.visual_mode == VisualMode::Bars,
                        "Bars",
                    )
                    .clicked()
                    {
                        self.config.visual_mode = VisualMode::Bars;
                        changed = true;
                    }
                    if icon_button(
                        ui,
                        egui::include_image!("../../assets/icons/wave.svg"),
                        self.config.visual_mode == VisualMode::Waveform,
                        "Wave",
                    )
                    .clicked()
                    {
                        self.config.visual_mode = VisualMode::Waveform;
                        changed = true;
                    }
                    if icon_button(
                        ui,
                        egui::include_image!("../../assets/icons/ring.svg"),
                        self.config.visual_mode == VisualMode::Circular,
                        "Ring",
                    )
                    .clicked()
                    {
                        self.config.visual_mode = VisualMode::Circular;
                        changed = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(
                            ui,
                            egui::include_image!("../../assets/icons/setting.svg"),
                            self.config.settings_panel_open,
                            "Settings",
                        )
                        .clicked()
                        {
                            self.config.settings_panel_open = !self.config.settings_panel_open;
                            changed = true;
                        }
                        if icon_button(
                            ui,
                            egui::include_image!("../../assets/icons/pin.svg"),
                            self.config.window_always_on_top,
                            "Pin",
                        )
                        .clicked()
                        {
                            self.config.window_always_on_top = !self.config.window_always_on_top;
                            changed = true;
                        }
                    });
                });
            });
        changed
    }
}

impl eframe::App for VisualizerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().max(1.0 / 240.0);
        self.last_frame_time = now;
        self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;

        let samples = self.audio_engine.drain_samples(8192);
        let (sample_rate, channels) = {
            let state = self.audio_state.lock();
            (state.sample_rate, state.channels)
        };
        self.fft_output = self
            .fft_processor
            .process(&samples, sample_rate, channels, &self.config);

        let palette = self.config.palette();
        self.apply_window_commands(ctx);
        self.apply_position_lock(ctx);
        let mut changed = self.top_bar(ctx);
        self.apply_visual_opacity(ctx, palette.background);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(color_with_opacity(palette.background, self.config.opacity)),
            )
            .show(ctx, |ui| {
                renderer::render(
                    self.config.visual_mode,
                    ui,
                    &self.fft_output,
                    &self.config,
                    self.start_time.elapsed().as_secs_f32(),
                    dt,
                );
            });

        if self.config.settings_panel_open {
            let mut restart_audio = false;
            changed |= settings_panel::render(
                ctx,
                &mut self.config,
                &self.devices,
                &self.audio_state.lock(),
                self.fps,
                &mut restart_audio,
            );
            if restart_audio {
                self.audio_engine
                    .restart(self.config.selected_device.as_deref());
            }
        }

        if changed {
            self.save.mark_dirty();
        }
        self.save.tick(&self.config);
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save.flush(&self.config);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        color_with_opacity(self.config.palette().background, self.config.opacity)
            .to_normalized_gamma_f32()
    }
}

impl VisualizerApp {
    fn apply_window_commands(&mut self, ctx: &egui::Context) {
        if self.applied_window_top == Some(self.config.window_always_on_top) {
            return;
        }

        let level = if self.config.window_always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
        self.applied_window_top = Some(self.config.window_always_on_top);
    }

    fn apply_position_lock(&mut self, ctx: &egui::Context) {
        if !self.config.window_always_on_top {
            self.locked_window_position = None;
            return;
        }

        let current_position = ctx.input(|input| input.viewport().outer_rect.map(|rect| rect.min));
        let Some(current_position) = current_position else {
            return;
        };

        let locked_position = *self.locked_window_position.get_or_insert(current_position);
        if locked_position.distance(current_position) > 0.5 {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(locked_position));
        }
    }

    fn apply_visual_opacity(&mut self, ctx: &egui::Context, background: egui::Color32) {
        let fill = color_with_opacity(background, self.config.opacity);
        if self.applied_visual_fill == Some(fill) {
            return;
        }

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = fill;
        visuals.window_fill = fill;
        visuals.extreme_bg_color = fill;
        ctx.set_visuals(visuals);
        self.applied_visual_fill = Some(fill);
    }
}

fn color_with_opacity(color: egui::Color32, opacity: f32) -> egui::Color32 {
    let alpha = (opacity.clamp(0.05, 1.0) * 255.0).round() as u8;
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn icon_button(
    ui: &mut egui::Ui,
    source: egui::ImageSource<'static>,
    selected: bool,
    tooltip: &'static str,
) -> egui::Response {
    let tint = if selected {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_white_alpha(170)
    };
    let image = egui::Image::new(source)
        .fit_to_exact_size(egui::vec2(18.0, 18.0))
        .tint(tint);

    ui.add(
        egui::Button::image(image)
            .selected(selected)
            .min_size(egui::vec2(30.0, 30.0)),
    )
    .on_hover_text(tooltip)
}
