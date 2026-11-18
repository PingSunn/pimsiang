use anyhow::Result;
use app::onboarding::View;
use platform::{Permission, PermissionStatus};
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::egui_surface::Surface;
use crate::theme::{self, Badge, Variant};

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
        let surface = Surface::open(event_loop, title, LogicalSize::new(540.0, 760.0), false)?;
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
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(colors.background))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    hero(ui, view);
                    egui::Frame::new()
                        .inner_margin(egui::Margin {
                            left: theme::SPACE_6 as i8,
                            right: theme::SPACE_6 as i8,
                            top: theme::SPACE_2 as i8,
                            bottom: theme::SPACE_6 as i8,
                        })
                        .show(ui, |ui| steps(ui, view, grant, retry));
                });
        });
}

fn hero(ui: &mut egui::Ui, view: &View) {
    let colors = theme::colors(ui);
    let top = ui.cursor().min;
    let width = ui.available_width();
    let background = ui.painter().add(egui::Shape::Noop);
    let frame = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(
            theme::SPACE_8 as i8,
            theme::SPACE_4 as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(width - 2.0 * theme::SPACE_8);
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = theme::SPACE_2;
                theme::mascot::show(ui, theme::Pose::Waving, 112.0);
                ui.add_space(theme::SPACE_1);
                ui.label(theme::display(view.heading, theme::TEXT_TITLE));
                ui.label(
                    egui::RichText::new(theme::WORDMARK_THAI)
                        .size(theme::TEXT_SM)
                        .color(colors.accent_text),
                );
                ui.label(
                    egui::RichText::new(view.intro)
                        .size(theme::TEXT_BASE)
                        .color(colors.muted_foreground),
                );
            });
        });
    let rect =
        egui::Rect::from_min_max(top, egui::pos2(top.x + width, frame.response.rect.bottom()));
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), colors.accent_soft);
    mesh.colored_vertex(rect.right_top(), colors.accent_soft);
    mesh.colored_vertex(rect.right_bottom(), egui::Color32::TRANSPARENT);
    mesh.colored_vertex(rect.left_bottom(), egui::Color32::TRANSPARENT);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().set(background, egui::Shape::mesh(mesh));
}

fn steps(ui: &mut egui::Ui, view: &View, grant: &mut Option<Permission>, retry: &mut bool) {
    let colors = theme::colors(ui);
    ui.spacing_mut().item_spacing.y = theme::SPACE_3;
    if !view.note.is_empty() {
        theme::banner(ui, colors.warning, |ui| theme::warning(ui, view.note));
    }
    let missing = view
        .rows
        .iter()
        .any(|row| row.status != PermissionStatus::Granted);
    let mascot = if missing { 56.0 } else { 0.0 };
    theme::with_control(
        ui,
        mascot,
        |ui| {
            ui.spacing_mut().item_spacing.y = theme::SPACE_3;
            ui.label(theme::medium(&view.progress, theme::TEXT_SM).color(colors.muted_foreground));
            theme::segments(ui, view.steps_done, view.steps_total);
        },
        |ui| {
            if missing {
                theme::mascot::show(ui, theme::Pose::Confused, mascot);
            }
        },
    );
    ui.add_space(theme::SPACE_2);
    let mut number = 0;
    if let Some(model) = &view.model {
        number += 1;
        theme::card(ui, |ui| {
            theme::with_control(
                ui,
                64.0,
                |ui| {
                    ui.horizontal_top(|ui| {
                        theme::step_marker(ui, number, false);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = theme::SPACE_2;
                            theme::heading(ui, model.title);
                            theme::description(ui, model.explain);
                            if let Some(fraction) = model.fraction {
                                ui.add_space(theme::SPACE_1);
                                theme::progress(ui, fraction);
                            }
                            if model.failed {
                                theme::destructive(ui, &model.detail);
                            } else {
                                theme::description(ui, &model.detail);
                            }
                            if let Some(label) = model.retry
                                && theme::button(ui, Variant::Primary, label).clicked()
                            {
                                *retry = true;
                            }
                        });
                    });
                },
                |ui| theme::mascot::show(ui, theme::Pose::Writing, 56.0),
            );
        });
    } else {
        number += 1;
    }
    for row in &view.rows {
        number += 1;
        let granted = row.status == PermissionStatus::Granted;
        theme::card(ui, |ui| {
            let button_width = row
                .button
                .map_or(0.0, |button| theme::button_width(ui, button.label));
            theme::with_control(
                ui,
                button_width,
                |ui| {
                    ui.horizontal_top(|ui| {
                        theme::step_marker(ui, number, granted);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = theme::SPACE_2;
                            ui.spacing_mut().interact_size.y = 0.0;
                            ui.horizontal(|ui| {
                                theme::heading(ui, row.title);
                                let kind = match row.status {
                                    PermissionStatus::Granted => Badge::Success,
                                    PermissionStatus::NotDetermined => Badge::Neutral,
                                    PermissionStatus::Denied => Badge::Destructive,
                                };
                                theme::badge(ui, kind, row.status_label);
                            });
                            theme::description(ui, &row.why);
                        });
                    });
                },
                |ui| {
                    if let Some(button) = row.button
                        && theme::button(ui, Variant::Primary, button.label).clicked()
                    {
                        *grant = Some(button.permission);
                    }
                },
            );
        });
    }
    if !view.footer.is_empty() {
        theme::banner(ui, colors.success, |ui| {
            ui.horizontal(|ui| {
                theme::icons::allocate(ui, theme::Icon::Check, 18.0, colors.success);
                theme::success(ui, &view.footer);
            });
        });
    }
}
