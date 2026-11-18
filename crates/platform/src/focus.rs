#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    TextField,
    NoTextField,
    Unknown,
}

pub const TEXT_ROLES: [&str; 3] = ["AXTextField", "AXTextArea", "AXComboBox"];
pub const TEXT_SUBROLES: [&str; 1] = ["AXSearchField"];
pub const CHROMIUM_BUNDLE_IDS: [&str; 5] = [
    "com.google.Chrome",
    "com.microsoft.VSCode",
    "com.tinyspeck.slackmacgap",
    "com.hnc.Discord",
    "notion.id",
];
pub const UNCERTAIN_ROLES: [&str; 2] = ["AXWebArea", "AXGroup"];

pub fn classify(role: Option<&str>, subrole: Option<&str>, has_selected_text_range: bool) -> Focus {
    let Some(role) = role else {
        return Focus::Unknown;
    };
    if TEXT_ROLES.contains(&role) || subrole.is_some_and(|subrole| TEXT_SUBROLES.contains(&subrole))
    {
        Focus::TextField
    } else if UNCERTAIN_ROLES.contains(&role) {
        Focus::Unknown
    } else if has_selected_text_range {
        Focus::TextField
    } else {
        Focus::NoTextField
    }
}

pub fn classify_missing(bundle_id: Option<&str>, has_chromium_framework: bool) -> Focus {
    match bundle_id {
        None => Focus::Unknown,
        Some(id) if has_chromium_framework || CHROMIUM_BUNDLE_IDS.contains(&id) => Focus::Unknown,
        Some(_) => Focus::NoTextField,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_roles_are_text_fields() {
        for role in TEXT_ROLES {
            assert_eq!(classify(Some(role), None, false), Focus::TextField);
        }
    }

    #[test]
    fn web_areas_and_groups_are_unknown() {
        for role in UNCERTAIN_ROLES {
            assert_eq!(classify(Some(role), None, false), Focus::Unknown);
            assert_eq!(classify(Some(role), None, true), Focus::Unknown);
        }
    }

    #[test]
    fn a_missing_role_is_unknown() {
        assert_eq!(classify(None, None, false), Focus::Unknown);
    }

    #[test]
    fn other_roles_count_only_with_a_text_range() {
        assert_eq!(
            classify(Some("AXScrollArea"), None, false),
            Focus::NoTextField
        );
        assert_eq!(classify(Some("AXList"), None, false), Focus::NoTextField);
        assert_eq!(classify(Some("AXStaticText"), None, true), Focus::TextField);
    }

    #[test]
    fn a_search_field_subrole_is_a_text_field() {
        assert_eq!(
            classify(Some("AXScrollArea"), Some("AXSearchField"), false),
            Focus::TextField
        );
        assert_eq!(
            classify(Some("AXScrollArea"), Some("AXOther"), false),
            Focus::NoTextField
        );
    }

    #[test]
    fn nothing_focused_in_a_native_app_is_no_text_field() {
        assert_eq!(
            classify_missing(Some("com.apple.finder"), false),
            Focus::NoTextField
        );
    }

    #[test]
    fn nothing_focused_in_a_chromium_app_is_unknown() {
        assert_eq!(
            classify_missing(Some("com.example.App"), true),
            Focus::Unknown
        );
        for id in CHROMIUM_BUNDLE_IDS {
            assert_eq!(classify_missing(Some(id), false), Focus::Unknown);
        }
    }

    #[test]
    fn nothing_focused_without_a_frontmost_app_is_unknown() {
        assert_eq!(classify_missing(None, false), Focus::Unknown);
    }
}
