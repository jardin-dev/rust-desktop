//! Visualiseur : courbe d'onde réactive, dégradé blanc/cyan du skin.

use eframe::egui::{self, Color32, Pos2, Stroke};

use crate::audio::BAND_COUNT;
use crate::theme::{VISUALIZER_HEIGHT, VISUALIZER_WIDTH};
use crate::wave::Wave;

/// La simulation d'onde avance d'un pas par image et a été réglée pour
/// 60 i/s. Le visualiseur tournant désormais à ~30 i/s, on effectue deux pas
/// par image afin de conserver exactement la même vitesse d'animation (le
/// coût, quelques milliers d'opérations, est négligeable devant le rendu).
const WAVE_STEPS_PER_FRAME: usize = 2;

pub struct Visualizer {
    wave: Wave,
    bands: Vec<f32>,
}

impl Visualizer {
    pub fn new() -> Self {
        Self {
            wave: Wave::new(VISUALIZER_WIDTH, VISUALIZER_HEIGHT, BAND_COUNT),
            bands: vec![0.0; BAND_COUNT],
        }
    }

    pub fn set_bands(&mut self, bands: &[f32]) {
        let count = self.bands.len().min(bands.len());
        self.bands[..count].copy_from_slice(&bands[..count]);
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let width = rect.width().max(1.0);

        let points = {
            for _ in 1..WAVE_STEPS_PER_FRAME {
                self.wave.update(&self.bands);
            }
            self.wave.update(&self.bands)
        };
        let painter = ui.painter();

        for pair in points.windows(2) {
            let [a, b] = [pair[0], pair[1]];
            let middle = (a[0] + b[0]) * 0.5 / width;
            let color = sample_gradient(middle);
            if color.a() == 0 {
                continue;
            }
            painter.line_segment(
                [Pos2::new(a[0], a[1]), Pos2::new(b[0], b[1])],
                Stroke::new(2.1, color),
            );
        }
    }
}

fn sample_gradient(t: f32) -> Color32 {
    const STOPS: [(f32, [u8; 4]); 6] = [
        (0.01, [255, 255, 255, 0]),
        (0.30, [255, 255, 255, 255]),
        (0.45, [54, 242, 245, 255]),
        (0.55, [54, 242, 245, 255]),
        (0.70, [255, 255, 255, 255]),
        (0.99, [255, 255, 255, 0]),
    ];

    let t = t.clamp(0.0, 1.0);
    if t <= STOPS[0].0 {
        return Color32::TRANSPARENT;
    }
    if t >= STOPS[STOPS.len() - 1].0 {
        return Color32::TRANSPARENT;
    }

    for window in STOPS.windows(2) {
        let (start, end) = (window[0], window[1]);
        if t >= start.0 && t <= end.0 {
            let local = (t - start.0) / (end.0 - start.0);
            let color = [
                lerp_u8(start.1[0], end.1[0], local),
                lerp_u8(start.1[1], end.1[1], local),
                lerp_u8(start.1[2], end.1[2], local),
                lerp_u8(start.1[3], end.1[3], local),
            ];
            return Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
        }
    }

    Color32::TRANSPARENT
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}
