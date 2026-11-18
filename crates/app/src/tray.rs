use std::path::PathBuf;

use anyhow::{Context, Result};
use platform::Status;
use tray_icon::menu::{Menu, MenuId, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

const ICON_SIZE: u32 = 44;

pub struct MenuBar {
    tray: TrayIcon,
    open_logs: MenuId,
    quit: MenuId,
}

pub enum MenuAction {
    OpenLogFolder,
    Quit,
}

impl MenuBar {
    pub fn new() -> Result<Self> {
        let open_logs = MenuItem::new("Open log folder", true, None);
        let quit = MenuItem::new("Quit PimSiang", true, None);
        let menu = Menu::new();
        menu.append_items(&[&open_logs, &quit])
            .context("building the menu")?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("PimSiang")
            .with_icon(icon_for(Status::Ready)?)
            .build()
            .context("creating the menu bar icon")?;
        Ok(Self {
            tray,
            open_logs: open_logs.id().clone(),
            quit: quit.id().clone(),
        })
    }

    pub fn show(&self, status: Status) -> Result<()> {
        self.tray.set_icon(Some(icon_for(status)?))?;
        self.tray.set_tooltip(Some(match status {
            Status::Ready => "PimSiang: ready",
            Status::Listening => "PimSiang: listening",
            Status::Transcribing => "PimSiang: transcribing",
        }))?;
        Ok(())
    }

    pub fn action(&self, id: &MenuId) -> Option<MenuAction> {
        if *id == self.open_logs {
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

fn icon_for(status: Status) -> Result<Icon> {
    let (r, g, b) = match status {
        Status::Ready => (142, 142, 147),
        Status::Listening => (255, 59, 48),
        Status::Transcribing => (255, 159, 10),
    };
    let centre = (ICON_SIZE as f32 - 1.0) / 2.0;
    let radius = ICON_SIZE as f32 * 0.32;
    let mut rgba = Vec::with_capacity((ICON_SIZE * ICON_SIZE * 4) as usize);
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let distance = ((x as f32 - centre).powi(2) + (y as f32 - centre).powi(2)).sqrt();
            let coverage = (radius + 0.5 - distance).clamp(0.0, 1.0);
            rgba.extend_from_slice(&[r, g, b, (coverage * 255.0) as u8]);
        }
    }
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).context("building the menu bar icon image")
}
