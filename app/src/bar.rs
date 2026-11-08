//! Bandes d'ombre latérales (`Bar.ini`) : noir 50 % vers transparent.

use eframe::egui::{
    self,
    epaint::{Mesh, Vertex, WHITE_UV},
    Color32, Rect, Shape,
};

pub fn draw(ui: &egui::Ui) {
    let rect = ui.max_rect();
    let painter = ui.painter();

    let black = Color32::from_rgba_unmultiplied(0, 0, 0, 128);
    let transparent = Color32::TRANSPARENT;

    let left = Rect::from_min_max(
        rect.left_top(),
        egui::pos2(rect.left() + rect.width() * 0.4, rect.bottom()),
    );
    gradient_rect(painter, left, black, transparent);

    let right = Rect::from_min_max(
        egui::pos2(rect.left() + rect.width() * 0.6, rect.top()),
        rect.right_bottom(),
    );
    gradient_rect(painter, right, transparent, black);
}

fn gradient_rect(painter: &egui::Painter, rect: Rect, left: Color32, right: Color32) {
    let mut mesh = Mesh::default();
    mesh.vertices = vec![
        Vertex {
            pos: rect.left_top(),
            uv: WHITE_UV,
            color: left,
        },
        Vertex {
            pos: rect.right_top(),
            uv: WHITE_UV,
            color: right,
        },
        Vertex {
            pos: rect.right_bottom(),
            uv: WHITE_UV,
            color: right,
        },
        Vertex {
            pos: rect.left_bottom(),
            uv: WHITE_UV,
            color: left,
        },
    ];
    mesh.indices = vec![0, 1, 2, 0, 2, 3];
    painter.add(Shape::mesh(mesh));
}
