use crate::PillAction;

pub const SNAP_DISTANCE: f64 = 12.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PillPlacement {
    Notch,
    #[default]
    Floating,
}

impl PillPlacement {
    pub const ALL: [PillPlacement; 2] = [PillPlacement::Notch, PillPlacement::Floating];

    pub fn name(self) -> &'static str {
        match self {
            Self::Notch => "notch",
            Self::Floating => "floating",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|placement| placement.name() == name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PillPosition {
    pub x: f64,
    pub y: f64,
}

impl Default for PillPosition {
    fn default() -> Self {
        Self { x: 0.5, y: 0.12 }
    }
}

impl PillPosition {
    pub fn new(x: f64, y: f64) -> Option<Self> {
        let valid = |value: f64| value.is_finite() && (0.0..=1.0).contains(&value);
        (valid(x) && valid(y)).then_some(Self { x, y })
    }

    pub fn parse(text: &str) -> Option<Self> {
        let (x, y) = text.split_once(',')?;
        Self::new(x.trim().parse().ok()?, y.trim().parse().ok()?)
    }

    pub fn text(self) -> String {
        format!("{:.4},{:.4}", self.x, self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn top(self) -> f64 {
        self.y + self.height
    }

    pub fn right(self) -> f64 {
        self.x + self.width
    }

    pub fn centre(self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.top()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NotchSize {
    pub width: f64,
    pub height: f64,
}

fn fit(origin: f64, size: f64, low: f64, extent: f64) -> f64 {
    if size >= extent {
        low + (extent - size) / 2.0
    } else {
        origin.clamp(low, low + extent - size)
    }
}

pub fn floating_frame(visible: Rect, position: PillPosition, size: Size) -> Rect {
    let centre_x = visible.x + position.x * visible.width;
    let centre_y = visible.y + position.y * visible.height;
    Rect::new(
        fit(
            centre_x - size.width / 2.0,
            size.width,
            visible.x,
            visible.width,
        ),
        fit(
            centre_y - size.height / 2.0,
            size.height,
            visible.y,
            visible.height,
        ),
        size.width,
        size.height,
    )
}

pub fn dragged_position(visible: Rect, size: Size, centre: (f64, f64)) -> PillPosition {
    let frame = floating_frame(
        visible,
        PillPosition {
            x: ((centre.0 - visible.x) / visible.width.max(1.0)).clamp(0.0, 1.0),
            y: ((centre.1 - visible.y) / visible.height.max(1.0)).clamp(0.0, 1.0),
        },
        size,
    );
    let (mut x, y) = frame.centre();
    if (x - (visible.x + visible.width / 2.0)).abs() <= SNAP_DISTANCE {
        x = visible.x + visible.width / 2.0;
    }
    PillPosition {
        x: ((x - visible.x) / visible.width.max(1.0)).clamp(0.0, 1.0),
        y: ((y - visible.y) / visible.height.max(1.0)).clamp(0.0, 1.0),
    }
}

pub struct NotchLayout {
    pub capsule_height: f64,
    pub overhang: f64,
    pub menu_bar_gap: f64,
}

pub fn notch_frame(
    screen: Rect,
    visible_top: f64,
    notch: Option<NotchSize>,
    width: f64,
    layout: &NotchLayout,
) -> Rect {
    let (top_inset, overhang) = match notch {
        Some(notch) => (notch.height, layout.overhang),
        None => (0.0, 0.0),
    };
    let height = top_inset + overhang + layout.capsule_height;
    let top = match notch {
        Some(_) => screen.top() + overhang,
        None => visible_top - layout.menu_bar_gap,
    };
    Rect::new(
        screen.x + (screen.width - width) / 2.0,
        top - height,
        width,
        height,
    )
}

pub fn hint_frame(visible: Rect, pill: Rect, size: Size, gap: f64) -> Rect {
    let above = pill.top() + gap;
    let y = if above + size.height <= visible.top() {
        above
    } else {
        pill.y - gap - size.height
    };
    Rect::new(
        fit(
            pill.centre().0 - size.width / 2.0,
            size.width,
            visible.x,
            visible.width,
        ),
        fit(y, size.height, visible.y, visible.height),
        size.width,
        size.height,
    )
}

pub fn accepts_mouse(placing: bool, click: Option<PillAction>) -> bool {
    placing || click.is_some()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    start: PillPosition,
    current: PillPosition,
    dragged: bool,
}

impl Placement {
    pub fn enter(start: PillPosition) -> Self {
        Self {
            start,
            current: start,
            dragged: false,
        }
    }

    pub fn start(&self) -> PillPosition {
        self.start
    }

    pub fn current(&self) -> PillPosition {
        self.current
    }

    pub fn drag(&mut self, to: PillPosition) {
        if to != self.current {
            self.current = to;
            self.dragged = true;
        }
    }

    pub fn done(self) -> PillPosition {
        self.current
    }

    pub fn escape(self) -> Option<PillPosition> {
        self.dragged.then_some(self.current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Permission;

    const PILL: Size = Size {
        width: 200.0,
        height: 44.0,
    };

    fn screen() -> Rect {
        Rect::new(0.0, 0.0, 1440.0, 900.0)
    }

    fn visible() -> Rect {
        Rect::new(0.0, 70.0, 1440.0, 800.0)
    }

    #[test]
    fn a_fraction_maps_to_the_pill_centre_in_the_visible_frame() {
        let frame = floating_frame(visible(), PillPosition { x: 0.5, y: 0.5 }, PILL);

        assert_eq!(frame.centre(), (720.0, 470.0));
        assert_eq!((frame.width, frame.height), (200.0, 44.0));
    }

    #[test]
    fn the_default_position_is_bottom_centre() {
        let frame = floating_frame(visible(), PillPosition::default(), PILL);

        assert_eq!(frame.x + frame.width / 2.0, 720.0);
        assert_eq!(frame.y + frame.height / 2.0, 70.0 + 0.12 * 800.0);
    }

    #[test]
    fn the_pill_stays_fully_inside_on_every_edge() {
        let corners = [
            (0.0, 0.0, 0.0, 70.0),
            (1.0, 0.0, 1240.0, 70.0),
            (0.0, 1.0, 0.0, 826.0),
            (1.0, 1.0, 1240.0, 826.0),
        ];
        for (x, y, origin_x, origin_y) in corners {
            let frame = floating_frame(visible(), PillPosition { x, y }, PILL);

            assert_eq!((frame.x, frame.y), (origin_x, origin_y), "at {x},{y}");
        }
    }

    #[test]
    fn growing_wider_keeps_the_centre_until_an_edge_is_reached() {
        let position = PillPosition { x: 0.5, y: 0.12 };
        let narrow = floating_frame(visible(), position, PILL);
        let wide = floating_frame(visible(), position, Size::new(520.0, 44.0));

        assert_eq!(narrow.centre().0, wide.centre().0);

        let near_left = PillPosition { x: 0.1, y: 0.12 };
        let narrow = floating_frame(visible(), near_left, PILL);
        let wide = floating_frame(visible(), near_left, Size::new(520.0, 44.0));
        assert_eq!(narrow.x, 44.0);
        assert_eq!(wide.x, 0.0);

        let near_right = PillPosition { x: 0.95, y: 0.12 };
        let wide = floating_frame(visible(), near_right, Size::new(520.0, 44.0));
        assert_eq!(wide.right(), 1440.0);
    }

    #[test]
    fn a_pill_larger_than_the_screen_is_centred() {
        let frame = floating_frame(
            Rect::new(100.0, 0.0, 400.0, 300.0),
            PillPosition { x: 0.0, y: 0.5 },
            Size::new(520.0, 44.0),
        );

        assert_eq!(frame.centre().0, 300.0);
    }

    #[test]
    fn screens_with_different_visible_frames_use_the_same_fraction() {
        let position = PillPosition { x: 0.25, y: 0.1 };
        let main = floating_frame(visible(), position, PILL);
        let side = floating_frame(Rect::new(1440.0, -200.0, 1920.0, 1055.0), position, PILL);

        assert_eq!(main.centre(), (360.0, 150.0));
        assert_eq!(side.centre(), (1440.0 + 480.0, -200.0 + 105.5));
    }

    #[test]
    fn dragging_clamps_the_centre_so_the_pill_stays_on_screen() {
        let low = dragged_position(visible(), PILL, (-500.0, -500.0));
        let frame = floating_frame(visible(), low, PILL);
        assert_eq!((frame.x, frame.y), (0.0, 70.0));

        let high = dragged_position(visible(), PILL, (9_000.0, 9_000.0));
        let frame = floating_frame(visible(), high, PILL);
        assert_eq!((frame.right(), frame.top()), (1440.0, 870.0));
    }

    #[test]
    fn dragging_snaps_to_the_horizontal_centre_within_twelve_points() {
        let snapped = dragged_position(visible(), PILL, (731.0, 200.0));
        let free = dragged_position(visible(), PILL, (740.0, 200.0));

        assert_eq!(snapped.x, 0.5);
        assert_ne!(free.x, 0.5);
    }

    #[test]
    fn a_dragged_position_lands_back_on_the_same_frame() {
        let centre = (300.0, 400.0);
        let position = dragged_position(visible(), PILL, centre);
        let frame = floating_frame(visible(), position, PILL);

        assert!((frame.centre().0 - centre.0).abs() < 1e-9);
        assert!((frame.centre().1 - centre.1).abs() < 1e-9);
    }

    const LAYOUT: NotchLayout = NotchLayout {
        capsule_height: 44.0,
        overhang: 14.0,
        menu_bar_gap: 6.0,
    };

    #[test]
    fn on_a_notched_screen_the_pill_hangs_from_the_top_edge() {
        let frame = notch_frame(
            screen(),
            876.0,
            Some(NotchSize {
                width: 200.0,
                height: 32.0,
            }),
            220.0,
            &LAYOUT,
        );

        assert_eq!(frame.top(), 914.0);
        assert_eq!(frame.height, 90.0);
        assert_eq!(frame.x, 610.0);
    }

    #[test]
    fn without_a_notch_the_pill_sits_below_the_menu_bar() {
        let frame = notch_frame(screen(), 876.0, None, 220.0, &LAYOUT);

        assert_eq!(frame.top(), 870.0);
        assert_eq!(frame.height, 44.0);
        assert_eq!(frame.centre().0, 720.0);
    }

    #[test]
    fn the_notch_placement_ignores_the_screen_origin_offset() {
        let side = Rect::new(1440.0, -100.0, 1920.0, 1080.0);
        let frame = notch_frame(side, 956.0, None, 200.0, &LAYOUT);

        assert_eq!(frame.centre().0, 1440.0 + 960.0);
        assert_eq!(frame.top(), 950.0);
    }

    #[test]
    fn the_hint_sits_above_the_pill_and_flips_below_at_the_top() {
        let hint = Size::new(380.0, 44.0);
        let low = Rect::new(620.0, 150.0, 200.0, 44.0);
        let high = Rect::new(620.0, 826.0, 200.0, 44.0);

        assert_eq!(hint_frame(visible(), low, hint, 10.0).y, 204.0);
        assert_eq!(hint_frame(visible(), high, hint, 10.0).y, 772.0);
    }

    #[test]
    fn the_hint_stays_on_screen_beside_an_edge() {
        let hint = Size::new(380.0, 44.0);
        let corner = Rect::new(0.0, 70.0, 200.0, 44.0);

        let frame = hint_frame(visible(), corner, hint, 10.0);

        assert_eq!(frame.x, 0.0);
    }

    #[test]
    fn placements_and_positions_round_trip_through_text() {
        for placement in PillPlacement::ALL {
            assert_eq!(PillPlacement::from_name(placement.name()), Some(placement));
        }
        let position = PillPosition { x: 0.5, y: 0.125 };

        assert_eq!(PillPosition::parse(&position.text()), Some(position));
        assert_eq!(PillPlacement::default(), PillPlacement::Floating);
    }

    #[test]
    fn bad_positions_do_not_parse() {
        for text in [
            "",
            "0.5",
            "a,b",
            "0.5,",
            "1.5,0.2",
            "-0.1,0.2",
            "NaN,0.2",
            "0.5,inf",
            "0.5,0.2,0.1",
        ] {
            assert_eq!(PillPosition::parse(text), None, "{text:?}");
        }
        assert_eq!(
            PillPosition::parse(" 0.25 , 0.75 "),
            Some(PillPosition { x: 0.25, y: 0.75 })
        );
    }

    #[test]
    fn placement_done_saves_the_dragged_position() {
        let start = PillPosition::default();
        let moved = PillPosition { x: 0.3, y: 0.4 };
        let mut placement = Placement::enter(start);
        assert_eq!(placement.current(), start);
        placement.drag(moved);

        assert_eq!(placement.done(), moved);
    }

    #[test]
    fn placement_done_without_a_drag_keeps_the_start() {
        let placement = Placement::enter(PillPosition::default());

        assert_eq!(placement.done(), PillPosition::default());
    }

    #[test]
    fn escape_without_a_drag_cancels() {
        let mut placement = Placement::enter(PillPosition::default());
        placement.drag(PillPosition::default());

        assert_eq!(placement.escape(), None);
    }

    #[test]
    fn escape_after_a_drag_saves() {
        let moved = PillPosition { x: 0.9, y: 0.9 };
        let mut placement = Placement::enter(PillPosition::default());
        placement.drag(moved);

        assert_eq!(placement.escape(), Some(moved));
    }

    #[test]
    fn the_pill_accepts_the_mouse_only_while_placing_or_missing_a_permission() {
        let click = Some(PillAction::OpenSettings(Permission::Microphone));

        assert!(!accepts_mouse(false, None));
        assert!(accepts_mouse(false, click));
        assert!(accepts_mouse(true, None));
        assert!(accepts_mouse(true, click));
    }
}
