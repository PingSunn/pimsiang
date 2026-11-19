use app::i18n::Language;
use std::path::Path;

use app::model_download::ModelStatus;
use app::onboarding::{Button, is_bundled, view as view_with};
use platform::{Permission, PermissionReport, PermissionStatus, TriggerKey};

fn report(mic: PermissionStatus, ax: PermissionStatus, im: PermissionStatus) -> PermissionReport {
    PermissionReport {
        microphone: mic,
        accessibility: ax,
        input_monitoring: im,
    }
}

const GRANTED: PermissionStatus = PermissionStatus::Granted;
const DENIED: PermissionStatus = PermissionStatus::Denied;
const UNKNOWN: PermissionStatus = PermissionStatus::NotDetermined;

fn view(language: Language, report: &PermissionReport, bundled: bool) -> app::onboarding::View {
    view_with(
        language,
        TriggerKey::RightOption,
        report,
        bundled,
        &ModelStatus::Ready,
        1_000_000_000,
    )
}

fn has_thai(text: &str) -> bool {
    text.chars().any(|c| ('\u{0E00}'..='\u{0E7F}').contains(&c))
}

#[test]
fn thai_system_language_selects_thai() {
    assert_eq!(Language::from_tags(&["th-TH", "en-US"]), Language::Thai);
    assert_eq!(Language::from_tags(&["th"]), Language::Thai);
}

#[test]
fn any_other_first_language_selects_english() {
    assert_eq!(Language::from_tags(&["en-US", "th-TH"]), Language::English);
    assert_eq!(Language::from_tags(&["ja-JP"]), Language::English);
    assert_eq!(Language::from_tags::<&str>(&[]), Language::English);
}

#[test]
fn every_string_exists_in_both_languages() {
    for language in [Language::English, Language::Thai] {
        let thai = language == Language::Thai;
        let rows = view(language, &report(DENIED, DENIED, UNKNOWN), false).rows;
        assert_eq!(rows.len(), 3);
        let intro = language.intro_left(3);
        let hint = language.setup_hint();
        let mut texts = vec![
            language.terminal_note(),
            language.window_title(),
            language.heading(),
            intro.as_str(),
            hint,
            language.setup_bubble(),
            language.menu_setup(),
            language.menu_all_granted(),
            language.menu_microphone(),
            language.menu_system_default(),
        ];
        for row in &rows {
            texts.push(row.title);
            texts.push(row.why.as_str());
            texts.push(row.status_label);
            texts.push(row.button.map(|b| b.label).unwrap_or("x"));
        }
        for text in texts {
            assert!(!text.is_empty());
            assert_eq!(has_thai(text), thai, "{text}");
        }
    }
}

#[test]
fn rows_follow_the_report_order() {
    let v = view(Language::English, &report(GRANTED, DENIED, UNKNOWN), true);
    let order: Vec<_> = v.rows.iter().map(|r| r.permission).collect();
    assert_eq!(
        order,
        [
            Permission::Microphone,
            Permission::Accessibility,
            Permission::InputMonitoring
        ]
    );
    assert!(v.rows[0].button.is_none());
    assert!(v.rows[1].button.is_some());
    assert!(!v.done);
}

#[test]
fn undetermined_microphone_offers_to_ask_the_other_cases_open_settings() {
    let ask = view(Language::English, &report(UNKNOWN, GRANTED, GRANTED), true).rows[0]
        .button
        .unwrap();
    let open = view(Language::English, &report(DENIED, GRANTED, GRANTED), true).rows[0]
        .button
        .unwrap();
    assert_ne!(ask.label, open.label);
    assert_eq!(ask.permission, Permission::Microphone);
    assert_eq!(open.permission, Permission::Microphone);
}

#[test]
fn onboarding_is_done_when_everything_is_granted() {
    let v = view(Language::Thai, &report(GRANTED, GRANTED, GRANTED), true);
    assert!(v.done);
    assert!(v.rows.iter().all(|r| r.button.is_none()));
}

