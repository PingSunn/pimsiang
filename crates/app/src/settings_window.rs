use anyhow::Result;
use app::i18n::Language;
use app::preferences::Preferences;
use engine::LanguageMode;
use platform::TriggerKey;
use platform_macos::login_item::LoginItem;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::egui_surface::Surface;
use crate::theme::{self, Variant};

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
            LogicalSize::new(520.0, 560.0),
            false,
        )?;
        Ok(Self {
            surface,
            language,
            preferences,
            bundled,
            login,
            error: None,
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
            WindowEvent::RedrawRequested => return self.draw(),
            _ => {}
        }
        self.surface.forward(event);
        Outcome::Idle
    }

    fn draw(&mut self) -> Outcome {
        let language = self.language;
        let before = self.preferences;
        let mut preferences = before;
        let bundled = self.bundled;
        let login = self.login;
        let error = self.error.clone();
        let mut open_login_items = false;
        let repaint_now = self.surface.frame(|ui| {
            show(
                ui,
                language,
                &mut preferences,
                bundled,
                login,
                error.as_deref(),
                &mut open_login_items,
            );
        });
        let outcome = if open_login_items {
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

fn show(
    ui: &mut egui::Ui,
    language: Language,
    preferences: &mut Preferences,
    bundled: bool,
    login: LoginItem,
    error: Option<&str>,
    open_login_items: &mut bool,
) {
    theme::page(ui, |ui| {
        theme::title(ui, language.settings_title());
        theme::description(ui, language.settings_applies_now());
        ui.add_space(theme::SPACE_1);

        theme::card(ui, |ui| {
            theme::heading(ui, language.settings_trigger_key());
            theme::select(ui, "trigger-key", preferences.trigger_key.label(), |ui| {
                for key in TriggerKey::ALL {
                    theme::option(ui, &mut preferences.trigger_key, key, key.label());
                }
            });
            theme::description(ui, language.settings_trigger_key_help());
        });

        theme::card(ui, |ui| {
            theme::heading(ui, language.settings_language_mode());
            theme::radio(
                ui,
                &mut preferences.language,
                LanguageMode::ThMix,
                language.settings_language_mix(),
            );
            theme::radio(
                ui,
                &mut preferences.language,
                LanguageMode::ThTh,
                language.settings_language_thai(),
            );
            theme::description(ui, language.settings_language_help());
        });

        theme::card(ui, |ui| {
            ui.spacing_mut().item_spacing.y = theme::SPACE_3;
            theme::switch(
                ui,
                &mut preferences.sounds,
                true,
                language.settings_sounds(),
            );
            theme::switch(
                ui,
                &mut preferences.clipboard_restore,
                true,
                language.settings_clipboard_restore(),
            );
            theme::switch(
                ui,
                &mut preferences.unload_when_idle,
                true,
                language.settings_unload_when_idle(),
            );
            theme::switch(
                ui,
                &mut preferences.launch_at_login,
                bundled,
                language.settings_launch_at_login(),
            );
            if !bundled {
                theme::description(ui, language.settings_login_unbundled());
            } else if login == LoginItem::RequiresApproval {
                theme::warning(ui, language.settings_login_needs_approval());
                if theme::button(
                    ui,
                    Variant::Outline,
                    language.settings_login_open_settings(),
                )
                .clicked()
                {
                    *open_login_items = true;
                }
            }
        });
        if let Some(error) = error {
            theme::warning(ui, error);
        }
    });
}
