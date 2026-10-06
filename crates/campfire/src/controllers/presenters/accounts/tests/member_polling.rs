//! Raw responses from `Rooms::MembersController` at d7c7de92, including its 13 named
//! Rails cases and boundary/escaping/cache probes. Every case owns a fresh seed copy.
use super::*;

async fn compare(names: &[&str]) {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../vectors/member-polling.json"
    ))
    .unwrap();
    let mut checked = 0;
    for case in corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| names.contains(&case["name"].as_str().unwrap()))
    {
        let name = case["name"].as_str().unwrap();
        let clock = std::sync::Arc::new(campfire_kit::FrozenClock::new(
            corpus["now"].as_str().unwrap().parse().unwrap(),
        ));
        let test = boot_seed_with_clock("default", clock)
            .await
            .expect("default parity seed");
        let statements: Vec<String> = corpus["setup"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|_| case["setup"] != false)
            .chain(case["sql"].as_array().unwrap())
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        test.booted
            .app
            .db
            .write(move |tx| {
                for sql in statements {
                    tx.conn().execute_batch(&sql)?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = test.browser("198.51.100.118");
        if let Some(viewer) = case["viewer"].as_str() {
            browser.cookies.insert(
                "session_token".into(),
                test.label(&format!("session_cookies.{viewer}")),
            );
        }
        let mut header_values: Vec<_> = case["headers"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_owned()))
            .collect();
        if let Some(token) = case["token"].as_str() {
            header_values.push(("Authorization".into(), format!("Bearer {token}")));
        }
        let headers: Vec<_> = header_values.iter().map(|(key, value)| (key.as_str(), value.as_str())).collect();
        let response = browser
            .request(Method::GET, case["path"].as_str().unwrap(), &headers, None)
            .await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{name}: {}",
            response.text()
        );
        assert_eq!(
            response.text(),
            case["body"].as_str().unwrap(),
            "{name}: raw Rails response"
        );
        for (header, value) in case["response_headers"].as_object().unwrap() {
            assert_eq!(response.header(header), value.as_str(), "{name}: {header}");
        }
        if response.status == StatusCode::OK {
            let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
            for member in json["members"].as_array().unwrap() {
                let keys: Vec<_> = member
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect();
                assert_eq!(
                    keys,
                    [
                        "id",
                        "name",
                        "avatar_url",
                        "bot",
                        "online",
                        "presence",
                        "status",
                        "starred"
                    ],
                    "{name}: no private fields"
                );
            }
        } else {
            assert!(response.body.is_empty(), "{name}: no private room data");
        }
        checked += 1;
    }
    assert_eq!(checked, names.len(), "every named oracle case must run");
}

#[tokio::test]
async fn member_polling_matches_active_members_order_status_escaping_and_fresh_avatars() {
    compare(&[
        "baseline",
        "active_custom",
        "custom_expired",
        "custom_untrimmed",
        "html_escaping_and_id_order",
        "fresh_avatar",
    ])
    .await;
}
#[tokio::test]
async fn member_polling_matches_meeting_ooo_precedence_and_the_members_zone() {
    compare(&[
        "meeting",
        "meeting_custom",
        "meeting_dnd",
        "meeting_invisible",
        "meeting_quiet_hours",
        "meeting_off",
        "ooo",
        "ooo_own_zone",
        "ooo_wins_custom",
        "ooo_invisible_custom",
        "calendar_ooo",
    ])
    .await;
}
#[tokio::test]
async fn member_polling_matches_lease_boundaries_activity_and_revoked_sessions() {
    compare(&[
        "idle",
        "active_cutoff",
        "nil_activity",
        "expiry_boundary",
        "expired_lease",
        "wrong_session_identity",
        "active_beats_idle",
        "revoked_session",
    ])
    .await;
}
#[tokio::test]
async fn member_polling_matches_manual_dnd_invisible_and_offline_presence() {
    compare(&["dnd", "invisible", "offline_dnd"]).await;
}
#[tokio::test]
async fn member_polling_excludes_inactive_users() {
    compare(&["inactive"]).await;
}
#[tokio::test]
async fn member_polling_requires_authentication_and_alive_room_membership() {
    compare(&[
        "unsigned",
        "unsigned_accept",
        "closed_forbidden",
        "missing_room",
        "deleted_room",
        "no_membership",
        "bot_key_forbidden",
        "agent_token_forbidden",
    ])
    .await;
}
#[tokio::test]
async fn member_polling_matches_viewer_stars_and_rails_conditional_requests() {
    compare(&[
        "stars_david",
        "stars_jason",
        "no_conditional_304",
        "exact_etag_304",
        "etag_list",
        "other_viewer_star_validator",
    ])
    .await;
}
#[tokio::test]
async fn member_polling_matches_agent_checkins_suspension_working_expiry_and_legacy_bots() {
    compare(&[
        "agent_checked_in",
        "agent_suspended",
        "agent_note",
        "agent_working",
        "agent_working_boundary",
        "agent_blank_note",
        "legacy_bot_presence",
    ])
    .await;
}

#[tokio::test]
async fn member_polling_matches_the_full_parity_seed_for_open_closed_and_direct_rooms() {
    compare(&["seed_open", "seed_closed", "seed_direct"]).await;
}
