use std::path::PathBuf;

use anyhow::{Context, Result};
use std::cell::{Cell, RefCell};

use app::i18n::Language;
use audio::MicrophoneChoice;
use platform::{MicrophoneMenu, PermissionReport, Status, TriggerKey};
use tray_icon::menu::accelerator::{Accelerator, Code, Modifiers};
use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

const FIRST_DYNAMIC_ITEM: usize = 3;
const GLYPH: &[u8] = include_bytes!("../assets/menubar/glyph.png");
const GLYPH_LISTENING: &[u8] = include_bytes!("../assets/menubar/glyph-listening.png");
const GLYPH_TRANSCRIBING: &[u8] = include_bytes!("../assets/menubar/glyph-transcribing.png");
const GLYPH_ATTENTION: &[u8] = include_bytes!("../assets/menubar/glyph-attention.png");

pub struct MenuBar {
    tray: TrayIcon,
    menu: Menu,
    language: Language,
    header_title: MenuItem,
    header_subtitle: MenuItem,
    setup_permissions: MenuItem,
    inserted: RefCell<Vec<MenuItem>>,
    update_version: RefCell<Option<String>>,
    report: Cell<Option<PermissionReport>>,
    trigger_key: Cell<TriggerKey>,
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
        let header_title = MenuItem::new("", false, None);
        let header_subtitle = MenuItem::new("", false, None);
        let setup_permissions = MenuItem::new(language.menu_setup(), true, None);
        let microphones = Submenu::new(language.menu_microphone(), true);
        let update_notice = MenuItem::new(language.menu_update(""), true, None);
        let dismiss_update = MenuItem::new(language.menu_dismiss_update(), true, None);
        let paste_last = MenuItem::new(
            language.menu_paste_last(),
            true,
            Some(Accelerator::new(
                Modifiers::CONTROL | Modifiers::ALT,
                Code::KeyV,
            )),
        );
        let history = MenuItem::new(
            language.menu_history(),
            true,
            Some(Accelerator::new(Modifiers::META, Code::KeyY)),
        );
        let settings = MenuItem::new(
            language.menu_settings(),
            true,
            Some(Accelerator::new(Modifiers::META, Code::Comma)),
        );
        let open_logs = MenuItem::new(language.menu_open_logs(), true, None);
        let quit = MenuItem::new(
            language.menu_quit(),
            true,
            Some(Accelerator::new(Modifiers::META, Code::KeyQ)),
        );
        let menu = Menu::new();
        menu.append_items(&[
            &header_title,
            &header_subtitle,
            &PredefinedMenuItem::separator(),
            &paste_last,
            &history,
            &microphones,
            &PredefinedMenuItem::separator(),
            &settings,
            &open_logs,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .context("building the menu")?;
        let bar = Self {
            tray: TrayIconBuilder::new()
                .with_menu(Box::new(menu.clone()))
                .with_tooltip("PimSiang")
                .with_icon_templated(icon_for(Status::Ready, false)?)
                .build()
                .context("creating the menu bar icon")?,
            menu,
            language,
            setup_permissions_id: setup_permissions.id().clone(),
            microphones,
            microphone_ids: RefCell::new(Vec::new()),
            header_title,
            header_subtitle,
            setup_permissions,
            inserted: RefCell::new(Vec::new()),
            update_version: RefCell::new(None),
            report: Cell::new(None),
            trigger_key: Cell::new(TriggerKey::default()),
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
        };
        bar.refresh()?;
        Ok(bar)
    }

    pub fn show(&self, status: Status) -> Result<()> {
        self.status.set(status);
        self.refresh()
    }

    pub fn show_permissions(&self, report: &PermissionReport) -> Result<()> {
        self.warning.set(!report.all_granted());
        self.report.set(Some(*report));
        self.refresh()
    }

    pub fn show_trigger_key(&self, key: TriggerKey) -> Result<()> {
        self.trigger_key.set(key);
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
        if let Some(version) = version {
            self.update_notice
                .set_text(self.language.menu_update(version));
        }
        *self.update_version.borrow_mut() = version.map(str::to_owned);
        self.sync_dynamic_items()
    }

    fn sync_dynamic_items(&self) -> Result<()> {
        for item in self.inserted.take() {
            self.menu.remove(&item)?;
        }
        let mut wanted = Vec::new();
        if self.warning.get() {
            wanted.push(self.setup_permissions.clone());
        }
        if self.update_version.borrow().is_some() {
            wanted.push(self.update_notice.clone());
            wanted.push(self.dismiss_update.clone());
        }
        for (offset, item) in wanted.iter().enumerate() {
            self.menu.insert(item, FIRST_DYNAMIC_ITEM + offset)?;
        }
        *self.inserted.borrow_mut() = wanted;
        Ok(())
    }

    pub fn flash_update_copied(&self) {
        self.update_notice
            .set_text(self.language.menu_update_copied());
    }

    fn refresh(&self) -> Result<()> {
        let status = self.status.get();
        let warning = self.warning.get() && status == Status::Ready;
        let missing = self
            .report
            .get()
            .filter(|report| warning && !report.all_granted());
        let (title, subtitle) =
            self.language
                .menu_header(status, missing.as_ref(), self.trigger_key.get());
        self.header_title.set_text(title);
        self.header_subtitle.set_text(subtitle);
        self.sync_dynamic_items()?;
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

pub fn open_url(url: &str) -> Result<()> {
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .with_context(|| format!("opening {url}"))?;
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
