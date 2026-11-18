use std::path::PathBuf;

use anyhow::{Context, Result};
use std::cell::Cell;

use app::i18n::Language;
use platform::{PermissionReport, Status};
use tray_icon::menu::{Menu, MenuId, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

const ICON_SIZE: u32 = 44;

pub struct MenuBar {
    tray: TrayIcon,
    language: Language,
    permissions_info: MenuItem,
    setup_permissions: MenuItem,
    setup_permissions_id: MenuId,
    open_logs: MenuId,
    quit: MenuId,
    status: Cell<Status>,
    warning: Cell<bool>,
}

pub enum MenuAction {
    SetupPermissions,
    OpenLogFolder,
    Quit,
}

impl MenuBar {
    pub fn new(language: Language) -> Result<Self> {
        let permissions_info = MenuItem::new(language.menu_all_granted(), false, None);
        let setup_permissions = MenuItem::new(language.menu_setup(), false, None);
        let open_logs = MenuItem::new("Open log folder", true, None);
        let quit = MenuItem::new("Quit PimSiang", true, None);
        let menu = Menu::new();
        menu.append_items(&[&permissions_info, &setup_permissions, &open_logs, &quit])
            .context("building the menu")?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("PimSiang")
            .with_icon(icon_for(Status::Ready, false)?)
            .build()
            .context("creating the menu bar icon")?;
        Ok(Self {
            tray,
            language,
            setup_permissions_id: setup_permissions.id().clone(),
            permissions_info,
            setup_permissions,
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

    fn refresh(&self) -> Result<()> {
        let status = self.status.get();
        let warning = self.warning.get() && status == Status::Ready;
        self.tray.set_icon(Some(icon_for(status, warning)?))?;
        self.tray.set_tooltip(Some(if warning {
            "PimSiang: permission missing"
        } else {
            match status {
                Status::Ready => "PimSiang: ready",
                Status::Listening => "PimSiang: listening",
                Status::Transcribing => "PimSiang: transcribing",
            }
        }))?;
        Ok(())
    }

    pub fn action(&self, id: &MenuId) -> Option<MenuAction> {
        if *id == self.setup_permissions_id {
            Some(MenuAction::SetupPermissions)
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
    let (r, g, b) = if warning {
        (255, 204, 0)
    } else {
        match status {
            Status::Ready => (142, 142, 147),
            Status::Listening => (255, 59, 48),
            Status::Transcribing => (255, 159, 10),
        }
    };
    let centre = (ICON_SIZE as f32 - 1.0) / 2.0;
    let radius = ICON_SIZE as f32 * 0.32;
    let mut rgba = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let distance = ((x as f32 - centre).powi(2) + (y as f32 - centre).powi(2)).sqrt();
            let coverage = (radius + 0.5 - distance).clamp(0.0, 1.0);
            let mark = warning && exclamation_mark(x, y);
            let (r, g, b) = if mark { (60, 40, 0) } else { (r, g, b) };
            rgba.extend_from_slice(&[r, g, b, (coverage * 255.0) as u8]);
        }
    }
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).context("building the menu bar icon image")
}

fn exclamation_mark(x: u32, y: u32) -> bool {
    let size = ICON_SIZE as f32;
    let (x, y) = (x as f32 / size, y as f32 / size);
    let centred = (0.46..=0.54).contains(&x);
    centred && ((0.28..=0.56).contains(&y) || (0.62..=0.70).contains(&y))
}
