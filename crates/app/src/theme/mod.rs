use egui::{
    Align, Color32, CornerRadius, Frame, Id, Layout, Margin, Rect, Response, RichText, Role, Sense,
    Stroke, StrokeKind, TextEdit, Theme, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

pub mod icons;
pub mod keys;
pub mod mascot;

pub use icons::Icon;
pub use keys::{Glyph, Part};
pub use mascot::Pose;

pub const RADIUS: u8 = 8;
pub const RADIUS_LG: u8 = 14;
pub const RADIUS_FULL: u8 = 255;

pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_5: f32 = 20.0;
pub const SPACE_6: f32 = 24.0;
pub const SPACE_8: f32 = 32.0;

pub const TEXT_XS: f32 = 12.0;
pub const TEXT_SM: f32 = 13.0;
pub const TEXT_BASE: f32 = 14.0;
pub const TEXT_XL: f32 = 20.0;
pub const TEXT_TITLE: f32 = 26.0;

const WEIGHT_MEDIUM: f32 = 500.0;
const WEIGHT_BOLD: f32 = 650.0;

const CONTROL_HEIGHT: f32 = 36.0;
pub const DISPLAY_FAMILY: &str = "Mitr";
pub const WORDMARK_THAI: &str = "พิมพ์เสียง";

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub sidebar: Color32,
    pub card: Color32,
    pub card_hover: Color32,
    pub muted: Color32,
    pub muted_foreground: Color32,
    pub foreground: Color32,
    pub border: Color32,
    pub input: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_pressed: Color32,
    pub accent_foreground: Color32,
    pub accent_text: Color32,
    pub accent_soft: Color32,
    pub selection: Color32,
    pub knob_off: Color32,
    pub destructive: Color32,
    pub destructive_foreground: Color32,
    pub warning: Color32,
    pub ink2: Color32,
    pub faint: Color32,
    pub wattle: Color32,
    pub on_wattle: Color32,
    pub strip: Color32,
    pub strip_key: Color32,
    pub strip_text: Color32,
    pub ready_dot: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub const LIGHT: Palette = Palette {
    background: rgb(0xfbf7ee),
    sidebar: rgb(0xf6f1e4),
    card: rgb(0xffffff),
    card_hover: rgb(0xfffcf5),
    muted: rgb(0xf1ebdb),
    muted_foreground: rgb(0x6b645a),
    foreground: rgb(0x1c1a17),
    border: rgb(0xe8e0cf),
    input: rgb(0xd2c8b0),
    accent: rgb(0x1c1a17),
    accent_hover: rgb(0x36322b),
    accent_pressed: rgb(0x0d0c0a),
    accent_foreground: rgb(0xffd23f),
    accent_text: rgb(0x1c1a17),
    accent_soft: rgb(0xffe89a),
    selection: rgb(0xffe89a),
    knob_off: rgb(0xffffff),
    destructive: rgb(0xc8303a),
    destructive_foreground: rgb(0xffffff),
    warning: rgb(0xa85f00),
    ink2: rgb(0x4f483f),
    faint: rgb(0x8a8276),
    wattle: rgb(0xffd23f),
    on_wattle: rgb(0x1c1a17),
    strip: rgb(0x1c1a17),
    strip_key: rgb(0x2b2824),
    strip_text: rgb(0xd9d2c3),
    ready_dot: rgb(0x2e9e5b),
};

pub const DARK: Palette = Palette {
    background: rgb(0x16140f),
    sidebar: rgb(0x1b1914),
    card: rgb(0x1f1c17),
    card_hover: rgb(0x27241d),
    muted: rgb(0x2b2721),
    muted_foreground: rgb(0xa8a091),
    foreground: rgb(0xfbf7ee),
    border: rgb(0x332f27),
    input: rgb(0x4d473c),
    accent: rgb(0xffd23f),
    accent_hover: rgb(0xffdd6b),
    accent_pressed: rgb(0xe8b92a),
    accent_foreground: rgb(0x1c1a17),
    accent_text: rgb(0xffd23f),
    accent_soft: Color32::from_rgba_unmultiplied_const(0xff, 0xd2, 0x3f, 0x26),
    selection: Color32::from_rgba_unmultiplied_const(0xff, 0xd2, 0x3f, 0x4d),
    knob_off: rgb(0xc9c0ae),
    destructive: rgb(0xe5484d),
    destructive_foreground: rgb(0xffffff),
    warning: rgb(0xf28c28),
    ink2: rgb(0xd9d2c3),
    faint: rgb(0x8a8276),
    wattle: rgb(0xffd23f),
    on_wattle: rgb(0x1c1a17),
    strip: rgb(0x0f0e0b),
    strip_key: rgb(0x26231d),
    strip_text: rgb(0xd9d2c3),
    ready_dot: rgb(0x5fcb7e),
};

pub fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Light => LIGHT,
        Theme::Dark => DARK,
    }
}

