use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use app::onboarding::View;
use egui::{Color32, FontData, FontDefinitions, FontFamily, RichText, ViewportId};
use egui_wgpu::winit::Painter;
use egui_wgpu::{RendererOptions, WgpuConfiguration};
use platform::{Permission, PermissionStatus};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

const ANUPHAN: &[u8] = include_bytes!("../assets/Anuphan[wght].ttf");
const FONT_NAME: &str = "Anuphan";

pub enum Outcome {
    Idle,
    Close,
    Grant(Permission),
}

pub struct OnboardingWindow {
    window: Arc<Window>,
    context: egui::Context,
    input: egui_winit::State,
    painter: Painter,
}

impl OnboardingWindow {
    pub fn open(event_loop: &ActiveEventLoop, title: &str) -> Result<Self> {
        let attributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(LogicalSize::new(540.0, 560.0))
            .with_resizable(false);
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("creating the setup window")?,
        );
        let context = egui::Context::default();
        context.set_fonts(fonts());
        let input = egui_winit::State::new(
            context.clone(),
            ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );
        let mut painter = pollster::block_on(Painter::new(
            context.clone(),
            WgpuConfiguration::default(),
            false,
            RendererOptions::default(),
        ));
        pollster::block_on(painter.set_window(ViewportId::ROOT, Some(window.clone())))
            .map_err(|error| anyhow!("preparing the setup window for drawing: {error}"))?;
        window.focus_window();
        window.request_redraw();
        Ok(Self {
            window,
            context,
            input,
            painter,
        })
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn focus(&self) {
        self.window.focus_window();
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn handle(&mut self, event: &WindowEvent, view: &View) -> Outcome {
        match event {
            WindowEvent::CloseRequested => return Outcome::Close,
            WindowEvent::Resized(size) => self.resize(*size),
            WindowEvent::RedrawRequested => return self.draw(view),
            _ => {}
        }
        let response = self.input.on_window_event(&self.window, event);
        if response.repaint {
            self.window.request_redraw();
        }
        Outcome::Idle
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let (Some(width), Some(height)) = (
            std::num::NonZeroU32::new(size.width),
            std::num::NonZeroU32::new(size.height),
        ) {
            self.painter
                .on_window_resized(ViewportId::ROOT, width, height);
        }
    }

    fn draw(&mut self, view: &View) -> Outcome {
        let raw_input = self.input.take_egui_input(&self.window);
        let mut grant = None;
        let output = self
            .context
            .run_ui(raw_input, |ui| show(ui, view, &mut grant));
        self.input
            .handle_platform_output(&self.window, output.platform_output);
        let primitives = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        let mut textures = output.textures_delta;
        self.painter.paint_and_update_textures(
            ViewportId::ROOT,
            output.pixels_per_point,
            [0.1, 0.1, 0.1, 1.0],
            &primitives,
            &mut textures,
            Vec::new(),
            &self.window,
        );
        let repaint_now = output
            .viewport_output
            .get(&ViewportId::ROOT)
            .is_some_and(|viewport| viewport.repaint_delay.is_zero());
        if repaint_now || grant.is_some() {
            self.window.request_redraw();
        }
        match grant {
            Some(permission) => Outcome::Grant(permission),
            None => Outcome::Idle,
        }
    }
}

fn fonts() -> FontDefinitions {
    let mut definitions = FontDefinitions::default();
    definitions.font_data.insert(
        FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(ANUPHAN)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        definitions
            .families
            .entry(family)
            .or_default()
            .insert(0, FONT_NAME.to_owned());
    }
    definitions
}

fn show(ui: &mut egui::Ui, view: &View, grant: &mut Option<Permission>) {
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
