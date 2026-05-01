use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum VisualMode {
    Bars,
    Waveform,
    Circular,
    #[serde(alias = "Particles")]
    LegacyParticles,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum RingStyle {
    #[default]
    Wave,
    Bars,
}

impl RingStyle {
    pub const ALL: [RingStyle; 2] = [RingStyle::Wave, RingStyle::Bars];

    pub fn label(self) -> &'static str {
        match self {
            RingStyle::Wave => "Wave",
            RingStyle::Bars => "Bars",
        }
    }
}

impl VisualMode {
    pub const ALL: [VisualMode; 3] = [VisualMode::Bars, VisualMode::Waveform, VisualMode::Circular];

    pub fn label(self) -> &'static str {
        match self {
            VisualMode::Bars => "Bars",
            VisualMode::Waveform => "Wave",
            VisualMode::Circular => "Ring",
            VisualMode::LegacyParticles => "Bars",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub visual_mode: VisualMode,
    pub color_theme: usize,
    pub custom_color_primary: [f32; 3],
    pub bar_count: usize,
    pub bar_gap: f32,
    pub bar_rounding: f32,
    #[serde(default = "default_block_style")]
    pub block_style: bool,
    #[serde(default)]
    pub ring_style: RingStyle,
    pub show_mirror: bool,
    pub show_peak_hold: bool,
    pub sensitivity: f32,
    pub smoothing_decay: f32,
    pub selected_device: Option<String>,
    pub window_always_on_top: bool,
    pub opacity: f32,
    pub settings_panel_open: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            visual_mode: VisualMode::Bars,
            color_theme: 0,
            custom_color_primary: [0.0, 0.85, 1.0],
            bar_count: 64,
            bar_gap: 0.2,
            bar_rounding: 0.0,
            block_style: true,
            ring_style: RingStyle::Wave,
            show_mirror: true,
            show_peak_hold: true,
            sensitivity: 1.2,
            smoothing_decay: 0.15,
            selected_device: None,
            window_always_on_top: false,
            opacity: 0.96,
            settings_panel_open: true,
        }
    }
}

fn default_block_style() -> bool {
    true
}

#[derive(Clone)]
pub struct Palette {
    pub name: &'static str,
    pub primary: egui::Color32,
    pub secondary: egui::Color32,
    pub background: egui::Color32,
}

pub const THEME_NAMES: [&str; 10] = [
    "Gradient Neon",
    "Gradient Ember",
    "Gradient Aurora",
    "Gradient Candy",
    "Gradient Ocean",
    "Flat Violet",
    "Flat Cyan",
    "Flat Rose",
    "Flat Lime",
    "Flat Amber",
];

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        let mut config: Self = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        config.normalize();
        config
    }

    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, text);
        }
    }

    pub fn palette(&self) -> Palette {
        match self.color_theme.min(THEME_NAMES.len() - 1) {
            0 => Palette {
                name: "Gradient Neon",
                primary: egui::Color32::from_rgb(0, 230, 255),
                secondary: egui::Color32::from_rgb(180, 90, 255),
                background: egui::Color32::from_rgb(7, 10, 18),
            },
            1 => Palette {
                name: "Gradient Ember",
                primary: egui::Color32::from_rgb(255, 152, 72),
                secondary: egui::Color32::from_rgb(235, 47, 75),
                background: egui::Color32::from_rgb(18, 11, 9),
            },
            2 => Palette {
                name: "Gradient Aurora",
                primary: egui::Color32::from_rgb(108, 255, 172),
                secondary: egui::Color32::from_rgb(90, 112, 255),
                background: egui::Color32::from_rgb(6, 13, 18),
            },
            3 => Palette {
                name: "Gradient Candy",
                primary: egui::Color32::from_rgb(255, 104, 204),
                secondary: egui::Color32::from_rgb(140, 96, 255),
                background: egui::Color32::from_rgb(14, 8, 24),
            },
            4 => Palette {
                name: "Gradient Ocean",
                primary: egui::Color32::from_rgb(76, 214, 255),
                secondary: egui::Color32::from_rgb(47, 104, 255),
                background: egui::Color32::from_rgb(5, 11, 22),
            },
            5 => flat_palette(
                "Flat Violet",
                egui::Color32::from_rgb(190, 92, 255),
                egui::Color32::from_rgb(9, 7, 19),
            ),
            6 => flat_palette(
                "Flat Cyan",
                egui::Color32::from_rgb(71, 217, 255),
                egui::Color32::from_rgb(5, 12, 18),
            ),
            7 => flat_palette(
                "Flat Rose",
                egui::Color32::from_rgb(255, 96, 162),
                egui::Color32::from_rgb(17, 7, 14),
            ),
            8 => flat_palette(
                "Flat Lime",
                egui::Color32::from_rgb(125, 242, 111),
                egui::Color32::from_rgb(7, 15, 8),
            ),
            _ => flat_palette(
                "Flat Amber",
                egui::Color32::from_rgb(255, 184, 72),
                egui::Color32::from_rgb(18, 11, 5),
            ),
        }
    }

    fn normalize(&mut self) {
        if matches!(self.visual_mode, VisualMode::LegacyParticles) {
            self.visual_mode = VisualMode::Bars;
        }
        self.bar_count = match self.bar_count {
            0..=47 => 32,
            48..=79 => 64,
            80..=111 => 96,
            _ => 128,
        };
        self.bar_gap = self.bar_gap.clamp(0.0, 0.5);
        self.bar_rounding = self.bar_rounding.clamp(0.0, 18.0);
        self.sensitivity = self.sensitivity.clamp(0.5, 3.0);
        self.smoothing_decay = self.smoothing_decay.clamp(0.05, 0.4);
        self.opacity = self.opacity.clamp(0.3, 1.0);
        self.color_theme = self.color_theme.min(THEME_NAMES.len() - 1);
    }
}

fn flat_palette(name: &'static str, color: egui::Color32, background: egui::Color32) -> Palette {
    Palette {
        name,
        primary: color,
        secondary: color,
        background,
    }
}

pub struct DebouncedConfigSave {
    dirty: bool,
    last_change: Instant,
}

impl DebouncedConfigSave {
    pub fn new() -> Self {
        Self {
            dirty: false,
            last_change: Instant::now(),
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_change = Instant::now();
    }

    pub fn tick(&mut self, config: &Config) {
        if self.dirty && self.last_change.elapsed() >= Duration::from_millis(500) {
            config.save();
            self.dirty = false;
        }
    }

    pub fn flush(&mut self, config: &Config) {
        if self.dirty {
            config.save();
            self.dirty = false;
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_owned());
    PathBuf::from(base)
        .join("AudioVisualizer")
        .join("config.json")
}
