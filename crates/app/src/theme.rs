use egui::{
    Align, Color32, CornerRadius, Frame, Id, Margin, Response, RichText, Role, Sense, Stroke,
    StrokeKind, TextEdit, Theme, Ui, Vec2, WidgetInfo, vec2,
};

pub const RADIUS: u8 = 8;
pub const RADIUS_FULL: u8 = 255;

pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_6: f32 = 24.0;

pub const TEXT_XS: f32 = 12.0;
pub const TEXT_SM: f32 = 13.0;
pub const TEXT_BASE: f32 = 14.0;
pub const TEXT_LG: f32 = 16.0;
pub const TEXT_XL: f32 = 20.0;
pub const TEXT_TITLE: f32 = 24.0;

const CONTROL_HEIGHT: f32 = 34.0;

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub foreground: Color32,
    pub card: Color32,
    pub muted: Color32,
    pub muted_foreground: Color32,
    pub border: Color32,
    pub input: Color32,
    pub primary: Color32,
    pub primary_foreground: Color32,
    pub destructive: Color32,
    pub destructive_foreground: Color32,
    pub ring: Color32,
    pub success: Color32,
    pub warning: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub const LIGHT: Palette = Palette {
    background: rgb(0xffffff),
    foreground: rgb(0x09090b),
    card: rgb(0xffffff),
    muted: rgb(0xf4f4f5),
    muted_foreground: rgb(0x71717a),
    border: rgb(0xe4e4e7),
    input: rgb(0xe4e4e7),
    primary: rgb(0x18181b),
    primary_foreground: rgb(0xfafafa),
    destructive: rgb(0xdc2626),
    destructive_foreground: rgb(0xfafafa),
    ring: rgb(0xa1a1aa),
    success: rgb(0x16a34a),
    warning: rgb(0xd97706),
};

pub const DARK: Palette = Palette {
    background: rgb(0x09090b),
    foreground: rgb(0xfafafa),
    card: rgb(0x09090b),
    muted: rgb(0x27272a),
    muted_foreground: rgb(0xa1a1aa),
    border: rgb(0x27272a),
    input: rgb(0x3f3f46),
    primary: rgb(0xfafafa),
    primary_foreground: rgb(0x18181b),
    destructive: rgb(0xdc2626),
    destructive_foreground: rgb(0xfafafa),
    ring: rgb(0xd4d4d8),
    success: rgb(0x22c55e),
    warning: rgb(0xf59e0b),
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
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(match theme {
            Theme::Light => 30,
            Theme::Dark => 120,
        }),
    };
    visuals.extreme_bg_color = colors.background;
    visuals.faint_bg_color = colors.muted;
    visuals.code_bg_color = colors.muted;
    visuals.override_text_color = Some(colors.foreground);
    visuals.hyperlink_color = colors.foreground;
    visuals.warn_fg_color = colors.warning;
    visuals.error_fg_color = colors.destructive;
    visuals.selection.bg_fill = colors.ring.gamma_multiply(0.45);
    visuals.selection.stroke = Stroke::new(1.0, colors.foreground);
    visuals.text_cursor.stroke = Stroke::new(1.5, colors.foreground);
    visuals.striped = false;
    visuals.slider_trailing_fill = true;

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = colors.card;
    widgets.noninteractive.weak_bg_fill = colors.card;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, colors.border);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors.foreground);
    widgets.noninteractive.corner_radius = radius;

    widgets.inactive.bg_fill = colors.background;
    widgets.inactive.weak_bg_fill = colors.background;
    widgets.inactive.bg_stroke = Stroke::new(1.0, colors.input);
    widgets.inactive.fg_stroke = Stroke::new(1.0, colors.foreground);
    widgets.inactive.corner_radius = radius;

    widgets.hovered.bg_fill = colors.muted;
    widgets.hovered.weak_bg_fill = colors.muted;
    widgets.hovered.bg_stroke = Stroke::new(1.0, colors.input);
    widgets.hovered.fg_stroke = Stroke::new(1.0, colors.foreground);
    widgets.hovered.corner_radius = radius;
    widgets.hovered.expansion = 0.0;

    widgets.active.bg_fill = colors.muted;
    widgets.active.weak_bg_fill = colors.muted;
    widgets.active.bg_stroke = Stroke::new(1.0, colors.ring);
    widgets.active.fg_stroke = Stroke::new(1.0, colors.foreground);
    widgets.active.corner_radius = radius;
    widgets.active.expansion = 0.0;

    widgets.open.bg_fill = colors.muted;
    widgets.open.weak_bg_fill = colors.muted;
    widgets.open.bg_stroke = Stroke::new(1.0, colors.input);
    widgets.open.fg_stroke = Stroke::new(1.0, colors.foreground);
    widgets.open.corner_radius = radius;

    style.visuals = visuals;
    style
}