#[test]
fn the_menu_summary_names_what_is_missing() {
    let r = report(DENIED, GRANTED, UNKNOWN);
    assert_eq!(
        Language::English.menu_missing(&r),
        "Missing permissions: Microphone, Input Monitoring"
    );
    assert!(has_thai(&Language::Thai.menu_missing(&r)));
    let _: Option<Button> = None;
}

#[test]
fn running_from_a_terminal_adds_a_note_about_the_terminal_app() {
    let r = report(DENIED, DENIED, DENIED);
    assert!(view(Language::English, &r, true).note.is_empty());
    assert!(view(Language::English, &r, false).note.contains("terminal"));
    assert!(has_thai(view(Language::Thai, &r, false).note));
}

#[test]
fn a_binary_inside_an_app_bundle_is_bundled() {
    assert!(is_bundled(Path::new(
        "/Applications/PimSiang.app/Contents/MacOS/pimsiang"
    )));
    assert!(!is_bundled(Path::new("/repo/target/debug/pimsiang")));
}

fn model_view(language: Language, status: ModelStatus) -> app::onboarding::View {
    view_with(
        language,
        TriggerKey::RightOption,
        &report(GRANTED, GRANTED, GRANTED),
        true,
        &status,
        1_000_000_000,
    )
}

#[test]
fn a_downloading_model_explains_the_size_and_shows_progress() {
    let v = model_view(
        Language::English,
        ModelStatus::Downloading {
            done: 250_000_000,
            total: 1_000_000_000,
        },
    );
    let row = v.model;
    assert!(row.explain.contains("1 GB"));
    assert_eq!(row.detail, "250 of 1,000 MB");
    assert_eq!(row.fraction, Some(0.25));
    assert!(!row.failed);
    assert!(row.retry.is_none());
    assert!(!v.done);
}

#[test]
fn the_download_size_text_follows_the_real_size() {
    use app::i18n::download_size;
    assert_eq!(download_size(2_606_098_275), "2.6 GB");
    assert_eq!(download_size(994_516_953), "1 GB");
    assert_eq!(download_size(1_500_000_000), "1.5 GB");
    let v = view_with(
        Language::Thai,
        TriggerKey::RightOption,
        &report(GRANTED, GRANTED, GRANTED),
        true,
        &ModelStatus::Verifying,
        2_606_098_275,
    );
    assert!(v.model.explain.contains("2.6 GB"));
}

#[test]
fn a_failed_download_shows_the_reason_and_a_retry() {
    for language in [Language::English, Language::Thai] {
        let v = model_view(
            language,
            ModelStatus::Failed("the server answered 500".to_owned()),
        );
        let row = v.model;
        assert!(row.failed);
        assert!(row.detail.contains("the server answered 500"));
        assert!(!row.retry.unwrap().is_empty());
        assert!(!v.done);
    }
}

#[test]
fn onboarding_is_done_only_when_the_model_is_ready_too() {
    let ready = model_view(Language::English, ModelStatus::Ready);
    assert!(ready.done);
    assert!(ready.model.done);
    assert!(!model_view(Language::English, ModelStatus::Verifying).done);
}

#[test]
fn model_strings_exist_in_both_languages() {
    for language in [Language::English, Language::Thai] {
        let thai = language == Language::Thai;
        let strings = [
            language.model_title().to_owned(),
            language.model_explain(1_000_000_000),
            language.model_downloading(1, 2),
            language.model_verifying().to_owned(),
            language.model_failed("x"),
            language.model_ready().to_owned(),
            language.model_retry().to_owned(),
        ];
        for text in strings {
            assert!(!text.is_empty());
            assert_eq!(has_thai(&text), thai, "{text}");
        }
    }
}

