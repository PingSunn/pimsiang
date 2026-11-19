use std::cell::Cell;
use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use egui::{FontData, FontDefinitions, FontFamily, ViewportId};
use egui_wgpu::winit::Painter;
use egui_wgpu::{RendererOptions, WgpuConfiguration};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::theme;

const ANUPHAN: &[u8] = include_bytes!("../assets/Anuphan[wght].ttf");
const MITR: &[u8] = include_bytes!("../assets/Mitr-SemiBold.ttf");
const FONT_NAME: &str = "Anuphan";
const DISPLAY_NAME: &str = "Mitr";
const MIN_WIDTH: f64 = 400.0;
const MIN_HEIGHT: f64 = 360.0;

pub struct Surface {
    window: Arc<Window>,
    context: egui::Context,
    input: egui_winit::State,
    painter: Painter,
    refocus: Cell<bool>,
}

impl Surface {
    pub fn open(
        event_loop: &ActiveEventLoop,
        title: &str,
        size: LogicalSize<f64>,
        resizable: bool,
    ) -> Result<Self> {
        let attributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(size)
            .with_resizable(resizable);
        let attributes = if resizable {
            attributes.with_min_inner_size(LogicalSize::new(MIN_WIDTH, MIN_HEIGHT))
        } else {
            attributes
        };
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("creating the window")?,
        );
        let context = egui::Context::default();
        context.set_fonts(fonts());
        theme::apply(&context);
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
            .map_err(|error| anyhow!("preparing the window for drawing: {error}"))?;
        window.focus_window();
        window.request_redraw();
        Ok(Self {
            window,
            context,
            input,
            painter,
            refocus: Cell::new(false),
        })
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn focus(&self) {
        if self.window.is_minimized() == Some(true) {
            self.window.set_minimized(false);
            self.refocus.set(true);
        }
        self.window.focus_window();
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        {
            self.painter
                .on_window_resized(ViewportId::ROOT, width, height);
        }
    }

    pub fn inject(&mut self, events: Vec<egui::Event>) {
        if events.is_empty() {
            return;
        }
        self.input.egui_input_mut().events.extend(events);
        self.window.request_redraw();
    }

    pub fn popup_open(&self) -> bool {
        egui::Popup::is_any_open(&self.context)
    }

    pub fn forward(&mut self, event: &WindowEvent) {
        if matches!(event, WindowEvent::Occluded(false)) && self.refocus.take() {
            self.window.focus_window();
        }
        let response = self.input.on_window_event(&self.window, event);
        if response.repaint {
            self.window.request_redraw();
        }
    }

    pub fn frame(&mut self, show: impl FnMut(&mut egui::Ui)) -> bool {
        let raw_input = self.input.take_egui_input(&self.window);
        let output = self.context.run_ui(raw_input, show);
        for command in &output.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command
                && let Err(error) = platform_macos::copy_text(text)
            {
                tracing::warn!(%error, "cannot copy to the clipboard");
            }
        }
        self.input
            .handle_platform_output(&self.window, output.platform_output);
        let primitives = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        let mut textures = output.textures_delta;
        let background = theme::palette(self.context.theme()).background;
        let clear = egui::Rgba::from(background).to_array();
        self.painter.paint_and_update_textures(
            ViewportId::ROOT,
            output.pixels_per_point,
            clear,
            &primitives,
            &mut textures,
            Vec::new(),
            &self.window,
        );
        output
            .viewport_output
            .get(&ViewportId::ROOT)
            .is_some_and(|viewport| viewport.repaint_delay.is_zero())
    }
}

pub(crate) fn fonts() -> FontDefinitions {
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
    definitions.font_data.insert(
        DISPLAY_NAME.to_owned(),
        Arc::new(FontData::from_static(MITR)),
    );
    definitions.families.insert(
        FontFamily::Name(theme::DISPLAY_FAMILY.into()),
        vec![DISPLAY_NAME.to_owned(), FONT_NAME.to_owned()],
    );
    definitions
}
