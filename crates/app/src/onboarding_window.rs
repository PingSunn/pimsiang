use anyhow::Result;
use app::onboarding::View;
use platform::{Permission, PermissionStatus};
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::egui_surface::Surface;
use crate::theme::{self, Badge, Marker, Variant};

pub enum Outcome {
    Idle,
    Close,
    Grant(Permission),
    RetryModel,
}

pub struct OnboardingWindow {
    surface: Surface,
}

impl OnboardingWindow {
    pub fn open(event_loop: &ActiveEventLoop, title: &str) -> Result<Self> {
        let surface = Surface::open(event_loop, title, LogicalSize::new(560.0, 800.0), false)?;
        Ok(Self { surface })
    }

    pub fn id(&self) -> WindowId {
        self.surface.id()
    }

    pub fn focus(&self) {
        self.surface.focus();
    }

    pub fn request_redraw(&self) {
        self.surface.request_redraw();
    }

    pub fn handle(&mut self, event: &WindowEvent, view: &View) -> Outcome {
        match event {
            WindowEvent::CloseRequested => return Outcome::Close,
            WindowEvent::Resized(size) => self.surface.resize(*size),
            WindowEvent::RedrawRequested => return self.draw(view),
            _ => {}
        }
        self.surface.forward(event);
        Outcome::Idle
    }

    fn draw(&mut self, view: &View) -> Outcome {
        let mut grant = None;
        let mut retry = false;
        let mut close = false;
        let repaint_now = self
            .surface
            .frame(|ui| show(ui, view, &mut grant, &mut retry, &mut close));
        if repaint_now || grant.is_some() || retry || close {
            self.surface.request_redraw();
        }
        match grant {
            _ if close => Outcome::Close,
            Some(permission) => Outcome::Grant(permission),
            None if retry => Outcome::RetryModel,
            None => Outcome::Idle,
        }
    }
}

const SIDE: f32 = 28.0;
const ROW_PAD: f32 = 16.0;

pub(crate) fn show(
    ui: &mut egui::Ui,
    view: &View,
    grant: &mut Option<Permission>,
    retry: &mut bool,
    close: &mut bool,
) {
    let colors = theme::colors(ui);
    if theme::close_pressed(ui) || theme::escape_pressed(ui) {
        *close = true;
    }
    egui::Panel::bottom("setup-footer")
        .frame(
            egui::Frame::new()
                .fill(colors.background)
                .inner_margin(egui::Margin {
                    left: SIDE as i8,
                    right: SIDE as i8,
                    top: 18,
                    bottom: 22,
                }),
        )
        .show_separator_line(false)
        .show(ui, |ui| footer(ui, view));
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(colors.background))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    hero(ui, view);
                    egui::Frame::new()
                        .inner_margin(egui::Margin {
                            left: SIDE as i8,
                            right: SIDE as i8,
                            top: theme::SPACE_6 as i8,
                            bottom: theme::SPACE_4 as i8,
                        })
                        .show(ui, |ui| steps(ui, view, grant, retry));
                });
        });
}

fn hero_pose(view: &View) -> theme::Pose {
    let denied = view
        .rows
        .iter()
        .any(|row| row.status == PermissionStatus::Denied);
    let permissions_left = view
        .rows
        .iter()
        .any(|row| row.status != PermissionStatus::Granted);
    if denied {
        theme::Pose::Confused
    } else if view.steps_done >= view.steps_total {
        theme::Pose::Default
    } else if !view.model.done && !permissions_left {
        theme::Pose::Writing
    } else {
        theme::Pose::Waving
    }
}

