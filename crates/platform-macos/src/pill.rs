use std::cell::{Cell, RefCell};
use std::ptr::{self, NonNull};
use std::rc::Rc;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSBackingStoreType, NSBox, NSBoxType, NSColor, NSEvent, NSEventMask, NSEventType, NSFont,
    NSForegroundColorAttributeName, NSImage, NSImageView, NSLineBreakMode, NSMutableParagraphStyle,
    NSPanel, NSParagraphStyleAttributeName, NSScreen, NSTextAlignment, NSTextField,
    NSTitlePosition, NSView, NSWindowCollectionBehavior, NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::{
    NSData, NSMutableAttributedString, NSPoint, NSRange, NSRect, NSSize, NSString,
};
use platform::pill_layout::{
    NotchLayout, NotchSize, Placement, Rect, Size, accepts_mouse, dragged_position, floating_frame,
    hint_frame, notch_frame,
};
use platform::{PillAction, PillContent, PillPlacement, PillPosition, PillState};

use crate::permissions;

const STATUS_LEVEL: NSWindowLevel = 25;
const FONT_SIZE: f64 = 15.0;
const BADGE_FONT_SIZE: f64 = 12.0;
const LABEL_HEIGHT: f64 = 22.0;
const CAPSULE_HEIGHT: f64 = 44.0;
const PADDING_LEFT: f64 = 8.0;
const PADDING_RIGHT: f64 = 18.0;
const MIN_WIDTH: f64 = 140.0;
const MAX_WIDTH: f64 = 520.0;
const CORNER_RADIUS: f64 = 14.0;
const MENU_BAR_GAP: f64 = 6.0;
const HINT_HEIGHT: f64 = 44.0;
const HINT_GAP: f64 = 10.0;
const HINT_PADDING: f64 = 16.0;
const HINT_BUTTON_HEIGHT: f64 = 28.0;
const HINT_BUTTON_PADDING: f64 = 14.0;
const HINT_BUTTON_MARGIN: f64 = 8.0;
const NOTCH_LAYOUT: NotchLayout = NotchLayout {
    capsule_height: CAPSULE_HEIGHT,
    overhang: CORNER_RADIUS,
    menu_bar_gap: MENU_BAR_GAP,
};
const HEAD_SIZE: f64 = 32.0;
const GAP: f64 = 10.0;
const CARET_WIDTH: f64 = 2.0;
const CARET_HEIGHT: f64 = 18.0;
const DOT_SIZE: f64 = 5.0;
const DOT_GAP: f64 = 3.0;
const DOT_ALPHAS: [f64; 3] = [1.0, 0.6, 0.3];
const BADGE_HEIGHT: f64 = 20.0;
const BADGE_PADDING: f64 = 8.0;
const FEATHER: (u8, u8, u8) = (0x1c, 0x1a, 0x17);
const PAPER: (u8, u8, u8) = (0xfb, 0xf7, 0xee);
const WATTLE: (u8, u8, u8) = (0xff, 0xd2, 0x3f);
const UNCERTAIN: (u8, u8, u8) = (0x8a, 0x82, 0x76);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PillArt {
    Default,
    Listening,
    Writing,
    Confused,
}

impl PillArt {
    pub fn for_state(state: &PillState) -> Self {
        match state {
            PillState::Listening | PillState::Partial(_) => Self::Listening,
            PillState::Transcribing => Self::Writing,
            PillState::Error(_) => Self::Confused,
            PillState::LoadingModel | PillState::Notice(_) | PillState::Hidden => Self::Default,
        }
    }

    pub fn png(self) -> &'static [u8] {
        match self {
            Self::Default => include_bytes!("../assets/pill-default.png"),
            Self::Listening => include_bytes!("../assets/pill-listening.png"),
            Self::Writing => include_bytes!("../assets/pill-writing.png"),
            Self::Confused => include_bytes!("../assets/pill-confused.png"),
        }
    }

    fn image(self) -> Option<Retained<NSImage>> {
        let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(self.png()))?;
        image.setSize(NSSize::new(HEAD_SIZE, HEAD_SIZE));
        Some(image)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PillEvent {
    Moved(PillPosition),
    Done,
}

