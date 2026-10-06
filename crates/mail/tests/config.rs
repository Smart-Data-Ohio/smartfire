use campfire_mail::config::Config;
fn config(pairs: &[(&str, &str)]) -> Config {
    Config::from_map(
        &pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
    .unwrap()
}
#[test]
fn security_configuration_requires_smtp_sender_and_origin() {
    assert!(
        config(&[
            ("SMTP_ADDRESS", "smtp.example.com"),
            ("MAILER_FROM", "alerts@example.com"),
            ("APP_URL", "https://example.com")
        ])
        .security_configured()
    );
    for missing in ["SMTP_ADDRESS", "MAILER_FROM", "APP_URL"] {
        let pairs = [
            ("SMTP_ADDRESS", "smtp.example.com"),
            ("MAILER_FROM", "alerts@example.com"),
            ("APP_URL", "https://example.com"),
        ]
        .into_iter()
        .filter(|(k, _)| *k != missing)
        .collect::<Vec<_>>();
        assert!(!config(&pairs).security_configured());
    }
}
#[test]
fn mail_is_unconfigured_by_default() {
    assert!(!Config::default().two_factor_configured());
    assert!(!Config::default().security_configured());
}
#[test]
fn smtp_environment_contract() {
    let c = config(&[("SMTP_ADDRESS", "mail.test")]);
    let smtp = c.smtp.unwrap();
    assert_eq!(smtp.port, 25);
    assert!(smtp.enable_starttls);
    let c = config(&[
        ("SMTP_ADDRESS", "mail.test"),
        ("SMTP_PORT", " 40001suffix"),
        ("SMTP_ENABLE_STARTTLS", "false"),
        ("SMTP_DOMAIN", "mail.test"),
        ("SMTP_USER_NAME", "fixture-user"),
        ("SMTP_PASSWORD", "fixture-password"),
        ("SMTP_AUTHENTICATION", "login"),
    ]);
    let smtp = c.smtp.unwrap();
    assert_eq!(smtp.port, 40001);
    assert!(!smtp.enable_starttls);
    assert_eq!(smtp.domain.as_deref(), Some("mail.test"));
    assert_eq!(smtp.authentication.as_deref(), Some("login"));
    assert!(
        config(&[
            ("SMTP_ADDRESS", "mail.test"),
            ("SMTP_ENABLE_STARTTLS", "FALSE")
        ])
        .smtp
        .unwrap()
        .enable_starttls
    );
}
#[test]
fn invalid_absolute_origins_fail_at_boot() {
    for url in [
        "example.com",
        "ftp://example.com",
        "https://user:password@example.com",
        "https://example.com/path",
        "https://example.com?x=1",
        "https://example.com#fragment",
    ] {
        assert!(
            Config::from_map(&[("APP_URL".into(), url.into())].into()).is_err(),
            "{url}"
        );
    }
    assert_eq!(
        config(&[("APP_URL", "http://example.com:40000/")])
            .app_url
            .as_deref(),
        Some("http://example.com:40000")
    );
}
#[test]
fn configuration_debug_never_prints_passwords() {
    let c = config(&[
        ("SMTP_ADDRESS", "mail.test"),
        ("SMTP_PASSWORD", "private-smtp-value"),
        ("RAILS_INBOUND_EMAIL_PASSWORD", "private-ingress-value"),
    ]);
    let formatted = format!("{c:?}");
    assert!(!formatted.contains("private-smtp-value"));
    assert!(!formatted.contains("private-ingress-value"));
}