pub fn colors(ui: &Ui) -> Palette {
    palette(ui.ctx().theme())
}

pub fn mix(from: Color32, to: Color32, amount: f32) -> Color32 {
    from.lerp_to_gamma(to, amount)
}

pub fn tint(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

pub fn apply(context: &egui::Context) {
    for theme in [Theme::Light, Theme::Dark] {
        context.set_style_of(theme, style(theme));
    }
}

fn style(theme: Theme) -> egui::Style {
    let colors = palette(theme);
    let mut style = egui::Style::default();
    style.spacing.item_spacing = vec2(SPACE_2, SPACE_2);
    style.spacing.button_padding = vec2(SPACE_4, SPACE_2);
    style.spacing.interact_size.y = CONTROL_HEIGHT;
    style.spacing.icon_width = 16.0;
    style.spacing.icon_spacing = SPACE_2;
    style.spacing.combo_width = 200.0;
    style.spacing.menu_margin = Margin::same(SPACE_1 as i8);
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating = true;
    for (text_style, size) in [
        (egui::TextStyle::Small, TEXT_XS),
        (egui::TextStyle::Body, TEXT_BASE),
        (egui::TextStyle::Button, TEXT_BASE),
        (egui::TextStyle::Monospace, TEXT_BASE),
        (egui::TextStyle::Heading, TEXT_XL),
    ] {
        if let Some(font) = style.text_styles.get_mut(&text_style) {
            font.size = size;
        }
    }

    let mut visuals = match theme {
        Theme::Light => egui::Visuals::light(),
        Theme::Dark => egui::Visuals::dark(),
    };
    let radius = CornerRadius::same(RADIUS);
    visuals.panel_fill = colors.background;
    visuals.window_fill = colors.card;
    visuals.window_stroke = Stroke::new(1.0, colors.border);
    visuals.window_corner_radius = radius;
    visuals.menu_corner_radius = radius;
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 6],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(match theme {
            Theme::Light => 36,
            Theme::Dark => 140,
        }),
    };
    visuals.extreme_bg_color = colors.card;
    visuals.faint_bg_color = colors.muted;
    visuals.code_bg_color = colors.muted;
    visuals.override_text_color = Some(colors.foreground);
    visuals.hyperlink_color = colors.accent_text;
    visuals.warn_fg_color = colors.warning;
    visuals.error_fg_color = colors.destructive;
    visuals.selection.bg_fill = colors.selection;
    visuals.selection.stroke = Stroke::new(1.0, colors.accent_text);
    visuals.text_cursor.stroke = Stroke::new(1.5, colors.accent_text);
    visuals.striped = false;
    visuals.slider_trailing_fill = true;

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = colors.card;
    widgets.noninteractive.weak_bg_fill = colors.card;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, colors.border);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors.foreground);
    widgets.noninteractive.corner_radius = radius;

    widgets.inactive.bg_fill = colors.card;
    widgets.inactive.weak_bg_fill = colors.card;
    widgets.inactive.bg_stroke = Stroke::new(1.0, colors.input);
    widgets.inactive.fg_stroke = Stroke::new(1.0, colors.foreground);
    widgets.inactive.corner_radius = radius;

    widgets.hovered.bg_fill = colors.card_hover;
    widgets.hovered.weak_bg_fill = colors.card_hover;
    widgets.hovered.bg_stroke = Stroke::new(1.0, colors.accent);
    widgets.hovered.fg_stroke = Stroke::new(1.0, colors.foreground);
    widgets.hovered.corner_radius = radius;
    widgets.hovered.expansion = 0.0;

    widgets.active.bg_fill = colors.muted;
    widgets.active.weak_bg_fill = colors.muted;
    widgets.active.bg_stroke = Stroke::new(1.0, colors.accent);
    widgets.active.fg_stroke = Stroke::new(1.0, colors.foreground);
    widgets.active.corner_radius = radius;
    widgets.active.expansion = 0.0;

    widgets.open.bg_fill = colors.card_hover;
    widgets.open.weak_bg_fill = colors.card_hover;
    widgets.open.bg_stroke = Stroke::new(1.0, colors.accent);
    widgets.open.fg_stroke = Stroke::new(1.0, colors.foreground);
    widgets.open.corner_radius = radius;

    style.visuals = visuals;
    style
}

pub fn medium(text: &str, size: f32) -> RichText {
    RichText::new(text)
        .size(size)
        .variation(b"wght", WEIGHT_MEDIUM)
}

