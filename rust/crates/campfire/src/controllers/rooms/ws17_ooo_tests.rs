//! Named uncached DM integration replays; compare the complete owned wrapper to actual Rails.
use crate::controllers::presenters::{status_settings, test_support::*};
use axum::http::{Method, StatusCode};
use campfire_db::{Room, RoomType, Timestamp};
use serde_json::Value;
fn golden() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../crates/views/tests/golden/ws17-dm-profile.json"
    ))
    .unwrap()
}
async fn replay(names: &[&str]) {
    let data = golden();
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the actual parity seed");
    let group = app
        .db()
        .write(|tx| Room::create_for(tx, RoomType::Direct, None, KEVIN, &[DAVID, JASON, KEVIN]))
        .await
        .unwrap();
    let notice_class = regex::Regex::new(r#"\bclass="[^"]*\booo-notice(?:\s|")"#).unwrap();
    for name in names {
        let row = data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == *name)
            .unwrap()
            .clone();
        let fixture = row.clone();
        app.db().write(move |tx| {
            tx.conn().execute("DELETE FROM calendar_meeting_caches",[])?;
            tx.conn().execute("UPDATE users SET ooo_until=NULL,ooo_note=NULL,ooo_calendar_enabled=0,presence_setting='auto',time_zone='UTC'",[])?;
            for (id,attrs) in fixture["attrs"].as_object().unwrap() {
                for (key,value) in attrs.as_object().unwrap() {
                    let value=match value {Value::Bool(b)=>rusqlite::types::Value::Integer(i64::from(*b)),Value::String(s) if key=="ooo_until"=>rusqlite::types::Value::Text(Timestamp::from_jiff(s.parse().unwrap()).to_db()),Value::String(s)=>rusqlite::types::Value::Text(s.clone()),_=>panic!("invalid fixture")};
                    tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"),rusqlite::params![value,id.parse::<i64>().unwrap()])?;
                }
            }
            if fixture["name"]=="invisible_calendar" {
                tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,fetched_at,ooo_intervals,created_at,updated_at) VALUES (?,?,'[[\"2026-03-02T15:55:00Z\",\"2026-03-04T16:00:00Z\"]]',?,?)",rusqlite::params![DAVID,tx.now(),tx.now(),tx.now()])?;
            }
            Ok(())
        }).await.unwrap();
        let viewer = row["viewer_id"].as_i64().unwrap();
        let room_id = if row["group"] == true {
            group.id
        } else if row["direct"] == true {
            DIRECT_DAVID_JASON
        } else {
            ALL_TALK
        };
        let secrets = app.booted.app.secrets.clone();
        let now = app.db().env().now();
        let members = app
            .db()
            .read(move |conn| {
                status_settings::ooo_notice_members(
                    conn,
                    &secrets,
                    &Room::find(conn, room_id)?,
                    viewer,
                    now,
                )
            })
            .await
            .unwrap();
        let expected = serde_json::from_value::<
            Vec<campfire_views::users::statuses::OooNoticeMember>,
        >(row["members"].clone())
        .unwrap();
        assert_eq!(members, expected, "{name}: complete uncached member facts");
        let mut browser = app.sign_in(viewer).await;
        let reply = browser.get(&format!("/rooms/{room_id}")).await;
        assert_eq!(reply.status, StatusCode::OK, "{name}: {}", reply.text());
        let html = reply.text();
        let expected = row["html"].as_str().unwrap();
        let notices = notice_class.find_iter(&html).count();
        if expected.is_empty() {
            assert!(!html.contains("id=\"ooo-notices\""));
            assert_eq!(notices, 0);
        } else {
            assert!(
                html.contains(expected),
                "{name}: complete Rails wrapper absent"
            );
            assert!(
                html.find("id=\"ooo-notices\"").unwrap() < html.find("id=\"composer\"").unwrap(),
                "above composer"
            );
            assert_eq!(
                notices,
                row["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|member| member["visible"] == true)
                    .count()
            );
        }
        assert!(!html.contains("<b>gone</b>"));
    }
}
macro_rules! scenario {($name:ident,$($case:literal),+) => {#[tokio::test]async fn $name(){replay(&[$($case),+]).await;}};}
scenario!(ws17_dm_ooo_shows_notice_above_composer, "fixed_return");
scenario!(ws17_dm_ooo_escapes_member_note, "escaped_note");
scenario!(
    ws17_dm_ooo_renders_per_viewer_without_shared_fragment,
    "viewer_jason",
    "viewer_david"
);
scenario!(ws17_group_dm_ooo_one_line_per_recipient, "group");
scenario!(ws17_channel_has_no_ooo_notice, "channel");
scenario!(ws17_dm_with_nobody_out_has_no_notice, "nobody_out");
scenario!(
    ws17_invisible_manual_ooo_has_no_dm_notice,
    "invisible_manual"
);
scenario!(
    ws17_invisible_calendar_ooo_has_no_dm_notice,
    "invisible_calendar"
);

#[tokio::test]
async fn ws17_profile_mounts_live_badge_and_viewer_scoped_allowance_control() {
    let app = TestApp::boot().await.expect("parity seed");
    app.db().write(|tx|{tx.conn().execute("UPDATE users SET custom_status_text='<busy & away>',custom_status_emoji='🌱',presence_setting='dnd' WHERE id=?",[JASON])?;Ok(())}).await.unwrap();
    app.db()
        .write(|tx| {
            let session_id = tx.conn().query_row(
                "SELECT id FROM sessions WHERE user_id=? LIMIT 1",
                [JASON],
                |row| row.get(0),
            )?;
            campfire_db::WorkspacePresenceLease::establish(tx, JASON, session_id)?;
            Ok(())
        })
        .await
        .unwrap();
    let mut browser = app.david();
    let path = format!("/users/{JASON}");
    let allowance = format!("/users/{JASON}/dnd_allowance");
    let html = browser.get(&path).await.text();
    let signed = rails_compat::turbo::signed_stream_name(
        &app.booted.app.secrets,
        &[&crate::channels::user_gid(JASON).to_param(), "status"],
    );
    assert!(html.contains(&format!("signed-stream-name=\"{signed}\"")));
    assert!(html.contains(&format!("id=\"status_badge_user_{JASON}\"")));
    assert!(html.contains("data-presence=\"dnd\""));
    assert!(html.contains("🌱 &lt;busy &amp; away&gt;"));
    assert!(html.contains("Allow during DND"));
    assert_eq!(
        browser
            .write(Req::new(Method::POST, &allowance))
            .await
            .status,
        StatusCode::FOUND
    );
    assert!(browser.get(&path).await.text().contains("Mute during DND"));
    assert!(
        !app.sign_in(KEVIN)
            .await
            .get(&path)
            .await
            .text()
            .contains("Mute during DND")
    );
    assert_eq!(
        browser
            .write(Req::new(Method::DELETE, &allowance))
            .await
            .status,
        StatusCode::FOUND
    );
    assert!(browser.get(&path).await.text().contains("Allow during DND"));
    assert!(
        !browser
            .get(&format!("/users/{DAVID}"))
            .await
            .text()
            .contains("during DND")
    );
}
