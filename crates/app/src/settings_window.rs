use anyhow::Result;
use app::i18n::Language;
use app::preferences::Preferences;
use egui::{Color32, RichText};
use engine::LanguageMode;
use platform::TriggerKey;
use platform_macos::login_item::LoginItem;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::egui_surface::Surface;

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

const MUTED: Color32 = Color32::from_rgb(152, 152, 157);
const WARNING: Color32 = Color32::from_rgb(255, 159, 10);

fn show(
    ui: &mut egui::Ui,
    language: Language,
    preferences: &mut Preferences,
    bundled: bool,
    login: LoginItem,
    error: Option<&str>,
    open_login_items: &mut bool,
) {
    egui::Frame::default().inner_margin(20.0).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 10.0;
        ui.label(RichText::new(language.settings_title()).size(22.0).strong());
        ui.label(
            RichText::new(language.settings_applies_now())
                .size(13.0)
                .color(MUTED),
        );
        ui.add_space(8.0);

        ui.label(
            RichText::new(language.settings_trigger_key())
                .size(15.0)
                .strong(),
        );
        egui::ComboBox::from_id_salt("trigger-key")
            .selected_text(RichText::new(preferences.trigger_key.label()).size(15.0))
            .show_ui(ui, |ui| {
                for key in TriggerKey::ALL {
                    ui.selectable_value(
                        &mut preferences.trigger_key,
                        key,
                        RichText::new(key.label()).size(15.0),
                    );
                }
            });
        ui.label(
            RichText::new(language.settings_trigger_key_help())
                .size(13.0)
                .color(MUTED),
        );
        ui.add_space(8.0);

        ui.label(
            RichText::new(language.settings_language_mode())
                .size(15.0)
                .strong(),
        );
        ui.radio_value(
            &mut preferences.language,
            LanguageMode::ThMix,
            RichText::new(language.settings_language_mix()).size(14.0),
        );
        ui.radio_value(
            &mut preferences.language,
            LanguageMode::ThTh,
            RichText::new(language.settings_language_thai()).size(14.0),
        );
        ui.label(
            RichText::new(language.settings_language_help())
                .size(13.0)
                .color(MUTED),
        );
        ui.add_space(8.0);

        ui.checkbox(
            &mut preferences.sounds,
            RichText::new(language.settings_sounds()).size(14.0),
        );
        ui.checkbox(
            &mut preferences.clipboard_restore,
            RichText::new(language.settings_clipboard_restore()).size(14.0),
        );
        ui.add_enabled(
            bundled,
            egui::Checkbox::new(
                &mut preferences.launch_at_login,
                RichText::new(language.settings_launch_at_login()).size(14.0),
            ),
        );
        if !bundled {
            ui.label(
                RichText::new(language.settings_login_unbundled())
                    .size(13.0)
                    .color(MUTED),
            );
        } else if login == LoginItem::RequiresApproval {
            ui.label(
                RichText::new(language.settings_login_needs_approval())
                    .size(13.0)
                    .color(WARNING),
            );
            if ui
                .button(RichText::new(language.settings_login_open_settings()).size(13.0))
                .clicked()
            {
                *open_login_items = true;
            }
        }
        if let Some(error) = error {
            ui.label(RichText::new(error).size(13.0).color(WARNING));
        }
    });
}
