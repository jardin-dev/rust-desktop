//! Rendu de l'horloge : cadran, aiguilles, bloc de date et verre.

use std::f32::consts::{FRAC_PI_2, TAU};

use chrono::{Datelike, Local, Timelike};
use eframe::egui::{self, Align2, Color32, Pos2, Rect, Stroke, Vec2};

use crate::assets::Textures;
use crate::theme::*;

const DAYS: [&str; 7] = [
    "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche",
];
const MONTHS: [&str; 12] = [
    "Janvier", "Février", "Mars", "Avril", "Mai", "Juin", "Juillet", "Août",
    "Septembre", "Octobre", "Novembre", "Décembre",
];

pub fn draw(ui: &egui::Ui, textures: &Textures) {
    let painter = ui.painter();
    let center = CLOCK_CENTER;

    draw_dial(painter, center);

    // La marque est sous les aiguilles, comme dans le skin d'origine : le
    // texte est donc dessiné avant elles.
    shadowed_text(
        painter,
        center + Vec2::new(0.0, 22.2),
        Align2::CENTER_TOP,
        "C A N T A R E L L",
        brand_font(),
        WHITE_190,
    );

    draw_hands(painter, center);

    let glass_size = 48.0 * SCALE;
    painter.image(
        textures.glass.id(),
        Rect::from_center_size(center, Vec2::splat(glass_size)),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::from_white_alpha(GLASS_ALPHA),
    );

    let now = Local::now();
    let day = DAYS[now.weekday().num_days_from_monday() as usize];
    let date = format!("{:02}", now.day());
    let month = MONTHS[now.month0() as usize];

    // Le jour est sous le swoosh, comme dans le skin d'origine.
    shadowed_text(
        painter,
        center + Vec2::new(-56.0, 80.0),
        Align2::RIGHT_TOP,
        day,
        date_font(),
        WHITE,
    );

    let swoosh_width = 96.2;
    let swoosh_height = swoosh_width * 641.0 / 683.0;
    painter.image(
        textures.swoosh.id(),
        Rect::from_min_size(
            center + Vec2::new(-96.2, 85.0),
            Vec2::new(swoosh_width, swoosh_height),
        ),
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        CYAN,
    );

    painter.text(
        center + Vec2::new(-44.0, 110.0),
        Align2::CENTER_TOP,
        date,
        date_font(),
        BLACK,
    );
    shadowed_text(
        painter,
        center + Vec2::new(-18.0, 140.0),
        Align2::LEFT_TOP,
        month,
        date_font(),
        WHITE,
    );
}

fn draw_dial(painter: &egui::Painter, center: Pos2) {
    // Fond noir du cadran : il couvre tout le disque, sous le liseré, les
    // graduations et les aiguilles. Le reste de la fenêtre reste transparent.
    painter.circle_filled(center, OUTER_RING + 0.8, Color32::from_black_alpha(220) );

    // Ombre légère sur les bords clairs.
    painter.circle_stroke(
        center,
        OUTER_RING + 0.8,
        Stroke::new(2.0, Color32::from_black_alpha(45)),
    );
    painter.circle_stroke(
        center,
        OUTER_RING,
        Stroke::new(1.2, Color32::from_rgba_premultiplied(230, 230, 230, 230)),
    );

    for tick in 0..12 {
        let angle = -FRAC_PI_2 + tick as f32 / 12.0 * TAU;
        let direction = Vec2::new(angle.cos(), angle.sin());
        let major = tick % 3 == 0;
        let inner = if major {
            TICK_INNER_MAJOR
        } else {
            TICK_INNER_MINOR
        };
        painter.line_segment(
            [center + direction * inner, center + direction * TICK_OUTER],
            Stroke::new(if major { 1.6 } else { 1.1 }, WHITE),
        );
    }

    // Petit point central noir puis cyan.
    painter.circle_filled(center, 6.2, Color32::from_black_alpha(110));
    painter.circle_filled(center, 4.8, CYAN);
    painter.circle_stroke(center, 6.4, Stroke::new(0.8, WHITE_192));
}

fn draw_hands(painter: &egui::Painter, center: Pos2) {
    let now = Local::now();
    let seconds = now.num_seconds_from_midnight() as f32
        + now.nanosecond() as f32 / 1_000_000_000.0;

    let hour_fraction = (seconds % 43_200.0) / 43_200.0;
    let minute_fraction = (seconds % 3_600.0) / 3_600.0;
    let second_fraction = (seconds % 60.0) / 60.0;

    hand(
        painter,
        center,
        -FRAC_PI_2 + hour_fraction * TAU,
        HOUR_HAND,
        HOUR_WIDTH,
        CYAN,
    );
    hand(
        painter,
        center,
        -FRAC_PI_2 + minute_fraction * TAU,
        MINUTE_HAND,
        MINUTE_WIDTH,
        CYAN,
    );
    hand(
        painter,
        center,
        -FRAC_PI_2 + second_fraction * TAU,
        SECOND_HAND,
        SECOND_WIDTH,
        WHITE_150,
    );
}

fn hand(
    painter: &egui::Painter,
    center: Pos2,
    angle: f32,
    length: f32,
    width: f32,
    color: Color32,
) {
    let direction = Vec2::new(angle.cos(), angle.sin());
    let tip = center + direction * length;
    painter.line_segment([center, tip], Stroke::new(width, color));
    painter.circle_filled(tip, width * 0.5, color);
    painter.circle_filled(center, width * 0.55, color);
}

fn shadowed_text(
    painter: &egui::Painter,
    position: Pos2,
    align: Align2,
    text: &str,
    font: egui::FontId,
    color: Color32,
) {
    painter.text(
        position + Vec2::new(1.2, 1.4),
        align,
        text,
        font.clone(),
        Color32::from_black_alpha(80),
    );
    painter.text(position, align, text, font, color);
}
