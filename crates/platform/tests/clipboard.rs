use platform::clipboard::{Pasteboard, Restore, put_text};

#[derive(Default)]
struct FakePasteboard {
    contents: Option<String>,
    count: i64,
    unreadable: bool,
}

impl FakePasteboard {
    fn holding(text: &str) -> Self {
        Self {
            contents: Some(text.to_owned()),
            ..Self::default()
        }
    }

    fn user_copies(&mut self, text: &str) {
        self.contents = Some(text.to_owned());
        self.count += 1;
    }
}

impl Pasteboard for FakePasteboard {
    type Snapshot = Option<String>;

    fn change_count(&self) -> i64 {
        self.count
    }

    fn snapshot(&mut self) -> Option<Self::Snapshot> {
        (!self.unreadable).then(|| self.contents.clone())
    }

    fn write_text(&mut self, text: &str) -> Result<(), String> {
        self.contents = Some(text.to_owned());
        self.count += 1;
        Ok(())
    }

    fn restore(&mut self, snapshot: Self::Snapshot) -> Result<(), String> {
        self.contents = snapshot;
        self.count += 1;
        Ok(())
    }
}

#[test]
fn the_text_is_put_on_the_clipboard_for_the_paste() {
    let mut pasteboard = FakePasteboard::holding("before");

    put_text(&mut pasteboard, "สวัสดี", true).unwrap();

    assert_eq!(pasteboard.contents.as_deref(), Some("สวัสดี"));
}

#[test]
fn the_previous_clipboard_comes_back_after_the_paste() {
    let mut pasteboard = FakePasteboard::holding("before");

    let pending = put_text(&mut pasteboard, "สวัสดี", true).unwrap().unwrap();
    let outcome = pending.restore(&mut pasteboard);

    assert_eq!(outcome, Restore::Restored);
    assert_eq!(pasteboard.contents.as_deref(), Some("before"));
}

#[test]
fn an_empty_clipboard_is_emptied_again() {
    let mut pasteboard = FakePasteboard::default();

    let pending = put_text(&mut pasteboard, "สวัสดี", true).unwrap().unwrap();
    pending.restore(&mut pasteboard);

    assert_eq!(pasteboard.contents, None);
}

#[test]
fn with_restoring_off_the_text_stays_on_the_clipboard() {
    let mut pasteboard = FakePasteboard::holding("before");

    let pending = put_text(&mut pasteboard, "สวัสดี", false).unwrap();

    assert!(pending.is_none());
    assert_eq!(pasteboard.contents.as_deref(), Some("สวัสดี"));
}

#[test]
fn something_the_user_copied_meanwhile_is_not_overwritten() {
    let mut pasteboard = FakePasteboard::holding("before");

    let pending = put_text(&mut pasteboard, "สวัสดี", true).unwrap().unwrap();
    pasteboard.user_copies("copied during the paste");
    let outcome = pending.restore(&mut pasteboard);

    assert_eq!(outcome, Restore::ClipboardChanged);
    assert_eq!(
        pasteboard.contents.as_deref(),
        Some("copied during the paste")
    );
}

#[test]
fn an_unreadable_clipboard_is_pasted_over_without_a_restore() {
    let mut pasteboard = FakePasteboard::holding("before");
    pasteboard.unreadable = true;

    let pending = put_text(&mut pasteboard, "สวัสดี", true).unwrap();

    assert!(pending.is_none());
    assert_eq!(pasteboard.contents.as_deref(), Some("สวัสดี"));
}