#[test]
fn the_texts_name_the_configured_trigger_key() {
    let granted = report(GRANTED, GRANTED, GRANTED);
    for language in [Language::English, Language::Thai] {
        let v = view_with(
            language,
            TriggerKey::RightCommand,
            &report(DENIED, DENIED, DENIED),
            true,
            &ModelStatus::Ready,
            1_000_000_000,
        );
        assert!(v.rows[0].why.contains("Right Command"));
        assert!(v.rows[2].why.contains("Right Command"));
        assert!(!v.rows[0].why.contains("Right Option"));
        let done = view_with(
            language,
            TriggerKey::RightControl,
            &granted,
            true,
            &ModelStatus::Ready,
            1_000_000_000,
        );
        assert_eq!(done.trigger_key, TriggerKey::RightControl);
    }
}

#[test]
fn progress_counts_the_model_and_every_granted_permission() {
    let partial = report(GRANTED, UNKNOWN, DENIED);
    let downloading = view_with(
        Language::English,
        TriggerKey::RightOption,
        &partial,
        true,
        &ModelStatus::Downloading { done: 1, total: 4 },
        1_000_000_000,
    );
    assert_eq!((downloading.steps_done, downloading.steps_total), (1, 4));
    assert_eq!(downloading.progress, "1 of 4 ready");

    let ready = view(Language::Thai, &report(GRANTED, GRANTED, GRANTED), true);
    assert_eq!((ready.steps_done, ready.steps_total), (4, 4));
    assert_eq!(ready.progress, "พร้อมแล้ว 4 จาก 4");
}

#[test]
fn the_intro_counts_the_steps_that_are_left() {
    let none_left = view(Language::English, &report(GRANTED, GRANTED, GRANTED), true);
    assert_eq!(none_left.intro, Language::English.intro_left(0));
    let thai = model_view(
        Language::Thai,
        ModelStatus::Downloading { done: 1, total: 4 },
    );
    assert!(thai.intro.contains("อีก 1 ขั้นตอน"));
    let all_open = view_with(
        Language::English,
        TriggerKey::RightOption,
        &report(UNKNOWN, UNKNOWN, UNKNOWN),
        true,
        &ModelStatus::Verifying,
        1_000_000_000,
    );
    assert!(all_open.intro.starts_with("4 steps left"));
}

#[test]
fn the_settings_chip_follows_the_model_state() {
    use app::model_chip::{ModelChip, model_chip};
    assert_eq!(model_chip(&ModelStatus::Ready, false), ModelChip::Ready);
    assert_eq!(model_chip(&ModelStatus::Ready, true), ModelChip::Loading);
    assert_eq!(
        model_chip(
            &ModelStatus::Downloading {
                done: 412,
                total: 1000
            },
            true
        ),
        ModelChip::Downloading { percent: 41 }
    );
    assert_eq!(
        model_chip(&ModelStatus::Downloading { done: 0, total: 0 }, false),
        ModelChip::Downloading { percent: 0 }
    );
    assert_eq!(
        model_chip(&ModelStatus::Verifying, false),
        ModelChip::Verifying
    );
    assert_eq!(
        model_chip(&ModelStatus::Failed("x".to_owned()), false),
        ModelChip::Missing
    );
}

#[test]
fn every_chip_text_exists_in_both_languages() {
    use app::model_chip::ModelChip;
    for language in [Language::English, Language::Thai] {
        for chip in [
            ModelChip::Ready,
            ModelChip::Loading,
            ModelChip::Downloading { percent: 41 },
            ModelChip::Verifying,
            ModelChip::Missing,
        ] {
            let text = language.model_chip(chip);
            assert_eq!(has_thai(&text), language == Language::Thai, "{text}");
        }
    }
    assert_eq!(
        Language::Thai.model_chip(ModelChip::Ready),
        "พร้อมใช้ · โมเดล OpenThai"
    );
}

#[test]
fn the_powered_by_credit_exists_in_both_languages() {
    assert_eq!(Language::English.powered_by(), "Powered by OpenThai ASR");
    assert_eq!(Language::Thai.powered_by(), "ขับเคลื่อนด้วย OpenThai ASR");
    assert!(has_thai(Language::Thai.powered_by()));
    assert!(!has_thai(Language::English.powered_by()));
}