pub type PillEvents = Rc<dyn Fn(PillEvent)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementEnd {
    Done,
    Escape,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PillLayout {
    pub placement: PillPlacement,
    pub position: PillPosition,
}

struct Placing {
    session: Placement,
    visible: Rect,
    size: Size,
    grab: Option<(f64, f64)>,
}

struct Hint {
    panel: Retained<NSPanel>,
    background: Retained<NSBox>,
    label: Retained<NSTextField>,
    done: Retained<NSBox>,
    done_label: Retained<NSTextField>,
}

pub struct PillPanel {
    mtm: MainThreadMarker,
    panel: Retained<NSPanel>,
    label: Retained<NSTextField>,
    shape: Retained<NSBox>,
    head: Retained<NSImageView>,
    caret: Retained<NSBox>,
    dots: [Retained<NSBox>; 3],
    badge: Retained<NSBox>,
    badge_label: Retained<NSTextField>,
    art: Cell<PillArt>,
    images: [Option<Retained<NSImage>>; 4],
    click: Rc<Cell<Option<PillAction>>>,
    placing: Rc<RefCell<Option<Placing>>>,
    hint: Hint,
    monitor: Option<Retained<AnyObject>>,
}

struct Anchor {
    frame: Rect,
    visible: Rect,
    notch: Option<NotchSize>,
}

fn rect(frame: NSRect) -> Rect {
    Rect::new(
        frame.origin.x,
        frame.origin.y,
        frame.size.width,
        frame.size.height,
    )
}

fn ns_rect(frame: Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(frame.x, frame.y),
        NSSize::new(frame.width, frame.height),
    )
}

impl PillPanel {
    pub fn new(mtm: MainThreadMarker, events: PillEvents) -> Self {
        let panel = overlay_panel(mtm, NSSize::new(MIN_WIDTH, CAPSULE_HEIGHT));

        let label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        label.setFont(Some(&NSFont::systemFontOfSize(FONT_SIZE)));
        label.setTextColor(Some(&rgb(PAPER, 1.0)));
        label.setAlignment(NSTextAlignment::Left);
        label.setLineBreakMode(NSLineBreakMode::ByTruncatingHead);

        let shape = filled_box(mtm, FEATHER, 1.0);
        shape.setBorderWidth(0.0);
        let head = NSImageView::new(mtm);
        let images = [
            PillArt::Default,
            PillArt::Listening,
            PillArt::Writing,
            PillArt::Confused,
        ]
        .map(PillArt::image);
        if let Some(image) = &images[0] {
            head.setImage(Some(image));
        }
        let caret = filled_box(mtm, WATTLE, 1.0);
        let dots = DOT_ALPHAS.map(|alpha| {
            let dot = filled_box(mtm, WATTLE, alpha);
            dot.setCornerRadius(DOT_SIZE / 2.0);
            dot
        });
        let badge = filled_box(mtm, WATTLE, 1.0);
        badge.setCornerRadius(BADGE_HEIGHT / 2.0);
        let badge_label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        badge_label.setFont(Some(&NSFont::boldSystemFontOfSize(BADGE_FONT_SIZE)));
        badge_label.setTextColor(Some(&rgb(FEATHER, 1.0)));
        badge_label.setAlignment(NSTextAlignment::Center);

        let container = NSView::new(mtm);
        container.addSubview(&shape);
        container.addSubview(&head);
        container.addSubview(&label);
        container.addSubview(&caret);
        for dot in &dots {
            container.addSubview(dot);
        }
        container.addSubview(&badge);
        container.addSubview(&badge_label);
        panel.setContentView(Some(&container));
        caret.setHidden(true);
        for dot in &dots {
            dot.setHidden(true);
        }
        badge.setHidden(true);
        badge_label.setHidden(true);

        let hint = build_hint(mtm);
        let click = Rc::new(Cell::new(None));
        let placing = Rc::new(RefCell::new(None));
        let monitor = install_mouse_monitor(&panel, &hint, click.clone(), placing.clone(), events);

        Self {
            mtm,
            panel,
            label,
            shape,
            head,
            caret,
            dots,
            badge,
            badge_label,
            art: Cell::new(PillArt::Default),
            images,
            click,
            placing,
            hint,
            monitor,
        }
    }

    pub fn is_placing(&self) -> bool {
        self.placing.borrow().is_some()
    }

    pub fn show(
        &self,
        content: Option<&PillContent>,
        art: PillArt,
        click: Option<PillAction>,
        layout: PillLayout,
    ) {
        if self.is_placing() {
            return;
        }
        let Some(content) = content else {
            self.click.set(None);
            self.panel.orderOut(None);
            return;
        };
        self.click.set(click);
        self.panel
            .setIgnoresMouseEvents(!accepts_mouse(false, click));
        self.render(content, art, layout);
    }

