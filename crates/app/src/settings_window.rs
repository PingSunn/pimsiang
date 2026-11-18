use anyhow::Result;
use app::i18n::Language;
use app::model_chip::ModelChip;
use app::preferences::Preferences;
use engine::LanguageMode;
use platform::{PillPlacement, PillPosition, ShowInDock, TriggerKey};
use platform_macos::PlacementEnd;
use platform_macos::login_item::{self, LoginItem};
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::egui_surface::Surface;
use crate::theme::{self, Icon, Variant};

const WIDTH: f64 = 780.0;
const HEIGHT: f64 = 560.0;
const SIDEBAR_WIDTH: f32 = 220.0;

pub enum Outcome {
    Idle,
    Close,
    Changed(Preferences),
    OpenLoginItems,
    OpenHistory,
    PlacePill,
    EndPlacement(PlacementEnd),
}

pub struct SettingsWindow {
    surface: Surface,
    language: Language,
    preferences: Preferences,
    bundled: bool,
    login: LoginItem,
    error: Option<String>,
    section: Section,
    model: ModelChip,
    placing: bool,
    pill_preview: Option<PillPosition>,
}

impl SettingsWindow {
    pub fn open(
        event_loop: &ActiveEventLoop,
        language: Language,
        preferences: Preferences,
        bundled: bool,
        login: LoginItem,
        model: ModelChip,
    ) -> Result<Self> {
        let surface = Surface::open(
            event_loop,
            language.settings_title(),
            LogicalSize::new(WIDTH, HEIGHT),
            false,
        )?;
        Ok(Self {
            surface,
            language,
            preferences,
            bundled,
            login,
            error: None,
            section: Section::General,
            model,
            placing: false,
            pill_preview: None,
        })
    }

    pub fn id(&self) -> WindowId {
        self.surface.id()
    }

    pub fn focus(&self) {
        self.surface.focus();
    }

    pub fn popup_open(&self) -> bool {
        self.surface.popup_open()
    }

    pub fn show_model(&mut self, model: ModelChip) {
        if self.model != model {
            self.model = model;
            self.surface.request_redraw();
        }
    }

    pub fn show_placement(&mut self, placing: bool) {
        if self.placing != placing || (!placing && self.pill_preview.is_some()) {
            self.placing = placing;
            if !placing {
                self.pill_preview = None;
            }
            self.surface.request_redraw();
        }
    }

    pub fn show_pill_preview(&mut self, position: PillPosition) {
        self.pill_preview = Some(position);
        self.surface.request_redraw();
    }

    pub fn show_result(
        &mut self,
        preferences: Preferences,
        login: LoginItem,
        error: Option<String>,
    ) {
        self.preferences = preferences;
        self.login = login;
        self.error = error;
        self.surface.request_redraw();
    }

    pub fn handle(&mut self, event: &WindowEvent) -> Outcome {
        match event {
            WindowEvent::CloseRequested => return Outcome::Close,
            WindowEvent::Resized(size) => self.surface.resize(*size),
            WindowEvent::Focused(true) => {
                self.login = login_item::status();
                self.surface.request_redraw();
            }
            WindowEvent::RedrawRequested => return self.draw(),
            _ => {}
        }
        self.surface.forward(event);
        Outcome::Idle
    }

