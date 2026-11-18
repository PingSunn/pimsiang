use anyhow::Result;
use app::history_view::{PAGE_SIZE, format_created};
use app::i18n::Language;
use egui::{Color32, RichText};
use store::{Entry, Kind};
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::egui_surface::Surface;

pub enum Outcome {
    Idle,
    Close,
    Refresh,
    Copy(String),
    Delete(i64),
    Clear,
}

pub struct HistoryWindow {
    surface: Surface,
    language: Language,
    query: String,
    limit: usize,
    entries: Vec<Entry>,
    loaded: bool,
    confirming_clear: bool,
    copied: Option<i64>,
    unavailable: bool,
}

impl HistoryWindow {
    pub fn open(
        event_loop: &ActiveEventLoop,
        language: Language,
        unavailable: bool,
    ) -> Result<Self> {
        let surface = Surface::open(
            event_loop,
            language.history_title(),
            LogicalSize::new(560.0, 640.0),
            true,
        )?;
        Ok(Self {
            surface,
            language,
            query: String::new(),
            limit: PAGE_SIZE,
            entries: Vec::new(),
            loaded: false,
            confirming_clear: false,
            copied: None,
            unavailable,
        })
    }

    pub fn id(&self) -> WindowId {
        self.surface.id()
    }

    pub fn focus(&self) {
        self.surface.focus();
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn set_entries(&mut self, entries: Vec<Entry>) {
        self.entries = entries;
        self.loaded = true;
        self.confirming_clear = self.confirming_clear && !self.entries.is_empty();
        self.surface.request_redraw();
    }

    pub fn handle(&mut self, event: &WindowEvent) -> Outcome {
        match event {
            WindowEvent::CloseRequested => return Outcome::Close,
            WindowEvent::Resized(size) => self.surface.resize(*size),
            WindowEvent::RedrawRequested => return self.draw(),
            _ => {}
        }
        self.surface.forward(event);
        Outcome::Idle
    }

    fn draw(&mut self) -> Outcome {
        let mut outcome = Outcome::Idle;
        let language = self.language;
        let mut state = State {
            query: &mut self.query,
            limit: &mut self.limit,
            confirming_clear: &mut self.confirming_clear,
            copied: &mut self.copied,
        };
        let entries = &self.entries;
        let loaded = self.loaded;
        let unavailable = self.unavailable;
        let repaint_now = self.surface.frame(|ui| {
            show(
                ui,
                language,
                &mut state,
                entries,
                loaded,
                unavailable,
                &mut outcome,
            );
        });
        if repaint_now || !matches!(outcome, Outcome::Idle) {
            self.surface.request_redraw();
        }
        outcome
    }
}

struct State<'a> {
    query: &'a mut String,
    limit: &'a mut usize,
    confirming_clear: &'a mut bool,
    copied: &'a mut Option<i64>,
}

const MUTED: Color32 = Color32::from_rgb(152, 152, 157);
const WARNING: Color32 = Color32::from_rgb(255, 159, 10);

fn show(
    ui: &mut egui::Ui,
    language: Language,
    state: &mut State<'_>,
    entries: &[Entry],
    loaded: bool,
    unavailable: bool,
    outcome: &mut Outcome,
) {
    egui::Frame::default().inner_margin(16.0).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        if unavailable {
            ui.label(
                RichText::new(language.history_unavailable())
                    .size(15.0)
                    .color(WARNING),
            );
            return;
        }
        ui.horizontal(|ui| {
            let clear_width = 120.0;
            let search = ui.add(
                egui::TextEdit::singleline(state.query)
                    .hint_text(language.history_search_hint())
                    .font(egui::FontId::proportional(16.0))
                    .desired_width(ui.available_width() - clear_width),
            );
            if search.changed() {
                *state.limit = PAGE_SIZE;
                *state.confirming_clear = false;
                *outcome = Outcome::Refresh;
            }
            let can_clear = !entries.is_empty() && !*state.confirming_clear;
            if ui
                .add_enabled(
                    can_clear,
                    egui::Button::new(RichText::new(language.history_clear_all()).size(14.0)),
                )
                .clicked()
            {
                *state.confirming_clear = true;
            }
        });
        if *state.confirming_clear {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(language.history_confirm_clear())
                        .size(15.0)
                        .color(WARNING),
                );
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new(language.history_confirm_yes()).size(14.0))
                        .clicked()
                    {
                        *state.confirming_clear = false;
                        *outcome = Outcome::Clear;
                    }
                    if ui
                        .button(RichText::new(language.history_cancel()).size(14.0))
                        .clicked()
                    {
                        *state.confirming_clear = false;
                    }
                });
            });
        }
        if loaded && entries.is_empty() {
            let message = if state.query.trim().is_empty() {
                language.history_empty()
            } else {
                language.history_no_match()
            };
            ui.add_space(24.0);
            ui.label(RichText::new(message).size(15.0).color(MUTED));
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in entries {
                    ui.push_id(entry.id, |ui| {
                        show_entry(ui, language, state, entry, outcome)
                    });
                }
                if entries.len() >= *state.limit
                    && ui
                        .button(RichText::new(language.history_show_more()).size(14.0))
                        .clicked()
                {
                    *state.limit += PAGE_SIZE;
                    *outcome = Outcome::Refresh;
                }
            });
    });
}

fn show_entry(
    ui: &mut egui::Ui,
    language: Language,
    state: &mut State<'_>,
    entry: &Entry,
    outcome: &mut Outcome,
) {
    egui::Frame::group(ui.style())
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format_created(entry.created))
                        .size(13.0)
                        .color(MUTED),
                );
                let color = match entry.kind {
                    Kind::Pasted => Color32::from_rgb(52, 199, 89),
                    Kind::Note => WARNING,
                };
                ui.label(
                    RichText::new(language.history_kind(entry.kind))
                        .size(13.0)
                        .color(color),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(RichText::new(language.history_delete()).size(13.0))
                        .clicked()
                    {
                        *outcome = Outcome::Delete(entry.id);
                    }
                    let label = if *state.copied == Some(entry.id) {
                        language.history_copied()
                    } else {
                        language.history_copy()
                    };
                    if ui.button(RichText::new(label).size(13.0)).clicked() {
                        *state.copied = Some(entry.id);
                        *outcome = Outcome::Copy(entry.text.clone());
                    }
                });
            });
            ui.label(RichText::new(&entry.text).size(16.0));
        });
}