    pub fn begin_placement(
        &self,
        content: &PillContent,
        art: PillArt,
        position: PillPosition,
        hint: &str,
        done: &str,
    ) {
        self.click.set(None);
        let layout = PillLayout {
            placement: PillPlacement::Floating,
            position,
        };
        let frame = self.render(content, art, layout);
        self.panel.setIgnoresMouseEvents(!accepts_mouse(true, None));
        let visible = self.anchor().visible;
        let session = Placement::enter(position);
        *self.placing.borrow_mut() = Some(Placing {
            session,
            visible,
            size: Size::new(frame.width, frame.height),
            grab: None,
        });
        let size = self.hint.fit(hint, done);
        self.hint
            .panel
            .setFrame_display(ns_rect(hint_frame(visible, frame, size, HINT_GAP)), true);
        self.hint.panel.orderFrontRegardless();
    }

    pub fn finish_placement(&self, end: PlacementEnd) -> Option<PillPosition> {
        let placing = self.placing.borrow_mut().take()?;
        self.hint.panel.orderOut(None);
        self.panel.orderOut(None);
        self.panel.setIgnoresMouseEvents(true);
        match end {
            PlacementEnd::Done => Some(placing.session.done()),
            PlacementEnd::Escape => placing.session.escape(),
        }
    }

    fn render(&self, content: &PillContent, art: PillArt, layout: PillLayout) -> Rect {
        self.label.setAttributedStringValue(&styled(content));
        if self.art.replace(art) != art {
            let index = match art {
                PillArt::Default => 0,
                PillArt::Listening => 1,
                PillArt::Writing => 2,
                PillArt::Confused => 3,
            };
            self.head.setImage(self.images[index].as_deref());
        }
        let badge_text = content.badge.as_deref();
        if let Some(text) = badge_text {
            self.badge_label.setStringValue(&NSString::from_str(text));
        }
        let badge_width = badge_text.map_or(0.0, |_| {
            self.badge_label
                .sizeThatFits(NSSize::new(MAX_WIDTH, BADGE_HEIGHT))
                .width
                + 2.0 * BADGE_PADDING
        });
        let accessory_width = accessory_width(content, badge_width);

        let anchor = self.anchor();
        let natural = self
            .label
            .sizeThatFits(NSSize::new(MAX_WIDTH, LABEL_HEIGHT))
            .width;
        let chrome = PADDING_LEFT + HEAD_SIZE + GAP + accessory_width + PADDING_RIGHT;
        let notch = match layout.placement {
            PillPlacement::Notch => anchor.notch,
            PillPlacement::Floating => None,
        };
        let notch_width = notch.map_or(0.0, |notch| notch.width);
        let width = (natural + chrome)
            .clamp(MIN_WIDTH, MAX_WIDTH)
            .max(notch_width);
        let text_width = natural.min(width - chrome);
        let target = match layout.placement {
            PillPlacement::Notch => notch_frame(
                anchor.frame,
                anchor.visible.top(),
                notch,
                width,
                &NOTCH_LAYOUT,
            ),
            PillPlacement::Floating => floating_frame(
                anchor.visible,
                layout.position,
                Size::new(width, CAPSULE_HEIGHT),
            ),
        };
        let frame = ns_rect(target);
        self.panel.setFrame_display(frame, true);
        let radius = match notch {
            Some(_) => CORNER_RADIUS,
            None => frame.size.height / 2.0,
        };
        self.shape.setCornerRadius(radius);
        self.shape
            .setFrame(NSRect::new(NSPoint::new(0.0, 0.0), frame.size));

        let used = PADDING_LEFT + HEAD_SIZE + GAP + text_width + accessory_width + PADDING_RIGHT;
        let start = ((width - used) / 2.0).max(0.0) + PADDING_LEFT;
        let middle = CAPSULE_HEIGHT / 2.0;
        self.head.setFrame(NSRect::new(
            NSPoint::new(start, middle - HEAD_SIZE / 2.0),
            NSSize::new(HEAD_SIZE, HEAD_SIZE),
        ));
        let text_x = start + HEAD_SIZE + GAP;
        self.label.setFrame(NSRect::new(
            NSPoint::new(text_x, middle - LABEL_HEIGHT / 2.0),
            NSSize::new(text_width, LABEL_HEIGHT),
        ));
        let cursor = text_x + text_width + GAP;
        self.caret.setHidden(!content.caret);
        if content.caret {
            self.caret.setFrame(NSRect::new(
                NSPoint::new(cursor, middle - CARET_HEIGHT / 2.0),
                NSSize::new(CARET_WIDTH, CARET_HEIGHT),
            ));
        }
        for (index, dot) in self.dots.iter().enumerate() {
            dot.setHidden(!content.dots);
            dot.setFrame(NSRect::new(
                NSPoint::new(
                    cursor + index as f64 * (DOT_SIZE + DOT_GAP),
                    middle - DOT_SIZE / 2.0,
                ),
                NSSize::new(DOT_SIZE, DOT_SIZE),
            ));
        }
        self.badge.setHidden(badge_text.is_none());
        self.badge_label.setHidden(badge_text.is_none());
        if badge_text.is_some() {
            self.badge.setFrame(NSRect::new(
                NSPoint::new(cursor, middle - BADGE_HEIGHT / 2.0),
                NSSize::new(badge_width, BADGE_HEIGHT),
            ));
            let label_height = self
                .badge_label
                .sizeThatFits(NSSize::new(MAX_WIDTH, BADGE_HEIGHT))
                .height;
            self.badge_label.setFrame(NSRect::new(
                NSPoint::new(cursor, middle - label_height / 2.0),
                NSSize::new(badge_width, label_height),
            ));
        }
        self.panel.orderFrontRegardless();
        target
    }

