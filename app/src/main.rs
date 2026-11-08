//! Miklfe — widget de bureau natif inspiré du skin Rainmeter du même nom.
//!
//! L'horloge, le visualiseur audio et les bandes d'ombre sont trois fenêtres
//! sans bordure et transparentes. La fenêtre racine n'est qu'un hôte invisible
//! d'un pixel : sur Windows, seuls les viewports enfants conservent la
//! transparence avec le backend OpenGL.
//!
//! Aucun réglage : les positions sont mémorisées automatiquement dans
//! `%APPDATA%\Miklfe\window.json`.

#![windows_subsystem = "windows"]
mod assets;
mod audio;
mod bar;
mod clock;
mod state;
mod theme;
mod visualizer;
mod wave;
mod win;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use eframe::egui::{self, ViewportBuilder, ViewportCommand};
use state::{Position, WindowState};

use crate::audio::{AudioEngine, BAND_COUNT};
use crate::visualizer::Visualizer;

/// Cadence des fenêtres animées (visualiseur et hôte) : ~30 i/s. Au-delà, le
/// gain visuel est imperceptible pour ces widgets et le coût CPU double.
const ANIMATED_FRAME_INTERVAL: Duration = Duration::from_millis(33);

/// L'horloge n'a aucune raison de suivre les animations : deux mises à jour
/// par seconde suffisent pour l'aiguille des secondes, sans charge mesurable.
const CLOCK_FRAME_INTERVAL: Duration = Duration::from_millis(500);

/// Cadence de contrôle des cadres Win32. Après une veille, Windows peut
/// redessiner la zone non cliente des fenêtres sans prévenir ; on vérifie donc
/// périodiquement qu'aucun titre n'est réapparu.
const CHROME_CHECK_INTERVAL: Duration = Duration::from_millis(500);

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    // Garde mono-instance : une nouvelle fenêtre trouve l'ancienne fenêtre
    // par son titre fixe (FindWindowW). Si deux instances tournent de pair,
    // les deux manipulent les mêmes fenêtres et l'ordre Z/DWM se corrompt.
    // Un mutex nommé fait quitter la seconde instance immédiatement.
    {
        use windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS;
        use windows_sys::Win32::System::Threading::CreateMutexW;
        let name: Vec<u16> = "MiklfeWidgetSingleInstanceMutex\0"
            .encode_utf16()
            .collect();
        unsafe {
            let _ = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
            if windows_sys::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS {
                return Ok(());
            }
        }
    }

    let demo = std::env::args().any(|argument| argument == "--demo");
    let saved = WindowState::load();

    // Hôte minuscule et hors écran : il ne sert qu'à faire tourner la boucle
    // d'événements et à posséder les trois viewports visibles.
    let viewport = ViewportBuilder::default()
        .with_title("Miklfe host")
        .with_inner_size([1.0, 1.0])
        .with_position([-200.0, -200.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_resizable(false)
        .with_taskbar(false);

    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        glow_options: eframe::egui_glow::GlowConfiguration {
            vsync: true,
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "Miklfe",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, saved, demo)))),
    )
}

fn clock_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("miklfe-clock")
}

fn visualizer_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("miklfe-visualizer")
}

fn bar_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("miklfe-bar")
}

/// État partagé entre l'hôte, l'horloge et le visualiseur.
struct Shared {
    clock_target: Position,
    clock_live: Position,
    visualizer: Position,
    quit: bool,
}