    fn draw(&mut self) -> Outcome {
        let before = self.preferences;
        let mut preferences = before;
        let inputs = Inputs {
            language: self.language,
            bundled: self.bundled,
            login: self.login,
            error: self.error.clone(),
            model: self.model,
            version: env!("CARGO_PKG_VERSION"),
            placing: self.placing,
            pill_preview: self.pill_preview,
        };
        let mut section = self.section;
        let mut actions = Actions::default();
        let repaint_now = self.surface.frame(|ui| {
            show(ui, &inputs, &mut section, &mut preferences, &mut actions);
        });
        self.section = section;
        let outcome = if actions.close {
            Outcome::Close
        } else if actions.open_login_items {
            Outcome::OpenLoginItems
        } else if actions.open_history {
            Outcome::OpenHistory
        } else if actions.place_pill {
            Outcome::PlacePill
        } else if actions.done_placement {
            Outcome::EndPlacement(PlacementEnd::Done)
        } else if actions.escape_placement {
            Outcome::EndPlacement(PlacementEnd::Escape)
        } else if preferences != before {
            self.preferences = preferences;
            Outcome::Changed(preferences)
        } else {
            Outcome::Idle
        };
        if repaint_now || !matches!(outcome, Outcome::Idle) {
            self.surface.request_redraw();
        }
        outcome
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    General,
    Trigger,
    SoundsClipboard,
    History,
}

impl Section {
    pub(crate) const ALL: [Section; 4] = [
        Section::General,
        Section::Trigger,
        Section::SoundsClipboard,
        Section::History,
    ];

    fn title(self, language: Language) -> &'static str {
        match self {
            Self::General => language.section_general(),
            Self::Trigger => language.section_trigger(),
            Self::SoundsClipboard => language.section_sounds_clipboard(),
            Self::History => language.history_heading(),
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::General => Icon::Sliders,
            Self::Trigger => Icon::Key,
            Self::SoundsClipboard => Icon::Clipboard,
            Self::History => Icon::Clock,
        }
    }

    fn subtitle(self, language: Language) -> &'static str {
        match self {
            Self::Trigger => language.settings_trigger_key_help(),
            _ => language.settings_applies_now(),
        }
    }
}

pub(crate) struct Inputs {
    pub(crate) language: Language,
    pub(crate) bundled: bool,
    pub(crate) login: LoginItem,
    pub(crate) error: Option<String>,
    pub(crate) model: ModelChip,
    pub(crate) version: &'static str,
    pub(crate) placing: bool,
    pub(crate) pill_preview: Option<PillPosition>,
}

#[derive(Default)]
pub(crate) struct Actions {
    pub(crate) open_login_items: bool,
    pub(crate) open_history: bool,
    pub(crate) place_pill: bool,
    pub(crate) done_placement: bool,
    pub(crate) escape_placement: bool,
    pub(crate) close: bool,
}

pub(crate) fn show(
    ui: &mut egui::Ui,
    inputs: &Inputs,
    section: &mut Section,
    preferences: &mut Preferences,
    actions: &mut Actions,
) {
    let colors = theme::colors(ui);
    let popup_open = egui::Popup::is_any_open(ui.ctx());
    let language = inputs.language;
    let nav = egui::Panel::left("settings-nav")
        .exact_size(SIDEBAR_WIDTH)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(colors.sidebar)
                .inner_margin(egui::Margin::symmetric(12, 14)),
        )
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.add_space(theme::SPACE_5);
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(8, 0))
                .show(ui, |ui| theme::sidebar_brand(ui, inputs.version));
            ui.add_space(theme::SPACE_5);
            for item in Section::ALL {
                let selected = *section == item;
                if theme::nav_item(ui, item.icon(), item.title(language), selected).clicked() {
                    if item == Section::History {
                        actions.open_history = true;
                    } else {
                        *section = item;
                    }
                }
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                let (tone, text) = chip(language, inputs.model);
                theme::status_chip(ui, tone, &text);
            });
        });
    let rect = nav.response.rect;
    ui.painter().vline(
        rect.right() - 0.5,
        rect.y_range(),
        egui::Stroke::new(1.0, colors.border),
    );
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(colors.background))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Frame::new()
                        .inner_margin(egui::Margin {
                            left: 40,
                            right: 40,
                            top: 44,
                            bottom: 28,
                        })
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 22.0;
                            ui.vertical(|ui| {
                                theme::page_title(
                                    ui,
                                    section.title(language),
                                    section.subtitle(language),
                                );
                            });
                            if let Some(error) = &inputs.error {
                                theme::banner(ui, colors.warning, |ui| {
                                    theme::warning(ui, error);
                                });
                            }
                            match *section {
                                Section::General => general(ui, inputs, preferences, actions),
                                Section::Trigger => trigger(ui, language, preferences),
                                Section::SoundsClipboard => {
                                    sounds_clipboard(ui, language, preferences)
                                }
                                Section::History => {}
                            }
                        });
                });
        });
    if !popup_open && inputs.placing && theme::escape_pressed(ui) {
        actions.escape_placement = true;
    } else if !popup_open && (theme::escape_pressed(ui) || theme::close_pressed(ui)) {
        actions.close = true;
    }
}

