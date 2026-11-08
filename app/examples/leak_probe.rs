//! Banc d'essai mémoire : eframe/glow minimal, sans le code du widget.
//!
//! Lancer :
//!   cargo run --release --example leak_probe
//!
//! Variables d'environnement :
//!   LEAK_TRANSPARENT=0|1  fenêtre transparente (défaut 1)
//!   LEAK_SIZE=200         côté de la fenêtre en points (défaut 200)
//!   LEAK_REPAINT_MS=16    intervalle de repaint (0 = aucun, défaut 16)
//!   LEAK_POS=100,100      position de la fenêtre
//!   LEAK_VIEWPORTS=0|1    ajoute un viewport enfant à 60 fps (défaut 0)
//!   LEAK_CHILD_DRAW=0|1   le viewport enfant dessine aussi le contenu (défaut 1)
//!   LEAK_CHILD_REPAINT=0|1 le viewport enfant se rafraîchit à 60 fps (défaut 1)
//!   LEAK_DRAW=none|one|text|shapes|both  contenu dessiné à chaque image
//!   LEAK_ON_BOTTOM=0|1    niveau de fenêtre AlwaysOnBottom (défaut 0)
//!   LEAK_ON_TOP=0|1       niveau de fenêtre AlwaysOnTop (défaut 0)
//!   LEAK_VSYNC=0|1        synchronisation verticale (défaut 1)

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use eframe::egui;

/// Compteur d'allocations Rust : permet de distinguer une fuite du tas
/// applicatif d'une fuite du pilote graphique (mémoire privée hors tas).
struct CountingAlloc;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let now = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(now, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
            let now = LIVE.fetch_add(new_size, Ordering::Relaxed) + new_size;
            PEAK.fetch_max(now, Ordering::Relaxed);
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;


fn env_bool(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|value| value != "0")
}

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[derive(Clone, Copy, PartialEq)]
enum DrawMode {
    None,
    One,
    Text,
    Shapes,
    Both,
}

impl DrawMode {
    fn from_env() -> Self {
        match std::env::var("LEAK_DRAW").ok().as_deref() {
            Some("one") => Self::One,
            Some("text") => Self::Text,
            Some("shapes") => Self::Shapes,
            Some("both") => Self::Both,
            _ => Self::None,
        }
    }

    fn text(self) -> bool {
        matches!(self, Self::Text | Self::Both)
    }

    fn shapes(self) -> bool {
        matches!(self, Self::Shapes | Self::Both)
    }

    fn one(self) -> bool {
        matches!(self, Self::One)
    }
}

struct Probe {
    repaint: Option<Duration>,
    add_viewport: bool,
    child_draw: bool,
    child_repaint: bool,
    draw: DrawMode,
}

fn paint_content(ui: &egui::Ui, draw: DrawMode) {
    let painter = ui.painter();
    let rect = ui.max_rect();
    let center = rect.center();

    if draw.one() {
        painter.line_segment(
            [rect.left_top() + egui::Vec2::new(10.0, 10.0), rect.right_bottom()],
            egui::Stroke::new(1.0, egui::Color32::WHITE),
        );
    }

    if draw.shapes() {
        for tick in 0..12 {
            let angle = std::f32::consts::TAU * tick as f32 / 12.0;
            let direction = egui::Vec2::new(angle.cos(), angle.sin());
            painter.line_segment(
                [
                    center + direction * (rect.width() * 0.2),
                    center + direction * (rect.width() * 0.45),
                ],
                egui::Stroke::new(1.2, egui::Color32::WHITE),
            );
        }
        painter.circle_stroke(center, rect.width() * 0.45, egui::Stroke::new(1.0, egui::Color32::CYAN));
        painter.circle_filled(center, 5.0, egui::Color32::CYAN);
        painter.circle_filled(center, 2.0, egui::Color32::BLACK);
    }

    if draw.text() {
        // Même profil que l'horloge : quatre textes, chacun doublé d'une ombre.
        let font = egui::FontId::proportional(18.0);
        let texts = [
            "C A N T A R E L L",
            "Lundi",
            "01",
            "Novembre",
        ];
        for (index, text) in texts.iter().enumerate() {
            let position = rect.left_top()
                + egui::Vec2::new(8.0, 8.0 + index as f32 * 24.0);
            painter.text(
                position + egui::Vec2::new(1.2, 1.4),
                egui::Align2::LEFT_TOP,
                *text,
                font.clone(),
                egui::Color32::from_black_alpha(80),
            );
            painter.text(
                position,
                egui::Align2::LEFT_TOP,
                *text,
                font.clone(),
                egui::Color32::WHITE,
            );
        }
    }
}

