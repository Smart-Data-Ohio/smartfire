//! `app/controllers/switchers_controller.rb`: scoped quick-switcher JSON.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, require_current_user};
use crate::controllers::presenters::page::db_error;
use campfire_kit::{Ctx, Result, StatusCode, format};

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    c.respond_to(&[&format::JSON])?;
    let user = require_current_user(c)?.clone();
    let base_url = c.url_for("");
    let secrets = c.app().secrets.clone();
    let payload = c
        .app()
        .db
        .read(move |conn| super::presenters::switcher::load(conn, &secrets, &user, &base_url))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &payload)
}

#[cfg(test)]
mod tests {
    use crate::controllers::presenters::test_support::*;
    use axum::http::StatusCode;
    use campfire_db::{ChannelThread, Membership, NewChannelThread, Room, RoomType, User};

    #[tokio::test]
    async fn switcher_requires_sign_in_and_rejects_bot_credentials() {
        let app = TestApp::boot().await.expect("seed required");
        let reply = app.anonymous().get("/switcher.json").await;
        assert_eq!(reply.status, StatusCode::FOUND);
        let reply = app
            .anonymous()
            .get(&format!("/switcher.json?bot_key={BENDER_KEY}"))
            .await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn switcher_scopes_private_hidden_deleted_rooms_and_threads() {
        let app = TestApp::boot().await.expect("seed required");
        let hidden = app
            .db()
            .write(|tx| {
                let secret = Room::create_for(
                    tx,
                    RoomType::Closed,
                    Some("Outsider secret"),
                    JASON,
                    &[JASON],
                )?;
                ChannelThread::create(
                    tx,
                    NewChannelThread {
                        room_id: secret.id,
                        creator_id: JASON,
                        name: Some("Secret thread".into()),
                        ..Default::default()
                    },
                )?;
                let deleted = Room::create_for(
                    tx,
                    RoomType::Closed,
                    Some("Pending deletion"),
                    DAVID,
                    &[DAVID],
                )?;
                ChannelThread::create(
                    tx,
                    NewChannelThread {
                        room_id: deleted.id,
                        creator_id: DAVID,
                        name: Some("Deleted thread".into()),
                        ..Default::default()
                    },
                )?;
                deleted.begin_destroy(tx)?;
                Membership::find_by_room_and_user(tx.conn(), ALL_TALK, DAVID)?
                    .unwrap()
                    .update_involvement(tx, Some(campfire_db::Involvement::Invisible))?;
                Ok(secret.id)
            })
            .await
            .unwrap();
        let response = app.david().get("/switcher.json").await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        let value = response.json();
        let rooms = value["rooms"].as_array().unwrap();
        assert!(
            rooms.iter().all(|r| r["id"] != hidden
                && r["id"] != ALL_TALK
                && r["id"] != DIRECT_KEVIN_BENDER)
        );
        assert!(
            value["threads"]
                .as_array()
                .unwrap()
                .iter()
                .all(|t| t["name"] != "Secret thread" && t["name"] != "Deleted thread")
        );
        assert_eq!(
            app.david().get("/switcher").await.status,
            StatusCode::NOT_ACCEPTABLE
        );
    }

    #[tokio::test]
    async fn switcher_seed_payload_matches_pinned_rails() {
        let app = TestApp::boot().await.expect("seed required");
        let actual = app.david().get("/switcher.json").await;
        assert_eq!(actual.status, StatusCode::OK, "{}", actual.text());
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../../../../vectors/switcher.json")).unwrap();
        assert_eq!(actual.json(), expected["seed"]);
        assert_eq!(actual.text(), expected["seed_body"].as_str().unwrap());
        app.db()
            .write(|tx| {
                tx.conn().execute(
                    "UPDATE rooms SET name=? WHERE id=?",
                    ("Escaped <&>\u{2028}\u{2029}", ALL_TALK),
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let escaped = app.david().get("/switcher.json").await;
        assert_eq!(escaped.text(), expected["escaped_body"].as_str().unwrap());
    }

    #[tokio::test]
    async fn switcher_preserves_group_names_room_kinds_flags_and_thread_limit() {
        let app = TestApp::boot().await.expect("seed required");
        let (group, solo) = app
            .db()
            .write(|tx| {
                let group = Room::find_or_create_direct_for(tx, &[DAVID, JASON, KEVIN], DAVID)?;
                let mut named = group.clone();
                named.rename_direct(tx, "Weekend", DAVID)?;
                let solo = Room::find_or_create_direct_for(tx, &[DAVID], DAVID)?;
                let mut member =
                    Membership::find_by_room_and_user(tx.conn(), group.id, DAVID)?.unwrap();
                member.favorite(tx)?;
                member.update_involvement(tx, Some(campfire_db::Involvement::Muted))?;
                tx.conn().execute(
                    "UPDATE memberships SET unread_at=? WHERE id=?",
                    (tx.now(), member.id),
                )?;
                for (kind, name) in [
                    (RoomType::Voice, "Voice"),
                    (RoomType::Stage, "Stage"),
                    (RoomType::Board, "Board"),
                ] {
                    Room::create_for(tx, kind, Some(name), DAVID, &[DAVID])?;
                }
                for i in 0..20 {
                    ChannelThread::create(
                        tx,
                        NewChannelThread {
                            room_id: ALL_TALK,
                            creator_id: DAVID,
                            name: Some(format!("Thread {i}")),
                            last_activity_at: Some(
                                tx.now().since(jiff::SignedDuration::from_secs(i)),
                            ),
                            ..Default::default()
                        },
                    )?;
                }
                let inactive = User::create(
                    tx,
                    campfire_db::NewUser {
                        name: "Hidden Person".into(),
                        email_address: Some("hidden-person@example.test".into()),
                        ..Default::default()
                    },
                )?;
                tx.conn()
                    .execute("UPDATE users SET status=2 WHERE id=?", [inactive.id])?;
                Ok((group.id, solo.id))
            })
            .await
            .unwrap();
        let reply = app.david().get("/switcher.json").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let value = reply.json();
        let group = value["rooms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == group)
            .unwrap();
        // The switcher deliberately lists full peer names, even for a named group.
        assert_eq!(group["name"], "Jason and Kevin");
        assert_eq!(group["kind"], "group");
        assert_eq!(group["favorite"], true);
        assert_eq!(group["muted"], true);
        assert_eq!(group["unread"], true);
        let solo = value["rooms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == solo)
            .unwrap();
        assert_eq!(solo["name"], "David");
        assert_eq!(solo["kind"], "dm");
        for kind in ["voice", "stage", "board"] {
            assert!(
                value["rooms"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["kind"] == kind)
            );
        }
        let people = value["people"].as_array().unwrap();
        assert!(
            people
                .iter()
                .all(|p| p["id"] != DAVID && p["id"] != BENDER && p["name"] != "Hidden Person")
        );
        assert_eq!(
            people.iter().find(|p| p["id"] == JASON).unwrap()["dm_url"],
            format!("/rooms/{DIRECT_DAVID_JASON}")
        );
        assert_eq!(value["threads"].as_array().unwrap().len(), 15);
        assert_eq!(value["threads"][0]["name"], "Thread 19");
    }
}