fn chip(language: Language, model: ModelChip) -> (theme::ChipTone, String) {
    let tone = match model {
        ModelChip::Ready => theme::ChipTone::Ready,
        ModelChip::Missing => theme::ChipTone::Missing,
        ModelChip::Loading | ModelChip::Downloading { .. } | ModelChip::Verifying => {
            theme::ChipTone::Busy
        }
    };
    (tone, language.model_chip(model))
}

fn general(
    ui: &mut egui::Ui,
    inputs: &Inputs,
    preferences: &mut Preferences,
    actions: &mut Actions,
) {
    let language = inputs.language;
    let colors = theme::colors(ui);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = theme::SPACE_2;
        theme::section_label(ui, language.settings_language_mode());
        let gap = 10.0;
        let width = (ui.available_width() - gap) / 2.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            theme::language_card(
                ui,
                &mut preferences.language,
                LanguageMode::ThMix,
                language.settings_language_mix(),
                &language.settings_language_mix_example(),
                width,
            );
            theme::language_card(
                ui,
                &mut preferences.language,
                LanguageMode::ThTh,
                language.settings_language_thai(),
                &language.settings_language_thai_example(),
                width,
            );
        });
        theme::description(ui, language.settings_language_help());
    });
    pill_position(ui, inputs, preferences, actions);
    show_in_dock(ui, language, preferences);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = theme::SPACE_2;
        theme::section_label(ui, language.settings_system_group());
        theme::group(ui, |ui| {
            let login_help = if inputs.bundled {
                language.settings_launch_at_login_help()
            } else {
                language.settings_login_unbundled()
            };
            theme::setting_row(
                ui,
                language.settings_launch_at_login(),
                Some(login_help),
                40.0,
                |ui| {
                    theme::toggle(
                        ui,
                        &mut preferences.launch_at_login,
                        inputs.bundled,
                        language.settings_launch_at_login(),
                    );
                },
            );
            theme::divider(ui);
            theme::setting_row(
                ui,
                language.settings_unload_when_idle(),
                Some(language.settings_unload_when_idle_help()),
                40.0,
                |ui| {
                    theme::toggle(
                        ui,
                        &mut preferences.unload_when_idle,
                        true,
                        language.settings_unload_when_idle(),
                    );
                },
            );
        });
    });
    if inputs.bundled && inputs.login == LoginItem::RequiresApproval {
        theme::banner(ui, colors.warning, |ui| {
            theme::warning(ui, language.settings_login_needs_approval());
            if theme::button(
                ui,
                Variant::Outline,
                language.settings_login_open_settings(),
            )
            .clicked()
            {
                actions.open_login_items = true;
            }
        });
    }
}

fn show_in_dock(ui: &mut egui::Ui, language: Language, preferences: &mut Preferences) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = theme::SPACE_2;
        theme::section_label(ui, language.settings_dock_group());
        let options = [
            (
                ShowInDock::Always,
                vec![theme::Part::Text(language.settings_dock_always())],
            ),
            (
                ShowInDock::WhileWindowOpen,
                vec![theme::Part::Text(language.settings_dock_while_open())],
            ),
            (
                ShowInDock::Never,
                vec![theme::Part::Text(language.settings_dock_never())],
            ),
        ];
        theme::segmented(ui, &mut preferences.show_in_dock, &options);
        theme::description(ui, language.settings_dock_help());
    });
}

