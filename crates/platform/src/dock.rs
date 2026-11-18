#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShowInDock {
    Always,
    #[default]
    WhileWindowOpen,
    Never,
}

pub fn dock_visible(show: ShowInDock, any_window_open: bool) -> bool {
    match show {
        ShowInDock::Always => true,
        ShowInDock::WhileWindowOpen => any_window_open,
        ShowInDock::Never => false,
    }
}

#[derive(Debug, Default)]
pub struct DockState {
    visible: bool,
}

impl DockState {
    pub fn update(&mut self, show: ShowInDock, any_window_open: bool) -> Option<bool> {
        let visible = dock_visible(show, any_window_open);
        (visible != self.visible).then(|| {
            self.visible = visible;
            visible
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_while_window_open() {
        assert_eq!(ShowInDock::default(), ShowInDock::WhileWindowOpen);
    }

    #[test]
    fn rule_covers_every_combination() {
        let expected = [
            (ShowInDock::Always, false, true),
            (ShowInDock::Always, true, true),
            (ShowInDock::WhileWindowOpen, false, false),
            (ShowInDock::WhileWindowOpen, true, true),
            (ShowInDock::Never, false, false),
            (ShowInDock::Never, true, false),
        ];
        for (show, open, visible) in expected {
            assert_eq!(dock_visible(show, open), visible, "{show:?} open={open}");
        }
    }

    #[test]
    fn starts_hidden_and_reports_nothing_while_hidden() {
        let mut dock = DockState::default();
        assert_eq!(dock.update(ShowInDock::WhileWindowOpen, false), None);
    }

    #[test]
    fn opening_and_closing_the_last_window_reports_each_change_once() {
        let mut dock = DockState::default();
        let show = ShowInDock::WhileWindowOpen;
        assert_eq!(dock.update(show, true), Some(true));
        assert_eq!(dock.update(show, true), None);
        assert_eq!(dock.update(show, false), Some(false));
        assert_eq!(dock.update(show, false), None);
    }

    #[test]
    fn changing_the_preference_while_a_window_is_open() {
        let mut dock = DockState::default();
        assert_eq!(dock.update(ShowInDock::WhileWindowOpen, true), Some(true));
        assert_eq!(dock.update(ShowInDock::Always, true), None);
        assert_eq!(dock.update(ShowInDock::Never, true), Some(false));
        assert_eq!(dock.update(ShowInDock::WhileWindowOpen, true), Some(true));
    }

    #[test]
    fn always_shows_without_a_window_and_never_hides_with_one() {
        let mut dock = DockState::default();
        assert_eq!(dock.update(ShowInDock::Always, false), Some(true));
        assert_eq!(dock.update(ShowInDock::Never, false), Some(false));
        assert_eq!(dock.update(ShowInDock::Never, true), None);
    }
}