pub fn bold(text: &str, size: f32) -> RichText {
    RichText::new(text)
        .size(size)
        .variation(b"wght", WEIGHT_BOLD)
}

pub fn display(text: &str, size: f32) -> RichText {
    RichText::new(text)
        .size(size)
        .family(egui::FontFamily::Name(DISPLAY_FAMILY.into()))
}

pub fn description(ui: &mut Ui, text: &str) {
    let color = colors(ui).muted_foreground;
    ui.label(RichText::new(text).size(TEXT_SM).color(color));
}

pub fn warning(ui: &mut Ui, text: &str) {
    let color = colors(ui).warning;
    ui.label(medium(text, TEXT_SM).color(color));
}

pub fn destructive(ui: &mut Ui, text: &str) {
    let color = colors(ui).destructive;
    ui.label(medium(text, TEXT_SM).color(color));
}

pub fn group<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let colors = colors(ui);
    Frame::new()
        .fill(colors.card)
        .stroke(Stroke::new(1.0, colors.border))
        .corner_radius(CornerRadius::same(RADIUS_LG))
        .inner_margin(Margin::symmetric(SPACE_4 as i8, 0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            add_contents(ui)
        })
        .inner
}

pub fn divider(ui: &mut Ui) {
    let color = colors(ui).border;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, color);
}

pub fn with_control<R>(
    ui: &mut Ui,
    control_width: f32,
    content: impl FnOnce(&mut Ui),
    control: impl FnOnce(&mut Ui) -> R,
) -> R {
    let left = ui.cursor().min.x;
    let width = ui.available_width();
    let content_width = (width - control_width - SPACE_4).max(80.0);
    let mut content_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(
                ui.cursor().min,
                vec2(content_width, ui.available_height()),
            ))
            .layout(Layout::top_down(Align::Min)),
    );
    content(&mut content_ui);
    let rect = content_ui.min_rect();
    let control_rect = Rect::from_min_max(
        pos2(left + width - control_width, rect.top()),
        pos2(left + width, rect.bottom()),
    );
    let mut control_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(control_rect)
            .layout(Layout::right_to_left(Align::Center)),
    );
    let result = control(&mut control_ui);
    ui.allocate_rect(rect.union(control_ui.min_rect()), Sense::hover());
    result
}

pub fn setting_row<R>(
    ui: &mut Ui,
    title: &str,
    description: Option<&str>,
    control_width: f32,
    control: impl FnOnce(&mut Ui) -> R,
) -> R {
    let colors = colors(ui);
    with_control(
        ui,
        control_width,
        |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.add_space(14.0);
            ui.label(medium(title, TEXT_BASE));
            if let Some(text) = description {
                ui.label(
                    RichText::new(text)
                        .size(TEXT_XS)
                        .color(colors.muted_foreground),
                );
            }
            ui.add_space(14.0);
        },
        control,
    )
}
pub fn button_width(ui: &Ui, text: &str) -> f32 {
    let galley = egui::WidgetText::from(bold(text, TEXT_SM)).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    galley.size().x + 2.0 * SPACE_4 + 2.0
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Primary,
    Secondary,
    Outline,
    Ghost,
    Destructive,
}

pub fn button(ui: &mut Ui, variant: Variant, text: &str) -> Response {
    button_enabled(ui, variant, true, text)
}

pub fn button_enabled(ui: &mut Ui, variant: Variant, enabled: bool, text: &str) -> Response {
    button_sized(ui, variant, enabled, text, CONTROL_HEIGHT, 10)
}

pub fn button_sized(
    ui: &mut Ui,
    variant: Variant,
    enabled: bool,
    text: &str,
    height: f32,
    radius: u8,
) -> Response {
    let colors = colors(ui);
    let transparent = Color32::TRANSPARENT;
    let (fill, hover, pressed, stroke, hover_stroke, text_color) = match variant {
        Variant::Primary => (
            colors.accent,
            colors.accent_hover,
            colors.accent_pressed,
            transparent,
            transparent,
            colors.accent_foreground,
        ),
        Variant::Secondary => (
            colors.muted,
            mix(colors.muted, colors.foreground, 0.08),
            mix(colors.muted, colors.foreground, 0.14),
            transparent,
            transparent,
            colors.foreground,
        ),
        Variant::Outline => (
            colors.card,
            colors.card_hover,
            colors.muted,
            colors.input,
            colors.accent,
            colors.foreground,
        ),
        Variant::Ghost => (
            transparent,
            colors.muted,
            mix(colors.muted, colors.foreground, 0.08),
            transparent,
            transparent,
            colors.foreground,
        ),
        Variant::Destructive => (
            colors.destructive,
            mix(colors.destructive, Color32::WHITE, 0.12),
            mix(colors.destructive, Color32::BLACK, 0.15),
            transparent,
            transparent,
            colors.destructive_foreground,
        ),
    };
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        for (state, background, border) in [
            (&mut widgets.inactive, fill, stroke),
            (&mut widgets.hovered, hover, hover_stroke),
            (&mut widgets.active, pressed, hover_stroke),
        ] {
            state.weak_bg_fill = background;
            state.bg_fill = background;
            state.bg_stroke = Stroke::new(1.5, border);
            state.corner_radius = CornerRadius::same(radius);
            state.expansion = 0.0;
        }
        ui.add_enabled(
            enabled,
            egui::Button::new(bold(text, TEXT_SM).color(text_color)).min_size(vec2(0.0, height)),
        )
    })
    .inner
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Primary,
    Danger,
}

