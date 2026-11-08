//! Constantes visuelles reprises du skin `Miklfe`.

use eframe::egui::{Color32, FontFamily, FontId};

/// Facteur d'échelle du skin (`Scale=3.7`).
pub const SCALE: f32 = 3.7;

pub const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
pub const WHITE_192: Color32 = Color32::from_rgba_premultiplied(192, 192, 192, 192);
pub const WHITE_150: Color32 = Color32::from_rgba_premultiplied(150, 150, 150, 150);
pub const WHITE_190: Color32 = Color32::from_rgba_premultiplied(190, 190, 190, 190);
pub const CYAN: Color32 = Color32::from_rgb(54, 242, 245);
pub const BLACK: Color32 = Color32::from_rgb(0, 0, 0);

pub const GLASS_ALPHA: u8 = 200;

// Géométrie du cadran, en pixels logiques (unités du skin × SCALE).
pub const OUTER_RING: f32 = 24.0 * SCALE;
pub const TICK_OUTER: f32 = 23.0 * SCALE;
pub const TICK_INNER_MAJOR: f32 = 20.5 * SCALE;
pub const TICK_INNER_MINOR: f32 = 21.5 * SCALE;
pub const HOUR_HAND: f32 = 12.0 * SCALE;
pub const MINUTE_HAND: f32 = 18.0 * SCALE;
pub const SECOND_HAND: f32 = 18.0 * SCALE;
pub const HOUR_WIDTH: f32 = 1.5 * SCALE;
pub const MINUTE_WIDTH: f32 = 1.0 * SCALE;
pub const SECOND_WIDTH: f32 = 1.0 * SCALE;

/// Taille de la fenêtre de l'horloge.
///
/// La largeur doit contenir le bloc de date : le mois commence à
/// `CLOCK_CENTER.x - 18` et « Septembre »/« Novembre » mesurent environ 150 px
/// en Arca Majora 2 Heavy à 26 px (le skin d'origine s'auto-dimensionnait).
pub const CLOCK_WIDTH: f32 = 380.0;
pub const CLOCK_HEIGHT: f32 = 310.0;

/// Centre du cadran à l'intérieur de la fenêtre de l'horloge.
pub const CLOCK_CENTER: eframe::egui::Pos2 = eframe::egui::pos2(222.0, 118.0);

/// Taille de la fenêtre du visualiseur (`BarWidth` / `BarHeight`).
pub const VISUALIZER_WIDTH: f32 = 607.0;
pub const VISUALIZER_HEIGHT: f32 = 400.0;

pub fn brand_font() -> FontId {
    FontId::new(11.0, FontFamily::Name("miklfe-brand".into()))
}

pub fn date_font() -> FontId {
    FontId::new(26.0, FontFamily::Name("miklfe-date".into()))
}

