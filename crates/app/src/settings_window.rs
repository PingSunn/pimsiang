use anyhow::Result;
use app::i18n::Language;
use app::preferences::Preferences;
use engine::LanguageMode;
use platform::TriggerKey;
use platform_macos::login_item::{self, LoginItem};
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::egui_surface::Surface;
use crate::theme::{self, Icon, Variant};

const WIDTH: f64 = 720.0;
const HEIGHT: f64 = 520.0;
const SIDEBAR_WIDTH: f32 = 208.0;

pub enum Outcome {
    Idle,
    Close,
    Changed(Preferences),
    OpenLoginItems,
}

pub struct SettingsWindow {
    surface: Surface,
    language: Language,
    preferences: Preferences,
    bundled: bool,
    login: LoginItem,
    error: Option<String>,
    section: Section,
}

impl SettingsWindow {
    pub fn open(
        event_loop: &ActiveEventLoop,
        language: Language,
        preferences: Preferences,
        bundled: bool,
        login: LoginItem,
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
        })
    }

    pub fn id(&self) -> WindowId {
        self.surface.id()
    }

    pub fn focus(&self) {
        self.surface.focus();
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
}

impl Section {
    pub(crate) const ALL: [Section; 3] =
        [Section::General, Section::Trigger, Section::SoundsClipboard];

    fn title(self, language: Language) -> &'static str {
        match self {
            Self::General => language.section_general(),
            Self::Trigger => language.section_trigger(),
            Self::SoundsClipboard => language.section_sounds_clipboard(),
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::General => Icon::Sliders,
            Self::Trigger => Icon::Key,
            Self::SoundsClipboard => Icon::Speaker,
        }
    }
}

pub(crate) struct Inputs {
    pub(crate) language: Language,
    pub(crate) bundled: bool,
    pub(crate) login: LoginItem,
    pub(crate) error: Option<String>,
}

#[derive(Default)]
pub(crate) struct Actions {
    pub(crate) open_login_items: bool,
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
        .frame(egui::Frame::new().fill(colors.sidebar))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            theme::header(ui, |_| {});
            ui.add_space(theme::SPACE_4);
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(theme::SPACE_3 as i8, 0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = theme::SPACE_1;
                    for item in Section::ALL {
                        if theme::nav_item(ui, item.icon(), item.title(language), *section == item)
                            .clicked()
                        {
                            *section = item;
                        }
                    }
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
                        .inner_margin(egui::Margin::symmetric(28, 28))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = theme::SPACE_4;
                            theme::title(ui, section.title(language));
                            ui.add_space(-theme::SPACE_2);
                            theme::description(ui, language.settings_applies_now());
                            ui.add_space(theme::SPACE_1);
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
                            }
                        });
                });
        });
    if !popup_open && (theme::escape_pressed(ui) || theme::close_pressed(ui)) {
        actions.close = true;
    }
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
        theme::heading(ui, language.settings_language_mode());
        theme::choice(
            ui,
            &mut preferences.language,
            LanguageMode::ThMix,
            language.settings_language_mix(),
        );
        theme::choice(
            ui,
            &mut preferences.language,
            LanguageMode::ThTh,
            language.settings_language_thai(),
        );
        theme::description(ui, language.settings_language_help());
    });
    ui.add_space(theme::SPACE_1);
    theme::group(ui, |ui| {
        let login_help = (!inputs.bundled).then(|| language.settings_login_unbundled());
        theme::setting_row(
            ui,
            language.settings_launch_at_login(),
            login_help,
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
        theme::setting_row(ui, language.settings_unload_when_idle(), None, 40.0, |ui| {
            theme::toggle(
                ui,
                &mut preferences.unload_when_idle,
                true,
                language.settings_unload_when_idle(),
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

fn trigger(ui: &mut egui::Ui, language: Language, preferences: &mut Preferences) {
    theme::card(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = theme::SPACE_3;
            ui.add_space(theme::SPACE_2);
            theme::keycap(ui, preferences.trigger_key.label());
            ui.add_space(theme::SPACE_1);
            ui.scope(|ui| {
                ui.set_max_width(400.0);
                theme::description(ui, language.settings_trigger_key_help());
            });
            ui.add_space(theme::SPACE_1);
        });
    });
    theme::group(ui, |ui| {
        theme::setting_row(ui, language.settings_trigger_key(), None, 200.0, |ui| {
            let name = language.settings_trigger_key();
            theme::select(
                ui,
                "trigger-key",
                name,
                preferences.trigger_key.label(),
                |ui| {
                    for key in TriggerKey::ALL {
                        theme::option(ui, &mut preferences.trigger_key, key, key.label());
                    }
                },
            );
        });
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
