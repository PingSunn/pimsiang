use anyhow::Result;
use app::app_menu::EditCommand;
use app::history_view::{self, PAGE_SIZE, Page};
use app::i18n::Language;
use chrono::{FixedOffset, Local};
use platform_macos::read_text;
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
    page: Page,
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
            LogicalSize::new(640.0, 720.0),
            true,
        )?;
        Ok(Self {
            surface,
            language,
            query: String::new(),
            limit: PAGE_SIZE,
            page: Page {
                entries: Vec::new(),
                matched: 0,
                total: 0,
            },
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

    pub fn edit(&mut self, command: EditCommand) {
        let clipboard = if command.needs_clipboard() {
            read_text()
        } else {
            None
        };
        self.surface.inject(command.events(clipboard.as_deref()));
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn set_page(&mut self, page: Page) {
        self.page = page;
        self.loaded = true;
        self.confirming_clear = self.confirming_clear && !self.page.entries.is_empty();
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
            clock: Clock::now(),
        };
        let page = &self.page;
        let loaded = self.loaded;
        let unavailable = self.unavailable;
        let repaint_now = self.surface.frame(|ui| {
            show(
                ui,
                language,
                &mut state,
                page,
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

#[derive(Clone, Copy)]
pub(crate) struct Clock {
    pub(crate) now: i64,
    pub(crate) zone: Zone,
}

#[derive(Clone, Copy)]
pub(crate) enum Zone {
    Local,
    #[cfg_attr(not(test), allow(dead_code))]
    Fixed(FixedOffset),
}

impl Clock {
    fn now() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs() as i64)
            .unwrap_or(0);
        Self {
            now,
            zone: Zone::Local,
        }
    }

    fn groups<'a>(&self, entries: &'a [Entry]) -> Vec<history_view::Group<'a>> {
        match self.zone {
            Zone::Local => history_view::group_by_day(entries, self.now, Local),
            Zone::Fixed(offset) => history_view::group_by_day(entries, self.now, offset),
        }
    }

    fn year(&self) -> i32 {
        match self.zone {
            Zone::Local => history_view::year_of(self.now, Local),
            Zone::Fixed(offset) => history_view::year_of(self.now, offset),
        }
    }

    fn time(&self, created: i64) -> String {
        match self.zone {
            Zone::Local => history_view::format_time(created, Local),
            Zone::Fixed(offset) => history_view::format_time(created, offset),
        }
    }
}

pub(crate) struct State<'a> {
    pub(crate) query: &'a mut String,
    pub(crate) limit: &'a mut usize,
    pub(crate) confirming_clear: &'a mut bool,
    pub(crate) copied: &'a mut Option<i64>,
    pub(crate) focus_search: &'a mut bool,
    pub(crate) clock: Clock,
}

const SEARCH_ID: &str = "history-search";

pub(crate) fn show(
    ui: &mut egui::Ui,
    language: Language,
    state: &mut State<'_>,
    page: &Page,
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
    let header = egui::Panel::top("history-header")
        .frame(
            egui::Frame::new()
                .fill(colors.sidebar)
                .inner_margin(egui::Margin {
                    left: theme::SPACE_5 as i8,
                    right: theme::SPACE_5 as i8,
                    top: 14,
                    bottom: 16,
                }),
        )
        .show_separator_line(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 14.0;
            ui.horizontal(|ui| {
                theme::row_end(ui, |ui| {
                    ui.label(
                        egui::RichText::new(language.history_private())
                            .size(theme::TEXT_XS)
                            .color(colors.muted_foreground),
                    );
                });
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme::SPACE_3;
                let mut title = egui::text::LayoutJob::simple_singleline(
                    language.history_heading().to_owned(),
                    egui::FontId::new(24.0, egui::FontFamily::Name(theme::DISPLAY_FAMILY.into())),
                    colors.foreground,
                );
                title.sections[0].format.line_height = Some(31.0);
                ui.label(title);
                if unavailable {
                    return;
                }
                let clear_label = language.history_clear_all();
                let clear_width = theme::button_width(ui, clear_label) + theme::SPACE_2;
                let width = ui.available_width() - clear_width - theme::SPACE_3;
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
                let can_clear = !page.entries.is_empty() && !*state.confirming_clear;
                theme::row_end(ui, |ui| {
                    if theme::button_sized(ui, Variant::Outline, can_clear, clear_label, 40.0, 12)
                        .clicked()
                    {
                        *state.confirming_clear = true;
                    }
                });
            });
            if *state.confirming_clear && !unavailable {
                confirm_clear(ui, language, state, outcome);
            }
        });
    ui.painter().hline(
        header.response.rect.x_range(),
        header.response.rect.bottom() - 0.5,
        egui::Stroke::new(1.0, colors.border),
    );
    if !unavailable {
        let footer = egui::Panel::bottom("history-footer")
            .frame(
                egui::Frame::new()
                    .fill(colors.background)
                    .inner_margin(egui::Margin::symmetric(theme::SPACE_5 as i8, 12)),
            )
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let text = if state.query.trim().is_empty() {
                        language.history_footer_total(page.total)
                    } else {
                        language.history_footer_found(page.matched, page.total)
                    };
                    ui.label(
                        egui::RichText::new(text)
                            .size(theme::TEXT_XS)
                            .color(colors.muted_foreground),
                    );
                    theme::row_end(ui, |ui| {
                        ui.label(
                            egui::RichText::new(language.history_footer_hint())
                                .size(theme::TEXT_XS)
                                .color(colors.muted_foreground),
                        );
                    });
                });
            });
        ui.painter().hline(
            footer.response.rect.x_range(),
            footer.response.rect.top() + 0.5,
            egui::Stroke::new(1.0, colors.border),
        );
    }
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(colors.background))
        .show(ui, |ui| {
            if unavailable {
                egui::Frame::new()
                    .inner_margin(theme::SPACE_5)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            theme::mascot::show(ui, theme::Pose::Oops, 120.0);
                        });
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
                        .inner_margin(egui::Margin {
                            left: theme::SPACE_5 as i8,
                            right: theme::SPACE_5 as i8,
                            top: 6,
                            bottom: 20,
                        })
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = theme::SPACE_1;
                            if loaded && page.entries.is_empty() {
                                empty(ui, language, state.query.trim().is_empty());
                            }
                            let groups = state.clock.groups(&page.entries);
                            let year = state.clock.year();
                            for (index, group) in groups.iter().enumerate() {
                                ui.add_space(if index == 0 { 12.0 } else { 14.0 });
                                ui.label(
                                    theme::bold(
                                        &language.history_day(group.day, year),
                                        theme::TEXT_XS,
                                    )
                                    .color(colors.muted_foreground),
                                );
                                ui.add_space(theme::SPACE_1);
                                for entry in group.entries {
                                    ui.push_id(entry.id, |ui| {
                                        show_entry(ui, language, state, entry, outcome)
                                    });
                                }
                            }
                            if page.entries.len() >= *state.limit
                                && page.matched > page.entries.len()
                            {
                                ui.add_space(theme::SPACE_3);
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
        let pose = if no_query {
            theme::Pose::Sleeping
        } else {
            theme::Pose::Searching
        };
        theme::mascot::show(ui, pose, 120.0);
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
    let id = ui.scope_id();
    let copy_id = id.with("copy");
    let delete_id = id.with("delete");
    let rect_id = id.with("row");
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let hovered = ui
        .data(|data| data.get_temp::<egui::Rect>(rect_id))
        .zip(pointer)
        .is_some_and(|(rect, pointer)| rect.contains(pointer));
    let focused = ui.memory(|memory| memory.has_focus(copy_id) || memory.has_focus(delete_id));
    let active = hovered || focused;
    let background = ui.painter().add(egui::Shape::Noop);
    let radius = egui::CornerRadius::same(theme::RADIUS_LG);
    let ranges = history_view::match_ranges(&entry.text, state.query);
    let frame = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(theme::SPACE_4 as i8, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 6.0;
            theme::highlighted(ui, &entry.text, &ranges, 15.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 30.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let muted = colors.muted_foreground;
                    ui.label(
                        egui::RichText::new(state.clock.time(entry.created))
                            .size(theme::TEXT_XS)
                            .color(muted),
                    );
                    if entry.kind == Kind::Note {
                        ui.label(
                            egui::RichText::new("\u{b7}")
                                .size(theme::TEXT_XS)
                                .color(muted),
                        );
                        theme::badge(ui, Badge::Note, language.history_kind(Kind::Note));
                    }
                    theme::row_end(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if theme::row_action(
                            ui,
                            delete_id,
                            Icon::Trash,
                            None,
                            language.history_delete(),
                            Tone::Danger,
                            active,
                        )
                        .clicked()
                        {
                            *outcome = Outcome::Delete(entry.id);
                        }
                        let copied = *state.copied == Some(entry.id);
                        let (icon, label) = if copied {
                            (Icon::Check, language.history_copied())
                        } else {
                            (Icon::Copy, language.history_copy())
                        };
                        if theme::row_action(
                            ui,
                            copy_id,
                            icon,
                            Some(label),
                            language.history_copy(),
                            Tone::Primary,
                            active,
                        )
                        .clicked()
                        {
                            *state.copied = Some(entry.id);
                            *outcome = Outcome::Copy(entry.text.clone());
                        }
                    });
                },
            );
        });
    let rect = frame.response.rect;
    ui.data_mut(|data| data.insert_temp(rect_id, rect));
    if active {
        ui.painter().set(
            background,
            egui::Shape::Vec(vec![
                egui::epaint::RectShape::filled(
                    rect.translate(egui::vec2(0.0, 2.0)),
                    radius,
                    colors.border,
                )
                .into(),
                egui::epaint::RectShape::new(
                    rect,
                    radius,
                    colors.card,
                    egui::Stroke::new(1.0, colors.border),
                    egui::StrokeKind::Inside,
                )
                .into(),
            ]),
        );
    }
}
