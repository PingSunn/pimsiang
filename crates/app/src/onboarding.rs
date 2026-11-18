use std::path::Path;

use platform::{Permission, PermissionReport, PermissionStatus, TriggerKey};

use crate::i18n::Language;
use crate::model_download::ModelStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Button {
    pub permission: Permission,
    pub label: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub permission: Permission,
    pub title: &'static str,
    pub why: String,
    pub status: PermissionStatus,
    pub status_label: &'static str,
    pub button: Option<Button>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelRow {
    pub title: &'static str,
    pub explain: String,
    pub detail: String,
    pub fraction: Option<f32>,
    pub failed: bool,
    pub retry: Option<&'static str>,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub title: &'static str,
    pub heading: &'static str,
    pub bubble: &'static str,
    pub intro: String,
    pub note: &'static str,
    pub model: ModelRow,
    pub rows: Vec<Row>,
    pub done: bool,
    pub steps_done: usize,
    pub steps_total: usize,
    pub progress: String,
    pub trigger_key: TriggerKey,
    pub model_done: &'static str,
    pub hint: &'static str,
}

pub fn is_bundled(executable: &Path) -> bool {
    executable
        .components()
        .any(|part| part.as_os_str().to_string_lossy().ends_with(".app"))
}

fn model_row(language: Language, status: &ModelStatus, size: u64) -> ModelRow {
    let row = |detail: String, fraction, failed, retry| ModelRow {
        title: language.model_title(),
        explain: language.model_explain(size),
        detail,
        fraction,
        failed,
        retry,
        done: false,
    };
    match status {
        ModelStatus::Ready => ModelRow {
            done: true,
            ..row(String::new(), None, false, None)
        },
        ModelStatus::Downloading { done, total } => row(
            language.model_downloading(*done, *total),
            Some(if *total == 0 {
                0.0
            } else {
                (*done as f64 / *total as f64) as f32
            }),
            false,
            None,
        ),
        ModelStatus::Verifying => row(
            language.model_verifying().to_owned(),
            Some(1.0),
            false,
            None,
        ),
        ModelStatus::Failed(reason) => row(
            language.model_failed(reason),
            None,
            true,
            Some(language.model_retry()),
        ),
    }
}

pub fn view(
    language: Language,
    key: TriggerKey,
    report: &PermissionReport,
    bundled: bool,
    model: &ModelStatus,
    size: u64,
) -> View {
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
                why: language.permission_why(permission, key),
                status,
                status_label: language.status_label(status),
                button,
            }
        })
        .collect();
    let done = report.all_granted() && *model == ModelStatus::Ready;
    let steps_total = Permission::ALL.len() + 1;
    let steps_done = Permission::ALL
        .into_iter()
        .filter(|permission| report.status(*permission) == PermissionStatus::Granted)
        .count()
        + usize::from(*model == ModelStatus::Ready);
    View {
        title: language.window_title(),
        heading: language.heading(),
        bubble: language.setup_bubble(),
        intro: language.intro_left(steps_total - steps_done),
        note: if bundled {
            ""
        } else {
            language.terminal_note()
        },
        model: model_row(language, model, size),
        rows,
        done,
        steps_done,
        steps_total,
        progress: language.setup_progress(steps_done, steps_total),
        trigger_key: key,
        hint: language.setup_hint(),
        model_done: language.model_done_badge(),
    }
}
