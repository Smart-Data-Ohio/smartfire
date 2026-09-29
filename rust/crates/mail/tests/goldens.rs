use campfire_mail::{
    config::Config,
    outbound::{self, SignIn, User},
    parse::html_to_text,
};
#[test]
fn html_text_matches_nokogiri_corpus() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
    for case in v["html"].as_array().unwrap() {
        assert_eq!(
            html_to_text(case["html"].as_str().unwrap()).unwrap(),
            case["expected"].as_str().unwrap(),
            "{case}"
        );
    }
}
#[test]
fn multipart_messages_match_rails_byte_for_byte() {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
    let config = Config {
        mailer_from: Some("Smartfire <alerts@example.com>".into()),
        app_url: Some("https://example.com".into()),
        ..Default::default()
    };
    for case in v["messages"].as_array().unwrap() {
        let user = User {
            name: case["name"].as_str().unwrap().into(),
            email: case["email"].as_str().unwrap().into(),
        };
        let now = case["timestamp"].as_str().unwrap().parse().unwrap();
        let message = if case["kind"] == "sign_in" {
            outbound::new_sign_in_alert(
                &config,
                Some(&SignIn {
                    user,
                    device: case["device"].as_str().unwrap().into(),
                    created_at: now,
                }),
            )
            .unwrap()
        } else {
            outbound::lockout_notice(&user)
        };
        assert_eq!(message.text, case["text"].as_str().unwrap());
        assert_eq!(message.html, case["html"].as_str().unwrap());
        assert_eq!(
            String::from_utf8(
                message
                    .encoded(
                        now,
                        case["message_id"].as_str().unwrap(),
                        case["boundary"].as_str().unwrap()
                    )
                    .unwrap()
            )
            .unwrap(),
            case["raw"].as_str().unwrap()
        );
    }
}
#[test]
fn revoked_session_skips_delivery() {
    assert!(outbound::new_sign_in_alert(&Config::default(), None).is_none());
}
#[test]
fn header_injection_is_rejected() {
    let m = outbound::lockout_notice(&User {
        name: "Person".into(),
        email: "victim@example.com\r\nBcc: attacker@example.com".into(),
    });
    assert!(
        m.encoded(
            "2026-09-29T12:30:00Z".parse().unwrap(),
            "id@example.com",
            "test-boundary"
        )
        .is_err()
    );
}
