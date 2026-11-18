use anyhow::Result;
use app::onboarding::View;
use egui::{Color32, RichText};
use platform::{Permission, PermissionStatus};
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::egui_surface::Surface;

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
    egui::Frame::default().inner_margin(24.0).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 10.0;
        ui.label(RichText::new(view.heading).size(26.0).strong());
        ui.label(RichText::new(view.intro).size(15.0));
        if !view.note.is_empty() {
            ui.label(
                RichText::new(view.note)
                    .size(14.0)
                    .color(Color32::from_rgb(255, 159, 10)),
            );
        }
        ui.add_space(8.0);
        if let Some(model) = &view.model {
            egui::Frame::group(ui.style())
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(model.title).size(18.0).strong());
                    ui.label(RichText::new(model.explain).size(14.0));
                    if let Some(fraction) = model.fraction {
                        ui.add(egui::ProgressBar::new(fraction));
                    }
                    let color = if model.failed {
                        Color32::from_rgb(255, 69, 58)
                    } else {
                        Color32::from_rgb(255, 159, 10)
                    };
                    ui.label(RichText::new(&model.detail).size(14.0).color(color));
                    if let Some(label) = model.retry
                        && ui.button(RichText::new(label).size(15.0)).clicked()
                    {
                        *retry = true;
                    }
                });
        }
        for row in &view.rows {
            egui::Frame::group(ui.style())
                .inner_margin(14.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(row.title).size(18.0).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(row.status_label).size(14.0).color(
                                match row.status {
                                    PermissionStatus::Granted => Color32::from_rgb(52, 199, 89),
                                    PermissionStatus::NotDetermined => {
                                        Color32::from_rgb(255, 159, 10)
                                    }
                                    PermissionStatus::Denied => Color32::from_rgb(255, 69, 58),
                                },
                            ));
                        });
                    });
                    ui.label(RichText::new(row.why).size(14.0));
                    if let Some(button) = row.button
                        && ui.button(RichText::new(button.label).size(15.0)).clicked()
                    {
                        *grant = Some(button.permission);
                    }
                });
        }
        if !view.footer.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new(view.footer)
                    .size(16.0)
                    .strong()
                    .color(Color32::from_rgb(52, 199, 89)),
            );
        }
    });
}