pub fn row_action(
    ui: &mut Ui,
    id: Id,
    icon: Icon,
    text: Option<&str>,
    label: &str,
    tone: Tone,
    visible: bool,
) -> Response {
    let colors = colors(ui);
    let galley = text.map(|text| {
        egui::WidgetText::from(bold(text, TEXT_XS)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Small,
        )
    });
    let size = match &galley {
        Some(galley) => vec2(10.0 + 13.0 + 6.0 + galley.size().x + 10.0, 30.0),
        None => Vec2::splat(30.0),
    };
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, label));
    let shown = visible || response.has_focus();
    if shown {
        let hovered = response.hovered();
        let (fill, color) = match tone {
            Tone::Primary => (
                if hovered {
                    mix(colors.wattle, Color32::WHITE, 0.25)
                } else {
                    colors.wattle
                },
                colors.on_wattle,
            ),
            Tone::Danger => (
                if hovered {
                    tint(colors.destructive, 0x22)
                } else {
                    colors.sidebar
                },
                if hovered {
                    colors.destructive
                } else {
                    colors.ink2
                },
            ),
        };
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(8), fill);
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                CornerRadius::same(8),
                Stroke::new(1.5, colors.foreground),
                StrokeKind::Inside,
            );
        }
        match galley {
            Some(galley) => {
                let icon_rect = Rect::from_center_size(
                    pos2(rect.left() + 10.0 + 6.5, rect.center().y),
                    Vec2::splat(13.0),
                );
                icons::paint(painter, icon_rect, icon, color);
                painter.galley(
                    pos2(
                        icon_rect.right() + 6.0,
                        rect.center().y - galley.size().y / 2.0,
                    ),
                    galley,
                    color,
                );
            }
            None => icons::paint(
                painter,
                Rect::from_center_size(rect.center(), Vec2::splat(13.0)),
                icon,
                color,
            ),
        }
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn text_input(ui: &mut Ui, id: Id, value: &mut String, hint: &str, width: f32) -> Response {
    let colors = colors(ui);
    let focused = ui.memory(|memory| memory.has_focus(id));
    let radius = CornerRadius::same(12);
    let hint_parts = [Part::Glyph(Glyph::Command), Part::Text("F")];
    let hint_font = egui::FontId::proportional(11.0);
    let hint_width = keys::parts_width(ui, &hint_parts, &hint_font, 10.0) + 12.0;
    let outcome = Frame::new()
        .fill(colors.card)
        .stroke(Stroke::new(2.0, colors.accent))
        .corner_radius(radius)
        .inner_margin(Margin::symmetric(SPACE_3 as i8, 0))
        .show(ui, |ui| {
            ui.set_height(36.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = SPACE_2;
                icons::allocate(ui, Icon::Search, 16.0, colors.accent_text);
                let edit = ui.add(
                    TextEdit::singleline(value)
                        .id(id)
                        .hint_text(hint)
                        .frame(Frame::NONE)
                        .font(egui::FontId::proportional(TEXT_BASE))
                        .align(egui::Align2::LEFT_CENTER)
                        .desired_width(
                            (width - 2.0 * SPACE_3 - 4.0 - 16.0 - SPACE_2 - hint_width - SPACE_2)
                                .max(40.0),
                        ),
                );
                let (rect, _) = ui.allocate_exact_size(vec2(hint_width, 20.0), Sense::hover());
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(5),
                    Stroke::new(1.0, colors.border),
                    StrokeKind::Inside,
                );
                keys::paint_parts(
                    ui,
                    pos2(rect.left() + 6.0, rect.center().y),
                    &hint_parts,
                    &hint_font,
                    10.0,
                    colors.muted_foreground,
                );
                edit
            })
            .inner
        });
    if focused {
        ui.painter().rect_stroke(
            outcome.response.rect.expand(2.0),
            CornerRadius::same(14),
            Stroke::new(3.0, colors.accent_soft),
            StrokeKind::Outside,
        );
    }
    outcome.inner
}

