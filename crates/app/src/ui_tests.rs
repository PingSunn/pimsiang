use app::app_menu::EditCommand;
use app::history_view::Page;
use app::i18n::Language;
use app::model_chip::ModelChip;
use app::model_download::ModelStatus;
use app::preferences::Preferences;
use egui::accesskit::Toggled;
use egui::{Id, Key, Modifiers, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::RecognitionMode;
use platform::{PillPlacement, ShowInDock};
use platform_macos::login_item::LoginItem;
use store::{Entry, Kind};

use crate::history_window::{self, Clock, Outcome};
use crate::settings_window::{self, Actions, Build, Inputs, Section};
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
        zone: history_window::Zone::Fixed(chrono::FixedOffset::east_opt(7 * 3600).unwrap()),
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

fn run_edit(harness: &mut Harness<'static, HistoryFixture>, command: EditCommand, text: &str) {
    for event in command.events(Some(text)) {
        harness.event(event);
    }
    harness.run();
}

#[test]
fn the_paste_menu_item_puts_thai_text_in_the_history_search_box() {
    let mut harness = history(fixture());
    run_edit(&mut harness, EditCommand::Paste, "สวัสดีครับ");
    assert_eq!(harness.state().query, "สวัสดีครับ");
}

#[test]
fn the_cut_menu_item_removes_the_selection_and_select_all_selects_it() {
    let mut harness = history(HistoryFixture {
        query: "สวัสดี".to_owned(),
        ..fixture()
    });
    run_edit(&mut harness, EditCommand::SelectAll, "");
    run_edit(&mut harness, EditCommand::Cut, "");
    assert_eq!(harness.state().query, "");
}

#[test]
fn the_undo_and_redo_menu_items_step_through_the_search_box_edits() {
    let mut harness = history(fixture());
    run_edit(&mut harness, EditCommand::Paste, "abc");
    harness.step();
    run_edit(&mut harness, EditCommand::Undo, "");
    assert_eq!(harness.state().query, "");
    run_edit(&mut harness, EditCommand::Redo, "");
    assert_eq!(harness.state().query, "abc");
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
    build: Option<Build>,
    placing: bool,
}

fn settings(fixture: SettingsFixture) -> Harness<'static, SettingsFixture> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(780.0, 600.0))
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
                    build: fixture.build.clone(),
                    version: "0.1",
                    placing: fixture.placing,
                    pill_preview: None,
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
        build: None,
        placing: false,
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
fn the_powered_by_credit_is_shown_and_opens_the_openthai_page() {
    let mut harness = settings(settings_fixture());
    assert!(!harness.state().actions.open_powered_by);
    harness
        .get_by_role_and_label(Role::Link, "Powered by OpenThai ASR")
        .click();
    harness.run();
    assert!(harness.state().actions.open_powered_by);
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
fn choosing_notch_or_floating_changes_the_saved_placement() {
    let mut harness = settings(settings_fixture());
    harness
        .get_by_role_and_label(Role::RadioButton, "Notch")
        .scroll_to_me();
    harness.run();
    harness
        .get_by_role_and_label(Role::RadioButton, "Notch")
        .click();
    harness.run();
    assert_eq!(
        harness.state().preferences.pill_placement,
        PillPlacement::Notch
    );
    harness
        .get_by_role_and_label(Role::RadioButton, "Floating")
        .scroll_to_me();
    harness.run();
    harness
        .get_by_role_and_label(Role::RadioButton, "Floating")
        .click();
    harness.run();
    assert_eq!(
        harness.state().preferences.pill_placement,
        PillPlacement::Floating
    );
}

#[test]
fn choosing_a_recognition_mode_changes_the_saved_preference() {
    let mut harness = settings(settings_fixture());
    assert_eq!(
        harness.state().preferences.recognition_mode,
        RecognitionMode::Standard
    );
    harness
        .get_by_role_and_label(Role::RadioButton, "Conversation")
        .click();
    harness.run();
    assert_eq!(
        harness.state().preferences.recognition_mode,
        RecognitionMode::Conversation
    );
    harness
        .get_by_role_and_label(Role::RadioButton, "Standard")
        .click();
    harness.run();
    assert_eq!(
        harness.state().preferences.recognition_mode,
        RecognitionMode::Standard
    );
}

#[test]
fn choosing_a_precision_changes_the_saved_preference_and_light_is_the_default() {
    let mut harness = settings(settings_fixture());
    assert_eq!(
        harness.state().preferences.precision,
        store::Precision::Light
    );
    harness
        .get_by_role_and_label(Role::RadioButton, "Accurate")
        .click();
    harness.run();
    assert_eq!(
        harness.state().preferences.precision,
        store::Precision::Accurate
    );
    harness
        .get_by_role_and_label(Role::RadioButton, "Light")
        .click();
    harness.run();
    assert_eq!(
        harness.state().preferences.precision,
        store::Precision::Light
    );
}

#[test]
fn the_precision_cards_name_the_real_download_sizes() {
    let size =
        |precision| app::model_download::download_bytes(app::model_download::build_of(precision));
    assert_eq!(
        Language::English
            .settings_precision_hint(store::Precision::Accurate, size(store::Precision::Accurate)),
        "Best accuracy. Downloads about 2.6 GB."
    );
    assert_eq!(
        Language::English
            .settings_precision_hint(store::Precision::Light, size(store::Precision::Light)),
        "Downloads about 1 GB and uses less memory."
    );
    assert!(
        Language::Thai
            .settings_precision_hint(store::Precision::Accurate, size(store::Precision::Accurate))
            .contains("2.6 GB")
    );
}

#[test]
fn a_background_download_shows_its_progress_and_that_the_old_model_keeps_working() {
    let mut fixture = settings_fixture();
    fixture.build = Some(Build {
        status: ModelStatus::Downloading {
            done: 500_000_000,
            total: 2_606_000_000,
        },
        keeps_working: true,
    });
    let harness = settings(fixture);
    harness.get_by_label("500 of 2,606 MB");
    harness.get_by_label("The current model keeps working meanwhile.");
}

#[test]
fn a_failed_download_shows_the_error_and_retry_asks_for_another_attempt() {
    let mut fixture = settings_fixture();
    fixture.build = Some(Build {
        status: ModelStatus::Failed("the server answered 500".to_owned()),
        keeps_working: true,
    });
    let mut harness = settings(fixture);
    harness.get_by_label("Download failed: the server answered 500");
    harness.get_by_label("Try again").scroll_to_me();
    harness.run();
    harness.get_by_label("Try again").click();
    harness.run();
    assert!(harness.state().actions.retry_model);
}

#[test]
fn show_in_dock_reflects_the_preference_and_choosing_updates_it() {
    let mut fixture = settings_fixture();
    fixture.preferences.show_in_dock = ShowInDock::Always;
    let mut harness = settings(fixture);
    let selected = |harness: &Harness<'_, SettingsFixture>, label: &str| {
        harness
            .get_by_role_and_label(Role::RadioButton, label)
            .accesskit_node()
            .toggled()
    };
    assert_eq!(selected(&harness, "Always"), Some(Toggled::True));
    assert_eq!(selected(&harness, "Never"), Some(Toggled::False));
    harness
        .get_by_role_and_label(Role::RadioButton, "Never")
        .scroll_to_me();
    harness.run();
    harness
        .get_by_role_and_label(Role::RadioButton, "Never")
        .click();
    harness.run();
    assert_eq!(harness.state().preferences.show_in_dock, ShowInDock::Never);
    harness
        .get_by_role_and_label(Role::RadioButton, "While a window is open")
        .scroll_to_me();
    harness.run();
    harness
        .get_by_role_and_label(Role::RadioButton, "While a window is open")
        .click();
    harness.run();
    assert_eq!(
        harness.state().preferences.show_in_dock,
        ShowInDock::WhileWindowOpen
    );
}

#[test]
fn the_position_button_starts_placement_for_a_floating_pill() {
    let mut harness = settings(settings_fixture());
    harness.get_by_label("Position…").scroll_to_me();
    harness.run();
    harness.get_by_label("Position…").click();
    harness.run();
    assert!(harness.state().actions.place_pill);
}

#[test]
fn the_position_button_is_disabled_for_the_notch() {
    let mut fixture = settings_fixture();
    fixture.preferences.pill_placement = PillPlacement::Notch;
    let mut harness = settings(fixture);
    harness.get_by_label("Position…").scroll_to_me();
    harness.run();
    harness.get_by_label("Position…").click();
    harness.run();
    assert!(!harness.state().actions.place_pill);
}

#[test]
fn while_placing_done_ends_placement_and_escape_does_not_close_the_window() {
    let mut fixture = settings_fixture();
    fixture.placing = true;
    let mut harness = settings(fixture);
    harness.get_by_label("Done").scroll_to_me();
    harness.run();
    harness.get_by_label("Done").click();
    harness.run();
    assert!(harness.state().actions.done_placement);

    let mut fixture = settings_fixture();
    fixture.placing = true;
    let mut harness = settings(fixture);
    harness.key_press(Key::Escape);
    harness.step();
    assert!(harness.state().actions.escape_placement);
    assert!(!harness.state().actions.close);
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
    assert!(harness.ctx.memory(|memory| memory.focused().is_some()));
}
