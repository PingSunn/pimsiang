use engine::character_error_rate;

#[test]
fn identical_text_has_no_errors() {
    assert_eq!(character_error_rate("สวัสดีครับ", "สวัสดีครับ"), 0.0);
}

#[test]
fn spaces_do_not_count() {
    assert_eq!(character_error_rate("วันนี้ อากาศดี", "วันนี้อากาศดี"), 0.0);
}

#[test]
fn counts_edits_per_reference_character() {
    assert_eq!(character_error_rate("abcd", "abxd"), 0.25);
    assert_eq!(character_error_rate("abcd", "abd"), 0.25);
    assert_eq!(character_error_rate("abcd", "abcde"), 0.25);
    assert_eq!(character_error_rate("abcd", ""), 1.0);
}

#[test]
fn thai_combining_marks_are_characters() {
    assert_eq!(character_error_rate("ไม่", "ไม"), 1.0 / 3.0);
}
