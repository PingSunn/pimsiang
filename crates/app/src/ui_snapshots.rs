use std::path::PathBuf;

use app::i18n::Language;
use app::model_download::ModelStatus;
use app::onboarding;
use app::preferences::Preferences;
use egui::{Pos2, Theme, Vec2, pos2, vec2};
use egui_kittest::Harness;
use platform::{PermissionReport, PermissionStatus, TriggerKey};
use platform_macos::login_item::LoginItem;
use store::{Entry, Kind};

use crate::history_window::{self, Outcome as HistoryOutcome};
use crate::settings_window::{self, Actions, Inputs, Section};
use crate::{egui_surface, onboarding_window, theme};

const MARGIN: f32 = 8.0;
const THEMES: [(Theme, &str); 2] = [(Theme::Dark, "dark"), (Theme::Light, "light")];
const LANGUAGES: [(Language, &str); 2] = [(Language::Thai, "th"), (Language::English, "en")];

fn output_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/ui-snapshots");
    std::fs::create_dir_all(&dir).expect("creating the snapshot directory");
    dir
}

fn harness<'a>(size: Vec2, theme: Theme, ui: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    let mut harness = Harness::builder()
        .with_size(size + Vec2::splat(2.0 * MARGIN))
        .with_pixels_per_point(2.0)
        .with_theme(theme)
        .with_accessibility_check(false)
        .with_context_setup(|context| {
            context.set_fonts(egui_surface::fonts());
            theme::apply(context);
        })
        .wgpu()
        .build_ui(ui);
    harness.run();
    harness
}

fn save(harness: &mut Harness<'_>, name: &str) {
    let image = harness.render().expect("rendering the UI offscreen");
    let edge = (MARGIN * 2.0) as u32;
    image::imageops::crop_imm(
        &image,
        edge,
        edge,
        image.width() - 2 * edge,
        image.height() - 2 * edge,
    )
    .to_image()
    .save(output_dir().join(format!("{name}.png")))
    .expect("saving the snapshot");
}

