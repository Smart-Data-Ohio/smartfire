//! Four KeywordAlert model cases and ten Notifications::KeywordMatcher cases from Rails.
use super::*;
use crate::models::keyword_alert::matching_user_ids;
use crate::{Error, KeywordAlert};

#[test]
fn normalizes_whitespace() {
    let t = TestDb::new();
    let alert = t.write(|tx| KeywordAlert::create(tx, id("david"), "  deploy   freeze  "));
    assert_eq!(alert.phrase, "deploy freeze");
    let same = alert.clone();
    t.travel(1);
    let again = t.write(move |tx| {
        let mut alert = alert;
        alert.update(tx, "deploy\tfreeze")?;
        Ok(alert)
    });
    assert_eq!(same, again);
}

#[test]
fn rejects_blank_and_overlong_phrases() {
    let t = TestDb::new();
    for phrase in ["   ".to_string(), "é".repeat(81)] {
        assert!(matches!(
            t.try_write(move |tx| KeywordAlert::create(tx, id("david"), &phrase)),
            Err(Error::RecordInvalid(_))
        ));
    }
    assert_eq!(
        t.write(|tx| KeywordAlert::create(tx, id("david"), &"é".repeat(80)))
            .phrase
            .chars()
            .count(),
        80
    );
}

#[test]
fn rejects_case_insensitive_duplicates_per_user() {
    let t = TestDb::new();
    let mut alert = t.write(|tx| KeywordAlert::create(tx, id("david"), "Deploy"));
    assert!(matches!(
        t.try_write(|tx| KeywordAlert::create(tx, id("david"), "deploy")),
        Err(Error::RecordInvalid(_))
    ));
    t.write(|tx| KeywordAlert::create(tx, id("jason"), "deploy"));
    t.write(move |tx| alert.update(tx, "DEPLOY"));
}

#[test]
fn caps_each_user_at_twenty_phrases() {
    let t = TestDb::new();
    t.write(|tx| {
        for n in 0..20 {
            KeywordAlert::create(tx, id("david"), &format!("phrase {n}"))?;
        }
        Ok(())
    });
    let Error::RecordInvalid(errors) = t
        .try_write(|tx| KeywordAlert::create(tx, id("david"), "extra"))
        .unwrap_err()
    else {
        panic!()
    };
    assert_eq!(errors.on("base"), ["You can watch at most 20 keywords"]);
    t.write(|tx| {
        let mut first = KeywordAlert::for_user(tx.conn(), id("david"))?.remove(0);
        first.update(tx, "updated")
    });
}

fn matches(entries: &[(i64, &[&str])], text: &str) -> Vec<i64> {
    matching_user_ids(
        &entries
            .iter()
            .map(|(id, phrases)| (*id, phrases.iter().map(|p| p.to_string()).collect()))
            .collect::<Vec<_>>(),
        text,
    )
    .unwrap()
}

#[test]
fn matches_case_insensitively() {
    assert_eq!(matches(&[(1, &["deploy"])], "Time to DEPLOY"), [1]);
}

#[test]
fn matches_on_word_boundaries_only() {
    assert!(matches(&[(1, &["deploy"])], "Redeploying now").is_empty());
    assert_eq!(matches(&[(1, &["deploy"])], "deploy, then lunch"), [1]);
}

#[test]
fn matches_multi_word_phrases() {
    assert_eq!(
        matches(&[(1, &["deploy freeze"])], "There is a Deploy Freeze today"),
        [1]
    );
    assert!(matches(&[(1, &["deploy freeze"])], "Deploy the freeze ray").is_empty());
}

#[test]
fn treats_phrases_literally() {
    assert_eq!(
        matches(&[(1, &["v1.2 (rc)"])], "Shipping v1.2 (rc) now"),
        [1]
    );
    assert!(matches(&[(1, &["v1.2 (rc)"])], "Shipping v1X2 rc now").is_empty());
}

#[test]
fn returns_every_user_with_a_match() {
    assert_eq!(
        matches(
            &[
                (1, &["deploy"]),
                (2, &["lunch", "deploy"]),
                (3, &["nothing"])
            ],
            "Deploy after lunch"
        ),
        [1, 2]
    );
}

#[test]
fn overlapping_phrases_across_users_all_match() {
    assert_eq!(
        matches(
            &[(1, &["deploy failed"]), (2, &["deploy"])],
            "deploy failed again"
        ),
        [1, 2]
    );
}

#[test]
fn nested_phrases_match_the_same_user_once() {
    for phrases in [
        &["deploy", "deploy failed"][..],
        &["deploy failed again", "deploy"][..],
    ] {
        assert_eq!(matches(&[(1, phrases)], "deploy failed again"), [1]);
    }
}

#[test]
fn repeated_phrases_match_every_holder_once() {
    assert_eq!(
        matches(
            &[(1, &["deploy"]), (2, &["deploy", "again"])],
            "deploy, deploy, deploy again"
        ),
        [1, 2]
    );
}

#[test]
fn ignores_blank_phrases_and_text() {
    assert!(matches(&[(1, &["  "])], "Deploy now").is_empty());
    assert_eq!(matches(&[(1, &["  ", "deploy"])], "Deploy now"), [1]);
    assert!(matches(&[(1, &["deploy"])], "   ").is_empty());
    assert!(matches(&[], "Deploy now").is_empty());
}

#[test]
fn matches_line_breaks_without_matching_inside_unicode_words() {
    assert_eq!(
        matches(&[(1, &["deploy failed"])], "the deploy\nfailed again"),
        [1]
    );
    assert!(matches(&[(1, &["caf"])], "meet at the café").is_empty());
    assert_eq!(matches(&[(1, &["café"])], "meet at the Café today"), [1]);
}
