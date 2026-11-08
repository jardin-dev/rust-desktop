//! Persistance minimale des positions de fenêtres.
//!
//! Ce n'est pas un fichier de configuration : il ne contient que la dernière
//! position des deux widgets sur le bureau, et il est réécrit automatiquement.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WindowState {
    pub clock: Option<Position>,
    pub visualizer: Option<Position>,
}

impl WindowState {
    pub fn load() -> Self {
        let path = state_path();
        let Ok(contents) = fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str(&contents).unwrap_or_default()
    }

    pub fn save(&self) {
        let path = state_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let Ok(json) = serde_json::to_string_pretty(self) else {
            return;
        };
        let temporary = path.with_extension("json.tmp");
        if fs::write(&temporary, json).is_ok() {
            let _ = fs::rename(&temporary, &path);
        }
    }
}

fn state_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("rustdesktop").join("window.json")
}