fn every_look(
    name: &str,
    size: Vec2,
    mut ui: impl FnMut(&mut egui::Ui, Language),
    mut after: impl FnMut(&mut Harness<'_>),
) {
    for (language, language_name) in LANGUAGES {
        for (theme, theme_name) in THEMES {
            let mut harness = harness(size, theme, |root| ui(root, language));
            after(&mut harness);
            save(
                &mut harness,
                &format!("{name}-{language_name}-{theme_name}"),
            );
        }
    }
}

fn report(
    microphone: PermissionStatus,
    accessibility: PermissionStatus,
    input_monitoring: PermissionStatus,
) -> PermissionReport {
    PermissionReport {
        microphone,
        accessibility,
        input_monitoring,
    }
}

fn setup(name: &str, bundled: bool, report: PermissionReport, model: ModelStatus) {
    every_look(
        name,
        vec2(540.0, 760.0),
        |ui, language| {
            let view =
                onboarding::view(language, TriggerKey::RightOption, &report, bundled, &model);
            onboarding_window::show(ui, &view, &mut None, &mut false, &mut false);
        },
        |_| {},
    );
}

#[test]
#[ignore = "renders offscreen on the GPU: cargo test -p app ui_snapshots -- --ignored"]
fn setup_snapshots() {
    use PermissionStatus::{Denied, Granted, NotDetermined};
    setup(
        "setup-mixed",
        true,
        report(Granted, Denied, NotDetermined),
        ModelStatus::Downloading {
            done: 412_000_000,
            total: 1_000_000_000,
        },
    );
    setup(
        "setup-terminal-failed",
        false,
        report(NotDetermined, NotDetermined, NotDetermined),
        ModelStatus::Failed("network is unreachable".to_owned()),
    );
    setup(
        "setup-done",
        true,
        report(Granted, Granted, Granted),
        ModelStatus::Ready,
    );
}

fn settings(name: &str, section: Section, bundled: bool, login: LoginItem, error: Option<&str>) {
    every_look(
        name,
        vec2(720.0, 520.0),
        |ui, language| {
            let inputs = Inputs {
                language,
                bundled,
                login,
                error: error.map(str::to_owned),
            };
            let mut section = section;
            let mut preferences = Preferences {
                sounds: false,
                ..Preferences::default()
            };
            settings_window::show(
                ui,
                &inputs,
                &mut section,
                &mut preferences,
                &mut Actions::default(),
            );
        },
        |_| {},
    );
}

#[test]
#[ignore = "renders offscreen on the GPU: cargo test -p app ui_snapshots -- --ignored"]
fn settings_snapshots() {
    settings(
        "settings-general",
        Section::General,
        true,
        LoginItem::Disabled,
        None,
    );
    settings(
        "settings-general-approval",
        Section::General,
        true,
        LoginItem::RequiresApproval,
        Some("Could not change the login item: not allowed"),
    );
    settings(
        "settings-general-unbundled",
        Section::General,
        false,
        LoginItem::Unavailable,
        None,
    );
    settings(
        "settings-trigger",
        Section::Trigger,
        true,
        LoginItem::Enabled,
        None,
    );
    settings(
        "settings-sounds",
        Section::SoundsClipboard,
        true,
        LoginItem::Enabled,
        None,
    );
}

fn entries() -> Vec<Entry> {
    let entry = |id, text: &str, kind| Entry {
        id,
        text: text.to_owned(),
        created: 1_763_450_000 - id * 3_600,
        kind,
        model_id: None,
    };
    vec![
        entry(
            1,
            "ช่วยส่งรายงานประจำสัปดาห์ให้ทีมก่อนบ่ายสามโมงด้วยนะครับ",
            Kind::Pasted,
        ),
        entry(
            2,
            "เตือนให้โทรหาคุณสมชายเรื่องใบเสนอราคา และอย่าลืมถามเรื่องกำหนดส่งงานผู้ใหญ่ด้วย",
            Kind::Note,
        ),
        entry(
            3,
            "ขอ deploy ขึ้น production หลังจาก review pull request เสร็จแล้ว ผู้รับผิดชอบคือพี่ปิ่น",
            Kind::Pasted,
        ),
        entry(4, "Remember to renew the domain before Friday.", Kind::Note),
        entry(
            5,
            "สวัสดีครับ ผมอยากสอบถามเรื่องการเปลี่ยนแปลงตารางประชุมวันพฤหัสบดี ที่ผ่านมาติดภารกิจ",
            Kind::Pasted,
        ),
        entry(
            6,
            "นี่คือข้อความทดสอบ ภาษาไทย ผสม English words เช่น meeting, deadline และ sprint review",
            Kind::Pasted,
        ),
    ]
}

struct HistoryCase {
    name: &'static str,
    query: &'static str,
    entries: Vec<Entry>,
    confirming_clear: bool,
    unavailable: bool,
    size: Vec2,
    hover: Option<Pos2>,
    focus: bool,
}

fn history(case: HistoryCase) {
    let HistoryCase {
        name,
        query,
        entries,
        confirming_clear,
        unavailable,
        size,
        hover,
        focus,
    } = case;
    every_look(
        name,
        size,
        |ui, language| {
            let mut query = query.to_owned();
            let mut limit = 50;
            let mut confirming = confirming_clear;
            let mut copied = Some(2);
            let mut focus = focus;
            let mut state = history_window::State {
                query: &mut query,
                limit: &mut limit,
                confirming_clear: &mut confirming,
                copied: &mut copied,
                focus_search: &mut focus,
            };
            let mut outcome = HistoryOutcome::Idle;
            history_window::show(
                ui,
                language,
                &mut state,
                &entries,
                true,
                unavailable,
                &mut outcome,
            );
        },
        |harness| {
            if let Some(position) = hover {
                harness.hover_at(position);
                harness.run();
            }
        },
    );
}

#[test]
#[ignore = "renders offscreen on the GPU: cargo test -p app ui_snapshots -- --ignored"]
fn history_snapshots() {
    let size = vec2(560.0, 640.0);
    let base = || HistoryCase {
        name: "history-list",
        query: "",
        entries: entries(),
        confirming_clear: false,
        unavailable: false,
        size,
        hover: None,
        focus: false,
    };
    history(base());
    history(HistoryCase {
        name: "history-hover",
        hover: Some(pos2(280.0 + MARGIN, 250.0 + MARGIN)),
        ..base()
    });
    history(HistoryCase {
        name: "history-search",
        query: "รายงาน",
        entries: entries().into_iter().take(1).collect(),
        focus: true,
        ..base()
    });
    history(HistoryCase {
        name: "history-confirm",
        confirming_clear: true,
        ..base()
    });
    history(HistoryCase {
        name: "history-empty",
        entries: Vec::new(),
        ..base()
    });
    history(HistoryCase {
        name: "history-no-match",
        query: "ไม่มีคำนี้",
        entries: Vec::new(),
        ..base()
    });
    history(HistoryCase {
        name: "history-unavailable",
        unavailable: true,
        entries: Vec::new(),
        ..base()
    });
    history(HistoryCase {
        name: "history-small",
        size: vec2(400.0, 480.0),
        ..base()
    });
}

fn pill(ui: &mut egui::Ui, text: &str) {
    let font = egui::FontId::proportional(14.0);
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        font,
        egui::Color32::from_rgb(0xf3, 0xf3, 0xf8),
    );
    let size = vec2(
        (galley.size().x + 2.0 * 18.0 + 16.0).clamp(140.0, 520.0),
        34.0,
    );
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    painter.rect(
        rect,
        egui::CornerRadius::same(17),
        egui::Color32::from_rgb(0x0b, 0x0b, 0x11),
        egui::Stroke::new(1.0, egui::Color32::from_rgb(0x2a, 0x2a, 0x40)),
        egui::StrokeKind::Inside,
    );
    let start = rect.left() + (rect.width() - (8.0 + 8.0 + galley.size().x)) / 2.0;
    painter.circle_filled(
        pos2(start + 4.0, rect.center().y),
        4.0,
        egui::Color32::from_rgb(0x7a, 0x6b, 0xff),
    );
    painter.galley(
        pos2(start + 16.0, rect.center().y - galley.size().y / 2.0),
        galley,
        egui::Color32::WHITE,
    );
}

#[test]
#[ignore = "renders offscreen on the GPU: cargo test -p app ui_snapshots -- --ignored"]
fn pill_snapshots() {
    for (language, language_name) in LANGUAGES {
        let mut harness = harness(vec2(560.0, 200.0), Theme::Dark, |ui| {
            let background = egui::Color32::from_rgb(0x3a, 0x3d, 0x4a);
            ui.painter().rect_filled(ui.max_rect(), 0.0, background);
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 14.0;
                for state in [
                    platform::PillState::Listening,
                    platform::PillState::Partial(
                        "ช่วยส่งรายงานประจำสัปดาห์ให้ทีมก่อนบ่ายสามโมง".to_owned(),
                    ),
                    platform::PillState::Transcribing,
                ] {
                    if let Some(text) = language.pill_text(&state) {
                        pill(ui, &text);
                    }
                }
            });
        });
        save(&mut harness, &format!("pill-{language_name}"));
    }
}
