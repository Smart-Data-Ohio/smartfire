//! Complete owner-input fragments, with real Rails configuration/account states.
use crate::controllers::presenters::test_support::*;
use askama::Template;
use campfire_views::users;
async fn calendar_case(name: &str) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_profile_sections.json"
    ))
    .unwrap();
    let case = vectors["google_calendar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap();
    let data = serde_json::from_value(case["input"].clone()).unwrap();
    let actual = super::people_tests::render(&app, |_| {
        users::GoogleCalendar {
            sections: users::ProfileSections {
                google: data,
                ..Default::default()
            },
        }
        .render()
        .unwrap()
    });
    assert_eq!(
        actual,
        case["html"].as_str().unwrap(),
        "{name}: complete Google Calendar fragment"
    );
}
macro_rules! cases {
    ($($name:ident),* $(,)?) => { $(#[tokio::test] async fn $name() { calendar_case(stringify!($name)).await; })* };
}
cases!(
    unconfigured,
    missing,
    calendar_only,
    calendar_drive,
    drive_only,
    rejected_drive,
    rejected_calendar,
    retired_metadata,
    blank_reason
);
#[tokio::test]
async fn configured_calendar_profile_uses_real_account_metadata_and_forms() {
    let app = TestApp::boot_with_clock_and_env(
        seed_clock(),
        &[
            ("GOOGLE_CLIENT_ID", "parity-client"),
            ("GOOGLE_CLIENT_SECRET", "parity-secret"),
        ],
    )
    .await
    .expect("seed required");
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_profile_sections.json"
    ))
    .unwrap();
    for case in vectors["google_calendar"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["name"] != "unconfigured")
    {
        let case = case.clone();
        let setup = case.clone();
        app.db().write(move |tx| {
            tx.conn().execute("DELETE FROM google_accounts WHERE user_id=?",[DAVID])?;
            if setup["input"]["account_exists"]==true {
                tx.conn().execute("INSERT INTO google_accounts(user_id,email,scopes,disconnected_reason,created_at,updated_at) VALUES (?,?,?,?,?,?)",rusqlite::params![DAVID,setup["input"]["email"].as_str().unwrap(),setup["scopes"].as_str(),setup["reason"].as_str(),tx.now(),tx.now()])?;
            }
            Ok(())
        }).await.unwrap();
        let reply = app.david().get("/users/me/profile").await;
        assert_eq!(reply.status, axum::http::StatusCode::OK, "{}", case["name"]);
        let body = reply.text();
        let fragment = body
            .split_once("aria-labelledby=\"google-calendar-title\"")
            .unwrap()
            .1
            .split_once("</section>")
            .unwrap()
            .0;
        let input = &case["input"];
        let connected = input["connected"] == true;
        let calendar = input["calendar"] == true;
        assert!(!fragment.contains("not configured"));
        assert_eq!(
            fragment.contains("Connected as fixture&lt;&amp;&gt;@example.test"),
            connected && calendar,
            "{}",
            case["name"]
        );
        assert_eq!(
            fragment.contains("Disconnect</button>"),
            connected,
            "{}",
            case["name"]
        );
        assert_eq!(
            fragment.contains("Drive previews enabled"),
            connected && calendar && input["drive"] == true
        );
        assert_eq!(
            fragment.contains("Enable Drive previews"),
            connected && calendar && input["drive"] != true
        );
        assert_eq!(
            fragment.contains("name=\"features[]\" value=\"drive\""),
            (connected && calendar && input["drive"] != true)
                || (!(connected && calendar) && input["drive"] == true)
        );
        if input["account_exists"] != true {
            assert!(fragment.contains("Publish events you are going or maybe to"));
        } else if !connected {
            assert!(fragment.contains("Google rejected the connection, reconnect"));
        } else if !calendar {
            assert!(fragment.contains("Calendar permission needed, reconnect to publish events"));
        }
        let mut dom = campfire_richtext::dom::Dom::new();
        let root = dom.parse_fragment(&body).unwrap();
        let forms = dom.descendants(root).into_iter().filter(|id| dom.name(*id) == "form" && dom.attr(*id, "action") == Some("/google/connect")).collect::<Vec<_>>();
        assert_eq!(forms.len(), usize::from(!(connected && calendar && input["drive"] == true)), "{}: exact connect form count", case["name"]);
        for form in forms {
            assert_eq!(dom.attr(form, "method"), Some("post"));
            assert_eq!(dom.attr(form, "data-turbo"), Some("false"));
            let features = dom.descendants(form).into_iter().filter(|id| dom.name(*id) == "input" && dom.attr(*id, "name") == Some("features[]")).collect::<Vec<_>>();
            assert_eq!(features.len(), usize::from(input["drive"] == true || (connected && calendar)), "{}: exact replayed Drive feature count", case["name"]);
            for feature in features { assert_eq!(dom.attr(feature, "value"), Some("drive")); }
        }
        assert!(fragment.contains("name=\"authenticity_token\""));
        if connected && calendar { assert!(body.contains("never titles or attendees")); }
        let meetings = body
            .split_once("<legend class=\"txt-large\">Meetings</legend>")
            .unwrap()
            .1
            .split_once("</fieldset>")
            .unwrap()
            .0;
        assert_eq!(
            meetings.contains("type=\"checkbox\""),
            connected && calendar
        );
        assert_eq!(
            meetings.contains(">Connect Google Calendar</a>"),
            !(connected && calendar)
        );
    }
}

