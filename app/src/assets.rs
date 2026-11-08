//! Chargement des ressources graphiques et des polices.

use std::sync::Arc;

use eframe::egui;

pub struct Textures {
    pub glass: egui::TextureHandle,
    pub swoosh: egui::TextureHandle,
}

impl Textures {
    pub fn load(ctx: &egui::Context) -> Self {
        Self {
            glass: load_texture(ctx, "miklfe-glass", include_bytes!("../assets/Glass.png")),
            swoosh: load_texture(
                ctx,
                "miklfe-swoosh",
                include_bytes!("../assets/Background.png"),
            ),
        }
    }
}

fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> egui::TextureHandle {
    let image = image::load_from_memory(bytes)
        .expect("ressource PNG du skin Miklfe illisible")
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    ctx.load_texture(name, pixels, egui::TextureOptions::LINEAR)
}

/// Installe les deux familles de polices du skin.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Arca Majora 2 Heavy, fournie avec le skin pour le bloc de date.
    fonts.font_data.insert(
        "miklfe-date".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/Arca Majora 2 Heavy.ttf"
        ))),
    );
    fonts.families.insert(
        egui::FontFamily::Name("miklfe-date".into()),
        vec!["miklfe-date".to_owned(), "Ubuntu-Light".to_owned()],
    );

    // Segoe UI Light / Candara Light : la signature du cadran.
    let candidates = [
        "C:\\Windows\\Fonts\\segoeuil.ttf",
        "C:\\Windows\\Fonts\\Candaral.ttf",
        "C:\\Windows\\Fonts\\Candara.ttf",
    ];
    let mut brand = None;
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            brand = Some(bytes);
            break;
        }
    }

    let brand_family = if let Some(bytes) = brand {
        fonts.font_data.insert(
            "miklfe-brand".to_owned(),
            Arc::new(egui::FontData::from_owned(bytes)),
        );
        vec!["miklfe-brand".to_owned(), "Ubuntu-Light".to_owned()]
    } else {
        vec!["Ubuntu-Light".to_owned()]
    };
    fonts.families.insert(
        egui::FontFamily::Name("miklfe-brand".into()),
        brand_family,
    );

    ctx.set_fonts(fonts);
}
