use std::cell::Cell;

use objc2::rc::Retained;
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSBackingStoreType, NSBox, NSBoxType, NSColor, NSEvent, NSFont, NSImage, NSImageView,
    NSLineBreakMode, NSPanel, NSScreen, NSTextAlignment, NSTextField, NSTitlePosition, NSView,
    NSWindowCollectionBehavior, NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::{NSData, NSPoint, NSRect, NSSize, NSString};
use platform::PillState;

const STATUS_LEVEL: NSWindowLevel = 25;
const FONT_SIZE: f64 = 14.0;
const LABEL_HEIGHT: f64 = 20.0;
const PADDING_X: f64 = 18.0;
const PADDING_Y: f64 = 7.0;
const MIN_WIDTH: f64 = 140.0;
const MAX_WIDTH: f64 = 520.0;
const CORNER_RADIUS: f64 = 14.0;
const BORDER_WIDTH: f64 = 1.0;
const MENU_BAR_GAP: f64 = 6.0;
const HEAD_SIZE: f64 = 26.0;
const HEAD_GAP: f64 = 8.0;

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
    art: Cell<PillArt>,
    images: [Option<Retained<NSImage>>; 4],
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
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(MIN_WIDTH, 34.0)),
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
        label.setTextColor(Some(&rgb(0xfb, 0xf7, 0xee)));
        label.setAlignment(NSTextAlignment::Left);
        label.setLineBreakMode(NSLineBreakMode::ByTruncatingHead);

        let shape = NSBox::new(mtm);
        shape.setBoxType(NSBoxType::Custom);
        shape.setTitlePosition(NSTitlePosition::NoTitle);
        shape.setBorderWidth(BORDER_WIDTH);
        shape.setBorderColor(&rgb(0x3a, 0x35, 0x2e));
        shape.setFillColor(&rgb(0x1c, 0x1a, 0x17));
        shape.setCornerRadius(CORNER_RADIUS);
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
        let container = NSView::new(mtm);
        container.addSubview(&shape);
        container.addSubview(&head);
        container.addSubview(&label);
        panel.setContentView(Some(&container));

        Self {
            mtm,
            panel,
            label,
            shape,
            head,
            art: Cell::new(PillArt::Default),
            images,
        }
    }

    pub fn show(&self, text: Option<&str>, art: PillArt) {
        let Some(text) = text else {
            self.panel.orderOut(None);
            return;
        };
        self.label.setStringValue(&NSString::from_str(text));
        if self.art.replace(art) != art {
            let index = match art {
                PillArt::Default => 0,
                PillArt::Listening => 1,
                PillArt::Writing => 2,
                PillArt::Confused => 3,
            };
            self.head.setImage(self.images[index].as_deref());
        }
        let anchor = self.anchor();
        let text_width = self
            .label
            .sizeThatFits(NSSize::new(MAX_WIDTH, LABEL_HEIGHT))
            .width;
        let notch_width = anchor.notch.as_ref().map_or(0.0, |notch| notch.width);
        let width = (text_width + 2.0 * PADDING_X + HEAD_SIZE + HEAD_GAP)
            .clamp(MIN_WIDTH, MAX_WIDTH)
            .max(notch_width);
        let (top_inset, overhang) = match &anchor.notch {
            Some(notch) => (notch.height, CORNER_RADIUS),
            None => (0.0, 0.0),
        };
        let height = top_inset + overhang + LABEL_HEIGHT + 2.0 * PADDING_Y;
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
        let text_available = width - 2.0 * PADDING_X - HEAD_SIZE - HEAD_GAP;
        let shown_width = text_width.min(text_available);
        let start = (width - (HEAD_SIZE + HEAD_GAP + shown_width)) / 2.0;
        self.head.setFrame(NSRect::new(
            NSPoint::new(start, PADDING_Y + (LABEL_HEIGHT - HEAD_SIZE) / 2.0),
            NSSize::new(HEAD_SIZE, HEAD_SIZE),
        ));
        self.label.setFrame(NSRect::new(
            NSPoint::new(start + HEAD_SIZE + HEAD_GAP, PADDING_Y),
            NSSize::new(shown_width, LABEL_HEIGHT),
        ));
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

fn rgb(red: u8, green: u8, blue: u8) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(
        f64::from(red) / 255.0,
        f64::from(green) / 255.0,
        f64::from(blue) / 255.0,
        1.0,
    )
}

fn contains(frame: NSRect, point: NSPoint) -> bool {
    point.x >= frame.origin.x
        && point.x < frame.origin.x + frame.size.width
        && point.y >= frame.origin.y
        && point.y < frame.origin.y + frame.size.height
}
