use anyhow::Result;
use app::history_view::{PAGE_SIZE, format_created};
use app::i18n::Language;
use store::{Entry, Kind};
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::egui_surface::Surface;
use crate::theme::{self, Badge, Icon, Tone, Variant};

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
    focus_search: bool,
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
            focus_search: true,
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
            focus_search: &mut self.focus_search,
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

pub(crate) struct State<'a> {
    pub(crate) query: &'a mut String,
    pub(crate) limit: &'a mut usize,
    pub(crate) confirming_clear: &'a mut bool,
    pub(crate) copied: &'a mut Option<i64>,
    pub(crate) focus_search: &'a mut bool,
}

const SEARCH_ID: &str = "history-search";

pub(crate) fn show(
    ui: &mut egui::Ui,
    language: Language,
    state: &mut State<'_>,
    entries: &[Entry],
    loaded: bool,
    unavailable: bool,
    outcome: &mut Outcome,
) {
    let colors = theme::colors(ui);
    let search_id = egui::Id::unique(SEARCH_ID);
    if theme::close_pressed(ui) {
        *outcome = Outcome::Close;
    } else if theme::escape_pressed(ui) {
        if *state.confirming_clear {
            *state.confirming_clear = false;
        } else if !state.query.is_empty() {
            state.query.clear();
            *state.limit = PAGE_SIZE;
            *outcome = Outcome::Refresh;
        }
    }
    if theme::find_pressed(ui) {
        *state.focus_search = true;
    }
    if *state.focus_search && !unavailable {
        ui.memory_mut(|memory| memory.request_focus(search_id));
        *state.focus_search = false;
    }
    egui::Panel::top("history-header")
        .frame(egui::Frame::new().fill(colors.background))
        .show_separator_line(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            theme::header(ui, |ui| {
                ui.label(
                    theme::medium(language.history_heading(), theme::TEXT_BASE)
                        .color(colors.muted_foreground),
                );
            });
            if unavailable {
                return;
            }
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(
                    theme::SPACE_5 as i8,
                    theme::SPACE_3 as i8,
                ))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let clear_width = 124.0;
                        let width =
                            ui.available_width() - clear_width - ui.spacing().item_spacing.x;
                        let search = theme::text_input(
                            ui,
                            search_id,
                            state.query,
                            language.history_search_hint(),
                            width,
                        );
                        if search.changed() {
                            *state.limit = PAGE_SIZE;
                            *state.confirming_clear = false;
                            *state.copied = None;
                            *outcome = Outcome::Refresh;
                        }
                        let can_clear = !entries.is_empty() && !*state.confirming_clear;
                        theme::row_end(ui, |ui| {
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
                    });
                });
            if *state.confirming_clear {
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: theme::SPACE_5 as i8,
                        right: theme::SPACE_5 as i8,
                        top: 0,
                        bottom: theme::SPACE_3 as i8,
                    })
                    .show(ui, |ui| confirm_clear(ui, language, state, outcome));
            }
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(colors.background))
        .show(ui, |ui| {
            if unavailable {
                egui::Frame::new()
                    .inner_margin(theme::SPACE_5)
                    .show(ui, |ui| {
                        theme::banner(ui, colors.warning, |ui| {
                            theme::warning(ui, language.history_unavailable());
                        });
                    });
                return;
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(
                            theme::SPACE_5 as i8,
                            theme::SPACE_1 as i8,
                        ))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = theme::SPACE_2;
                            if loaded && entries.is_empty() {
                                empty(ui, language, state.query.trim().is_empty());
                            }
                            for entry in entries {
                                ui.push_id(entry.id, |ui| {
                                    show_entry(ui, language, state, entry, outcome)
                                });
                            }
                            if entries.len() >= *state.limit {
                                ui.add_space(theme::SPACE_1);
                                ui.vertical_centered(|ui| {
                                    if theme::button(
                                        ui,
                                        Variant::Secondary,
                                        language.history_show_more(),
                                    )
                                    .clicked()
                                    {
                                        *state.limit += PAGE_SIZE;
                                        *outcome = Outcome::Refresh;
                                    }
                                });
                            }
                            ui.add_space(theme::SPACE_3);
                        });
                });
        });
}

