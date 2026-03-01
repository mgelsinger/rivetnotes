pub fn can_uppercase(sel_start: i64, sel_end: i64) -> bool {
    sel_start != sel_end
}

pub fn can_lowercase(sel_start: i64, sel_end: i64) -> bool {
    sel_start != sel_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercase_disabled_for_empty_selection() {
        assert!(!can_uppercase(10, 10));
    }

    #[test]
    fn uppercase_enabled_for_non_empty_selection() {
        assert!(can_uppercase(5, 9));
        assert!(can_uppercase(9, 5));
    }

    #[test]
    fn lowercase_rules_match_uppercase() {
        assert!(!can_lowercase(0, 0));
        assert!(can_lowercase(0, 1));
    }
}
