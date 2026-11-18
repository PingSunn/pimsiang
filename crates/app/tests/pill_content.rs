use app::i18n::Language;
use platform::{PartialText, PillError, PillNotice, PillState};

#[test]
fn a_partial_is_split_into_settled_and_uncertain_text_with_a_caret() {
    let state = PillState::Partial(PartialText::new("ช่วยส่ง รายงาน", "ช่วยส่ง ".len()));
    let content = Language::Thai.pill_content(&state).unwrap();
    assert_eq!(content.settled, "ช่วยส่ง ");
    assert_eq!(content.tentative, "รายงาน");
    assert!(content.caret && !content.dots && content.badge.is_none());
}

#[test]
fn transcribing_says_it_is_writing_with_three_dots() {
    let content = Language::Thai
        .pill_content(&PillState::Transcribing)
        .unwrap();
    assert_eq!(content.settled, "กำลังเขียนให้…");
    assert!(content.dots && !content.caret);
}

#[test]
fn a_saved_note_carries_the_note_badge() {
    for notice in [
        PillNotice::QuickNoteSaved,
        PillNotice::NoTextFieldKeptAsNote,
        PillNotice::PasteFailedKeptAsNote,
    ] {
        let content = Language::Thai
            .pill_content(&PillState::Notice(notice))
            .unwrap();
        assert_eq!(content.badge.as_deref(), Some("โน้ต"));
    }
    let other = Language::Thai
        .pill_content(&PillState::Notice(PillNotice::NoTranscriptYet))
        .unwrap();
    assert!(other.badge.is_none());
}

#[test]
fn other_states_are_plain_text_and_hidden_has_no_content() {
    let error = PillState::Error(PillError::PasteFailed);
    let content = Language::English.pill_content(&error).unwrap();
    assert_eq!(content.settled, "Could not paste the text");
    assert!(content.tentative.is_empty() && !content.caret && !content.dots);
    assert_eq!(Language::English.pill_content(&PillState::Hidden), None);
}
