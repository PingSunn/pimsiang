use anyhow::{Context, Result};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

pub fn paste_last_hotkey() -> HotKey {
    HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyV)
}

pub fn is_paste_last(id: u32, state: HotKeyState) -> bool {
    state == HotKeyState::Pressed && id == paste_last_hotkey().id()
}

pub struct PasteLastShortcut {
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
}

impl PasteLastShortcut {
    pub fn register() -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("creating the hotkey manager")?;
        let hotkey = paste_last_hotkey();
        manager
            .register(hotkey)
            .context("registering the paste-last shortcut")?;
        Ok(Self { manager, hotkey })
    }
}

impl Drop for PasteLastShortcut {
    fn drop(&mut self) {
        let _ = self.manager.unregister(self.hotkey);
    }
}

pub fn on_paste_last(callback: impl Fn() + Send + Sync + 'static) {
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if is_paste_last(event.id, event.state) {
            callback();
        }
    }));
}
