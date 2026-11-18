use anyhow::Result;
use app::history_view::{PAGE_SIZE, format_created};
use app::i18n::Language;
use store::{Entry, Kind};
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::egui_surface::Surface;
use crate::theme::{self, Badge, Variant};

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

fn show(
    ui: &mut egui::Ui,
    language: Language,
    state: &mut State<'_>,
    entries: &[Entry],
    loaded: bool,
    unavailable: bool,
    outcome: &mut Outcome,
) {
    egui::Frame::new()
        .inner_margin(theme::SPACE_4)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = theme::SPACE_3;
            if unavailable {
                theme::warning(ui, language.history_unavailable());
                return;
            }
            ui.horizontal(|ui| {
                let clear_width = 120.0;
                let width = ui.available_width() - clear_width - ui.spacing().item_spacing.x;
                let search = theme::text_input(
                    ui,
                    egui::Id::unique("history-search"),
                    state.query,
                    language.history_search_hint(),
                    width,
                );
                if search.changed() {
                    *state.limit = PAGE_SIZE;
                    *state.confirming_clear = false;
                    *outcome = Outcome::Refresh;
                }
                let can_clear = !entries.is_empty() && !*state.confirming_clear;
                if theme::button_enabled(
                    ui,
                    Variant::Outline,
                    can_clear,
                    language.history_clear_all(),
                )
                .clicked()
                {
                    *state.confirming_clear = true;
                }
            });
            if *state.confirming_clear {
                theme::card(ui, |ui| {
                    theme::warning(ui, language.history_confirm_clear());
                    ui.horizontal(|ui| {
                        if theme::button(ui, Variant::Destructive, language.history_confirm_yes())
                            .clicked()
                        {
                            *state.confirming_clear = false;
                            *outcome = Outcome::Clear;
                        }
                        if theme::button(ui, Variant::Ghost, language.history_cancel()).clicked() {
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
                ui.add_space(theme::SPACE_6);
                theme::description(ui, message);
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
                        && theme::button(ui, Variant::Secondary, language.history_show_more())
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
    theme::card(ui, |ui| {
        ui.horizontal(|ui| {
            theme::description(ui, &format_created(entry.created));
            let kind = match entry.kind {
                Kind::Pasted => Badge::Success,
                Kind::Note => Badge::Warning,
            };
            theme::badge(ui, kind, language.history_kind(entry.kind));
            theme::row_end(ui, |ui| {
                if theme::icon_button(ui, language.history_delete()).clicked() {
                    *outcome = Outcome::Delete(entry.id);
                }
                let label = if *state.copied == Some(entry.id) {
                    language.history_copied()
                } else {
                    language.history_copy()
                };
                if theme::icon_button(ui, label).clicked() {
                    *state.copied = Some(entry.id);
                    *outcome = Outcome::Copy(entry.text.clone());
                }
            });
        });
        theme::label(ui, &entry.text);
    });
}
