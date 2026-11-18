use egui::{
    Align, Color32, CornerRadius, Frame, Id, Layout, Margin, Mesh, Rect, Response, RichText, Role,
    Sense, Shape, Stroke, StrokeKind, TextEdit, Theme, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

pub mod icons;
pub mod mascot;

pub use icons::Icon;
pub use mascot::Pose;

pub const RADIUS: u8 = 8;
pub const RADIUS_LG: u8 = 12;
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
pub const TEXT_LG: f32 = 16.0;
pub const TEXT_XL: f32 = 20.0;
pub const TEXT_TITLE: f32 = 26.0;

const WEIGHT_MEDIUM: f32 = 500.0;
const WEIGHT_BOLD: f32 = 650.0;

const CONTROL_HEIGHT: f32 = 34.0;
pub const HEADER_HEIGHT: f32 = 60.0;
pub const DISPLAY_FAMILY: &str = "Mitr";
pub const WORDMARK: &str = "PimSiang";
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
    pub success: Color32,
    pub warning: Color32,
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
    success: rgb(0x2e7d3a),
    warning: rgb(0xa85f00),
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
    success: rgb(0x5fcb7e),
    warning: rgb(0xf28c28),
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

pub fn title(ui: &mut Ui, text: &str) {
    ui.label(display(text, TEXT_TITLE));
}

pub fn heading(ui: &mut Ui, text: &str) {
    ui.label(bold(text, TEXT_LG));
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

pub fn success(ui: &mut Ui, text: &str) {
    let color = colors(ui).success;
    ui.label(bold(text, TEXT_LG).color(color));
}

pub fn gradient(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

pub fn logo(ui: &mut Ui, size: f32) {
    mascot::show(ui, Pose::HeadDefault, size);
}

pub fn wordmark(ui: &mut Ui) {
    let muted = colors(ui).muted_foreground;
    ui.spacing_mut().item_spacing.x = SPACE_2;
    ui.label(display(WORDMARK, TEXT_LG));
    ui.label(RichText::new(WORDMARK_THAI).size(TEXT_SM).color(muted));
}

pub fn brand(ui: &mut Ui, logo_size: f32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACE_3;
        logo(ui, logo_size);
        wordmark(ui);
    });
}

pub fn header<R>(ui: &mut Ui, add_trailing: impl FnOnce(&mut Ui) -> R) -> R {
    let colors = colors(ui);
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), HEADER_HEIGHT), Sense::hover());
    gradient(ui.painter(), rect, colors.accent_soft, Color32::TRANSPARENT);
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, colors.border),
    );
    let inner = rect.shrink2(vec2(SPACE_6, 0.0));
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            brand(ui, 30.0);
            ui.with_layout(Layout::right_to_left(Align::Center), add_trailing)
                .inner
        },
    )
    .inner
}

pub fn card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let colors = colors(ui);
    Frame::new()
        .fill(colors.card)
        .stroke(Stroke::new(1.0, colors.border))
        .corner_radius(CornerRadius::same(RADIUS_LG))
        .inner_margin(Margin::same(SPACE_4 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = SPACE_2;
            add_contents(ui)
        })
        .inner
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
    let content = ui.allocate_ui_with_layout(
        vec2(content_width, 0.0),
        Layout::top_down(Align::Min),
        content,
    );
    let rect = content.response.rect;
    let control_rect = Rect::from_min_max(
        pos2(left + width - control_width, rect.top()),
        pos2(left + width, rect.bottom()),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(control_rect)
            .layout(Layout::right_to_left(Align::Center)),
        control,
    )
    .inner
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
            ui.add_space(SPACE_4);
            ui.label(medium(title, TEXT_BASE));
            if let Some(text) = description {
                ui.label(
                    RichText::new(text)
                        .size(TEXT_SM)
                        .color(colors.muted_foreground),
                );
            }
            ui.add_space(SPACE_4);
        },
        control,
    )
}

pub fn button_width(ui: &Ui, text: &str) -> f32 {
    let galley = egui::WidgetText::from(medium(text, TEXT_BASE)).into_galley(
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
            state.bg_stroke = Stroke::new(1.0, border);
            state.expansion = 0.0;
        }
        ui.add_enabled(
            enabled,
            egui::Button::new(medium(text, TEXT_BASE).color(text_color))
                .min_size(vec2(0.0, CONTROL_HEIGHT)),
        )
    })
    .inner
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Danger,
    Success,
}