fn pill_position(
    ui: &mut egui::Ui,
    inputs: &Inputs,
    preferences: &mut Preferences,
    actions: &mut Actions,
) {
    let language = inputs.language;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = theme::SPACE_2;
        theme::section_label(ui, language.settings_pill_group());
        theme::group(ui, |ui| {
            ui.add_space(14.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = theme::SPACE_5;
                theme::pill_thumbnail(
                    ui,
                    preferences.pill_placement,
                    inputs.pill_preview.unwrap_or(preferences.pill_position),
                );
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = theme::SPACE_3;
                    let options = [
                        (
                            PillPlacement::Notch,
                            vec![theme::Part::Text(language.settings_pill_notch())],
                        ),
                        (
                            PillPlacement::Floating,
                            vec![theme::Part::Text(language.settings_pill_floating())],
                        ),
                    ];
                    theme::segmented(ui, &mut preferences.pill_placement, &options);
                    let help = match (preferences.pill_placement, inputs.placing) {
                        (PillPlacement::Floating, true) => language.settings_pill_placing_help(),
                        (PillPlacement::Floating, false) => language.settings_pill_floating_help(),
                        (PillPlacement::Notch, _) => language.settings_pill_notch_help(),
                    };
                    theme::description(ui, help);
                    let floating = preferences.pill_placement == PillPlacement::Floating;
                    if inputs.placing {
                        if theme::button(ui, Variant::Primary, language.settings_pill_done())
                            .clicked()
                        {
                            actions.done_placement = true;
                        }
                    } else if theme::button_enabled(
                        ui,
                        Variant::Outline,
                        floating,
                        language.settings_pill_position(),
                    )
                    .clicked()
                    {
                        actions.place_pill = true;
                    }
                });
            });
            ui.add_space(14.0);
        });
    });
}

pub(crate) fn glyph_of(key: TriggerKey) -> theme::Glyph {
    match key {
        TriggerKey::RightOption => theme::Glyph::Option,
        TriggerKey::RightCommand => theme::Glyph::Command,
        TriggerKey::RightControl => theme::Glyph::Control,
    }
}

fn trigger(ui: &mut egui::Ui, language: Language, preferences: &mut Preferences) {
    let chosen = match preferences.trigger_key {
        TriggerKey::RightOption => theme::StripKey::Option,
        TriggerKey::RightCommand => theme::StripKey::Command,
        TriggerKey::RightControl => theme::StripKey::Control,
    };
    let (hold, hold_text) = language.settings_gesture_hold();
    let (tap, tap_text) = language.settings_gesture_double_tap();
    let (note, note_text) = language.settings_gesture_note();
    theme::keyboard_strip(
        ui,
        chosen,
        [
            (None, hold, hold_text),
            (None, tap, tap_text),
            (Some(theme::Glyph::Shift), note, note_text),
        ],
    );
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = theme::SPACE_2;
        theme::section_label(ui, language.settings_trigger_use_key());
        let options: Vec<(TriggerKey, Vec<theme::Part<'_>>)> = TriggerKey::ALL
            .into_iter()
            .map(|key| {
                (
                    key,
                    vec![
                        theme::Part::Text(key.label()),
                        theme::Part::Glyph(glyph_of(key)),
                    ],
                )
            })
            .collect();
        theme::segmented(ui, &mut preferences.trigger_key, &options);
        let colors = theme::colors(ui);
        let (before, key, after) = language.settings_trigger_esc();
        theme::emphasised_line(
            ui,
            &[(before, false), (key, true), (after, false)],
            theme::TEXT_XS,
            colors.muted_foreground,
        );
    });
}

fn sounds_clipboard(ui: &mut egui::Ui, language: Language, preferences: &mut Preferences) {
    theme::group(ui, |ui| {
        theme::setting_row(ui, language.settings_sounds(), None, 40.0, |ui| {
            theme::toggle(
                ui,
                &mut preferences.sounds,
                true,
                language.settings_sounds(),
            );
        });
        theme::divider(ui);
        theme::setting_row(
            ui,
            language.settings_clipboard_restore(),
            None,
            40.0,
            |ui| {
                theme::toggle(
                    ui,
                    &mut preferences.clipboard_restore,
                    true,
                    language.settings_clipboard_restore(),
                );
            },
        );
    });
}
