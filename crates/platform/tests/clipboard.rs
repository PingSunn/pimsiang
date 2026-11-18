use std::time::Duration;

use platform::clipboard::{Clipboard, Gathered, Limits, Pasteboard, Restore, gather};

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
    let clipboard = Clipboard::new();
    let mut pasteboard = FakePasteboard::holding("before");

    clipboard.put_text(&mut pasteboard, "สวัสดี", true).unwrap();

    assert_eq!(pasteboard.contents.as_deref(), Some("สวัสดี"));
}

#[test]
fn the_previous_clipboard_comes_back_after_the_paste() {
    let clipboard = Clipboard::new();
    let mut pasteboard = FakePasteboard::holding("before");

    let pending = clipboard
        .put_text(&mut pasteboard, "สวัสดี", true)
        .unwrap()
        .unwrap();
    let outcome = clipboard.restore(&mut pasteboard, pending);

    assert_eq!(outcome, Restore::Restored);
    assert_eq!(pasteboard.contents.as_deref(), Some("before"));
}

#[test]
fn an_empty_clipboard_is_emptied_again() {
    let clipboard = Clipboard::new();
    let mut pasteboard = FakePasteboard::default();

    let pending = clipboard
        .put_text(&mut pasteboard, "สวัสดี", true)
        .unwrap()
        .unwrap();
    clipboard.restore(&mut pasteboard, pending);

    assert_eq!(pasteboard.contents, None);
}

#[test]
fn with_restoring_off_the_text_stays_on_the_clipboard() {
    let clipboard = Clipboard::new();
    let mut pasteboard = FakePasteboard::holding("before");

    let pending = clipboard.put_text(&mut pasteboard, "สวัสดี", false).unwrap();

    assert!(pending.is_none());
    assert_eq!(pasteboard.contents.as_deref(), Some("สวัสดี"));
}

#[test]
fn something_the_user_copied_meanwhile_is_not_overwritten() {
    let clipboard = Clipboard::new();
    let mut pasteboard = FakePasteboard::holding("before");

    let pending = clipboard
        .put_text(&mut pasteboard, "สวัสดี", true)
        .unwrap()
        .unwrap();
    pasteboard.user_copies("copied during the paste");
    let outcome = clipboard.restore(&mut pasteboard, pending);

    assert_eq!(outcome, Restore::ClipboardChanged);
    assert_eq!(
        pasteboard.contents.as_deref(),
        Some("copied during the paste")
    );
}

#[test]
fn an_unreadable_clipboard_is_pasted_over_without_a_restore() {
    let clipboard = Clipboard::new();
    let mut pasteboard = FakePasteboard::holding("before");
    pasteboard.unreadable = true;

    let pending = clipboard.put_text(&mut pasteboard, "สวัสดี", true).unwrap();

    assert!(pending.is_none());
    assert_eq!(pasteboard.contents.as_deref(), Some("สวัสดี"));
}

#[test]
fn back_to_back_pastes_restore_the_original_clipboard_once() {
    let clipboard = Clipboard::new();
    let mut pasteboard = FakePasteboard::holding("original");

    let first = clipboard
        .put_text(&mut pasteboard, "one", true)
        .unwrap()
        .unwrap();
    let second = clipboard
        .put_text(&mut pasteboard, "two", true)
        .unwrap()
        .unwrap();

    assert_eq!(
        clipboard.restore(&mut pasteboard, first),
        Restore::Superseded
    );
    assert_eq!(pasteboard.contents.as_deref(), Some("two"));
    assert_eq!(
        clipboard.restore(&mut pasteboard, second),
        Restore::Restored
    );
    assert_eq!(pasteboard.contents.as_deref(), Some("original"));
}

#[test]
fn a_copy_between_two_pastes_becomes_the_clipboard_to_restore() {
    let clipboard = Clipboard::new();
    let mut pasteboard = FakePasteboard::holding("original");

    clipboard
        .put_text(&mut pasteboard, "one", true)
        .unwrap()
        .unwrap();
    pasteboard.user_copies("copied");
    let second = clipboard
        .put_text(&mut pasteboard, "two", true)
        .unwrap()
        .unwrap();
    clipboard.restore(&mut pasteboard, second);

    assert_eq!(pasteboard.contents.as_deref(), Some("copied"));
}

#[test]
fn a_copy_after_the_last_of_two_pastes_is_left_alone() {
    let clipboard = Clipboard::new();
    let mut pasteboard = FakePasteboard::holding("original");

    clipboard
        .put_text(&mut pasteboard, "one", true)
        .unwrap()
        .unwrap();
    let second = clipboard
        .put_text(&mut pasteboard, "two", true)
        .unwrap()
        .unwrap();
    pasteboard.user_copies("copied");

    assert_eq!(
        clipboard.restore(&mut pasteboard, second),
        Restore::ClipboardChanged
    );
    assert_eq!(pasteboard.contents.as_deref(), Some("copied"));
}

fn text_types(kinds: &[&str]) -> Vec<Vec<String>> {
    vec![kinds.iter().map(|kind| (*kind).to_owned()).collect()]
}

#[test]
fn a_concealed_transient_or_generated_item_is_not_gathered() {
    for kind in [
        "org.nspasteboard.ConcealedType",
        "org.nspasteboard.TransientType",
        "org.nspasteboard.AutoGeneratedType",
    ] {
        let types = text_types(&["public.utf8-plain-text", kind]);
        let mut reads = 0;

        let gathered = gather(&types, &Limits::default(), |_, _| {
            reads += 1;
            Some(b"secret".to_vec())
        });

        assert_eq!(gathered, Gathered::Sensitive);
        assert_eq!(reads, 0);
    }
}

#[test]
fn ordinary_items_are_gathered_with_their_data() {
    let types = text_types(&["public.utf8-plain-text", "public.html"]);

    let gathered = gather(&types, &Limits::default(), |_, kind| {
        (kind != "public.html").then(|| b"hi".to_vec())
    });

    assert_eq!(
        gathered,
        Gathered::Items(vec![vec![(
            "public.utf8-plain-text".to_owned(),
            b"hi".to_vec()
        )]])
    );
}

#[test]
fn too_many_types_or_too_many_bytes_are_not_gathered() {
    let limits = Limits {
        max_types: 2,
        max_bytes: 4,
        budget: Duration::from_secs(5),
    };

    let many = text_types(&["a", "b", "c"]);
    assert_eq!(
        gather(&many, &limits, |_, _| Some(Vec::new())),
        Gathered::OverBudget
    );

    let big = text_types(&["a"]);
    assert_eq!(
        gather(&big, &limits, |_, _| Some(vec![0; 5])),
        Gathered::OverBudget
    );
}

#[test]
fn a_snapshot_that_overruns_its_time_budget_is_dropped() {
    let limits = Limits {
        budget: Duration::ZERO,
        ..Limits::default()
    };
    let types = text_types(&["a", "b"]);

    let gathered = gather(&types, &limits, |_, _| {
        std::thread::sleep(Duration::from_millis(5));
        Some(vec![1])
    });

    assert_eq!(gathered, Gathered::OverBudget);
}