fn hero(ui: &mut egui::Ui, view: &View) {
    let colors = theme::colors(ui);
    let width = ui.available_width();
    let mascot = 150.0;
    let text_left = SIDE + mascot + 4.0;
    let text_width = width - text_left - 40.0;

    let bubble = egui::WidgetText::from(theme::display(view.bubble, 15.0).color(colors.on_wattle))
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Body,
        );
    let mut title = egui::text::LayoutJob::simple(
        view.heading.to_owned(),
        egui::FontId::new(30.0, egui::FontFamily::Name(theme::DISPLAY_FAMILY.into())),
        colors.foreground,
        text_width,
    );
    title.sections[0].format.line_height = Some(39.0);
    let title = ui.fonts_mut(|fonts| fonts.layout_job(title));
    let mut subtitle = egui::text::LayoutJob::simple(
        view.intro.clone(),
        egui::FontId::proportional(theme::TEXT_BASE),
        colors.ink2,
        text_width,
    );
    subtitle.sections[0].format.line_height = Some(22.0);
    let subtitle = ui.fonts_mut(|fonts| fonts.layout_job(subtitle));

    let bubble_size = bubble.size() + egui::vec2(2.0 * 14.0 + 4.0, 2.0 * 6.0 + 4.0);
    let column = bubble_size.y + 6.0 + title.size().y + 6.0 + subtitle.size().y;
    let height = (column + 26.0).max(4.0 + mascot - 8.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, colors.accent_soft);
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        egui::Stroke::new(1.0, theme::mix(colors.accent_soft, colors.foreground, 0.12)),
    );
    let top = rect.bottom() - 26.0 - column;
    let bubble_rect =
        egui::Rect::from_min_size(egui::pos2(rect.left() + text_left, top), bubble_size);
    painter.rect(
        bubble_rect,
        egui::CornerRadius {
            nw: 14,
            ne: 14,
            se: 14,
            sw: 4,
        },
        egui::Color32::WHITE,
        egui::Stroke::new(2.0, colors.on_wattle),
        egui::StrokeKind::Inside,
    );
    painter.galley(
        bubble_rect.center() - bubble.size() / 2.0,
        bubble,
        colors.on_wattle,
    );
    let mut y = bubble_rect.bottom() + 6.0;
    painter.galley(
        egui::pos2(rect.left() + text_left, y),
        title.clone(),
        colors.foreground,
    );
    y += title.size().y + 6.0;
    painter.galley(
        egui::pos2(rect.left() + text_left, y),
        subtitle,
        colors.ink2,
    );
    let mascot_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + SIDE, rect.bottom() + 8.0 - mascot),
        egui::Vec2::splat(mascot),
    );
    theme::mascot::paint(ui, hero_pose(view), mascot_rect);
}

fn footer(ui: &mut egui::Ui, view: &View) {
    let colors = theme::colors(ui);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        egui::Frame::new()
            .fill(colors.card)
            .stroke(egui::Stroke::new(1.0, colors.input))
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(8, 2))
            .show(ui, |ui| {
                theme::keys::parts_label(
                    ui,
                    &[
                        theme::Part::Glyph(crate::settings_window::glyph_of(view.trigger_key)),
                        theme::Part::Text(view.trigger_key.label()),
                    ],
                    egui::FontId::proportional(theme::TEXT_XS),
                    theme::TEXT_XS,
                    colors.foreground,
                );
            });
        ui.label(
            egui::RichText::new(view.hint)
                .size(theme::TEXT_XS)
                .color(colors.muted_foreground),
        );
    });
}

