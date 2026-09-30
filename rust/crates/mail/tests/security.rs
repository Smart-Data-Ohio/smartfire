use base64::{Engine as _, engine::general_purpose::STANDARD};
use campfire_mail::{
    config::{Config, RelayAuth, relay_auth},
    parse::authenticated_sender,
};
#[test]
fn review_non_ascii_authentication_whitespace_is_rejected() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
    let case = corpus["auth"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["headers"][0] == "mx.mail.test; dkim=pass\u{00a0}header.d=example.com")
        .unwrap();
    assert_eq!(case["expected"], false);
    let header = case["headers"][0].as_str().unwrap();
    assert!(!authenticated_sender(
        &[header.into()],
        Some("mx.mail.test"),
        "member@example.com"
    ));
    let raw = format!(
        "From: member@example.com\r\nTo: room-token@mail.test\r\nAuthentication-Results: {header}\r\n\r\nHello"
    );
    let email = campfire_mail::parse::Email::parse(raw.as_bytes()).unwrap();
    assert!(!authenticated_sender(
        &email.auth_headers,
        Some("mx.mail.test"),
        "member@example.com"
    ));
}
#[test]
fn trusted_relay_authentication_matches_rails_corpus() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
    for case in corpus["auth"].as_array().unwrap() {
        let headers = case
            .get("parsed_headers")
            .unwrap_or(&case["headers"])
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            authenticated_sender(
                &headers,
                case["authserv_id"].as_str(),
                case["address"].as_str().unwrap()
            ),
            case["expected"].as_bool().unwrap(),
            "{case}"
        );
    }
}
#[test]
fn spoofed_sender_without_trusted_relay_is_never_a_member() {
    assert!(!authenticated_sender(
        &[],
        Some("mx.mail.test"),
        "member@example.com"
    ));
    assert!(!authenticated_sender(
        &["foreign.test; dkim=pass header.d=example.com".into()],
        Some("mx.mail.test"),
        "member@example.com"
    ));
    assert!(!authenticated_sender(
        &["mx.mail.test; spf=pass smtp.helo=example.com".into()],
        Some("mx.mail.test"),
        "member@example.com"
    ));
    assert!(!authenticated_sender(
        &["mx.mail.test; dkim=pass header.d=example.com".into()],
        None,
        "member@example.com"
    ));
}
#[test]
fn topmost_matching_relay_header_wins() {
    let h = vec![
        "mx.mail.test; dkim=fail header.d=example.com".into(),
        "mx.mail.test; dkim=pass header.d=example.com".into(),
    ];
    assert!(!authenticated_sender(
        &h,
        Some("mx.mail.test"),
        "member@example.com"
    ));
}
#[test]
fn relay_basic_auth_matrix() {
    let mut cfg = Config {
        domain: Some("mail.test".into()),
        ingress_password: Some("fixture-mail-password".into()),
        ..Default::default()
    };
    let good = format!(
        "Basic {}",
        STANDARD.encode("actionmailbox:fixture-mail-password")
    );
    assert_eq!(relay_auth(&cfg, Some(&good)), RelayAuth::Accepted);
    for bad in [
        None,
        Some("Basic invalid"),
        Some("Bearer fixture-mail-password"),
        Some("Basic YWN0aW9ubWFpbGJveDp3cm9uZw=="),
    ] {
        assert_eq!(relay_auth(&cfg, bad), RelayAuth::Unauthorized);
    }
    cfg.ingress_password = None;
    assert_eq!(relay_auth(&cfg, None), RelayAuth::MissingPassword);
    cfg.domain = None;
    assert_eq!(relay_auth(&cfg, Some(&good)), RelayAuth::Disabled);
}

#[test]
fn relay_base64_and_scheme_coercions_match_rails() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
    let cfg = Config {
        domain: Some("mail.test".into()),
        ingress_password: Some("fixture-mail-password".into()),
        ..Default::default()
    };
    for case in corpus["relay"].as_array().unwrap() {
        assert_eq!(
            relay_auth(&cfg, case["authorization"].as_str()) == RelayAuth::Accepted,
            case["expected"].as_bool().unwrap(),
            "{case}"
        );
    }
}

#[test]
fn parsed_mail_authentication_headers_match_rails_corpus() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
    for case in corpus["auth"].as_array().unwrap() {
        let fields = case["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| format!("Authentication-Results: {}\r\n", h.as_str().unwrap()))
            .collect::<String>();
        let raw = format!(
            "From: {}\r\nTo: room-token@mail.test\r\n{fields}\r\nHello",
            case["address"].as_str().unwrap()
        );
        let email = campfire_mail::parse::Email::parse(raw.as_bytes()).unwrap();
        if let Some(expected) = case.get("parsed_headers") {
            assert_eq!(serde_json::json!(email.auth_headers), *expected, "{case}");
        }
        assert_eq!(
            authenticated_sender(
                &email.auth_headers,
                case["authserv_id"].as_str(),
                case["address"].as_str().unwrap()
            ),
            case["expected"].as_bool().unwrap(),
            "{case}"
        );
    }
}
