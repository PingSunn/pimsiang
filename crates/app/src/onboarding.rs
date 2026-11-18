use std::path::Path;

use platform::{Permission, PermissionReport, PermissionStatus};

use crate::i18n::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Button {
    pub permission: Permission,
    pub label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub permission: Permission,
    pub title: &'static str,
    pub why: &'static str,
    pub status: PermissionStatus,
    pub status_label: &'static str,
    pub button: Option<Button>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub title: &'static str,
    pub heading: &'static str,
    pub intro: &'static str,
    pub note: &'static str,
    pub rows: Vec<Row>,
    pub done: bool,
    pub footer: &'static str,
}

pub fn is_bundled(executable: &Path) -> bool {
    executable
        .components()
        .any(|part| part.as_os_str().to_string_lossy().ends_with(".app"))
}

pub fn view(language: Language, report: &PermissionReport, bundled: bool) -> View {
    let rows = Permission::ALL
        .into_iter()
        .map(|permission| {
            let status = report.status(permission);
            let button = (status != PermissionStatus::Granted).then(|| Button {
                permission,
                label: match (permission, status) {
                    (Permission::Microphone, PermissionStatus::NotDetermined) => {
                        language.allow_microphone()
                    }
                    _ => language.open_settings(),
                },
            });
            Row {
                permission,
                title: language.permission_name(permission),
                why: language.permission_why(permission),
                status,
                status_label: language.status_label(status),
                button,
            }
        })
        .collect();
    let done = report.all_granted();
    View {
        title: language.window_title(),
        heading: language.heading(),
        intro: language.intro(),
        note: if bundled {
            ""
        } else {
            language.terminal_note()
        },
        rows,
        done,
        footer: if done { language.all_set() } else { "" },
    }
}
