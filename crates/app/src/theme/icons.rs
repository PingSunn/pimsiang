use egui::{Color32, Painter, Pos2, Rect, Stroke, StrokeKind, pos2, vec2};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Waveform,
    Sliders,
    Key,
    Speaker,
    Search,
    Copy,
    Trash,
    Check,
    Warning,
}

pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let half = rect.width().min(rect.height()) / 2.0;
    let center = rect.center();
    let stroke = Stroke::new((half * 2.0 / 16.0 * 1.5).max(1.0), color);
    let at = |x: f32, y: f32| pos2(center.x + x * half, center.y + y * half);
    let path =
        |points: &[(f32, f32)]| -> Vec<Pos2> { points.iter().map(|&(x, y)| at(x, y)).collect() };
    match icon {
        Icon::Waveform => {
            for (x, height) in [
                (-0.8, 0.3),
                (-0.4, 0.7),
                (0.0, 1.0),
                (0.4, 0.55),
                (0.8, 0.25),
            ] {
                painter.line_segment([at(x, -height), at(x, height)], stroke);
                for y in [-height, height] {
                    painter.circle_filled(at(x, y), stroke.width / 2.0, color);
                }
            }
        }
        Icon::Sliders => {
            for (y, knob) in [(-0.6, -0.3), (0.0, 0.45), (0.6, -0.05)] {
                painter.line_segment([at(-0.9, y), at(0.9, y)], stroke);
                painter.circle_filled(at(knob, y), half * 0.27, color);
            }
        }
        Icon::Key => {
            painter.rect_stroke(
                Rect::from_min_max(at(-0.95, -0.65), at(0.95, 0.65)),
                half * 0.25,
                stroke,
                StrokeKind::Middle,
            );
            for x in [-0.5, 0.0, 0.5] {
                painter.circle_filled(at(x, -0.2), stroke.width * 0.55, color);
            }
            painter.line_segment([at(-0.45, 0.28), at(0.45, 0.28)], stroke);
        }
        Icon::Speaker => {
            painter.add(egui::Shape::closed_line(
                path(&[
                    (-0.9, -0.3),
                    (-0.5, -0.3),
                    (-0.05, -0.75),
                    (-0.05, 0.75),
                    (-0.5, 0.3),
                    (-0.9, 0.3),
                ]),
                stroke,
            ));
            for radius in [0.4_f32, 0.8] {
                let arc: Vec<Pos2> = (-4..=4)
                    .map(|step| {
                        let angle = step as f32 * 0.17;
                        at(0.05 + radius * angle.cos(), radius * angle.sin())
                    })
                    .collect();
                painter.add(egui::Shape::line(arc, stroke));
            }
        }
        Icon::Search => {
            painter.circle_stroke(at(-0.15, -0.15), half * 0.62, stroke);
            painter.line_segment([at(0.32, 0.32), at(0.85, 0.85)], stroke);
        }
        Icon::Copy => {
            painter.rect_stroke(
                Rect::from_min_max(at(-0.85, -0.35), at(0.35, 0.85)),
                half * 0.2,
                stroke,
                StrokeKind::Middle,
            );
            painter.add(egui::Shape::line(
                path(&[
                    (-0.4, -0.35),
                    (-0.4, -0.85),
                    (0.85, -0.85),
                    (0.85, 0.4),
                    (0.35, 0.4),
                ]),
                stroke,
            ));
        }
        Icon::Trash => {
            painter.line_segment([at(-0.85, -0.5), at(0.85, -0.5)], stroke);
            painter.add(egui::Shape::line(
                path(&[(-0.3, -0.5), (-0.3, -0.85), (0.3, -0.85), (0.3, -0.5)]),
                stroke,
            ));
            painter.add(egui::Shape::line(
                path(&[(-0.6, -0.5), (-0.5, 0.85), (0.5, 0.85), (0.6, -0.5)]),
                stroke,
            ));
        }
        Icon::Check => {
            painter.add(egui::Shape::line(
                path(&[(-0.7, 0.0), (-0.2, 0.55), (0.75, -0.5)]),
                Stroke::new(stroke.width * 1.15, color),
            ));
        }
        Icon::Warning => {
            painter.add(egui::Shape::closed_line(
                path(&[(0.0, -0.85), (0.9, 0.7), (-0.9, 0.7)]),
                stroke,
            ));
            painter.line_segment([at(0.0, -0.25), at(0.0, 0.2)], stroke);
            painter.circle_filled(at(0.0, 0.46), stroke.width * 0.6, color);
        }
    }
}

pub fn allocate(ui: &mut egui::Ui, icon: Icon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    paint(ui.painter(), rect, icon, color);
}
