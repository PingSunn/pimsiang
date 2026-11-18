use app::history_view::Page;
use app::i18n::Language;
use app::model_chip::ModelChip;
use app::preferences::Preferences;
use egui::{Id, Key, Modifiers, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use platform_macos::login_item::LoginItem;
use store::{Entry, Kind};

use crate::history_window::{self, Clock, Outcome};
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
    matched: usize,
    total: usize,
}

fn clock() -> Clock {
    Clock {
        now: 1_763_450_000,
        offset: chrono::FixedOffset::east_opt(7 * 3600).unwrap(),
    }
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
        .with_size(egui::vec2(640.0, 720.0))
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
                    clock: clock(),
                };
                let page = Page {
                    entries: fixture.entries.clone(),
                    matched: fixture.matched,
                    total: fixture.total,
                };
                history_window::show(
                    ui,
                    Language::English,
                    &mut state,
                    &page,
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
        matched: 2,
        total: 7,
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
    model: ModelChip,
}

fn settings(fixture: SettingsFixture) -> Harness<'static, SettingsFixture> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(780.0, 560.0))
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
                    model: fixture.model,
                    version: "0.1",
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
        model: ModelChip::Ready,
    }
}

#[test]
fn choosing_a_sidebar_section_switches_the_content() {
    let mut harness = settings(settings_fixture());
    harness.get_by_label("Clipboard & Sounds").click();
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

#[test]
fn the_history_sidebar_item_opens_the_history_window_without_switching_section() {
    let mut harness = settings(settings_fixture());
    harness.get_by_label("History").click();
    harness.run();
    assert!(harness.state().actions.open_history);
    assert!(harness.state().section == Section::General);
}

#[test]
fn the_sidebar_chip_names_the_model_state() {
    let mut fixture = settings_fixture();
    fixture.model = ModelChip::Downloading { percent: 41 };
    let harness = settings(fixture);
    harness.get_by_label("Downloading the model · 41%");

    let harness = settings(settings_fixture());
    harness.get_by_label("Ready · model OpenThai");
}

#[test]
fn history_groups_entries_under_day_headings_and_counts_them_in_the_footer() {
    let harness = history(fixture());
    harness.get_by_label("Today");
    harness.get_by_label("7 entries");
}

#[test]
fn a_search_footer_counts_matches_out_of_the_total() {
    let harness = history(HistoryFixture {
        query: "hel".to_owned(),
        matched: 1,
        entries: vec![entry(1, "hello")],
        ..fixture()
    });
    harness.get_by_label("1 of 7 entries");
}

#[test]
fn row_actions_are_reachable_from_the_keyboard() {
    let mut harness = history(fixture());
    harness.key_press(Key::Tab);
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    assert!(
        harness
            .ctx
            .memory(|memory| memory.focused().is_some())
    );
}