    fn anchor(&self) -> Anchor {
        let mouse = NSEvent::mouseLocation();
        let screens = NSScreen::screens(self.mtm);
        let screen = screens
            .iter()
            .find(|screen| contains(screen.frame(), mouse))
            .or_else(|| NSScreen::mainScreen(self.mtm));
        match screen {
            Some(screen) => {
                let frame = screen.frame();
                let inset = screen.safeAreaInsets().top;
                let notch = (inset > 0.0).then(|| {
                    let left = screen.auxiliaryTopLeftArea().size.width;
                    let right = screen.auxiliaryTopRightArea().size.width;
                    NotchSize {
                        width: (frame.size.width - left - right).max(0.0),
                        height: inset,
                    }
                });
                Anchor {
                    frame: rect(frame),
                    visible: rect(screen.visibleFrame()),
                    notch,
                }
            }
            None => Anchor {
                frame: Rect::new(0.0, 0.0, 1440.0, 900.0),
                visible: Rect::new(0.0, 0.0, 1440.0, 876.0),
                notch: None,
            },
        }
    }
}

fn rgb((red, green, blue): (u8, u8, u8), alpha: f64) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(
        f64::from(red) / 255.0,
        f64::from(green) / 255.0,
        f64::from(blue) / 255.0,
        alpha,
    )
}

fn filled_box(mtm: MainThreadMarker, color: (u8, u8, u8), alpha: f64) -> Retained<NSBox> {
    let shape = NSBox::new(mtm);
    shape.setBoxType(NSBoxType::Custom);
    shape.setTitlePosition(NSTitlePosition::NoTitle);
    shape.setBorderWidth(0.0);
    shape.setFillColor(&rgb(color, alpha));
    shape
}

impl Drop for PillPanel {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        if let Some(monitor) = self.monitor.take() {
            unsafe { NSEvent::removeMonitor(&monitor) };
        }
    }
}

fn accessory_width(content: &PillContent, badge_width: f64) -> f64 {
    if content.caret {
        GAP + CARET_WIDTH
    } else if content.dots {
        GAP + 3.0 * DOT_SIZE + 2.0 * DOT_GAP
    } else if content.badge.is_some() {
        GAP + badge_width
    } else {
        0.0
    }
}

#[allow(unsafe_code)]
fn styled(content: &PillContent) -> Retained<NSMutableAttributedString> {
    let settled = NSString::from_str(&content.settled);
    let tentative = NSString::from_str(&content.tentative);
    let text = NSString::from_str(&format!("{}{}", content.settled, content.tentative));
    let attributed =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &text);
    let paper = rgb(PAPER, 1.0);
    let uncertain = rgb(UNCERTAIN, 1.0);
    let settled_length = settled.length();
    let ranges = [
        (NSRange::new(0, settled_length), &paper),
        (NSRange::new(settled_length, tentative.length()), &uncertain),
    ];
    for (range, color) in ranges {
        if range.length == 0 {
            continue;
        }
        unsafe {
            attributed.addAttribute_value_range(NSForegroundColorAttributeName, color, range);
        }
    }
    let font = NSFont::systemFontOfSize(FONT_SIZE);
    let paragraph = NSMutableParagraphStyle::new();
    paragraph.setLineBreakMode(NSLineBreakMode::ByTruncatingHead);
    paragraph.setAlignment(NSTextAlignment::Left);
    unsafe {
        attributed.addAttribute_value_range(
            objc2_app_kit::NSFontAttributeName,
            &font,
            NSRange::new(0, text.length()),
        );
        attributed.addAttribute_value_range(
            NSParagraphStyleAttributeName,
            &paragraph,
            NSRange::new(0, text.length()),
        );
    }
    attributed
}

