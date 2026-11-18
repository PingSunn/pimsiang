use app::i18n::Language;
use app::preferences::Preferences;
use egui::{Id, Key, Modifiers, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use platform_macos::login_item::LoginItem;
use store::{Entry, Kind};

use crate::history_window::{self, Outcome};
use crate::settings_window::{self, Actions, Inputs, Section};
use crate::{egui_surface, theme};

struct HistoryFixture {
    query: String,
    limit: usize,
    confirming_clear: bool,
    copied: Option<i64>,
    focus_search: bool,
    closed: bool,
    refreshed: bool,
    entries: Vec<Entry>,
}

fn entry(id: i64, text: &str) -> Entry {
    Entry {
        id,
        text: text.to_owned(),
        created: 1_763_450_000,
        kind: Kind::Pasted,
        model_id: None,
    }
}

fn history(fixture: HistoryFixture) -> Harness<'static, HistoryFixture> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 640.0))
        .with_context_setup(|context| {
            context.set_fonts(egui_surface::fonts());
            theme::apply(context);
        })
        .build_ui_state(
            |ui, fixture: &mut HistoryFixture| {
                let mut outcome = Outcome::Idle;
                let mut state = history_window::State {
                    query: &mut fixture.query,
                    limit: &mut fixture.limit,
                    confirming_clear: &mut fixture.confirming_clear,
                    copied: &mut fixture.copied,
                    focus_search: &mut fixture.focus_search,
                };
                history_window::show(
                    ui,
                    Language::English,
                    &mut state,
                    &fixture.entries,
                    true,
                    false,
                    &mut outcome,
                );
                match outcome {
                    Outcome::Close => fixture.closed = true,
                    Outcome::Refresh => fixture.refreshed = true,
                    _ => {}
                }
            },
            fixture,
        );
    harness.run();
    harness
}

fn fixture() -> HistoryFixture {
    HistoryFixture {
        query: String::new(),
        limit: 50,
        confirming_clear: false,
        copied: None,
        focus_search: true,
        closed: false,
        refreshed: false,
        entries: vec![entry(1, "hello"), entry(2, "world")],
    }
}

#[test]
fn the_history_search_box_has_focus_when_the_window_opens() {
    let harness = history(fixture());
    assert!(
        harness
            .ctx
            .memory(|memory| memory.has_focus(Id::unique("history-search")))
    );
    assert!(!harness.state().focus_search);
}

#[test]
fn escape_in_history_first_cancels_the_confirmation_then_clears_the_search() {
    let mut harness = history(HistoryFixture {
        query: "hel".to_owned(),
        confirming_clear: true,
        ..fixture()
    });
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().confirming_clear);
    assert_eq!(harness.state().query, "hel");

    harness.key_press(Key::Escape);
    harness.step();
    assert_eq!(harness.state().query, "");
    harness.run();
}

#[test]
fn escape_in_history_with_nothing_to_dismiss_keeps_the_window_open() {
    let mut harness = history(fixture());
    harness.key_press(Key::Escape);
    harness.step();
    assert!(!harness.state().closed && !harness.state().refreshed);
}

#[test]
fn command_w_closes_history() {
    let mut harness = history(fixture());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::W);
    harness.step();
    assert!(harness.state().closed);
}

#[test]
fn clearing_the_search_with_escape_refreshes_the_list() {
    let mut harness = history(HistoryFixture {
        query: "hel".to_owned(),
        ..fixture()
    });
    harness.key_press(Key::Escape);
    harness.step();
    assert!(harness.state().refreshed);
}

struct SettingsFixture {
    section: Section,
    preferences: Preferences,
    actions: Actions,
    login: LoginItem,
    bundled: bool,
}

fn settings(fixture: SettingsFixture) -> Harness<'static, SettingsFixture> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 520.0))
        .with_context_setup(|context| {
            context.set_fonts(egui_surface::fonts());
            theme::apply(context);
        })
        .build_ui_state(
            |ui, fixture: &mut SettingsFixture| {
                let inputs = Inputs {
                    language: Language::English,
                    bundled: fixture.bundled,
                    login: fixture.login,
                    error: None,
                };
                settings_window::show(
                    ui,
                    &inputs,
                    &mut fixture.section,
                    &mut fixture.preferences,
                    &mut fixture.actions,
                );
            },
            fixture,
        );
    harness.run();
    harness
}

fn settings_fixture() -> SettingsFixture {
    SettingsFixture {
        section: Section::General,
        preferences: Preferences::default(),
        actions: Actions::default(),
        login: LoginItem::Disabled,
        bundled: true,
    }
}

#[test]
fn choosing_a_sidebar_section_switches_the_content() {
    let mut harness = settings(settings_fixture());
    harness.get_by_label("Sounds & Clipboard").click();
    harness.run();
    assert!(harness.state().section == Section::SoundsClipboard);
    harness.get_by_label("Trigger").click();
    harness.run();
    assert!(harness.state().section == Section::Trigger);
}

#[test]
fn a_switch_shows_and_changes_the_saved_preference() {
    let mut fixture = settings_fixture();
    fixture.section = Section::SoundsClipboard;
    fixture.preferences.sounds = true;
    let mut harness = settings(fixture);
    harness
        .get_by_role_and_label(
            Role::CheckBox,
            "Play a sound when listening starts and stops",
        )
        .click();
    harness.run();
    assert!(!harness.state().preferences.sounds);
}

#[test]
fn escape_and_command_w_close_the_settings_window() {
    let mut harness = settings(settings_fixture());
    harness.key_press(Key::Escape);
    harness.step();
    assert!(harness.state().actions.close);

    let mut harness = settings(settings_fixture());
    harness.key_press_modifiers(Modifiers::COMMAND, Key::W);
    harness.step();
    assert!(harness.state().actions.close);
}

#[test]
fn the_login_switch_is_disabled_outside_the_installed_app() {
    let mut fixture = settings_fixture();
    fixture.bundled = false;
    fixture.preferences.launch_at_login = false;
    let mut harness = settings(fixture);
    harness
        .get_by_role_and_label(Role::CheckBox, "Open PimSiang at login")
        .click();
    harness.run();
    assert!(!harness.state().preferences.launch_at_login);
}
