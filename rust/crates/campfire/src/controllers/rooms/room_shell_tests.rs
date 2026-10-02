use crate::controllers::presenters::test_support::{DAVID, KEVIN, TestApp};
use campfire_db::{Room, RoomType};

#[tokio::test]
async fn room_shell_request_has_accessible_list_and_pending_message() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let mut browser = test.sign_in(DAVID).await;
    let html = browser.get("/rooms/486777696").await.text();
    assert!(html.contains(
        "class=\"messages\" role=\"log\" aria-live=\"polite\" aria-relevant=\"additions\""
    ));
    let pending = html
        .split("<script type=\"text/template\"")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    assert!(pending.contains("tabindex=\"-1\""));
    assert!(pending.contains("data-profile-card-url=\"/users/127326141/card\""));
    assert!(!pending.contains("message__actions"));
    assert_eq!(
        html.matches("<link rel=\"modulepreload\" href=\"/assets/controllers/")
            .count(),
        11
    );
}

#[tokio::test]
async fn room_shell_ooo_subscription_is_per_viewer_and_never_cached() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let room=test.db().write(|tx| {
        tx.conn().execute("UPDATE users SET ooo_until='2026-03-04 02:00:00',ooo_note='Trip <&>',time_zone='Pacific Time (US & Canada)' WHERE id=?",[KEVIN])?;
        Room::create_for(tx,RoomType::Direct,None,DAVID,&[DAVID,KEVIN])
    }).await.unwrap();
    let mut david = test.sign_in(DAVID).await;
    let mut kevin = test.sign_in(KEVIN).await;
    let path = format!("/rooms/{}", room.id);
    let html = david.get(&path).await.text();
    assert!(html.contains("Kevin is out of office until March 03, 2026. Trip &lt;&amp;&gt;"));
    assert!(html.contains(&format!("id=\"ooo_notice_user_{KEVIN}\"")));
    assert!(!html.contains(&format!("id=\"ooo_notice_user_{DAVID}\"")));
    let other = kevin.get(&path).await.text();
    assert!(!other.contains("Trip &lt;&amp;&gt;"));
    assert!(other.contains(&format!("id=\"ooo_notice_user_{DAVID}\"")));
    assert!(!other.contains(&format!("id=\"ooo_notice_user_{KEVIN}\"")));
    test.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET presence_setting='invisible' WHERE id=?",
                [KEVIN],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let invisible = david.get(&path).await.text();
    assert!(!invisible.contains("Trip &lt;&amp;&gt;"));
    assert!(invisible.contains(&format!("id=\"ooo_notice_user_{KEVIN}\"")));
    test.booted
        .jobs
        .shutdown(std::time::Duration::from_secs(1))
        .await;
}

#[tokio::test]
async fn room_shell_matches_thirty_eight_complete_rails_renders() {
    use crate::controllers::presenters::{page, view_context};
    use askama::Template;
    use campfire_views::helpers::request_forgery::{RequestSecrets, rendering_with};
    use campfire_views::{
        messages::UserView,
        rooms::{RoomView, shell},
    };
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let account = test.db().read(campfire_db::Account::first).await.unwrap();
    let actor = test
        .db()
        .read(|c| campfire_db::User::find(c, DAVID))
        .await
        .unwrap();
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("room_shell_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 38);
    for case in vectors["cases"].as_array().unwrap() {
        let i = &case["input"];
        let actual = page::render_detached_at(
            &test.booted.app,
            account.as_ref(),
            "http://campfire.test",
            |ctx| {
                let mut ctx = ctx.clone();
                ctx.current_user =
                    Some(view_context::current_user(&test.booted.app.secrets, &actor));
                rendering_with(
                    RequestSecrets {
                        tokens: Box::new(super::call_page_tests::Tokens),
                        csp_nonce: None,
                    },
                    || match case["partial"].as_str().unwrap() {
                        "pending" => {
                            let user: UserView = serde_json::from_value(i["user"].clone()).unwrap();
                            shell::Pending {
                                ctx: &ctx,
                                user: &user,
                            }
                            .render()
                            .unwrap()
                        }
                        "head" => shell::preloads(&ctx),
                        "area" => {
                            let room: RoomView = serde_json::from_value(i["room"].clone()).unwrap();
                            shell::area(&ctx, &room, i["scroll"].as_bool().unwrap(), "BODY").0
                        }
                        "list" => {
                            let room: RoomView = serde_json::from_value(i["room"].clone()).unwrap();
                            shell::list(
                                &ctx,
                                &room,
                                i["updated_at"].as_str().unwrap().parse().unwrap(),
                                "BODY",
                            )
                            .0
                        }
                        "unread" => shell::unread(i["count"].as_i64().unwrap()),
                        "jump" => shell::jump(&ctx, i["url"].as_str()).0,
                        "invitation" => shell::Invitation {
                            ctx: &ctx,
                            join_code: i["join_code"].as_str().unwrap(),
                        }
                        .render()
                        .unwrap(),
                        "ooo" => {
                            let notices: Vec<shell::Notice> =
                                serde_json::from_value(i["notices"].clone()).unwrap();
                            shell::Notices {
                                ctx: &ctx,
                                notices: &notices,
                            }
                            .render()
                            .unwrap()
                        }
                        _ => panic!("unknown room shell partial"),
                    },
                )
            },
        );
        let expected = case["html"].as_str().unwrap();
        assert!(
            crate::app::asset_goldens::compare(case["name"].as_str().unwrap(), &actual, expected),
            "{} actual:\n{actual}\nexpected:\n{expected}",
            case["name"]
        );
    }
    test.booted
        .jobs
        .shutdown(std::time::Duration::from_secs(1))
        .await;
}

#[tokio::test]
async fn room_shell_ooo_request_adapter_matches_recorded_calendar_and_manual_states() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let room = test
        .db()
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Direct,
                None,
                DAVID,
                &[DAVID, KEVIN, super::super::presenters::test_support::JASON],
            )
        })
        .await
        .unwrap();
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("room_shell_vectors.json")).unwrap();
    for case in vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["partial"] == "ooo")
    {
        let state = case["input"]["state"].clone();
        test.db().write(move |tx| {
            tx.conn().execute("UPDATE users SET ooo_until=?,ooo_note=?,presence_setting=?,time_zone=?,ooo_calendar_enabled=? WHERE id=?",rusqlite::params![state["ooo_until"].as_str().map(|s|campfire_db::Timestamp::from_jiff(s.parse().unwrap())),state["ooo_note"].as_str(),state["presence"].as_str(),state["zone"].as_str(),state["calendar"].as_bool(),KEVIN])?;
            tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES(?,'[]',?,?,?) ON CONFLICT(user_id) DO UPDATE SET ooo_intervals=excluded.ooo_intervals",rusqlite::params![KEVIN,state["intervals"].to_string(),tx.now(),tx.now()])?;Ok(())
        }).await.unwrap();
        let room = room.clone();
        let viewer = case["input"]["viewer_id"].as_i64().unwrap();
        let now = test.booted.app.db.env().now();
        let actual = test
            .db()
            .read(move |c| super::shell::load(c, &room, viewer, &[], now))
            .await
            .unwrap();
        let expected: Vec<campfire_views::rooms::shell::Notice> =
            serde_json::from_value(case["input"]["notices"].clone()).unwrap();
        assert_eq!(actual.notices, expected, "{}", case["name"]);
    }
    test.booted
        .jobs
        .shutdown(std::time::Duration::from_secs(1))
        .await;
}

