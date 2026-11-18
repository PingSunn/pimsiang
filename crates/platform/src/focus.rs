#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    TextField,
    NoTextField,
    Unknown,
}

pub const TEXT_ROLES: [&str; 4] = ["AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"];
pub const UNCERTAIN_ROLES: [&str; 2] = ["AXWebArea", "AXGroup"];

pub fn classify(role: Option<&str>, has_selected_text_range: bool) -> Focus {
    let Some(role) = role else {
        return Focus::Unknown;
    };
    if TEXT_ROLES.contains(&role) {
        Focus::TextField
    } else if UNCERTAIN_ROLES.contains(&role) {
        Focus::Unknown
    } else if has_selected_text_range {
        Focus::TextField
    } else {
        Focus::NoTextField
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_roles_are_text_fields() {
        for role in TEXT_ROLES {
            assert_eq!(classify(Some(role), false), Focus::TextField);
        }
    }

    #[test]
    fn web_areas_and_groups_are_unknown() {
        for role in UNCERTAIN_ROLES {
            assert_eq!(classify(Some(role), false), Focus::Unknown);
            assert_eq!(classify(Some(role), true), Focus::Unknown);
        }
    }

    #[test]
    fn a_missing_role_is_unknown() {
        assert_eq!(classify(None, false), Focus::Unknown);
    }

    #[test]
    fn other_roles_count_only_with_a_text_range() {
        assert_eq!(classify(Some("AXScrollArea"), false), Focus::NoTextField);
        assert_eq!(classify(Some("AXList"), false), Focus::NoTextField);
        assert_eq!(classify(Some("AXStaticText"), true), Focus::TextField);
    }
}