pub fn toggle(ui: &mut Ui, on: &mut bool, enabled: bool, text: &str) -> Response {
    let colors = colors(ui);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, mut response) = ui.allocate_exact_size(vec2(40.0, 24.0), sense);
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(Role::CheckBox, enabled, *on, text));
    let progress = ui.ctx().animate_bool_responsive(response.id, *on);
    let hovered = enabled && response.hovered();
    let mut fill = mix(
        if hovered {
            mix(colors.input, colors.muted_foreground, 0.3)
        } else {
            colors.input
        },
        if hovered {
            colors.accent_hover
        } else {
            colors.accent
        },
        progress,
    );
    if !enabled {
        fill = fill.gamma_multiply(0.45);
    }
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(12), fill);
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(14),
            Stroke::new(2.0, colors.accent_soft),
            StrokeKind::Outside,
        );
    }
    let radius = 9.0;
    let x = egui::lerp(
        (rect.left() + radius + 3.0)..=(rect.right() - radius - 3.0),
        progress,
    );
    let knob = mix(colors.knob_off, colors.accent_foreground, progress);
    painter.circle_filled(
        pos2(x, rect.center().y),
        radius,
        if enabled {
            knob
        } else {
            knob.gamma_multiply(0.7)
        },
    );
    response
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Success,
    Note,
}

pub fn badge(ui: &mut Ui, kind: Badge, text: &str) {
    let colors = colors(ui);
    let (fill, color) = match kind {
        Badge::Success => (colors.accent_soft, colors.accent_text),
        Badge::Note => (colors.accent, colors.accent_foreground),
    };
    let galley = egui::WidgetText::from(bold(text, TEXT_XS).color(color)).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Small,
    );
    let size = vec2(
        galley.size().x + 2.0 * SPACE_2 + 2.0,
        galley.size().y.max(18.0) + 4.0,
    );
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(RADIUS_FULL), fill);
    painter.galley(
        pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
}
pub fn progress(ui: &mut Ui, fraction: f32) {
    let colors = colors(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::hover());
    let radius = CornerRadius::same(4);
    ui.painter().rect_filled(rect, radius, colors.muted);
    let width = (rect.width() * fraction.clamp(0.0, 1.0)).max(0.0);
    if width > 0.0 {
        let filled = Rect::from_min_size(rect.min, vec2(width.max(8.0), rect.height()));
        ui.painter().rect_filled(filled, radius, colors.accent);
    }
}
pub fn segments(ui: &mut Ui, done: usize, total: usize) {
    let colors = colors(ui);
    let gap = SPACE_1;
    let count = total.max(1);
    let width = 28.0;
    let size = vec2(count as f32 * width + (count as f32 - 1.0) * gap, 6.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    for index in 0..count {
        let x = rect.left() + index as f32 * (width + gap);
        let segment = Rect::from_min_size(pos2(x, rect.top()), vec2(width, rect.height()));
        ui.painter().rect_filled(
            segment,
            CornerRadius::same(3),
            if index < done {
                colors.accent
            } else {
                colors.border
            },
        );
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    Done,
    Current,
    Waiting,
}

pub fn step_marker(ui: &mut Ui, number: usize, marker: Marker) {
    let colors = colors(ui);
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
    let painter = ui.painter();
    match marker {
        Marker::Done => {
            painter.circle_filled(rect.center(), 14.0, colors.accent);
            icons::paint(
                painter,
                Rect::from_center_size(rect.center(), Vec2::splat(14.0)),
                Icon::Check,
                colors.accent_foreground,
            );
        }
        Marker::Current | Marker::Waiting => {
            let (ring, text) = if marker == Marker::Current {
                (colors.foreground, colors.foreground)
            } else {
                (colors.input, colors.faint)
            };
            painter.circle_stroke(rect.center(), 13.0, Stroke::new(2.0, ring));
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                number.to_string(),
                egui::FontId::proportional(TEXT_SM),
                text,
            );
        }
    }
}
pub fn nav_item(ui: &mut Ui, icon: Icon, text: &str, selected: bool) -> Response {
    let colors = colors(ui);
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Tab, true, selected, text));
    let hovered = response.hovered();
    let radius = CornerRadius::same(10);
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, radius, colors.wattle);
    } else if hovered {
        painter.rect_filled(rect, radius, tint(colors.foreground, 0x0e));
    }
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.5, colors.accent),
            StrokeKind::Inside,
        );
    }
    let color = if selected {
        colors.on_wattle
    } else {
        colors.ink2
    };
    let background = if selected {
        colors.wattle
    } else {
        colors.sidebar
    };
    let icon_rect = Rect::from_center_size(
        pos2(rect.left() + 10.0 + 9.0, rect.center().y),
        Vec2::splat(18.0),
    );
    icons::paint_on(painter, icon_rect, icon, color, background);
    let galley = egui::WidgetText::from(if selected {
        bold(text, TEXT_BASE).color(color)
    } else {
        medium(text, TEXT_BASE).color(color)
    })
    .into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        rect.width() - 20.0 - 18.0 - SPACE_3,
        egui::TextStyle::Button,
    );
    ui.painter().galley(
        pos2(
            icon_rect.right() + 10.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
pub fn banner(ui: &mut Ui, color: Color32, add_contents: impl FnOnce(&mut Ui)) {
    Frame::new()
        .fill(tint(color, 0x16))
        .stroke(Stroke::new(1.0, tint(color, 0x50)))
        .corner_radius(CornerRadius::same(RADIUS_LG))
        .inner_margin(Margin::same(SPACE_4 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = SPACE_3;
            add_contents(ui)
        });
}

pub fn row_end<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    ui.with_layout(Layout::right_to_left(Align::Center), add_contents)
        .inner
}

pub fn escape_pressed(ui: &Ui) -> bool {
    ui.input(|input| input.key_pressed(egui::Key::Escape))
}

pub fn close_pressed(ui: &Ui) -> bool {
    ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::W))
}