pub fn page<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .inner_margin(Margin::same(SPACE_6 as i8))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = SPACE_3;
            add_contents(ui)
        })
        .inner
}

pub fn title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(TEXT_TITLE).strong());
}

pub fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(TEXT_LG).strong());
}

pub fn label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(TEXT_BASE));
}

pub fn description(ui: &mut Ui, text: &str) {
    let color = colors(ui).muted_foreground;
    ui.label(RichText::new(text).size(TEXT_SM).color(color));
}

pub fn warning(ui: &mut Ui, text: &str) {
    let color = colors(ui).warning;
    ui.label(RichText::new(text).size(TEXT_SM).color(color));
}

pub fn destructive(ui: &mut Ui, text: &str) {
    let color = colors(ui).destructive;
    ui.label(RichText::new(text).size(TEXT_SM).color(color));
}

pub fn success(ui: &mut Ui, text: &str) {
    let color = colors(ui).success;
    ui.label(RichText::new(text).size(TEXT_LG).strong().color(color));
}

pub fn card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let colors = colors(ui);
    Frame::new()
        .fill(colors.card)
        .stroke(Stroke::new(1.0, colors.border))
        .corner_radius(CornerRadius::same(RADIUS))
        .inner_margin(Margin::same(SPACE_4 as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = SPACE_2;
            add_contents(ui)
        })
        .inner
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
    let (fill, hover, stroke, text_color) = match variant {
        Variant::Primary => (
            colors.primary,
            colors.primary.gamma_multiply(0.9),
            Color32::TRANSPARENT,
            colors.primary_foreground,
        ),
        Variant::Secondary => (
            colors.muted,
            colors.muted.gamma_multiply(0.8),
            Color32::TRANSPARENT,
            colors.foreground,
        ),
        Variant::Outline => (
            colors.background,
            colors.muted,
            colors.input,
            colors.foreground,
        ),
        Variant::Ghost => (
            Color32::TRANSPARENT,
            colors.muted,
            Color32::TRANSPARENT,
            colors.foreground,
        ),
        Variant::Destructive => (
            colors.destructive,
            colors.destructive.gamma_multiply(0.9),
            Color32::TRANSPARENT,
            colors.destructive_foreground,
        ),
    };
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        for (state, background) in [
            (&mut widgets.inactive, fill),
            (&mut widgets.hovered, hover),
            (&mut widgets.active, hover),
        ] {
            state.weak_bg_fill = background;
            state.bg_fill = background;
            state.bg_stroke = Stroke::new(1.0, stroke);
            state.expansion = 0.0;
        }
        ui.add_enabled(
            enabled,
            egui::Button::new(RichText::new(text).size(TEXT_BASE).color(text_color))
                .min_size(vec2(0.0, CONTROL_HEIGHT)),
        )
    })
    .inner
}

pub fn icon_button(ui: &mut Ui, text: &str) -> Response {
    let colors = colors(ui);
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        widgets.inactive.bg_stroke = Stroke::NONE;
        ui.spacing_mut().button_padding = vec2(SPACE_3, SPACE_1);
        ui.add(
            egui::Button::new(
                RichText::new(text)
                    .size(TEXT_SM)
                    .color(colors.muted_foreground),
            )
            .min_size(vec2(0.0, 28.0)),
        )
    })
    .inner
}

