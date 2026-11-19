use std::path::PathBuf;

use app::history_view::Page;
use app::i18n::Language;
use app::model_chip::ModelChip;
use app::model_download::{self, ModelStatus};
use app::onboarding;
use app::preferences::Preferences;
use egui::{Pos2, Theme, Vec2, pos2, vec2};
use egui_kittest::Harness;
use platform::{PermissionReport, PermissionStatus, PillPlacement, PillPosition, TriggerKey};
use platform_macos::login_item::LoginItem;
use store::{Entry, Kind};

use crate::history_window::{self, Clock, Outcome as HistoryOutcome};
use crate::settings_window::{self, Actions, Build, Inputs, Section};
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
        vec2(560.0, 800.0),
        |ui, language| {
            let view = onboarding::view(
                language,
                TriggerKey::RightOption,
                &report,
                bundled,
                &model,
                model_download::download_bytes(engine::Precision::Int8),
            );
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
            done: 410_000_000,
            total: 994_516_953,
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

struct SettingsCase {
    name: &'static str,
    section: Section,
    bundled: bool,
    login: LoginItem,
    error: Option<&'static str>,
    model: ModelChip,
    build: Option<Build>,
    precision: store::Precision,
    height: f32,
    placement: PillPlacement,
    position: PillPosition,
    placing: bool,
}

fn settings(case: SettingsCase) {
    let SettingsCase {
        name,
        section,
        bundled,
        login,
        error,
        model,
        build,
        precision,
        height,
        placement,
        position,
        placing,
    } = case;
    every_look(
        name,
        vec2(780.0, height),
        |ui, language| {
            let inputs = Inputs {
                language,
                bundled,
                login,
                error: error.map(str::to_owned),
                model,
                build: build.clone(),
                version: "0.1",
                placing,
                pill_preview: None,
            };
            let mut section = section;
            let mut preferences = Preferences {
                sounds: false,
                pill_placement: placement,
                pill_position: position,
                precision,
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
    let case = |name, section, login, error, model| SettingsCase {
        name,
        section,
        bundled: true,
        login,
        error,
        model,
        build: None,
        precision: store::Precision::Light,
        height: 560.0,
        placement: PillPlacement::Floating,
        position: PillPosition::default(),
        placing: false,
    };
    let general = |case: SettingsCase| SettingsCase {
        height: 960.0,
        ..case
    };
    settings(general(case(
        "settings-general",
        Section::General,
        LoginItem::Disabled,
        None,
        ModelChip::Ready,
    )));
    settings(general(case(
        "settings-general-approval",
        Section::General,
        LoginItem::RequiresApproval,
        Some("Could not change the login item: not allowed"),
        ModelChip::Downloading { percent: 41 },
    )));
    settings(general(SettingsCase {
        bundled: false,
        ..case(
            "settings-general-unbundled",
            Section::General,
            LoginItem::Unavailable,
            None,
            ModelChip::Loading,
        )
    }));
    settings(general(SettingsCase {
        precision: store::Precision::Light,
        build: Some(Build {
            status: ModelStatus::Downloading {
                done: 1_070_000_000,
                total: 2_606_098_275,
            },
            keeps_working: true,
        }),
        ..case(
            "settings-general-precision-downloading",
            Section::General,
            LoginItem::Disabled,
            None,
            ModelChip::Downloading { percent: 41 },
        )
    }));
    settings(general(SettingsCase {
        build: Some(Build {
            status: ModelStatus::Failed("the server answered 500".to_owned()),
            keeps_working: true,
        }),
        ..case(
            "settings-general-precision-failed",
            Section::General,
            LoginItem::Disabled,
            None,
            ModelChip::Ready,
        )
    }));
    settings(general(SettingsCase {
        placement: PillPlacement::Notch,
        ..case(
            "settings-general-pill-notch",
            Section::General,
            LoginItem::Disabled,
            None,
            ModelChip::Ready,
        )
    }));
    settings(general(SettingsCase {
        position: PillPosition { x: 0.78, y: 0.7 },
        placing: true,
        ..case(
            "settings-general-pill-placing",
            Section::General,
            LoginItem::Disabled,
            None,
            ModelChip::Ready,
        )
    }));
    settings(case(
        "settings-trigger",
        Section::Trigger,
        LoginItem::Enabled,
        None,
        ModelChip::Missing,
    ));
    settings(case(
        "settings-sounds",
        Section::SoundsClipboard,
        LoginItem::Enabled,
        None,
        ModelChip::Ready,
    ));
}

const NOW: i64 = 1_763_468_640;

fn clock() -> Clock {
    Clock {
        now: NOW,
        zone: history_window::Zone::Fixed(
            chrono::FixedOffset::east_opt(7 * 3600).expect("a valid offset"),
        ),
    }
}

fn at(day: u32, hour: u32, minute: u32) -> i64 {
    use chrono::TimeZone;
    chrono::FixedOffset::east_opt(7 * 3600)
        .expect("a valid offset")
        .with_ymd_and_hms(2025, 11, day, hour, minute, 0)
        .single()
        .expect("a valid time")
        .timestamp()
}

fn entries() -> Vec<Entry> {
    let entry = |id, created, text: &str, kind| Entry {
        id,
        text: text.to_owned(),
        created,
        kind,
        model_id: None,
    };
    vec![
        entry(
            1,
            at(18, 13, 13),
            "ช่วยส่งรายงานประจำสัปดาห์ให้ทีมก่อนบ่ายสามโมงด้วยนะครับ",
            Kind::Pasted,
        ),
        entry(
            2,
            at(18, 11, 42),
            "ร่างรายงานไตรมาสสามให้เสร็จก่อนประชุมวันศุกร์ แล้วส่งให้พี่ปิ่นดูก่อน",
            Kind::Note,
        ),
        entry(
            3,
            at(17, 17, 5),
            "ขอ deploy ขึ้น production หลัง review pull request เสร็จ แนบรายงานผลเทสไว้ในการ์ดด้วย",
            Kind::Pasted,
        ),
        entry(
            4,
            at(17, 9, 31),
            "ส่งรายงานค่าใช้จ่ายเดือนตุลาคมให้บัญชีภายในวันพุธ",
            Kind::Pasted,
        ),
        entry(
            5,
            at(16, 15, 20),
            "สวัสดีครับ ผมอยากสอบถามเรื่องการเปลี่ยนแปลงตารางประชุมวันพฤหัสบดี ที่ผ่านมาติดภารกิจ",
            Kind::Pasted,
        ),
        entry(
            6,
            at(3, 10, 2),
            "นี่คือข้อความทดสอบ ภาษาไทย ผสม English words เช่น meeting, deadline และ sprint review",
            Kind::Pasted,
        ),
        entry(
            7,
            at(2, 8, 45),
            "Remember to renew the domain before Friday.",
            Kind::Note,
        ),
    ]
}

struct HistoryCase {
    name: &'static str,
    query: &'static str,
    entries: Vec<Entry>,
    matched: usize,
    total: usize,
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
        matched,
        total,
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
                clock: clock(),
            };
            let page = Page {
                entries: entries.clone(),
                matched,
                total,
            };
            let mut outcome = HistoryOutcome::Idle;
            history_window::show(
                ui,
                language,
                &mut state,
                &page,
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
    let size = vec2(640.0, 720.0);
    let base = || HistoryCase {
        name: "history-list",
        query: "",
        entries: entries(),
        matched: 7,
        total: 128,
        confirming_clear: false,
        unavailable: false,
        size,
        hover: None,
        focus: false,
    };
    history(base());
    history(HistoryCase {
        name: "history-hover",
        hover: Some(pos2(320.0 + MARGIN, 130.0 + MARGIN)),
        ..base()
    });
    history(HistoryCase {
        name: "history-search",
        query: "รายงาน",
        entries: entries()
            .into_iter()
            .filter(|entry| entry.text.contains("รายงาน"))
            .collect(),
        matched: 4,
        focus: true,
        hover: Some(pos2(320.0 + MARGIN, 170.0 + MARGIN)),
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
        matched: 0,
        total: 0,
        ..base()
    });
    history(HistoryCase {
        name: "history-no-match",
        query: "ไม่มีคำนี้",
        entries: Vec::new(),
        matched: 0,
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
        size: vec2(420.0, 520.0),
        ..base()
    });
}

fn pill(ui: &mut egui::Ui, content: &platform::PillContent, art: platform_macos::PillArt) {
    let paper = egui::Color32::from_rgb(0xfb, 0xf7, 0xee);
    let faded = egui::Color32::from_rgb(0x8a, 0x82, 0x76);
    let wattle = egui::Color32::from_rgb(0xff, 0xd2, 0x3f);
    let font = egui::FontId::proportional(15.0);
    let mut job = egui::text::LayoutJob::default();
    for (text, color) in [(&content.settled, paper), (&content.tentative, faded)] {
        if !text.is_empty() {
            job.append(
                text,
                0.0,
                egui::TextFormat {
                    font_id: font.clone(),
                    color,
                    ..Default::default()
                },
            );
        }
    }
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let badge = content.badge.as_ref().map(|text| {
        ui.painter().layout_no_wrap(
            text.clone(),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(0x1c, 0x1a, 0x17),
        )
    });
    let trailing = if content.caret {
        12.0
    } else if content.dots {
        39.0
    } else if let Some(badge) = &badge {
        badge.size().x + 26.0
    } else {
        0.0
    };
    let width = (8.0 + 32.0 + 10.0 + galley.size().x + trailing + 18.0).min(520.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 44.0), egui::Sense::hover());
    let key = egui::Id::unique(("pill-art", format!("{art:?}")));
    let texture = match ui
        .ctx()
        .data(|data| data.get_temp::<egui::TextureHandle>(key))
    {
        Some(texture) => texture,
        None => {
            let rgba = image::load_from_memory(art.png())
                .expect("decoding the Pill art")
                .to_rgba8();
            let texture = ui.ctx().load_texture(
                format!("pill-{art:?}"),
                egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            );
            ui.ctx()
                .data_mut(|data| data.insert_temp(key, texture.clone()));
            texture
        }
    };
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(22),
        egui::Color32::from_rgb(0x1c, 0x1a, 0x17),
    );
    egui::Image::new(&texture).paint_at(
        ui,
        egui::Rect::from_min_size(
            pos2(rect.left() + 8.0, rect.center().y - 16.0),
            vec2(32.0, 32.0),
        ),
    );
    let text_left = rect.left() + 8.0 + 32.0 + 10.0;
    let text_width = galley.size().x;
    ui.painter().galley(
        pos2(text_left, rect.center().y - galley.size().y / 2.0),
        galley,
        paper,
    );
    let mut x = text_left + text_width + 10.0;
    if content.caret {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(pos2(x - 8.0, rect.center().y - 9.0), vec2(2.0, 18.0)),
            0.0,
            wattle,
        );
    } else if content.dots {
        for alpha in [1.0, 0.6, 0.3] {
            ui.painter().circle_filled(
                pos2(x + 2.5, rect.center().y),
                2.5,
                wattle.gamma_multiply(alpha),
            );
            x += 8.0;
        }
    } else if let Some(badge) = badge {
        let badge_rect = egui::Rect::from_center_size(
            pos2(x + (badge.size().x + 16.0) / 2.0, rect.center().y),
            vec2(badge.size().x + 16.0, badge.size().y + 4.0),
        );
        ui.painter()
            .rect_filled(badge_rect, egui::CornerRadius::same(255), wattle);
        ui.painter().galley(
            badge_rect.center() - badge.size() / 2.0,
            badge,
            egui::Color32::from_rgb(0x1c, 0x1a, 0x17),
        );
    }
}

#[test]
#[ignore = "renders offscreen on the GPU: cargo test -p app ui_snapshots -- --ignored"]
fn pill_snapshots() {
    for (language, language_name) in LANGUAGES {
        let mut harness = harness(vec2(560.0, 260.0), Theme::Dark, |ui| {
            let background = egui::Color32::from_rgb(0x3a, 0x3d, 0x4a);
            ui.painter().rect_filled(ui.max_rect(), 0.0, background);
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 14.0;
                for state in [
                    platform::PillState::Partial(platform::PartialText::new(
                        "ช่วยส่งรายงานประจำสัปดาห์ให้ทีมก่อนบ่ายสามโมง",
                        "ช่วยส่งรายงานประจำสัปดาห์ให้ทีมก่อนบ่าย".len(),
                    )),
                    platform::PillState::Transcribing,
                    platform::PillState::Notice(platform::PillNotice::QuickNoteSaved),
                    platform::PillState::Error(platform::PillError::PermissionMissing(
                        platform::Permission::Microphone,
                    )),
                ] {
                    if let Some(content) = language.pill_content(&state) {
                        pill(ui, &content, platform_macos::PillArt::for_state(&state));
                    }
                }
            });
        });
        harness.run();
        save(&mut harness, &format!("pill-{language_name}"));
    }
}
