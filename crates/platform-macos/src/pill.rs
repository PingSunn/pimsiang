use std::cell::Cell;
use std::ptr::{self, NonNull};
use std::rc::Rc;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSBackingStoreType, NSBox, NSBoxType, NSColor, NSEvent, NSEventMask, NSFont,
    NSForegroundColorAttributeName, NSImage, NSImageView, NSLineBreakMode, NSPanel, NSScreen,
    NSTextAlignment, NSTextField, NSTitlePosition, NSView, NSWindowCollectionBehavior,
    NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::{
    NSData, NSMutableAttributedString, NSPoint, NSRange, NSRect, NSSize, NSString,
};
use platform::{PillAction, PillContent, PillState};

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
    monitor: Option<Retained<AnyObject>>,
}

struct Anchor {
    frame: NSRect,
    visible_top: f64,
    notch: Option<Notch>,
}

struct Notch {
    width: f64,
    height: f64,
}

impl PillPanel {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(MIN_WIDTH, CAPSULE_HEIGHT),
            ),
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

        let click = Rc::new(Cell::new(None));
        let monitor = install_click_monitor(&panel, click.clone());

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
            monitor,
        }
    }

    pub fn show(&self, content: Option<&PillContent>, art: PillArt, click: Option<PillAction>) {
        let Some(content) = content else {
            self.click.set(None);
            self.panel.orderOut(None);
            return;
        };
        self.click.set(click);
        self.panel.setIgnoresMouseEvents(click.is_none());
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
        let notch_width = anchor.notch.as_ref().map_or(0.0, |notch| notch.width);
        let width = (natural + chrome)
            .clamp(MIN_WIDTH, MAX_WIDTH)
            .max(notch_width);
        let text_width = natural.min(width - chrome);
        let (top_inset, overhang) = match &anchor.notch {
            Some(notch) => (notch.height, CORNER_RADIUS),
            None => (0.0, 0.0),
        };
        let height = top_inset + overhang + CAPSULE_HEIGHT;
        let top = match anchor.notch {
            Some(_) => anchor.frame.origin.y + anchor.frame.size.height + overhang,
            None => anchor.visible_top - MENU_BAR_GAP,
        };
        let frame = NSRect::new(
            NSPoint::new(
                anchor.frame.origin.x + (anchor.frame.size.width - width) / 2.0,
                top - height,
            ),
            NSSize::new(width, height),
        );
        self.panel.setFrame_display(frame, true);
        let radius = match anchor.notch {
            Some(_) => CORNER_RADIUS,
            None => height / 2.0,
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
                let visible = screen.visibleFrame();
                let inset = screen.safeAreaInsets().top;
                let notch = (inset > 0.0).then(|| {
                    let left = screen.auxiliaryTopLeftArea().size.width;
                    let right = screen.auxiliaryTopRightArea().size.width;
                    Notch {
                        width: (frame.size.width - left - right).max(0.0),
                        height: inset,
                    }
                });
                Anchor {
                    frame,
                    visible_top: visible.origin.y + visible.size.height,
                    notch,
                }
            }
            None => Anchor {
                frame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1440.0, 900.0)),
                visible_top: 876.0,
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
    unsafe {
        attributed.addAttribute_value_range(
            objc2_app_kit::NSFontAttributeName,
            &font,
            NSRange::new(0, text.length()),
        );
    }
    attributed
}

#[allow(unsafe_code)]
fn install_click_monitor(
    panel: &Retained<NSPanel>,
    click: Rc<Cell<Option<PillAction>>>,
) -> Option<Retained<AnyObject>> {
    let window_number = panel.clone();
    let handler = RcBlock::new(move |event: NonNull<NSEvent>| -> *mut NSEvent {
        let event = unsafe { event.as_ref() };
        if event.windowNumber() == window_number.windowNumber()
            && let Some(PillAction::OpenSettings(permission)) = click.get()
        {
            permissions::open_settings(permission);
            return ptr::null_mut();
        }
        ptr::from_ref(event).cast_mut()
    });
    unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::LeftMouseDown, &handler)
    }
}

fn contains(frame: NSRect, point: NSPoint) -> bool {
    point.x >= frame.origin.x
        && point.x < frame.origin.x + frame.size.width
        && point.y >= frame.origin.y
        && point.y < frame.origin.y + frame.size.height
}