fn steps(ui: &mut egui::Ui, view: &View, grant: &mut Option<Permission>, retry: &mut bool) {
    let colors = theme::colors(ui);
    if !view.note.is_empty() {
        theme::banner(ui, colors.warning, |ui| theme::warning(ui, view.note));
        ui.add_space(theme::SPACE_3);
    }
    ui.horizontal(|ui| {
        ui.label(theme::bold(&view.progress, theme::TEXT_SM).color(colors.muted_foreground));
        theme::row_end(ui, |ui| {
            theme::segments(ui, view.steps_done, view.steps_total)
        });
    });
    ui.add_space(10.0);
    egui::Frame::new()
        .fill(colors.card)
        .stroke(egui::Stroke::new(1.0, colors.border))
        .corner_radius(egui::CornerRadius::same(theme::RADIUS_LG))
        .inner_margin(egui::Margin::symmetric(18, 0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            let first_open = view
                .rows
                .iter()
                .position(|row| row.status != PermissionStatus::Granted);
            for (index, row) in view.rows.iter().enumerate() {
                if index > 0 {
                    theme::divider(ui);
                }
                permission_row(ui, view, index, row, first_open == Some(index), grant);
                if index == 0 {
                    theme::divider(ui);
                    model_row(ui, view, retry);
                }
            }
        });
}

fn permission_row(
    ui: &mut egui::Ui,
    view: &View,
    index: usize,
    row: &app::onboarding::Row,
    first_open: bool,
    grant: &mut Option<Permission>,
) {
    let granted = row.status == PermissionStatus::Granted;
    let number = if index == 0 { 1 } else { index + 2 };
    let marker = if granted {
        Marker::Done
    } else if first_open {
        Marker::Current
    } else {
        Marker::Waiting
    };
    let _ = view;
    let control_width = match (granted, row.button) {
        (true, _) => badge_width(ui, row.status_label),
        (false, Some(button)) => theme::button_width(ui, button.label),
        (false, None) => 0.0,
    };
    ui.add_space(ROW_PAD);
    theme::with_control(
        ui,
        control_width,
        |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                theme::step_marker(ui, number, marker);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.spacing_mut().interact_size.y = 0.0;
                    ui.label(theme::bold(row.title, 15.0));
                    theme::description(ui, &row.why);
                    if row.status == PermissionStatus::Denied {
                        theme::destructive(ui, row.status_label);
                    }
                });
            });
        },
        |ui| {
            if granted {
                theme::badge(ui, Badge::Success, row.status_label);
            } else if let Some(button) = row.button {
                let variant = if first_open {
                    Variant::Primary
                } else {
                    Variant::Outline
                };
                if theme::button(ui, variant, button.label).clicked() {
                    *grant = Some(button.permission);
                }
            }
        },
    );
    ui.add_space(ROW_PAD);
}

fn model_row(ui: &mut egui::Ui, view: &View, retry: &mut bool) {
    let colors = theme::colors(ui);
    let model = &view.model;
    ui.add_space(ROW_PAD);
    let control_width = if model.done {
        badge_width(ui, view.model_done)
    } else {
        0.0
    };
    theme::with_control(
        ui,
        control_width,
        |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                theme::step_marker(
                    ui,
                    2,
                    if model.done {
                        Marker::Done
                    } else {
                        Marker::Current
                    },
                );
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.spacing_mut().interact_size.y = 0.0;
                    ui.label(theme::bold(model.title, 15.0));
                    theme::description(ui, &model.explain);
                    if let Some(fraction) = model.fraction {
                        ui.add_space(6.0);
                        theme::progress(ui, fraction);
                    }
                    if model.failed {
                        ui.add_space(6.0);
                        theme::destructive(ui, &model.detail);
                    } else if !model.detail.is_empty() {
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(&model.detail)
                                .size(theme::TEXT_XS)
                                .color(colors.muted_foreground),
                        );
                    }
                    if let Some(label) = model.retry {
                        ui.add_space(6.0);
                        if theme::button(ui, Variant::Primary, label).clicked() {
                            *retry = true;
                        }
                    }
                });
            });
        },
        |ui| {
            if model.done {
                theme::badge(ui, Badge::Success, view.model_done);
            }
        },
    );
    ui.add_space(ROW_PAD);
}

fn badge_width(ui: &egui::Ui, text: &str) -> f32 {
    let galley = egui::WidgetText::from(theme::bold(text, theme::TEXT_XS)).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Small,
    );
    galley.size().x + 2.0 * theme::SPACE_2 + 2.0
}