#[tokio::test]
async fn complete_status_panels_match_post_pin_rails_owner_facts() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_status_panels.json"
    ))
    .unwrap();
    for case in vectors["panels"].as_array().unwrap() {
        let actual = super::people_tests::render(&app, |ctx| {
            users::StatusPanel {
                ctx,
                sections: users::ProfileSections {
                    google: serde_json::from_value(case["google"].clone()).unwrap(),
                    status: serde_json::from_value(case["fields"].clone()).unwrap(),
                    ..Default::default()
                },
            }
            .render()
            .unwrap()
        });
        if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                format!("{dir}/panel-{}.actual", case["name"].as_str().unwrap()),
                &actual,
            )
            .unwrap();
            std::fs::write(
                format!("{dir}/panel-{}.expected", case["name"].as_str().unwrap()),
                case["html"].as_str().unwrap(),
            )
            .unwrap();
        }
        assert_eq!(
            actual,
            case["html"].as_str().unwrap(),
            "{}: complete status panel",
            case["name"]
        );
    }
}

#[tokio::test]
async fn live_status_sections_show_cache_errors_disconnects_and_manual_return_date() {
    let app = TestApp::boot_with_clock_and_env(
        seed_clock(),
        &[
            ("GOOGLE_CLIENT_ID", "parity-client"),
            ("GOOGLE_CLIENT_SECRET", "parity-secret"),
        ],
    )
    .await
    .expect("seed required");
    app.db().write(|tx| {
        tx.conn().execute("DELETE FROM google_accounts WHERE user_id=?",[DAVID])?;
        tx.conn().execute("DELETE FROM calendar_meeting_caches WHERE user_id=?",[DAVID])?;
        tx.conn().execute("INSERT INTO google_accounts(user_id,email,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![DAVID,"fixture<&>@example.test",tx.now(),tx.now()])?;
        tx.conn().execute("UPDATE users SET meeting_status_enabled=1,ooo_calendar_enabled=1,ooo_until=?,ooo_note=? WHERE id=?",rusqlite::params![campfire_db::Timestamp::from_jiff("2026-03-03T22:00:00Z".parse().unwrap()),"Back <&> soon",DAVID])?;
        tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,fetch_error,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![DAVID,"<Network & refresh>",tx.now(),tx.now()])?;
        Ok(())
    }).await.unwrap();
    let reply = app.david().get("/users/me/profile").await;
    assert_eq!(reply.status, axum::http::StatusCode::OK);
    let body = reply.text();
    assert_eq!(body.matches("&lt;Network &amp; refresh&gt;").count(), 2);
    assert!(body.contains("Out of office until March 03, 2026."));
    assert!(body.contains("value=\"Back &lt;&amp;&gt; soon\""));
    assert!(body.contains(
        "id=\"user_meeting_status_enabled\" value=\"1\" class=\"switch__input\" checked=\"checked\""
    ));
    assert!(body.contains(
        "id=\"user_ooo_calendar_enabled\" value=\"1\" class=\"switch__input\" checked=\"checked\""
    ));
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE google_accounts SET disconnected_reason='invalid_grant' WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = app.david().get("/users/me/profile").await;
    assert_eq!(reply.status, axum::http::StatusCode::OK);
    let body = reply.text();
    assert!(body.contains("Meeting status is on, but Google Calendar isn't connected."));
    assert!(body.contains("Calendar out-of-office is on, but Google Calendar isn't connected."));
    assert_eq!(body.matches(">Reconnect below</a>").count(), 2);
    assert!(!body.contains("&lt;Network &amp; refresh&gt;"));
    assert!(!body.contains("type=\"checkbox\" name=\"user[meeting_status_enabled]\""));
}
