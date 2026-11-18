use std::path::PathBuf;

use anyhow::{Context, Result};
use std::cell::{Cell, RefCell};

use app::i18n::Language;
use audio::MicrophoneChoice;
use platform::{MicrophoneMenu, PermissionReport, Status};
use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

const GLYPH: &[u8] = include_bytes!("../assets/menubar/glyph.png");
const GLYPH_LISTENING: &[u8] = include_bytes!("../assets/menubar/glyph-listening.png");
const GLYPH_TRANSCRIBING: &[u8] = include_bytes!("../assets/menubar/glyph-transcribing.png");
const GLYPH_ATTENTION: &[u8] = include_bytes!("../assets/menubar/glyph-attention.png");

pub struct MenuBar {
    tray: TrayIcon,
    menu: Menu,
    update_shown: Cell<bool>,
    language: Language,
    permissions_info: MenuItem,
    setup_permissions: MenuItem,
    setup_permissions_id: MenuId,
    microphones: Submenu,
    microphone_ids: RefCell<Vec<(MenuId, MicrophoneChoice)>>,
    history_id: MenuId,
    paste_last_id: MenuId,
    settings_id: MenuId,
    update_notice: MenuItem,
    update_notice_id: MenuId,
    dismiss_update: MenuItem,
    dismiss_update_id: MenuId,
    open_logs: MenuId,
    quit: MenuId,
    status: Cell<Status>,
    warning: Cell<bool>,
}

pub enum MenuAction {
    Microphone(MicrophoneChoice),
    SetupPermissions,
    OpenHistory,
    PasteLast,
    OpenSettings,
    CopyUpgradeCommand,
    DismissUpdate,
    OpenLogFolder,
    Quit,
}

impl MenuBar {
    pub fn new(language: Language) -> Result<Self> {
        let permissions_info = MenuItem::new(language.menu_all_granted(), false, None);
        let setup_permissions = MenuItem::new(language.menu_setup(), false, None);
        let microphones = Submenu::new(language.menu_microphone(), true);
        let update_notice = MenuItem::new(language.menu_update(""), true, None);
        let dismiss_update = MenuItem::new(language.menu_dismiss_update(), true, None);
        let history = MenuItem::new(language.menu_history(), true, None);
        let paste_last = MenuItem::new(language.menu_paste_last(), true, None);
        let settings = MenuItem::new(language.menu_settings(), true, None);
        let open_logs = MenuItem::new(language.menu_open_logs(), true, None);
        let quit = MenuItem::new(language.menu_quit(), true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &permissions_info,
            &setup_permissions,
            &microphones,
            &history,
            &paste_last,
            &settings,
            &open_logs,
            &quit,
        ])
        .context("building the menu")?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu.clone()))
            .with_tooltip("PimSiang")
            .with_icon_templated(icon_for(Status::Ready, false)?)
            .build()
            .context("creating the menu bar icon")?;
        Ok(Self {
            tray,
            menu,
            update_shown: Cell::new(false),
            language,
            setup_permissions_id: setup_permissions.id().clone(),
            microphones,
            microphone_ids: RefCell::new(Vec::new()),
            permissions_info,
            setup_permissions,
            history_id: history.id().clone(),
            paste_last_id: paste_last.id().clone(),
            settings_id: settings.id().clone(),
            update_notice_id: update_notice.id().clone(),
            update_notice,
            dismiss_update_id: dismiss_update.id().clone(),
            dismiss_update,
            status: Cell::new(Status::Ready),
            warning: Cell::new(false),
            open_logs: open_logs.id().clone(),
            quit: quit.id().clone(),
        })
    }

    pub fn show(&self, status: Status) -> Result<()> {
        self.status.set(status);
        self.refresh()
    }

    pub fn show_permissions(&self, report: &PermissionReport) -> Result<()> {
        let warning = !report.all_granted();
        self.warning.set(warning);
        self.setup_permissions.set_enabled(warning);
        self.permissions_info.set_text(if warning {
            self.language.menu_missing(report)
        } else {
            self.language.menu_all_granted().to_owned()
        });
        self.refresh()
    }

    pub fn show_microphones(&self, menu: &MicrophoneMenu) -> Result<()> {
        while self.microphones.remove_at(0).is_some() {}
        let mut ids = Vec::new();
        let default_item = CheckMenuItem::new(
            self.language.menu_system_default(),
            true,
            menu.selected.is_none(),
            None,
        );
        self.microphones.append(&default_item)?;
        ids.push((default_item.id().clone(), MicrophoneChoice::SystemDefault));
        for device in &menu.devices {
            let checked = menu.selected.as_deref() == Some(device.id.as_str());
            let item = CheckMenuItem::new(&device.name, true, checked, None);
            self.microphones.append(&item)?;
            ids.push((
                item.id().clone(),
                MicrophoneChoice::Device(device.id.clone()),
            ));
        }
        *self.microphone_ids.borrow_mut() = ids;
        Ok(())
    }

    pub fn show_update(&self, version: Option<&str>) -> Result<()> {
        match version {
            Some(version) => {
                self.update_notice
                    .set_text(self.language.menu_update(version));
                if !self.update_shown.replace(true) {
                    self.menu.insert(&self.update_notice, 3)?;
                    self.menu.insert(&self.dismiss_update, 4)?;
                }
            }
            None => {
                if self.update_shown.replace(false) {
                    self.menu.remove(&self.update_notice)?;
                    self.menu.remove(&self.dismiss_update)?;
                }
            }
        }
        Ok(())
    }

    pub fn flash_update_copied(&self) {
        self.update_notice
            .set_text(self.language.menu_update_copied());
    }

    fn refresh(&self) -> Result<()> {
        let status = self.status.get();
        let warning = self.warning.get() && status == Status::Ready;
        self.tray
            .set_icon_templated(Some(icon_for(status, warning)?))?;
        self.tray
            .set_tooltip(Some(self.language.tooltip(status, warning)))?;
        Ok(())
    }

    pub fn action(&self, id: &MenuId) -> Option<MenuAction> {
        let microphone = self
            .microphone_ids
            .borrow()
            .iter()
            .find(|(item, _)| item == id)
            .map(|(_, choice)| choice.clone());
        if let Some(choice) = microphone {
            Some(MenuAction::Microphone(choice))
        } else if *id == self.setup_permissions_id {
            Some(MenuAction::SetupPermissions)
        } else if *id == self.history_id {
            Some(MenuAction::OpenHistory)
        } else if *id == self.paste_last_id {
            Some(MenuAction::PasteLast)
        } else if *id == self.settings_id {
            Some(MenuAction::OpenSettings)
        } else if *id == self.update_notice_id {
            Some(MenuAction::CopyUpgradeCommand)
        } else if *id == self.dismiss_update_id {
            Some(MenuAction::DismissUpdate)
        } else if *id == self.open_logs {
            Some(MenuAction::OpenLogFolder)
        } else if *id == self.quit {
            Some(MenuAction::Quit)
        } else {
            None
        }
    }
}

pub fn open_folder(path: &PathBuf) -> Result<()> {
    std::process::Command::new("open")
        .arg(path)
        .spawn()
        .with_context(|| format!("opening {}", path.display()))?;
    Ok(())
}

fn icon_for(status: Status, warning: bool) -> Result<Icon> {
    let bytes = if warning {
        GLYPH_ATTENTION
    } else {
        match status {
            Status::Ready => GLYPH,
            Status::Listening => GLYPH_LISTENING,
            Status::Transcribing => GLYPH_TRANSCRIBING,
        }
    };
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .context("decoding the menu bar glyph")?
        .to_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).context("building the menu bar icon image")
}
