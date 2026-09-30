//! All five Rails integration declarations: inspect the actual room GET response.
use super::call_channel_tests::configured;
use crate::controllers::presenters::test_support::{
    ALL_TALK, DAVID, DIRECT_DAVID_JASON, JASON, KEVIN, TestApp,
};
use axum::http::StatusCode;
use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig};
use campfire_db::{CachedStatements, Membership, Room, RoomType, Session};
#[tokio::test]
async fn remaining_presence_integration_composes_live_quiet_direct_group_and_disabled_headers() {
    for configured_call in [true, false] {
        let Some(test) = TestApp::boot_with_huddle(if configured_call {
            configured()
        } else {
            Default::default()
        })
        .await
        else {
            return;
        };
        let group = test
            .db()
            .write(|tx| Room::create_for(tx, RoomType::Direct, None, DAVID, &[DAVID, JASON, KEVIN]))
            .await
            .unwrap();
        let mut browser = test.sign_in(DAVID).await;
        let quiet = browser.get(&format!("/rooms/{ALL_TALK}")).await;
        assert_eq!(quiet.status, StatusCode::OK);
        let quiet = header(&quiet.text());
        assert_eq!(quiet.contains("voice-stack"), configured_call);
        if configured_call {
            assert!(quiet.contains("aria-label=\"Nobody in huddle\""));
            assert!(!quiet.contains("voice-stack--live"));
            assert!(!quiet.contains("voice-stack__avatar\""));
            assert!(quiet.contains("data-huddle-participants-target=\"count\" hidden"));
            assert!(quiet.contains("Join huddle"));
        }
        for room in [ALL_TALK, DIRECT_DAVID_JASON, group.id] {
            test.db()
                .write(move |tx| {
                    let session = Session::start(tx, JASON, None, None)?;
                    let member =
                        Membership::find_by_room_and_user(tx.conn(), room, JASON)?.unwrap();
                    let grant = HuddleGrant::issue(
                        tx,
                        session.id,
                        member.id,
                        room,
                        &HuddleConfig {
                            api_secret: Some("ws13-fixture-api-secret".into()),
                            admin_configured: false,
                        },
                    )?;
                    tx.conn().execute_cached(
                        "UPDATE huddle_grants SET last_seen_at=? WHERE id=?",
                        rusqlite::params![tx.now(), grant.id],
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
            let response = browser.get(&format!("/rooms/{room}")).await;
            assert_eq!(response.status, StatusCode::OK);
            let html = header(&response.text());
            if !configured_call {
                assert!(!html.contains("voice-stack"));
                continue;
            }
            for attribute in [
                "voice-stack--live voice-stack--huddle",
                "aria-label=\"1 in huddle: Jason\"",
                "title=\"1 in huddle: Jason\"",
                "data-huddle-participants-interval-value=\"15000\"",
                "data-huddle-participants-label-value=\"in huddle\"",
                &format!("data-user-id=\"{JASON}\""),
            ] {
                assert!(html.contains(attribute), "missing {attribute}: {html}");
            }
            assert!(html.contains("data-huddle-participants-target=\"count\" >1</span>"));
            let stack_end = html
                .find("data-huddle-participants-target=\"count\"")
                .unwrap();
            let tail = &html[stack_end..];
            let after = &tail[tail.find("</span>\n</span>").unwrap() + "</span>\n</span>".len()..];
            assert!(
                after.trim_start().starts_with("<button type=\"button\""),
                "{after}"
            );
            assert!(after[..after.find("</button>").unwrap()].contains("huddle-launcher"));
        }
    }
}
fn header(html: &str) -> String {
    let start = html.find("class=\"room-header__actions\"").unwrap();
    let end = html[start..].find("</nav>").unwrap() + start;
    html[start..end].to_owned()
}
