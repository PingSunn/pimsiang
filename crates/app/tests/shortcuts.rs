use app::shortcuts::{is_paste_last, paste_last_hotkey};
use global_hotkey::HotKeyState;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};

#[test]
fn control_option_v_pressed_is_the_paste_last_shortcut() {
    assert!(is_paste_last(
        paste_last_hotkey().id(),
        HotKeyState::Pressed
    ));
}

#[test]
fn releasing_the_keys_is_not_a_paste() {
    assert!(!is_paste_last(
        paste_last_hotkey().id(),
        HotKeyState::Released
    ));
}

#[test]
fn other_combinations_are_not_the_paste_last_shortcut() {
    let others = [
        HotKey::new(Some(Modifiers::CONTROL), Code::KeyV),
        HotKey::new(Some(Modifiers::ALT), Code::KeyV),
        HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyC),
        HotKey::new(
            Some(Modifiers::META | Modifiers::CONTROL | Modifiers::ALT),
            Code::KeyV,
        ),
    ];
    for other in others {
        assert!(!is_paste_last(other.id(), HotKeyState::Pressed));
    }
}