pub fn find_pressed(ui: &Ui) -> bool {
    ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::F))
}

pub fn page_title(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.spacing_mut().item_spacing.y = SPACE_1;
    ui.label(display(title, TEXT_TITLE));
    description(ui, subtitle);
}

pub fn section_label(ui: &mut Ui, text: &str) {
    let color = colors(ui).muted_foreground;
    ui.label(bold(text, TEXT_XS).color(color));
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChipTone {
    Ready,
    Busy,
    Missing,
}

pub fn status_chip(ui: &mut Ui, tone: ChipTone, text: &str) {
    let colors = colors(ui);
    let dot = match tone {
        ChipTone::Ready => colors.ready_dot,
        ChipTone::Busy => colors.warning,
        ChipTone::Missing => colors.destructive,
    };
    let width = ui.available_width();
    let galley = egui::WidgetText::from(RichText::new(text).size(TEXT_XS).color(colors.ink2))
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Wrap),
            width - 2.0 * 10.0 - 8.0 - SPACE_2,
            egui::TextStyle::Small,
        );
    let height = galley.size().y.max(18.0) + 20.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, text));
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(10),
        colors.card,
        Stroke::new(1.0, colors.border),
        StrokeKind::Inside,
    );
    painter.circle_filled(pos2(rect.left() + 10.0 + 4.0, rect.center().y), 4.0, dot);
    painter.galley(
        pos2(
            rect.left() + 10.0 + 8.0 + SPACE_2,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        colors.ink2,
    );
}

pub fn sidebar_brand(ui: &mut Ui, version: &str) {
    let muted = colors(ui).muted_foreground;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        mascot::show(ui, Pose::AppIcon, 36.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(display(WORDMARK_THAI, 17.0));
            ui.label(
                RichText::new(format!("PIMSIANG {version}"))
                    .size(11.0)
                    .color(muted),
            );
        });
    });
}

pub fn emphasised_line(ui: &mut Ui, parts: &[(&str, bool)], size: f32, color: Color32) {
    let mut job = egui::text::LayoutJob::default();
    for (text, strong) in parts {
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(size),
                color,
                coords: egui::epaint::text::VariationCoords::new(if *strong {
                    [(b"wght", WEIGHT_BOLD)]
                } else {
                    [(b"wght", 400.0)]
                }),
                ..Default::default()
            },
        );
    }
    ui.label(job);
}

