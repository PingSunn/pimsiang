use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSBackingStoreType, NSBox, NSBoxType, NSColor, NSEvent, NSFont, NSLineBreakMode, NSPanel,
    NSScreen, NSTextAlignment, NSTextField, NSTitlePosition, NSView, NSWindowCollectionBehavior,
    NSWindowLevel, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use platform::PillState;

const STATUS_LEVEL: NSWindowLevel = 25;
const FONT_SIZE: f64 = 14.0;
const LABEL_HEIGHT: f64 = 20.0;
const PADDING_X: f64 = 18.0;
const PADDING_Y: f64 = 7.0;
const MIN_WIDTH: f64 = 140.0;
const MAX_WIDTH: f64 = 520.0;
const CORNER_RADIUS: f64 = 14.0;
const MENU_BAR_GAP: f64 = 6.0;

pub struct PillPanel {
    mtm: MainThreadMarker,
    panel: Retained<NSPanel>,
    label: Retained<NSTextField>,
    shape: Retained<NSBox>,
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
        label.setTextColor(Some(&NSColor::whiteColor()));
        label.setAlignment(NSTextAlignment::Center);
        label.setLineBreakMode(NSLineBreakMode::ByTruncatingHead);

        let shape = NSBox::new(mtm);
        shape.setBoxType(NSBoxType::Custom);
        shape.setTitlePosition(NSTitlePosition::NoTitle);
        shape.setBorderWidth(0.0);
        shape.setFillColor(&NSColor::blackColor());
        shape.setCornerRadius(CORNER_RADIUS);
        let container = NSView::new(mtm);
        container.addSubview(&shape);
        container.addSubview(&label);
        panel.setContentView(Some(&container));

        Self {
            mtm,
            panel,
            label,
            shape,
        }
    }

    pub fn show(&self, state: &PillState) {
        let text = match state {
            PillState::Hidden => {
                self.panel.orderOut(None);
                return;
            }
            PillState::Listening => "Listening…".to_owned(),
            PillState::Partial(text) => text.clone(),
            PillState::Transcribing => "Transcribing…".to_owned(),
            PillState::Notice(notice) => notice.message().to_owned(),
            PillState::Error(error) => error.message().to_owned(),
        };
        self.label.setStringValue(&NSString::from_str(&text));
        let anchor = self.anchor();
        let text_width = self
            .label
            .sizeThatFits(NSSize::new(MAX_WIDTH, LABEL_HEIGHT))
            .width;
        let notch_width = anchor.notch.as_ref().map_or(0.0, |notch| notch.width);
        let width = (text_width + 2.0 * PADDING_X)
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
        self.shape
            .setFrame(NSRect::new(NSPoint::new(0.0, 0.0), frame.size));
        self.label.setFrame(NSRect::new(
            NSPoint::new(PADDING_X, PADDING_Y),
            NSSize::new(width - 2.0 * PADDING_X, LABEL_HEIGHT),
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

fn contains(frame: NSRect, point: NSPoint) -> bool {
    point.x >= frame.origin.x
        && point.x < frame.origin.x + frame.size.width
        && point.y >= frame.origin.y
        && point.y < frame.origin.y + frame.size.height
}