struct App {
    state: WindowState,
    shared: Arc<Mutex<Shared>>,
    textures: Arc<assets::Textures>,
    visualizer: Arc<Mutex<Visualizer>>,
    audio: Option<AudioEngine>,
    bands: Vec<f32>,
    demo: bool,
    started: Instant,
    quit: bool,
    dirty: bool,
    last_save: Instant,
    last_win_tune: Instant,
    last_chrome_check: Instant,
    last_wall: SystemTime,
    last_desktop_check: Instant,
    desktop_shown: bool,
    win_tuned: bool,
    first_frame: bool,
    monitor_size: egui::Vec2,
    // Interrupteurs de diagnostic (variables d'environnement) utilisés pour
    // isoler la fuite mémoire. À retirer une fois la cause corrigée.
    no_stack: bool,
    no_desktop: bool,
    no_clock_draw: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, saved: WindowState, demo: bool) -> Self {
        assets::install_fonts(&cc.egui_ctx);
        let textures = Arc::new(assets::Textures::load(&cc.egui_ctx));
        let audio = if demo {
            None
        } else {
            Some(AudioEngine::start())
        };

        let placeholder = Position { x: 0.0, y: 0.0 };
        Self {
            state: saved,
            shared: Arc::new(Mutex::new(Shared {
                clock_target: placeholder,
                clock_live: placeholder,
                visualizer: placeholder,
                quit: false,
            })),
            textures,
            visualizer: Arc::new(Mutex::new(Visualizer::new())),
            audio,
            bands: vec![0.0; BAND_COUNT],
            demo,
            started: Instant::now(),
            quit: false,
            dirty: false,
            last_save: Instant::now(),
            last_win_tune: Instant::now() - Duration::from_secs(10),
            last_chrome_check: Instant::now() - Duration::from_secs(10),
            last_wall: SystemTime::now(),
            last_desktop_check: Instant::now() - Duration::from_secs(10),
            desktop_shown: false,
            win_tuned: false,
            first_frame: true,
            monitor_size: egui::vec2(1536.0, 864.0),
            no_stack: std::env::var_os("MIKLFE_NO_STACK").is_some(),
            no_desktop: std::env::var_os("MIKLFE_NO_DESKTOP").is_some(),
            no_clock_draw: std::env::var_os("MIKLFE_NO_CLOCK_DRAW").is_some(),
        }
    }

    fn initialize_positions(&mut self, ctx: &egui::Context) {
        let monitor = ctx
            .input(|input| input.viewport().monitor_size)
            .unwrap_or(self.monitor_size);
        self.monitor_size = monitor;

        let clock_size = egui::vec2(theme::CLOCK_WIDTH, theme::CLOCK_HEIGHT);
        let visualizer_size = egui::vec2(theme::VISUALIZER_WIDTH, theme::VISUALIZER_HEIGHT);

        let clock = self
            .state
            .clock
            .map(|position| clamp_position(position, monitor, clock_size))
            .unwrap_or_else(|| default_clock_position(monitor));
        let visualizer = self
            .state
            .visualizer
            .map(|position| clamp_position(position, monitor, visualizer_size))
            .unwrap_or_else(|| default_visualizer_position(monitor));

        if let Ok(mut shared) = self.shared.lock() {
            shared.clock_target = clock;
            shared.clock_live = clock;
            shared.visualizer = visualizer;
        }
    }

    fn update_bands(&mut self) {
        if let Some(audio) = &self.audio {
            audio.fill_bands(&mut self.bands);
        } else if self.demo {
            demo_bands(self.started.elapsed().as_secs_f32(), &mut self.bands);
        }

        if let Ok(mut visualizer) = self.visualizer.lock() {
            visualizer.set_bands(&self.bands);
        }
    }

    fn show_clock(&self, ctx: &egui::Context) {
        let target = self
            .shared
            .lock()
            .map(|shared| shared.clock_target)
            .unwrap_or(Position { x: 0.0, y: 0.0 });

        let builder = ViewportBuilder::default()
            .with_title("Miklfe")
            .with_inner_size([theme::CLOCK_WIDTH, theme::CLOCK_HEIGHT])
            .with_position([target.x, target.y])
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(egui::WindowLevel::AlwaysOnBottom)
            .with_resizable(false)
            .with_taskbar(false);

        let shared = Arc::clone(&self.shared);
        let textures = Arc::clone(&self.textures);
        let draw_clock = !self.no_clock_draw;

        ctx.show_viewport_deferred(clock_id(), builder, move |ui, _class| {
            let ctx = ui.ctx().clone();

            if draw_clock {
                clock::draw(ui, textures.as_ref());
            }

            let response = ui.interact(
                ui.max_rect(),
                ui.scope_id().with("miklfe-clock"),
                egui::Sense::click_and_drag(),
            );
            if response.drag_started_by(egui::PointerButton::Primary) {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }
            if response.hovered() {
                ctx.set_cursor_icon(egui::CursorIcon::Grab);
            }
            response.context_menu(|ui| clock_menu(ui, &shared));

            if let Some(rect) = ctx.input(|input| input.viewport().outer_rect) {
                let position = Position {
                    x: rect.min.x,
                    y: rect.min.y,
                };
                let released =
                    response.drag_stopped() || ctx.input(|input| input.pointer.any_released());
                if let Ok(mut shared) = shared.lock() {
                    shared.clock_live = position;
                    if released {
                        shared.clock_target = position;
                    }
                }
                if released {
                    win::set_desktop_z_order(
                        "Miklfe",
                        win::is_show_desktop_active("Miklfe host"),
                    );
                }
            }

            if ctx.input(|input| input.viewport().close_requested()) {
                if let Ok(mut shared) = shared.lock() {
                    shared.quit = true;
                }
            }

            ctx.request_repaint_after(CLOCK_FRAME_INTERVAL);
        });
    }

    fn show_visualizer(&self, ctx: &egui::Context) {
        let position = self
            .shared
            .lock()
            .map(|shared| shared.visualizer)
            .unwrap_or(Position { x: 0.0, y: 0.0 });

        let builder = ViewportBuilder::default()
            .with_title("Miklfe visualizer")
            .with_inner_size([theme::VISUALIZER_WIDTH, theme::VISUALIZER_HEIGHT])
            .with_position([position.x, position.y])
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(egui::WindowLevel::AlwaysOnBottom)
            .with_resizable(false)
            .with_taskbar(false)
            .with_mouse_passthrough(true);

        let visualizer = Arc::clone(&self.visualizer);
        let shared = Arc::clone(&self.shared);

        ctx.show_viewport_deferred(visualizer_id(), builder, move |ui, _class| {
            let ctx = ui.ctx().clone();

            if let Ok(mut visualizer) = visualizer.lock() {
                visualizer.draw(ui);
            } else {
                return;
            }

            if ctx.input(|input| input.viewport().close_requested()) {
                if let Ok(mut shared) = shared.lock() {
                    shared.quit = true;
                }
            }

            ctx.request_repaint_after(ANIMATED_FRAME_INTERVAL);
        });
    }

    fn show_bar(&self, ctx: &egui::Context) {
        let builder = ViewportBuilder::default()
            .with_title("Miklfe bar")
            .with_position([0.0, 0.0])
            .with_inner_size([self.monitor_size.x, self.monitor_size.y])
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(egui::WindowLevel::AlwaysOnBottom)
            .with_mouse_passthrough(true)
            .with_resizable(false)
            .with_taskbar(false)
            .with_active(false);

        ctx.show_viewport_deferred(bar_id(), builder, |ui, _class| {
            bar::draw(ui);
            ui.ctx().request_repaint_after(Duration::from_secs(3600));
        });
    }

    fn persist(&mut self, now: Instant) {
        let (clock, visualizer, quit) = match self.shared.lock() {
            Ok(shared) => (shared.clock_live, shared.visualizer, shared.quit),
            Err(_) => return,
        };

        let clock_changed = self.state.clock.is_none_or(|previous| {
            (previous.x - clock.x).abs() > 0.5 || (previous.y - clock.y).abs() > 0.5
        });
        if clock_changed {
            self.state.clock = Some(clock);
            self.dirty = true;
        }

        let visualizer_changed = self.state.visualizer.is_none_or(|previous| {
            (previous.x - visualizer.x).abs() > 0.5
                || (previous.y - visualizer.y).abs() > 0.5
        });
        if visualizer_changed {
            self.state.visualizer = Some(visualizer);
            self.dirty = true;
        }

        if quit {
            self.quit = true;
        }

        if self.dirty && self.last_save.elapsed() >= Duration::from_secs(1) {
            self.state.save();
            self.dirty = false;
            self.last_save = now;
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        if std::env::var_os("MIKLFE_RED").is_some() {
            [0.5, 0.0, 0.0, 0.5]
        } else {
            [0.0, 0.0, 0.0, 0.0]
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.first_frame {
            self.initialize_positions(&ctx);
            self.first_frame = false;
        }
        if let Some(size) = ctx.input(|input| input.viewport().monitor_size) {
            self.monitor_size = size;
        }

        let wall = SystemTime::now();
        let resumed = wall
            .duration_since(self.last_wall)
            .is_ok_and(|gap| gap >= Duration::from_secs(5));
        self.last_wall = wall;

        if resumed {
            // Un seul passage propre et immédiat pour toutes les fenêtres au réveil
            for title in &["Miklfe", "Miklfe visualizer", "Miklfe bar"] {
                win::tune(title);
            }
            // On verrouille directement pour empêcher le bloc à 100ms de relancer un tune superflu
            self.win_tuned = true;
            self.last_win_tune = Instant::now();
            ctx.request_repaint();
        }

        self.update_bands();
        if std::env::var_os("MIKLFE_NO_BAR").is_none() {
            self.show_bar(&ctx);
        }
        if std::env::var_os("MIKLFE_NO_VIZ").is_none() {
            self.show_visualizer(&ctx);
        }
        self.show_clock(&ctx);
        self.persist(Instant::now());

        if !self.no_desktop && self.last_desktop_check.elapsed() >= Duration::from_millis(150) {
            let desktop_shown = win::is_show_desktop_active("Miklfe host");
            if desktop_shown != self.desktop_shown {
                self.desktop_shown = desktop_shown;
                let order: [&str; 3] = if desktop_shown {
                    ["Miklfe bar", "Miklfe visualizer", "Miklfe"]
                } else {
                    ["Miklfe", "Miklfe visualizer", "Miklfe bar"]
                };
                for title in order {
                    win::set_desktop_z_order(title, desktop_shown);
                }
            }
            self.last_desktop_check = Instant::now();
        }

        if !self.no_stack {
            win::keep_widget_stack();
        }

        if self.last_chrome_check.elapsed() >= CHROME_CHECK_INTERVAL {
            let mut repaired = false;
            for title in ["Miklfe", "Miklfe visualizer", "Miklfe bar"] {
                repaired |= win::repair(title);
            }
            if repaired {
                // Le cadre retiré laisse une zone non cliente à redessiner.
                for id in [clock_id(), visualizer_id(), bar_id()] {
                    ctx.request_repaint_of(id);
                }
            }
            self.last_chrome_check = Instant::now();
        }

        if !self.win_tuned && self.last_win_tune.elapsed() >= Duration::from_millis(100) {
            self.win_tuned = ["Miklfe", "Miklfe visualizer", "Miklfe bar"]
                .iter()
                .all(|title| win::tune(title));
            self.last_win_tune = Instant::now();
        }

        if self.quit {
            self.state.save();
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        ctx.request_repaint_after(ANIMATED_FRAME_INTERVAL);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.state.save();
    }
}

fn clock_menu(ui: &mut egui::Ui, shared: &Arc<Mutex<Shared>>) {
    ui.set_min_width(210.0);

    if ui.button("Quitter").clicked() {
        if let Ok(mut shared) = shared.lock() {
            shared.quit = true;
        }
        ui.close();
    }
}

fn clamp_position(position: Position, monitor: egui::Vec2, size: egui::Vec2) -> Position {
    let min_x = 80.0;
    let max_x = (monitor.x - 80.0 - size.x).max(min_x);
    let min_y = 0.0;
    let max_y = (monitor.y - 60.0 - size.y).max(min_y);
    Position {
        x: position.x.clamp(min_x, max_x),
        y: position.y.clamp(min_y, max_y),
    }
}

fn default_clock_position(monitor: egui::Vec2) -> Position {
    Position {
        x: (monitor.x - theme::CLOCK_WIDTH - 48.0).max(12.0),
        y: 48.0,
    }
}

fn default_visualizer_position(monitor: egui::Vec2) -> Position {
    Position {
        x: ((monitor.x - theme::VISUALIZER_WIDTH) * 0.5).max(12.0),
        y: (monitor.y - theme::VISUALIZER_HEIGHT - 70.0).max(12.0),
    }
}

fn demo_bands(time: f32, out: &mut [f32]) {
    let count = out.len() as f32;
    let sweep = (time * 0.14).fract() * count;

    for (index, slot) in out.iter_mut().enumerate() {
        let x = index as f32;
        let peak = (-(x - sweep).powi(2) / 950.0).exp();
        let bass =
            (-(x - 12.0).powi(2) / 130.0).exp() * (0.4 + 0.4 * (time * 2.1).sin().abs());
        let ripple = 0.07 * (x * 0.37 + time * 4.0).sin().abs();
        let envelope = 0.72 + 0.28 * (time * 1.7).sin().abs();
        *slot = ((peak * 0.9 + bass * 0.8 + ripple) * envelope).clamp(0.0, 1.0);
    }
}