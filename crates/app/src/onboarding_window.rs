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
        let surface = Surface::open(event_loop, title, LogicalSize::new(540.0, 680.0), false)?;
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
        let repaint_now = self
            .surface
            .frame(|ui| show(ui, view, &mut grant, &mut retry));
        if repaint_now || grant.is_some() || retry {
            self.surface.request_redraw();
        }
        match grant {
            Some(permission) => Outcome::Grant(permission),
            None if retry => Outcome::RetryModel,
            None => Outcome::Idle,
        }
    }
}

fn show(ui: &mut egui::Ui, view: &View, grant: &mut Option<Permission>, retry: &mut bool) {
    theme::page(ui, |ui| {
        ui.spacing_mut().item_spacing.y = theme::SPACE_3;
        theme::title(ui, view.heading);
        theme::description(ui, view.intro);
        if !view.note.is_empty() {
            theme::warning(ui, view.note);
        }
        ui.add_space(theme::SPACE_1);
        if let Some(model) = &view.model {
            theme::card(ui, |ui| {
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
        }
        for row in &view.rows {
            theme::card(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::heading(ui, row.title);
                    theme::row_end(ui, |ui| {
                        let kind = match row.status {
                            PermissionStatus::Granted => Badge::Success,
                            PermissionStatus::NotDetermined => Badge::Neutral,
                            PermissionStatus::Denied => Badge::Destructive,
                        };
                        theme::badge(ui, kind, row.status_label);
                    });
                });
                theme::description(ui, &row.why);
                if let Some(button) = row.button {
                    ui.add_space(theme::SPACE_1);
                    let variant = match row.status {
                        PermissionStatus::Granted => Variant::Outline,
                        _ => Variant::Primary,
                    };
                    if theme::button(ui, variant, button.label).clicked() {
                        *grant = Some(button.permission);
                    }
                }
            });
        }
        if !view.footer.is_empty() {
            ui.add_space(theme::SPACE_2);
            theme::success(ui, &view.footer);
        }
    });
}
