use egui::{Color32, Painter, Pos2, Rect, Stroke, StrokeKind, pos2, vec2};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Sliders,
    Key,
    Clipboard,
    Clock,
    Search,
    Copy,
    Trash,
    Check,
    Warning,
}

pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    paint_on(painter, rect, icon, color, Color32::TRANSPARENT);
}

pub fn paint_on(painter: &Painter, rect: Rect, icon: Icon, color: Color32, background: Color32) {
    let half = rect.width().min(rect.height()) / 2.0;
    let center = rect.center();
    let stroke = Stroke::new((half * 2.0 / 16.0 * 1.5).max(1.0), color);
    let at = |x: f32, y: f32| pos2(center.x + x * half, center.y + y * half);
    let path =
        |points: &[(f32, f32)]| -> Vec<Pos2> { points.iter().map(|&(x, y)| at(x, y)).collect() };
    match icon {
        Icon::Sliders => {
            let d = |x: f32, y: f32| at((x - 9.0) / 9.0, (y - 9.0) / 9.0);
            for y in [5.0, 9.0, 13.0] {
                painter.line_segment([d(3.0, y), d(15.0, y)], stroke);
            }
            for (x, y) in [(6.0, 5.0), (12.0, 9.0), (8.0, 13.0)] {
                painter.circle(d(x, y), half * 1.6 / 9.0, background, stroke);
            }
        }
        Icon::Key => {
            let d = |x: f32, y: f32| at((x - 9.0) / 9.0, (y - 9.0) / 9.0);
            painter.rect_stroke(
                Rect::from_min_max(d(2.0, 4.5), d(16.0, 13.5)),
                half * 2.0 / 9.0,
                stroke,
                StrokeKind::Middle,
            );
            painter.line_segment([d(5.0, 11.0), d(13.0, 11.0)], stroke);
        }
        Icon::Clipboard => {
            let d = |x: f32, y: f32| at((x - 9.0) / 9.0, (y - 9.0) / 9.0);
            painter.rect_stroke(
                Rect::from_min_max(d(4.0, 3.0), d(14.0, 16.0)),
                half * 2.0 / 9.0,
                stroke,
                StrokeKind::Middle,
            );
            painter.add(egui::Shape::line(
                vec![d(7.0, 3.0), d(7.0, 2.0), d(11.0, 2.0), d(11.0, 3.0)],
                stroke,
            ));
        }
        Icon::Clock => {
            let d = |x: f32, y: f32| at((x - 9.0) / 9.0, (y - 9.0) / 9.0);
            painter.circle_stroke(d(9.0, 9.0), half * 6.5 / 9.0, stroke);
            painter.add(egui::Shape::line(
                vec![d(9.0, 5.5), d(9.0, 9.0), d(11.5, 10.5)],
                stroke,
            ));
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
