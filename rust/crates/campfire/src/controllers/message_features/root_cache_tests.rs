//! Real composite-key transitions and root cache invalidation, using pinned Rails rows.
use super::quote_integration_tests::app_rows;
use crate::controllers::presenters::{Presenter, page, test_support::*};
use axum::http::{Method, StatusCode};
use campfire_db::Message;
use serde_json::Value;
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/root_cache.json"
    ))
    .unwrap()
}
fn id(key: &str) -> i64 {
    oracle()[key].as_i64().unwrap()
}
async fn app() -> TestApp {
    app_rows(oracle()["rows"].clone()).await
}

#[tokio::test]
async fn composite_keys_and_private_provider_frames_match_actual_rails_transitions() {
    let app = app().await;
    for i in 0..6 {
        app.db()
            .write(move |tx| {
                match i {
                    1 => {
                        tx.conn().execute(
                            "UPDATE messages SET edited_at='2026-03-02 16:00:01' WHERE id=?",
                            [id("source_id")],
                        )?;
                    }
                    2 => {
                        tx.conn()
                            .execute("UPDATE users SET name='David renamed' WHERE id=?", [DAVID])?;
                    }
                    3 => {
                        tx.conn().execute(
                            "UPDATE rooms SET name='Quiet renamed' WHERE id=?",
                            [QUIET_CORNER],
                        )?;
                    }
                    4 => {
                        tx.conn().execute(
                            "UPDATE polls SET updated_at='2026-03-02 16:00:02' WHERE id=?",
                            [id("poll_id")],
                        )?;
                    }
                    5 => {
                        tx.conn().execute(
                            "DELETE FROM message_pins WHERE message_id=?",
                            [id("message_id")],
                        )?;
                    }
                    _ => {}
                };
                Ok(())
            })
            .await
            .unwrap();
        let state = app.booted.app.clone();
        app.db()
            .read(move |conn| {
                let message = Message::find(conn, id("message_id"))?;
                let p = Presenter::new(conn, &state, None)
                    .preload_search(std::slice::from_ref(&message))?;
                let view = p.message(&message)?;
                assert_eq!(
                    view.components.cache_key.as_deref(),
                    oracle()["cases"][i]["expanded"].as_str(),
                    "{}",
                    oracle()["cases"][i]["label"]
                );
                for zone in ["UTC", "Hawaii"] {
                    let mut p = Presenter::new(conn, &state, None);
                    p.cache_time_zone = campfire_views::time::Zone::lookup(zone).unwrap();
                    let p = p.preload_search(std::slice::from_ref(&message))?;
                    assert_eq!(
                        p.message_cache_key(&message)?,
                        oracle()["cases"][i]["expanded_zones"][zone]
                            .as_str()
                            .unwrap()
                    );
                }
                for (prefix, class, values, kind) in [
                    (
                        "github_pr_cards",
                        "github-pr-cards",
                        &view.components.github_cards,
                        "github",
                    ),
                    (
                        "fizzy_cards",
                        "fizzy-cards",
                        &view.components.fizzy_cards,
                        "fizzy",
                    ),
                ] {
                    assert_eq!(
                        campfire_views::messages::cards(&view, prefix, class, 0, values).0,
                        oracle()["cases"][i][kind].as_str().unwrap()
                    );
                }
                Ok(())
            })
            .await
            .unwrap();
    }
}
#[test]
fn direct_room_nil_names_digest_matches_ruby_array_inspect() {
    let names = vec![
        ("David".to_owned(), None),
        ("Joe".to_owned(), Some("Some room".to_owned())),
    ];
    assert_eq!(
        campfire_views::fragment_cache::keys::quote_nullable_names_inspect(&names),
        oracle()["nil_names_inspect"].as_str().unwrap()
    );
    assert_eq!(
        campfire_views::fragment_cache::keys::quote_nullable_names_digest(&names),
        oracle()["nil_names_digest"].as_str().unwrap()
    );
}
#[tokio::test]
async fn warm_root_fragments_refresh_after_legacy_source_edit_and_name_changes() {
    let app = app().await;
    let path = format!("/rooms/{QUIET_CORNER}/messages");
    let first = app.david().get(&path).await;
    assert_eq!(first.status, StatusCode::OK);
    assert!(first.text().contains("cached quote source"));
    app.db().write(|tx| {
        tx.conn().execute("UPDATE action_text_rich_texts SET body='<p>fresh quoted words</p>' WHERE record_type='Message' AND record_id=?",[id("source_id")])?;
        tx.conn().execute("UPDATE messages SET edited_at='2026-03-02 16:00:01' WHERE id=?",[id("source_id")])?;
        tx.conn().execute("UPDATE users SET name='David renamed' WHERE id=?",[DAVID])?;
        tx.conn().execute("UPDATE rooms SET name='Quiet renamed' WHERE id=?",[QUIET_CORNER])?;
        Ok(())
    }).await.unwrap();
    let second = app.david().get(&path).await;
    assert_eq!(second.status, StatusCode::OK);
    assert!(
        second
            .text()
            .contains("message-quote__excerpt\">fresh quoted words"),
        "{}",
        second.text()
    );
    assert!(
        second
            .text()
            .contains("message-quote__author\">David renamed")
            && second.text().contains("in Quiet renamed")
    );
}
#[tokio::test]
async fn warm_root_fragments_refresh_poll_only_and_unpin_changes() {
    let app = app().await;
    let path = format!("/rooms/{QUIET_CORNER}/messages");
    let first = app.david().get(&path).await;
    assert_eq!(first.status, StatusCode::OK);
    assert!(
        first
            .text()
            .contains("id=\"pin_badge_message_cache-root\" class=\"message__pin-badge\">")
    );
    app.db().write(|tx| {
        tx.conn().execute("UPDATE polls SET closed_at='2026-03-02 16:00:02', updated_at='2026-03-02 16:00:02' WHERE id=?",[id("poll_id")])?;
        tx.conn().execute("DELETE FROM message_pins WHERE message_id=?",[id("message_id")])?;
        Ok(())
    }).await.unwrap();
    let second = app.david().get(&path).await;
    assert_eq!(second.status, StatusCode::OK);
    assert!(
        second
            .text()
            .contains("id=\"pin_badge_message_cache-root\" class=\"message__pin-badge\" hidden>")
    );
    assert!(second.text().contains("· Closed"), "{}", second.text());
}
#[tokio::test]
async fn message_page_conditional_get_tracks_related_stamps_and_unpin_without_last_modified() {
    let app = app().await;
    let path = format!("/rooms/{QUIET_CORNER}/messages");
    let first = app.david().get(&path).await;
    assert_eq!(first.status, StatusCode::OK);
    assert!(first.headers.get("last-modified").is_none());
    let etag = first
        .headers
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let same = app
        .david()
        .send(Req::new(Method::GET, &path).header("if-none-match", &etag))
        .await;
    assert_eq!(same.status, StatusCode::NOT_MODIFIED);
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM message_pins WHERE message_id=?",
                [id("message_id")],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let unpinned = app
        .david()
        .send(Req::new(Method::GET, &path).header("if-none-match", &etag))
        .await;
    assert_eq!(unpinned.status, StatusCode::OK);
    let etag = unpinned
        .headers
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE polls SET updated_at='2026-03-02 16:00:00.000001' WHERE id=?",
                [id("poll_id")],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        app.david()
            .send(Req::new(Method::GET, &path).header("if-none-match", &etag))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        app.david()
            .send(
                Req::new(Method::GET, &path)
                    .header("if-modified-since", "Mon, 02 Mar 2026 16:00:05 GMT")
            )
            .await
            .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn private_provider_frames_render_without_provider_secrets() {
    let app = app().await;
    let state = app.booted.app.clone();
    app.db()
        .read(move |conn| {
            let message = Message::find(conn, id("message_id"))?;
            let p = Presenter::new(conn, &state, None)
                .preload_search(std::slice::from_ref(&message))?;
            let view = p.message(&message)?;
            let html = page::render_detached_at(&state, None, "http://campfire.test", |ctx| {
                campfire_views::messages::message(ctx, &view)
            });
            assert!(html.contains("github-pr-card-frame") && html.contains("fizzy-card-frame"));
            assert!(!html.contains("cache-owner") && !html.contains("cache-account"));
            Ok(())
        })
        .await
        .unwrap();
}