pub fn action(ui: &mut Ui, icon: Icon, text: &str, tone: Tone) -> Response {
    let colors = colors(ui);
    let galley = egui::WidgetText::from(medium(text, TEXT_SM)).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let icon_size = 14.0;
    let padding = vec2(SPACE_2, 0.0);
    let size = vec2(
        padding.x * 2.0 + icon_size + SPACE_1 + galley.size().x,
        28.0,
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, text));
    let hovered = response.hovered() || response.has_focus();
    let (resting, active, soft) = match tone {
        Tone::Neutral => (colors.muted_foreground, colors.foreground, colors.muted),
        Tone::Danger => (
            colors.muted_foreground,
            colors.destructive,
            tint(colors.destructive, 0x22),
        ),
        Tone::Success => (colors.success, colors.success, tint(colors.success, 0x1c)),
    };
    let color = if hovered { active } else { resting };
    let painter = ui.painter();
    if hovered {
        painter.rect_filled(rect, CornerRadius::same(6), soft);
    }
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            CornerRadius::same(6),
            Stroke::new(1.5, colors.accent),
            StrokeKind::Inside,
        );
    }
    let icon_rect = Rect::from_center_size(
        pos2(rect.left() + padding.x + icon_size / 2.0, rect.center().y),
        Vec2::splat(icon_size),
    );
    icons::paint(painter, icon_rect, icon, color);
    painter.galley(
        pos2(
            icon_rect.right() + SPACE_1,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn text_input(ui: &mut Ui, id: Id, value: &mut String, hint: &str, width: f32) -> Response {
    let colors = colors(ui);
    let focused = ui.memory(|memory| memory.has_focus(id));
    let stroke = if focused {
        Stroke::new(1.5, colors.accent)
    } else {
        Stroke::new(1.0, colors.input)
    };
    let radius = CornerRadius::same(RADIUS);
    let outcome = Frame::new()
        .fill(colors.card)
        .stroke(stroke)
        .corner_radius(radius)
        .inner_margin(Margin::symmetric(SPACE_3 as i8, 0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = SPACE_2;
                icons::allocate(
                    ui,
                    Icon::Search,
                    16.0,
                    if focused {
                        colors.accent_text
                    } else {
                        colors.muted_foreground
                    },
                );
                ui.add(
                    TextEdit::singleline(value)
                        .id(id)
                        .hint_text(hint)
                        .frame(Frame::NONE)
                        .font(egui::FontId::proportional(TEXT_BASE))
                        .align(egui::Align2::LEFT_CENTER)
                        .desired_width((width - 2.0 * SPACE_3 - 16.0 - SPACE_2 - 4.0).max(40.0)),
                )
            })
            .inner
        });
    if focused {
        ui.painter().rect_stroke(
            outcome.response.rect.expand(2.0),
            CornerRadius::same(RADIUS + 2),
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
    let (rect, mut response) = ui.allocate_exact_size(vec2(40.0, 22.0), sense);
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
    painter.rect_filled(rect, CornerRadius::same(RADIUS_FULL), fill);
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(2.0),
            CornerRadius::same(RADIUS_FULL),
            Stroke::new(2.0, colors.accent_soft),
            StrokeKind::Outside,
        );
    }
    let radius = 8.0;
    let x = egui::lerp(
        (rect.left() + radius + 3.0)..=(rect.right() - radius - 3.0),
        progress,
    );
    let center = pos2(x, rect.center().y);
    painter.circle_filled(
        center + vec2(0.0, 1.0),
        radius,
        Color32::from_black_alpha(40),
    );
    let knob = mix(colors.knob_off, colors.accent_foreground, progress);
    painter.circle_filled(
        center,
        radius,
        if enabled {
            knob
        } else {
            knob.gamma_multiply(0.7)
        },
    );
    response
}

pub fn select(
    ui: &mut Ui,
    salt: &str,
    name: &str,
    selected: &str,
    add_contents: impl FnOnce(&mut Ui),
) -> Response {
    let response = egui::ComboBox::from_id_salt(salt)
        .selected_text(RichText::new(selected).size(TEXT_BASE))
        .width(200.0)
        .show_ui(ui, add_contents)
        .response;
    response.widget_info(|| WidgetInfo::labeled(Role::ComboBox, true, name));
    response
}

pub fn option<T: PartialEq>(ui: &mut Ui, current: &mut T, value: T, text: &str) -> Response {
    ui.selectable_value(current, value, RichText::new(text).size(TEXT_BASE))
}

