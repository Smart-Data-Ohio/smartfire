use super::*;
#[test]
fn ws17_review_email_change_guard_matches_pinned_ruby_casecmp() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vectors/ws17_unicode.json")).unwrap();
    let t = TestDb::new();
    let mut user = t.read(|c| crate::User::find(c, id("david")));
    for row in v["comparisons"].as_array().unwrap() {
        user.email_address = Some(row["left"].as_str().unwrap().to_owned());
        assert_eq!(
            user.email_change_requested(row["right"].as_str().unwrap()),
            row["email_changing"].as_bool().unwrap(),
            "{row}"
        );
    }
}
