use anyhow::{Context, Result};
use app::app_menu::{
    CLOSE_ID, COPY_ID, CUT_ID, PASTE_ID, QUIT_ID, REDO_ID, SELECT_ALL_ID, SETTINGS_ID, UNDO_ID,
};
use app::i18n::Language;
use tray_icon::menu::accelerator::{Accelerator, Code, Modifiers};
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};

pub fn install(language: Language) -> Result<Menu> {
    let app = Submenu::with_items(
        "PimSiang",
        true,
        &[
            &PredefinedMenuItem::about(Some(language.menu_about()), None),
            &PredefinedMenuItem::separator(),
            &item(
                SETTINGS_ID,
                language.menu_settings(),
                Modifiers::META,
                Code::Comma,
            ),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(Some(language.menu_hide())),
            &PredefinedMenuItem::separator(),
            &item(QUIT_ID, language.menu_quit(), Modifiers::META, Code::KeyQ),
        ],
    )
    .context("building the app menu")?;
    let edit = Submenu::with_items(
        language.menu_edit(),
        true,
        &[
            &item(UNDO_ID, language.menu_undo(), Modifiers::META, Code::KeyZ),
            &item(
                REDO_ID,
                language.menu_redo(),
                Modifiers::META | Modifiers::SHIFT,
                Code::KeyZ,
            ),
            &PredefinedMenuItem::separator(),
            &item(CUT_ID, language.menu_cut(), Modifiers::META, Code::KeyX),
            &item(COPY_ID, language.menu_copy(), Modifiers::META, Code::KeyC),
            &item(PASTE_ID, language.menu_paste(), Modifiers::META, Code::KeyV),
            &item(
                SELECT_ALL_ID,
                language.menu_select_all(),
                Modifiers::META,
                Code::KeyA,
            ),
        ],
    )
    .context("building the Edit menu")?;
    let window = Submenu::with_items(
        language.menu_window(),
        true,
        &[
            &item(
                CLOSE_ID,
                language.menu_close_window(),
                Modifiers::META,
                Code::KeyW,
            ),
            &PredefinedMenuItem::minimize(Some(language.menu_minimize())),
        ],
    )
    .context("building the Window menu")?;
    let menu = Menu::with_items(&[&app, &edit, &window]).context("building the main menu")?;
    window.set_as_windows_menu_for_nsapp();
    menu.init_for_nsapp();
    Ok(menu)
}

fn item(id: &str, title: &str, modifiers: Modifiers, code: Code) -> MenuItem {
    MenuItem::with_id(id, title, true, Some(Accelerator::new(modifiers, code)))
}