#[tokio::test]
async fn room_shell_unread_pointer_matches_count_threshold_deleted_cursor_and_off_page_jump() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let room=test.db().write(|tx| {
        let room=Room::create_for(tx,RoomType::Closed,Some("Unread room"),DAVID,&[DAVID,KEVIN])?;
        for id in 10001..10008 {
            tx.conn().execute("INSERT INTO messages(id,room_id,creator_id,client_message_id,created_at,updated_at) VALUES(?,?,?,?,'2026-03-02 15:00:00','2026-03-02 15:00:00')",rusqlite::params![id,room.id,DAVID,format!("unread-{id}")])?;
        }
        tx.conn().execute("UPDATE memberships SET unread_at='2026-03-02 15:00:00',last_read_message_id=10001 WHERE room_id=? AND user_id=?",[room.id,DAVID])?;
        Ok(room)
    }).await.unwrap();
    let mut browser = test.sign_in(DAVID).await;
    let path = format!("/rooms/{}", room.id);
    let html = browser.get(&path).await.text();
    assert!(html.contains("data-unread-count=\"6\""));
    assert!(html.contains("data-messages-scroll-to-divider-value=\"true\""));
    assert!(
        html.find("id=\"message_unread-10001\"").unwrap()
            < html.find("id=\"unread-divider\"").unwrap()
    );
    assert!(
        html.find("id=\"unread-divider\"").unwrap()
            < html.find("id=\"message_unread-10002\"").unwrap()
    );
    test.db()
        .write(move |tx| {
            tx.conn()
                .execute("DELETE FROM messages WHERE id=10001", [])?;
            tx.conn().execute(
                "UPDATE memberships SET last_read_message_id=10002 WHERE room_id=? AND user_id=?",
                [room.id, DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let html = browser.get(&path).await.text();
    assert!(html.contains("data-unread-count=\"5\""));
    assert!(!html.contains("data-messages-scroll-to-divider-value"));
    let id = room.id;
    test.db().write(move |tx| {
        tx.conn().execute("DELETE FROM messages WHERE id=10002",[])?;
        for n in 10008..10060 {tx.conn().execute("INSERT INTO messages(id,room_id,creator_id,client_message_id,created_at,updated_at) VALUES(?,?,?,?,'2026-03-02 15:00:00','2026-03-02 15:00:00')",rusqlite::params![n,id,DAVID,format!("unread-{n}")])?;}
        Ok(())
    }).await.unwrap();
    let html = browser.get(&path).await.text();
    assert!(!html.contains("id=\"unread-divider\""));
    assert!(html.contains(&format!("href=\"{path}?message_id=10003\"")));
    let anchored = browser
        .get(&format!("{path}?message_id=10003"))
        .await
        .text();
    assert!(anchored.contains("data-unread-count=\"57\""));
    assert!(anchored.contains("id=\"unread-divider\""));
    test.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE memberships SET unread_at=NULL WHERE room_id=? AND user_id=?",
                [id, DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let read = browser.get(&path).await.text();
    assert!(!read.contains("id=\"jump-to-unread\""));
    assert!(!read.contains("id=\"unread-divider\""));
    test.booted
        .jobs
        .shutdown(std::time::Duration::from_secs(1))
        .await;
}