pub fn text_input(ui: &mut Ui, id: Id, value: &mut String, hint: &str, width: f32) -> Response {
    let colors = colors(ui);
    let focused = ui.memory(|memory| memory.has_focus(id));
    let stroke = if focused {
        Stroke::new(2.0, colors.ring)
    } else {
        Stroke::new(1.0, colors.input)
    };
    Frame::new()
        .fill(colors.background)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(RADIUS))
        .inner_margin(Margin::symmetric(SPACE_3 as i8, 7))
        .show(ui, |ui| {
            ui.add(
                TextEdit::singleline(value)
                    .id(id)
                    .hint_text(hint)
                    .frame(Frame::NONE)
                    .font(egui::FontId::proportional(TEXT_BASE))
                    .desired_width(width - 2.0 * SPACE_3 - 4.0),
            )
        })
        .inner
}

pub fn switch(ui: &mut Ui, on: &mut bool, enabled: bool, text: &str) -> Response {
    let colors = colors(ui);
    let mut response = ui
        .horizontal(|ui| {
            let size = vec2(36.0, 20.0);
            let sense = if enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let (rect, mut track) = ui.allocate_exact_size(size, sense);
            let label = ui.add_enabled(
                enabled,
                egui::Label::new(RichText::new(text).size(TEXT_BASE)).sense(sense),
            );
            track |= label;
            if track.clicked() {
                *on = !*on;
                track.mark_changed();
            }
            track.widget_info(|| WidgetInfo::selected(Role::CheckBox, enabled, *on, text));
            let progress = ui.ctx().animate_bool_responsive(track.id, *on);
            let mut fill = if *on { colors.primary } else { colors.input };
            if !enabled {
                fill = fill.gamma_multiply(0.5);
            }
            let painter = ui.painter();
            painter.rect_filled(rect, CornerRadius::same(RADIUS_FULL), fill);
            if track.has_focus() {
                painter.rect_stroke(
                    rect.expand(1.0),
                    CornerRadius::same(RADIUS_FULL),
                    Stroke::new(2.0, colors.ring),
                    StrokeKind::Outside,
                );
            }
            let radius = 8.0;
            let x = egui::lerp(
                (rect.left() + radius + 2.0)..=(rect.right() - radius - 2.0),
                progress,
            );
            let thumb = if *on {
                colors.primary_foreground
            } else {
                colors.background
            };
            painter.circle_filled(
                egui::pos2(x, rect.center().y),
                radius,
                if enabled {
                    thumb
                } else {
                    thumb.gamma_multiply(0.6)
                },
            );
            track
        })
        .inner;
    response.layer_id = ui.layer_id();
    response
}

pub fn select(
    ui: &mut Ui,
    salt: &str,
    selected: &str,
    add_contents: impl FnOnce(&mut Ui),
) -> Response {
    egui::ComboBox::from_id_salt(salt)
        .selected_text(RichText::new(selected).size(TEXT_BASE))
        .width(240.0)
        .show_ui(ui, add_contents)
        .response
}

pub fn option<T: PartialEq>(ui: &mut Ui, current: &mut T, value: T, text: &str) -> Response {
    ui.selectable_value(current, value, RichText::new(text).size(TEXT_BASE))
}

pub fn radio<T: PartialEq>(ui: &mut Ui, current: &mut T, value: T, text: &str) -> Response {
    ui.radio_value(current, value, RichText::new(text).size(TEXT_BASE))
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
        Badge::Success => (colors.success.gamma_multiply(0.15), colors.success),
        Badge::Neutral => (colors.muted, colors.muted_foreground),
        Badge::Warning => (colors.warning.gamma_multiply(0.15), colors.warning),
        Badge::Destructive => (colors.destructive.gamma_multiply(0.15), colors.destructive),
    };
    Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(RADIUS_FULL))
        .inner_margin(Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(TEXT_XS).strong().color(color));
        });
}

pub fn progress(ui: &mut Ui, fraction: f32) {
    let colors = colors(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::hover());
    let radius = CornerRadius::same(RADIUS_FULL);
    ui.painter().rect_filled(rect, radius, colors.muted);
    let width = (rect.width() * fraction.clamp(0.0, 1.0)).max(0.0);
    if width > 0.0 {
        let filled = egui::Rect::from_min_size(rect.min, Vec2::new(width.max(8.0), rect.height()));
        ui.painter().rect_filled(filled, radius, colors.primary);
    }
}

pub fn row_end<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    ui.with_layout(egui::Layout::right_to_left(Align::Center), add_contents)
        .inner
}