fn overlay_panel(mtm: MainThreadMarker, size: NSSize) -> Retained<NSPanel> {
    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        mtm.alloc(),
        NSRect::new(NSPoint::new(0.0, 0.0), size),
        NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );
    panel.setFloatingPanel(true);
    panel.setLevel(STATUS_LEVEL);
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    panel.setHasShadow(false);
    panel.setIgnoresMouseEvents(true);
    panel.setHidesOnDeactivate(false);
    panel.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    panel
}

fn build_hint(mtm: MainThreadMarker) -> Hint {
    let panel = overlay_panel(mtm, NSSize::new(MAX_WIDTH, HINT_HEIGHT));
    panel.setIgnoresMouseEvents(false);
    let background = filled_box(mtm, FEATHER, 1.0);
    background.setCornerRadius(HINT_HEIGHT / 2.0);
    let label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
    label.setFont(Some(&NSFont::systemFontOfSize(BADGE_FONT_SIZE + 1.0)));
    label.setTextColor(Some(&rgb(PAPER, 1.0)));
    label.setLineBreakMode(NSLineBreakMode::ByClipping);
    let done = filled_box(mtm, WATTLE, 1.0);
    done.setCornerRadius(HINT_BUTTON_HEIGHT / 2.0);
    let done_label = NSTextField::labelWithString(&NSString::from_str(""), mtm);
    done_label.setFont(Some(&NSFont::boldSystemFontOfSize(BADGE_FONT_SIZE + 1.0)));
    done_label.setTextColor(Some(&rgb(FEATHER, 1.0)));
    done_label.setAlignment(NSTextAlignment::Center);
    let container = NSView::new(mtm);
    container.addSubview(&background);
    container.addSubview(&label);
    container.addSubview(&done);
    container.addSubview(&done_label);
    panel.setContentView(Some(&container));
    Hint {
        panel,
        background,
        label,
        done,
        done_label,
    }
}

impl Hint {
    fn fit(&self, hint: &str, done: &str) -> Size {
        self.label.setStringValue(&NSString::from_str(hint));
        self.done_label.setStringValue(&NSString::from_str(done));
        let unbounded = NSSize::new(MAX_WIDTH * 2.0, HINT_HEIGHT);
        let label = self.label.sizeThatFits(unbounded);
        let done_text = self.done_label.sizeThatFits(unbounded);
        let button_width = done_text.width + 2.0 * HINT_BUTTON_PADDING;
        let width = HINT_PADDING + label.width + GAP + button_width + HINT_BUTTON_MARGIN;
        self.background.setFrame(NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(width, HINT_HEIGHT),
        ));
        self.label.setFrame(NSRect::new(
            NSPoint::new(HINT_PADDING, (HINT_HEIGHT - label.height) / 2.0),
            NSSize::new(label.width, label.height),
        ));
        let button_x = width - HINT_BUTTON_MARGIN - button_width;
        let button_y = (HINT_HEIGHT - HINT_BUTTON_HEIGHT) / 2.0;
        self.done.setFrame(NSRect::new(
            NSPoint::new(button_x, button_y),
            NSSize::new(button_width, HINT_BUTTON_HEIGHT),
        ));
        self.done_label.setFrame(NSRect::new(
            NSPoint::new(button_x, (HINT_HEIGHT - done_text.height) / 2.0),
            NSSize::new(button_width, done_text.height),
        ));
        Size::new(width, HINT_HEIGHT)
    }
}

fn mouse_point() -> (f64, f64) {
    let point = NSEvent::mouseLocation();
    (point.x, point.y)
}