pub fn choice<T: PartialEq>(ui: &mut Ui, current: &mut T, value: T, text: &str) -> Response {
    let colors = colors(ui);
    let selected = *current == value;
    let width = ui.available_width();
    let text_width = width - 2.0 * SPACE_4 - 20.0 - SPACE_3;
    let galley = egui::WidgetText::from(RichText::new(text).size(TEXT_BASE)).into_galley(
        ui,
        Some(egui::TextWrapMode::Wrap),
        text_width,
        egui::TextStyle::Body,
    );
    let height = (galley.size().y + 2.0 * SPACE_3).max(46.0);
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if response.clicked() && !selected {
        *current = value;
        response.mark_changed();
    }
    response.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, selected, text));
    let hovered = response.hovered();
    let radius = CornerRadius::same(RADIUS_LG);
    let painter = ui.painter();
    let (fill, stroke) = if selected {
        (colors.accent_soft, Stroke::new(1.5, colors.accent))
    } else if hovered {
        (colors.card_hover, Stroke::new(1.0, colors.input))
    } else {
        (colors.card, Stroke::new(1.0, colors.border))
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
    let dot = pos2(rect.left() + SPACE_4 + 10.0, rect.center().y);
    painter.circle_stroke(
        dot,
        9.0,
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
        painter.circle_filled(dot, 4.5, colors.accent);
    }
    painter.galley(
        pos2(
            rect.left() + SPACE_4 + 20.0 + SPACE_3,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        colors.foreground,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Success,
    Neutral,
    Warning,
    Destructive,
}

pub fn badge(ui: &mut Ui, kind: Badge, text: &str) {
    let colors = colors(ui);
    let (fill, color) = match kind {
        Badge::Success => (tint(colors.success, 0x26), colors.success),
        Badge::Neutral => (colors.muted, colors.muted_foreground),
        Badge::Warning => (tint(colors.warning, 0x26), colors.warning),
        Badge::Destructive => (tint(colors.destructive, 0x26), colors.destructive),
    };
    let galley = egui::WidgetText::from(medium(text, TEXT_XS).color(color)).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Small,
    );
    let size = vec2(10.0 + 6.0 + 6.0 + galley.size().x + 10.0, 22.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(RADIUS_FULL), fill);
    painter.circle_filled(pos2(rect.left() + 13.0, rect.center().y), 3.0, color);
    painter.galley(
        pos2(
            rect.left() + 22.0,
            rect.center().y - galley.size().y / 2.0 - 0.5,
        ),
        galley,
        color,
    );
}

pub fn progress(ui: &mut Ui, fraction: f32) {
    let colors = colors(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
    let radius = CornerRadius::same(RADIUS_FULL);
    ui.painter().rect_filled(rect, radius, colors.muted);
    let width = (rect.width() * fraction.clamp(0.0, 1.0)).max(0.0);
    if width > 0.0 {
        let filled = Rect::from_min_size(rect.min, vec2(width.max(6.0), rect.height()));
        ui.painter().rect_filled(filled, radius, colors.accent);
    }
}

pub fn segments(ui: &mut Ui, done: usize, total: usize) {
    let colors = colors(ui);
    let gap = SPACE_1;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
    let count = total.max(1);
    let width = (rect.width() - gap * (count as f32 - 1.0)) / count as f32;
    for index in 0..count {
        let x = rect.left() + index as f32 * (width + gap);
        let segment = Rect::from_min_size(pos2(x, rect.top()), vec2(width, rect.height()));
        ui.painter().rect_filled(
            segment,
            CornerRadius::same(RADIUS_FULL),
            if index < done {
                colors.accent
            } else {
                colors.muted
            },
        );
    }
}

pub fn step_marker(ui: &mut Ui, number: usize, done: bool) {
    let colors = colors(ui);
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
    let painter = ui.painter();
    if done {
        painter.circle_filled(rect.center(), 15.0, tint(colors.success, 0x2a));
        icons::paint(
            painter,
            Rect::from_center_size(rect.center(), Vec2::splat(14.0)),
            Icon::Check,
            colors.success,
        );
    } else {
        painter.circle_filled(rect.center(), 15.0, colors.accent_soft);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            number.to_string(),
            egui::FontId::proportional(TEXT_BASE),
            colors.accent_text,
        );
    }
}

pub fn nav_item(ui: &mut Ui, icon: Icon, text: &str, selected: bool) -> Response {
    let colors = colors(ui);
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Tab, true, selected, text));
    let hovered = response.hovered();
    let radius = CornerRadius::same(RADIUS);
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, radius, colors.accent_soft);
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
        colors.accent_text
    } else if hovered {
        colors.foreground
    } else {
        colors.muted_foreground
    };
    let icon_rect = Rect::from_center_size(
        pos2(rect.left() + SPACE_3 + 9.0, rect.center().y),
        Vec2::splat(18.0),
    );
    icons::paint(painter, icon_rect, icon, color);
    let galley = egui::WidgetText::from(if selected {
        bold(text, TEXT_BASE)
    } else {
        medium(text, TEXT_BASE)
    })
    .into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        rect.width() - 2.0 * SPACE_3 - 18.0 - SPACE_3,
        egui::TextStyle::Button,
    );
    ui.painter().galley(
        pos2(
            icon_rect.right() + SPACE_3,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn keycap(ui: &mut Ui, text: &str) {
    let colors = colors(ui);
    let galley = egui::WidgetText::from(bold(text, TEXT_LG)).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Body,
    );
    let size = vec2(galley.size().x + 2.0 * SPACE_6, 52.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    let radius = CornerRadius::same(10);
    painter.rect_filled(rect.translate(vec2(0.0, 3.0)), radius, colors.input);
    painter.rect(
        rect,
        radius,
        colors.muted,
        Stroke::new(1.0, colors.input),
        StrokeKind::Inside,
    );
    painter.galley(
        pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        colors.foreground,
    );
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