fn confirm_clear(
    ui: &mut egui::Ui,
    language: Language,
    state: &mut State<'_>,
    outcome: &mut Outcome,
) {
    let colors = theme::colors(ui);
    theme::banner(ui, colors.destructive, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme::SPACE_2;
            theme::icons::allocate(ui, Icon::Warning, 16.0, colors.destructive);
            ui.label(theme::medium(
                language.history_confirm_clear(),
                theme::TEXT_BASE,
            ));
        });
        ui.horizontal(|ui| {
            if theme::button(ui, Variant::Destructive, language.history_confirm_yes()).clicked() {
                *state.confirming_clear = false;
                *outcome = Outcome::Clear;
            }
            if theme::button(ui, Variant::Ghost, language.history_cancel()).clicked() {
                *state.confirming_clear = false;
            }
        });
    });
}

fn empty(ui: &mut egui::Ui, language: Language, no_query: bool) {
    let colors = theme::colors(ui);
    let message = if no_query {
        language.history_empty()
    } else {
        language.history_no_match()
    };
    ui.add_space(theme::SPACE_8 * 1.5);
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = theme::SPACE_4;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(72.0, 72.0), egui::Sense::hover());
        let painter = ui.painter();
        painter.circle_filled(rect.center(), 36.0, colors.accent_soft);
        painter.circle_stroke(
            rect.center(),
            36.0,
            egui::Stroke::new(1.0, theme::tint(colors.accent, 0x44)),
        );
        let icon = if no_query {
            Icon::Waveform
        } else {
            Icon::Search
        };
        theme::icons::paint(
            painter,
            egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(30.0)),
            icon,
            colors.accent_text,
        );
        ui.scope(|ui| {
            ui.set_max_width(320.0);
            ui.label(
                egui::RichText::new(message)
                    .size(theme::TEXT_BASE)
                    .color(colors.muted_foreground),
            );
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
    let colors = theme::colors(ui);
    let background = ui.painter().add(egui::Shape::Noop);
    let radius = egui::CornerRadius::same(theme::RADIUS_LG);
    let frame = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(
            theme::SPACE_4 as i8,
            theme::SPACE_3 as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = theme::SPACE_2;
            ui.label(egui::RichText::new(&entry.text).size(theme::TEXT_LG));
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 28.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = theme::SPACE_2;
                    theme::description(ui, &format_created(entry.created));
                    let kind = match entry.kind {
                        Kind::Pasted => Badge::Success,
                        Kind::Note => Badge::Warning,
                    };
                    theme::badge(ui, kind, language.history_kind(entry.kind));
                    theme::row_end(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = theme::SPACE_1;
                        if theme::action(ui, Icon::Trash, language.history_delete(), Tone::Danger)
                            .clicked()
                        {
                            *outcome = Outcome::Delete(entry.id);
                        }
                        let copied = *state.copied == Some(entry.id);
                        let (icon, label, tone) = if copied {
                            (Icon::Check, language.history_copied(), Tone::Success)
                        } else {
                            (Icon::Copy, language.history_copy(), Tone::Neutral)
                        };
                        if theme::action(ui, icon, label, tone).clicked() {
                            *state.copied = Some(entry.id);
                            *outcome = Outcome::Copy(entry.text.clone());
                        }
                    });
                },
            );
        });
    let rect = frame.response.rect;
    let hovered = ui.rect_contains_pointer(rect);
    let (fill, stroke) = if hovered {
        (
            colors.card_hover,
            egui::Stroke::new(1.0, theme::tint(colors.accent, 0x80)),
        )
    } else {
        (colors.card, egui::Stroke::new(1.0, colors.border))
    };
    ui.painter().set(
        background,
        egui::epaint::RectShape::new(rect, radius, fill, stroke, egui::StrokeKind::Inside),
    );
}
