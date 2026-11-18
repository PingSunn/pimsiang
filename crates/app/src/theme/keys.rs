use egui::{Color32, Painter, Pos2, Rect, Stroke, Ui, pos2, vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Glyph {
    Command,
    Option,
    Control,
    Shift,
}

#[derive(Clone, Copy)]
pub enum Part<'a> {
    Glyph(Glyph),
    Text(&'a str),
}

const GAP: f32 = 4.0;

pub fn paint_glyph(painter: &Painter, center: Pos2, size: f32, glyph: Glyph, color: Color32) {
    let half = size / 2.0;
    let stroke = Stroke::new((size / 9.0).max(1.0), color);
    let at = |x: f32, y: f32| pos2(center.x + x * half * 0.8, center.y + y * half * 0.8);
    match glyph {
        Glyph::Command => {
            painter.rect_stroke(
                Rect::from_min_max(at(-0.38, -0.38), at(0.38, 0.38)),
                0.0,
                stroke,
                egui::StrokeKind::Middle,
            );
            for (x, y) in [(-0.62, -0.62), (0.62, -0.62), (-0.62, 0.62), (0.62, 0.62)] {
                painter.circle_stroke(at(x, y), half * 0.8 * 0.26, stroke);
            }
        }
        Glyph::Option => {
            painter.add(egui::Shape::line(
                vec![at(-0.9, -0.55), at(-0.3, -0.55), at(0.3, 0.55), at(0.9, 0.55)],
                stroke,
            ));
            painter.line_segment([at(0.15, -0.55), at(0.9, -0.55)], stroke);
        }
        Glyph::Control => {
            painter.add(egui::Shape::line(
                vec![at(-0.65, 0.3), at(0.0, -0.4), at(0.65, 0.3)],
                stroke,
            ));
        }
        Glyph::Shift => {
            painter.add(egui::Shape::closed_line(
                vec![
                    at(0.0, -0.85),
                    at(0.85, 0.05),
                    at(0.42, 0.05),
                    at(0.42, 0.8),
                    at(-0.42, 0.8),
                    at(-0.42, 0.05),
                    at(-0.85, 0.05),
                ],
                stroke,
            ));
        }
    }
}

pub fn parts_width(ui: &Ui, parts: &[Part<'_>], font: &egui::FontId, size: f32) -> f32 {
    let mut width = 0.0;
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            width += GAP;
        }
        width += match part {
            Part::Glyph(_) => size,
            Part::Text(text) => ui
                .painter()
                .layout_no_wrap((*text).to_owned(), font.clone(), Color32::PLACEHOLDER)
                .size()
                .x,
        };
    }
    width
}

pub fn paint_parts(
    ui: &Ui,
    left_center: Pos2,
    parts: &[Part<'_>],
    font: &egui::FontId,
    size: f32,
    color: Color32,
) {
    let mut x = left_center.x;
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            x += GAP;
        }
        match part {
            Part::Glyph(glyph) => {
                paint_glyph(
                    ui.painter(),
                    pos2(x + size / 2.0, left_center.y),
                    size,
                    *glyph,
                    color,
                );
                x += size;
            }
            Part::Text(text) => {
                let galley = ui
                    .painter()
                    .layout_no_wrap((*text).to_owned(), font.clone(), color);
                let width = galley.size().x;
                ui.painter()
                    .galley(pos2(x, left_center.y - galley.size().y / 2.0), galley, color);
                x += width;
            }
        }
    }
}

pub fn parts_label(ui: &mut Ui, parts: &[Part<'_>], font: egui::FontId, size: f32, color: Color32) {
    let width = parts_width(ui, parts, &font, size);
    let height = ui.fonts_mut(|fonts| fonts.row_height(&font)).max(size);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), egui::Sense::hover());
    paint_parts(
        ui,
        pos2(rect.left(), rect.center().y),
        parts,
        &font,
        size,
        color,
    );
}