pub fn language_card<T: PartialEq>(
    ui: &mut Ui,
    current: &mut T,
    value: T,
    title: &str,
    example: &[(&str, bool)],
    width: f32,
) -> Response {
    let colors = colors(ui);
    let selected = *current == value;
    let inner_width = width - 2.0 * SPACE_4 - 2.0;
    let title_galley = egui::WidgetText::from(bold(title, TEXT_BASE)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        inner_width - 24.0,
        egui::TextStyle::Body,
    );
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = inner_width;
    for (text, strong) in example {
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(TEXT_SM),
                color: colors.ink2,
                line_height: Some(20.0),
                coords: egui::epaint::text::VariationCoords::new(if *strong {
                    [(b"wght", WEIGHT_BOLD)]
                } else {
                    [(b"wght", 400.0)]
                }),
                ..Default::default()
            },
        );
    }
    let example_galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let height = 14.0 + title_galley.size().y.max(20.0) + 6.0 + example_galley.size().y + 14.0;
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if response.clicked() && !selected {
        *current = value;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, selected, title));
    let radius = CornerRadius::same(RADIUS_LG);
    let painter = ui.painter();
    let (fill, stroke) = if selected {
        (colors.card, Stroke::new(2.0, colors.accent))
    } else if response.hovered() {
        (colors.card_hover, Stroke::new(1.5, colors.input))
    } else {
        (colors.card, Stroke::new(1.5, colors.border))
    };
    painter.rect(rect, radius, fill, stroke, StrokeKind::Inside);
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(RADIUS_LG + 2),
            Stroke::new(2.0, colors.accent_soft),
            StrokeKind::Outside,
        );
    }
    let dot = pos2(rect.left() + SPACE_4 + 8.0, rect.top() + 14.0 + 10.0);
    painter.circle_stroke(
        dot,
        7.5,
        Stroke::new(
            1.5,
            if selected {
                colors.accent
            } else {
                colors.input
            },
        ),
    );
    if selected {
        painter.circle_filled(dot, 4.0, colors.accent);
    }
    painter.galley(
        pos2(
            dot.x + 8.0 + 8.0,
            rect.top() + 14.0 + 10.0 - title_galley.size().y / 2.0,
        ),
        title_galley,
        colors.foreground,
    );
    painter.galley(
        pos2(
            rect.left() + SPACE_4,
            rect.bottom() - 14.0 - example_galley.size().y,
        ),
        example_galley,
        colors.ink2,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    current: &mut T,
    options: &[(T, Vec<Part<'_>>)],
) -> bool {
    let colors = colors(ui);
    let mut changed = false;
    Frame::new()
        .fill(colors.muted)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(4))
        .show(ui, |ui| {
            let count = options.len().max(1) as f32;
            let gap = 4.0;
            let width = (ui.available_width() - gap * (count - 1.0)) / count;
            ui.spacing_mut().item_spacing.x = gap;
            ui.horizontal(|ui| {
                for (value, parts) in options {
                    let label = parts
                        .iter()
                        .map(|part| match part {
                            Part::Text(text) => *text,
                            Part::Glyph(_) => "",
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let selected = *current == *value;
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(width, 40.0), Sense::click());
                    response.widget_info(|| {
                        WidgetInfo::selected(Role::RadioButton, true, selected, &label)
                    });
                    if response.clicked() && !selected {
                        *current = *value;
                        changed = true;
                    }
                    let radius = CornerRadius::same(9);
                    let painter = ui.painter();
                    if selected {
                        painter.rect_filled(
                            rect.translate(vec2(0.0, 1.0)),
                            radius,
                            tint(colors.foreground, 0x22),
                        );
                        painter.rect_filled(rect, radius, colors.card);
                    } else if response.hovered() {
                        painter.rect_filled(rect, radius, tint(colors.foreground, 0x0a));
                    }
                    if response.has_focus() {
                        painter.rect_stroke(
                            rect,
                            radius,
                            Stroke::new(1.5, colors.accent),
                            StrokeKind::Inside,
                        );
                    }
                    let color = if selected {
                        colors.foreground
                    } else {
                        colors.ink2
                    };
                    let font = egui::FontId::proportional(TEXT_BASE);
                    let total = keys::parts_width(ui, parts, &font, TEXT_BASE);
                    keys::paint_parts(
                        ui,
                        pos2(rect.center().x - total / 2.0, rect.center().y),
                        parts,
                        &font,
                        TEXT_BASE,
                        color,
                    );
                    response.on_hover_cursor(egui::CursorIcon::PointingHand);
                }
            });
        });
    changed
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StripKey {
    Option,
    Command,
    Control,
}

pub fn keyboard_strip(ui: &mut Ui, chosen: StripKey, tiles: [(Option<Glyph>, &str, &str); 3]) {
    let colors = colors(ui);
    Frame::new()
        .fill(colors.strip)
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin {
            left: 18,
            right: 18,
            top: 18,
            bottom: 16,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 14.0;
            keys_row(ui, chosen);
            gesture_tiles(ui, tiles);
        });
}

fn keys_row(ui: &mut Ui, chosen: StripKey) {
    let colors = colors(ui);
    let gap = 6.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
    let left: [(Part<'static>, f32); 4] = [
        (Part::Text("fn"), 44.0),
        (Part::Glyph(Glyph::Control), 44.0),
        (Part::Glyph(Glyph::Option), 44.0),
        (Part::Glyph(Glyph::Command), 56.0),
    ];
    let right: [(Glyph, f32, StripKey); 3] = [
        (Glyph::Command, 56.0, StripKey::Command),
        (Glyph::Option, 52.0, StripKey::Option),
        (Glyph::Control, 44.0, StripKey::Control),
    ];
    let key = |ui: &Ui, x: f32, width: f32, label: Option<Part<'_>>, active: bool| {
        let key_rect = Rect::from_min_size(pos2(x, rect.top()), vec2(width, rect.height()));
        let painter = ui.painter();
        if active {
            painter.rect_stroke(
                key_rect.expand(1.5),
                CornerRadius::same(9),
                Stroke::new(3.0, tint(colors.accent_soft, 0x55)),
                StrokeKind::Outside,
            );
        }
        painter.rect_filled(
            key_rect,
            CornerRadius::same(8),
            if active {
                colors.wattle
            } else {
                colors.strip_key
            },
        );
        if let Some(label) = label {
            let (size, color) = if active {
                (12.0, colors.on_wattle)
            } else {
                (11.0, colors.faint)
            };
            keys::paint_parts(
                ui,
                pos2(
                    key_rect.left() + 6.0 + 1.0,
                    key_rect.bottom() - 5.0 - size / 2.0,
                ),
                &[label],
                &egui::FontId::proportional(size),
                size,
                color,
            );
        }
    };
    let mut x = rect.left();
    for (label, width) in left {
        key(ui, x, width, Some(label), false);
        x += width + gap;
    }
    let right_width: f32 = right.iter().map(|(_, width, _)| width + gap).sum::<f32>() - gap;
    let mut x_right = rect.right() - right_width;
    let space = Rect::from_min_max(pos2(x, rect.top()), pos2(x_right - gap, rect.bottom()));
    ui.painter()
        .rect_filled(space, CornerRadius::same(8), colors.strip_key);
    for (glyph, width, which) in right {
        key(
            ui,
            x_right,
            width,
            Some(Part::Glyph(glyph)),
            which == chosen,
        );
        x_right += width + gap;
    }
}

fn gesture_tiles(ui: &mut Ui, tiles: [(Option<Glyph>, &str, &str); 3]) {
    let colors = colors(ui);
    let gap = 8.0;
    let width = (ui.available_width() - 2.0 * gap) / 3.0;
    let galleys: Vec<_> = tiles
        .iter()
        .map(|(glyph, title, body)| {
            let indent = if glyph.is_some() { TEXT_SM + 4.0 } else { 0.0 };
            let title = egui::WidgetText::from(bold(title, TEXT_SM).color(colors.wattle))
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Wrap),
                    width - 24.0,
                    egui::TextStyle::Body,
                );
            let mut job = egui::text::LayoutJob::simple(
                (*body).to_owned(),
                egui::FontId::proportional(TEXT_XS),
                colors.strip_text,
                width - 24.0,
            );
            job.sections[0].format.line_height = Some(18.0);
            (
                *glyph,
                indent,
                title,
                ui.fonts_mut(|fonts| fonts.layout_job(job)),
            )
        })
        .collect();
    let height = galleys
        .iter()
        .map(|(_, _, title, body)| title.size().y + 3.0 + body.size().y)
        .fold(0.0, f32::max)
        + 20.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    for (index, (glyph, indent, title, body)) in galleys.into_iter().enumerate() {
        let tile = Rect::from_min_size(
            pos2(rect.left() + index as f32 * (width + gap), rect.top()),
            vec2(width, height),
        );
        ui.painter()
            .rect_filled(tile, CornerRadius::same(10), colors.strip_key);
        let title_height = title.size().y;
        if let Some(glyph) = glyph {
            keys::paint_glyph(
                ui.painter(),
                tile.min + vec2(12.0 + TEXT_SM / 2.0, 10.0 + title_height / 2.0),
                TEXT_SM,
                glyph,
                colors.wattle,
            );
        }
        ui.painter()
            .galley(tile.min + vec2(12.0 + indent, 10.0), title, colors.wattle);
        ui.painter().galley(
            tile.min + vec2(12.0, 10.0 + title_height + 3.0),
            body,
            colors.strip_text,
        );
    }
}

pub fn highlighted(
    ui: &mut Ui,
    text: &str,
    ranges: &[std::ops::Range<usize>],
    size: f32,
) -> Response {
    let colors = colors(ui);
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    let plain = egui::TextFormat {
        font_id: egui::FontId::proportional(size),
        color: colors.foreground,
        line_height: Some((size * 1.65).round()),
        ..Default::default()
    };
    let marked = egui::TextFormat {
        background: colors.selection,
        expand_bg: 0.0,
        ..plain.clone()
    };
    let mut at = 0;
    for range in ranges {
        if range.start > at {
            job.append(&text[at..range.start], 0.0, plain.clone());
        }
        job.append(&text[range.clone()], 0.0, marked.clone());
        at = range.end;
    }
    if at < text.len() {
        job.append(&text[at..], 0.0, plain);
    }
    ui.label(job)
}