impl eframe::App for Probe {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        paint_content(ui, self.draw);

        if self.add_viewport {
            let builder = egui::ViewportBuilder::default()
                .with_title("Leak probe child")
                .with_inner_size([200.0, 200.0])
                .with_position([320.0, 100.0])
                .with_decorations(false)
                .with_transparent(true)
                .with_resizable(false)
                .with_taskbar(false);

            let child_draw = if self.child_draw { self.draw } else { DrawMode::None };
            let child_repaint = self.child_repaint;

            ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of("leak-probe-child"),
                builder,
                move |ui, _class| {
                    paint_content(ui, child_draw);
                    if child_repaint {
                        ui.ctx().request_repaint_after(Duration::from_millis(16));
                    }
                },
            );
        }

        if let Some(delay) = self.repaint {
            ctx.request_repaint_after(delay);
        }
    }
}

fn main() -> eframe::Result {
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(5));
        eprintln!(
            "LIVE={:.1} Ko  PEAK={:.1} Ko",
            LIVE.load(Ordering::Relaxed) as f64 / 1024.0,
            PEAK.load(Ordering::Relaxed) as f64 / 1024.0,
        );
    });

    let transparent = env_bool("LEAK_TRANSPARENT").unwrap_or(true);
    let size = env_f32("LEAK_SIZE", 200.0);
    let repaint_ms = env_f32("LEAK_REPAINT_MS", 16.0) as u64;
    let add_viewport = env_bool("LEAK_VIEWPORTS").unwrap_or(false);
    let child_draw = env_bool("LEAK_CHILD_DRAW").unwrap_or(true);
    let child_repaint = env_bool("LEAK_CHILD_REPAINT").unwrap_or(true);
    let on_bottom = env_bool("LEAK_ON_BOTTOM").unwrap_or(false);
    let on_top = env_bool("LEAK_ON_TOP").unwrap_or(false);
    let vsync = env_bool("LEAK_VSYNC").unwrap_or(true);
    let draw = DrawMode::from_env();

    let (x, y) = std::env::var("LEAK_POS")
        .ok()
        .and_then(|value| {
            let mut parts = value.split(',');
            Some((
                parts.next()?.trim().parse().ok()?,
                parts.next()?.trim().parse().ok()?,
            ))
        })
        .unwrap_or((100.0, 100.0));

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Leak probe")
        .with_inner_size([size, size])
        .with_position([x, y])
        .with_decorations(false)
        .with_transparent(transparent)
        .with_resizable(false)
        .with_taskbar(false);

    if on_bottom {
        viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
    }
    if on_top {
        viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnTop);
    }

    let renderer = eframe::Renderer::Wgpu;
    let _ = vsync; // vsync is ignored in the wgpu probe
    let options = eframe::NativeOptions {
        viewport,
        renderer,
        ..Default::default()
    };

    eframe::run_native(
        "Leak probe",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(Probe {
                repaint: (repaint_ms > 0).then(|| Duration::from_millis(repaint_ms)),
                add_viewport,
                child_draw,
                child_repaint,
                draw,
            }))
        }),
    )
}