fn move_to(
    placing: &mut Placing,
    panel: &NSPanel,
    hint: &NSPanel,
    mouse: (f64, f64),
) -> PillPosition {
    let (dx, dy) = placing.grab.unwrap_or((0.0, 0.0));
    let position = dragged_position(placing.visible, placing.size, (mouse.0 - dx, mouse.1 - dy));
    placing.session.drag(position);
    let frame = floating_frame(placing.visible, position, placing.size);
    panel.setFrame_display(ns_rect(frame), true);
    let hint_size = rect(hint.frame());
    hint.setFrame_display(
        ns_rect(hint_frame(
            placing.visible,
            frame,
            Size::new(hint_size.width, hint_size.height),
            HINT_GAP,
        )),
        true,
    );
    position
}

#[allow(unsafe_code)]
fn install_mouse_monitor(
    panel: &Retained<NSPanel>,
    hint: &Hint,
    click: Rc<Cell<Option<PillAction>>>,
    placing: Rc<RefCell<Option<Placing>>>,
    events: PillEvents,
) -> Option<Retained<AnyObject>> {
    let pill = panel.clone();
    let hint_panel = hint.panel.clone();
    let done = hint.done.clone();
    let handler = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
        let event = unsafe { event.as_ref() };
        let number = event.windowNumber();
        let on_pill = number == pill.windowNumber();
        let on_hint = number == hint_panel.windowNumber();
        let kind = event.r#type();
        if placing.borrow().is_some() {
            if !on_pill && !on_hint {
                return ptr::from_ref(event).cast_mut();
            }
            if kind == NSEventType::LeftMouseDown {
                if on_hint {
                    let local = event.locationInWindow();
                    if contains(done.frame(), local) {
                        events(PillEvent::Done);
                    }
                } else if let Some(state) = placing.borrow_mut().as_mut() {
                    let frame = pill.frame();
                    let mouse = mouse_point();
                    state.grab = Some((
                        mouse.0 - (frame.origin.x + frame.size.width / 2.0),
                        mouse.1 - (frame.origin.y + frame.size.height / 2.0),
                    ));
                }
            } else if kind == NSEventType::LeftMouseDragged && on_pill {
                let moved = placing
                    .borrow_mut()
                    .as_mut()
                    .filter(|state| state.grab.is_some())
                    .map(|state| move_to(state, &pill, &hint_panel, mouse_point()));
                if let Some(position) = moved {
                    events(PillEvent::Moved(position));
                }
            } else if kind == NSEventType::LeftMouseUp
                && let Some(state) = placing.borrow_mut().as_mut()
            {
                state.grab = None;
            }
            return ptr::null_mut();
        }
        if on_pill
            && kind == NSEventType::LeftMouseDown
            && let Some(PillAction::OpenSettings(permission)) = click.get()
        {
            permissions::open_settings(permission);
            return ptr::null_mut();
        }
        ptr::from_ref(event).cast_mut()
    });
    let mask =
        NSEventMask::LeftMouseDown | NSEventMask::LeftMouseDragged | NSEventMask::LeftMouseUp;
    unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &handler) }
}

fn contains(frame: NSRect, point: NSPoint) -> bool {
    point.x >= frame.origin.x
        && point.x < frame.origin.x + frame.size.width
        && point.y >= frame.origin.y
        && point.y < frame.origin.y + frame.size.height
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2_app_kit::NSParagraphStyle;
    use objc2_foundation::NSAttributedString;

    fn content(settled: &str, tentative: &str) -> PillContent {
        PillContent {
            settled: settled.to_owned(),
            tentative: tentative.to_owned(),
            caret: true,
            dots: false,
            badge: None,
        }
    }

    #[allow(unsafe_code)]
    fn line_break_at(text: &NSAttributedString, index: usize) -> Option<NSLineBreakMode> {
        let value = unsafe {
            text.attribute_atIndex_effectiveRange(
                NSParagraphStyleAttributeName,
                index,
                ptr::null_mut(),
            )
        }?;
        let style = value.downcast::<NSParagraphStyle>().ok()?;
        Some(style.lineBreakMode())
    }

    #[test]
    fn the_label_text_truncates_its_head_across_settled_and_tentative_text() {
        let text = styled(&content("ประเทศไทยมีประวัติศาสตร์", "ยาวนานหลายร้อยปี"));
        let last = text.length() - 1;

        assert_eq!(
            line_break_at(&text, 0),
            Some(NSLineBreakMode::ByTruncatingHead)
        );
        assert_eq!(
            line_break_at(&text, last),
            Some(NSLineBreakMode::ByTruncatingHead)
        );
    }
}
