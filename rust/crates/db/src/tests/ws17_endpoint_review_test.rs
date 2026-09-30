use super::*;
use crate::PushSubscription;
use serde_json::Value;

#[test]
fn ws17_review_endpoint_validation_and_writes_follow_rails_uri() {
    let v: Value =
        serde_json::from_str(include_str!("../../../../vectors/ws17_endpoint_urls.json")).unwrap();
    let t = TestDb::new();
    let before = t.read(PushSubscription::count);
    let mut differences = Vec::new();
    for row in v["rows"].as_array().unwrap() {
        let sub = PushSubscription::new(
            id("david"),
            row["endpoint"].as_str(),
            Some("test_key"),
            Some("test_auth"),
            None,
        );
        let resolver = |_: &str| Some("142.250.185.206".to_owned());
        let errors = sub.validate(&resolver).full_messages();
        let actual_errors = serde_json::to_value(errors).unwrap();
        let actual_ip = serde_json::to_value(sub.resolved_endpoint_ip(&resolver)).unwrap();
        let result = t.try_write(move |tx| {
            let saved = PushSubscription::create(tx, &sub, &resolver)?;
            saved.destroy(tx)
        });
        if actual_errors != row["errors"]
            || actual_ip != row["resolved"]
            || result.is_ok() != row["valid"].as_bool().unwrap()
        {
            differences.push(format!(
                "errors={actual_errors}, resolved={actual_ip}, saved={} {row}",
                result.is_ok()
            ));
        }
        assert_eq!(
            t.read(PushSubscription::count),
            before,
            "invalid writes must leave no rows: {row}"
        );
    }
    assert!(
        differences.is_empty(),
        "{} endpoint differences: {}",
        differences.len(),
        differences
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
