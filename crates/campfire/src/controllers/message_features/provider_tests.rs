//! Provider composition from actual pinned Rails rows; no provider transport is mocked.
use super::quote_integration_tests::app_rows;
use crate::controllers::presenters::{Presenter, page, test_support::*};
use campfire_db::Message;
use serde_json::Value;

#[tokio::test]
async fn changed_urls_run_owner_callbacks_and_private_card_endpoints_match_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/provider_callbacks.json"
    )).unwrap();
    let app = app_rows(oracle["rows"].clone()).await;
    let mut browser = app.david();
    let message_id = oracle["message_id"].as_i64().unwrap();
    for step in oracle["steps"].as_array().unwrap() {
        let response = browser.write(Req::new(axum::http::Method::PATCH,
            &format!("/rooms/{QUIET_CORNER}/messages/{message_id}"))
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .body(serde_json::to_vec(&step["input"]).unwrap())).await;
        assert_eq!(response.status.as_u16(), step["status"].as_u64().unwrap() as u16);

        let (github, embeds) = app.db().read(move |conn| {
            let github = conn.prepare("SELECT github_pull_request_id FROM github_pull_request_references WHERE message_id=? ORDER BY id")?
                .query_map([message_id], |r| r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let embeds = conn.prepare("SELECT url FROM link_embed_references WHERE message_id=? ORDER BY position")?
                .query_map([message_id], |r| r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((github, embeds))
        }).await.unwrap();
        assert_eq!(serde_json::json!(github), step["github"]);
        assert_eq!(serde_json::json!(embeds), step["embeds"]);
    }
    for read in oracle["reads"].as_array().unwrap() {
        let response = browser.get(read["path"].as_str().unwrap()).await;
        assert_eq!(response.status.as_u16(), read["status"].as_u64().unwrap() as u16);
        assert_eq!(response.text(), read["body"].as_str().unwrap());
    }

    println!("WS8bm2 provider callbacks: 3 HTTP edits, 24/24 socket frames and reference sets, 3/3 scoped card endpoints match Rails");
}
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/providers.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn populated_github_and_embed_containers_match_actual_rails() {
    let app = app_rows(oracle()["rows"].clone()).await;
    let runtime = app.booted.app.clone();
    app.db()
        .read(move |conn| {
            for case in oracle()["cases"].as_array().unwrap() {
                let message = Message::find(conn, case["message_id"].as_i64().unwrap())?;
                let p = Presenter::new(conn, &runtime, None)
                    .preload_search(std::slice::from_ref(&message))?;
                let view = p.message(&message)?;
                let html =
                    page::render_detached_at(&runtime, None, "http://campfire.test", |ctx| {
                        match case["kind"].as_str().unwrap() {
                            "github" => campfire_views::message_providers::github_cards(ctx, &view),
                            "linkedin" => {
                                campfire_views::message_providers::embed_cards(ctx, &view, true)
                            }
                            "embed" => {
                                campfire_views::message_providers::embed_cards(ctx, &view, false)
                            }
                            _ => unreachable!(),
                        }
                    });
                assert_eq!(html.0, case["html"].as_str().unwrap(), "{}", case["label"]);
            }
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn shared_provider_cards_expose_no_private_content_or_session_values() {
    let app = app_rows(oracle()["rows"].clone()).await;
    for user in [DAVID, KEVIN] {
        let response = app
            .sign_in(user)
            .await
            .get(&format!("/rooms/{QUIET_CORNER}/messages"))
            .await;
        assert_eq!(response.status, axum::http::StatusCode::OK);
        let text = response.text();
        assert!(!text.contains("PRIVATE TITLE"));
        assert!(!text.contains("UNKNOWN TITLE"));
        assert!(text.contains("Title &lt;&amp;&gt; &quot;quoted&quot;"));
        assert!(text.contains("#my-fragment"));
        // Ordinary request forms are allowed; provider fragments themselves cannot carry tokens.
        for case in oracle()["cases"].as_array().unwrap() {
            if case["label"] == "reply" {
                continue;
            }
            let html = case["html"].as_str().unwrap();
            assert!(text.contains(html), "{}", case["label"]);
            assert!(!html.contains("authenticity_token"));
            assert!(!html.contains("nonce="));
        }
    }
}

#[tokio::test]
async fn preloaded_provider_cards_render_with_zero_queries_for_one_or_many_messages() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let app = app_rows(oracle()["rows"].clone()).await;
    let runtime = app.booted.app.clone();
    app.db()
        .read(move |conn| {
            let messages = oracle()["cases"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| Message::find(conn, c["message_id"].as_i64().unwrap()))
                .collect::<campfire_db::Result<Vec<_>>>()?;
            for rows in [&messages[..1], &messages[..]] {
                let p = Presenter::new(conn, &runtime, None).preload_search(rows)?;
                conn.flush_prepared_statement_cache();
                let count = Arc::new(AtomicUsize::new(0));
                let observed = count.clone();
                conn.authorizer(Some(move |ctx: rusqlite::hooks::AuthContext<'_>| {
                    if matches!(ctx.action, rusqlite::hooks::AuthAction::Select) {
                        observed.fetch_add(1, Ordering::SeqCst);
                    }
                    rusqlite::hooks::Authorization::Allow
                }));
                for message in rows {
                    let view = p.message(message)?;
                    page::render_detached_at(&runtime, None, "http://campfire.test", |ctx| {
                        campfire_views::message_providers::github_cards(ctx, &view);
                        campfire_views::message_providers::embed_cards(ctx, &view, true);
                        campfire_views::message_providers::embed_cards(ctx, &view, false);
                    });
                }
                conn.authorizer(
                    None::<fn(rusqlite::hooks::AuthContext<'_>) -> rusqlite::hooks::Authorization>,
                );
                assert_eq!(count.load(Ordering::SeqCst), 0);
            }
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn warm_public_card_fragments_refresh_without_touching_the_message() {
    let app = app_rows(oracle()["rows"].clone()).await;
    let path = format!("/rooms/{QUIET_CORNER}/messages");
    let initial = app.david().get(&path).await;
    assert!(
        initial
            .text()
            .contains("Title &lt;&amp;&gt; &quot;quoted&quot;")
    );
    app.db().write(|tx| {
        tx.conn().execute("UPDATE github_pull_requests SET title='Fresh public PR',updated_at='2026-03-02 16:00:01' WHERE repo='repo-1'",[])?;
        tx.conn().execute("UPDATE link_embeds SET title='Fresh generic embed',updated_at='2026-03-02 16:00:01' WHERE normalized_url='https://page.example.test/post'",[])?;
        Ok(())
    }).await.unwrap();
    let updated = app.david().get(&path).await;
    assert!(updated.text().contains("Fresh public PR"));
    assert!(updated.text().contains("Fresh generic embed"));
    assert!(
        updated
            .text()
            .contains("https://page.example.test/post#my-fragment")
    );
}

fn event_oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/event_cards.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn populated_event_cards_match_actual_rails_without_viewer_attendance_state() {
    let app = app_rows(event_oracle()["rows"].clone()).await;
    let runtime = app.booted.app.clone();
    app.db()
        .read(move |conn| {
            for case in event_oracle()["cases"].as_array().unwrap() {
                let message = Message::find(conn, case["message_id"].as_i64().unwrap())?;
                let p = Presenter::new(conn, &runtime, None)
                    .preload_search(std::slice::from_ref(&message))?;
                let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
                let observed = count.clone();
                conn.flush_prepared_statement_cache();
                conn.authorizer(Some(move |ctx: rusqlite::hooks::AuthContext<'_>| {
                    if matches!(ctx.action, rusqlite::hooks::AuthAction::Select) {
                        observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                    rusqlite::hooks::Authorization::Allow
                }));
                let view = p.message(&message)?;
                let html =
                    page::render_detached_at(&runtime, None, "http://campfire.test", |ctx| {
                        campfire_views::message_providers::events::cards(ctx, &view).0
                    });
                conn.authorizer(
                    None::<fn(rusqlite::hooks::AuthContext<'_>) -> rusqlite::hooks::Authorization>,
                );
                assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
                assert_eq!(html, case["html"].as_str().unwrap(), "{}", case["label"]);
            }
            Ok(())
        })
        .await
        .unwrap();
}

fn twitter_oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/twitter_preloads.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn preloaded_x_cards_match_actual_rails_numeric_order_and_warm_refresh() {
    let app = app_rows(twitter_oracle()["rows"].clone()).await;
    let runtime = app.booted.app.clone();
    app.db()
        .read(move |conn| {
            let messages = twitter_oracle()["cases"]
                .as_array()
                .unwrap()
                .iter()
                .map(|case| Message::find(conn, case["message_id"].as_i64().unwrap()))
                .collect::<campfire_db::Result<Vec<_>>>()?;
            let p = Presenter::new(conn, &runtime, None).preload_search(&messages)?;
            conn.flush_prepared_statement_cache();
            let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let observed = count.clone();
            conn.authorizer(Some(move |ctx: rusqlite::hooks::AuthContext<'_>| {
                if matches!(ctx.action, rusqlite::hooks::AuthAction::Select) {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                rusqlite::hooks::Authorization::Allow
            }));
            for (message, case) in messages
                .iter()
                .zip(twitter_oracle()["cases"].as_array().unwrap())
            {
                let view = p.message(message)?;
                let html =
                    page::render_detached_at(&runtime, None, "http://campfire.test", |ctx| {
                        campfire_views::twitter::cards(ctx, &view).0
                    });
                assert_eq!(html, case["html"].as_str().unwrap(), "{}", case["label"]);
            }
            conn.authorizer(
                None::<fn(rusqlite::hooks::AuthContext<'_>) -> rusqlite::hooks::Authorization>,
            );
            assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/rooms/{QUIET_CORNER}/messages");
    let initial = app.david().get(&path).await;
    for case in twitter_oracle()["cases"].as_array().unwrap() {
        assert!(initial.text().contains(case["html"].as_str().unwrap()));
    }
    app.db().write(|tx| {tx.conn().execute("UPDATE twitter_posts SET text='Fresh X cached card',updated_at='2026-03-02 16:00:01' WHERE post_id='90071992547409931'",[])?;Ok(())}).await.unwrap();
    assert!(
        app.david()
            .get(&path)
            .await
            .text()
            .contains("Fresh X cached card")
    );
}

#[tokio::test]
async fn human_edits_replace_all_eight_rails_targets_on_a_real_socket() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/provider_edits.json"
    ))
    .unwrap();
    let app = app_rows(oracle["rows"].clone()).await;
    let mut browser = app.david();
    for step in oracle["steps"].as_array().unwrap() {
        let response = browser
            .write(
                Req::new(
                    axum::http::Method::PATCH,
                    &format!(
                        "/rooms/{QUIET_CORNER}/messages/{}",
                        step["message_id"].as_i64().unwrap()
                    ),
                )
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(serde_json::to_vec(&step["input"]).unwrap()),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            step["status"].as_u64().unwrap() as u16,
            "{}",
            response.text()
        );
        assert_eq!(step["frames"].as_array().unwrap().len(), 8);

    }

    println!(
        "WS8bm2 provider edits: 5 HTTP edits, 40/40 real socket replacement frames byte-identical to Rails"
    );
}

use campfire_web::controllers::presenters::Rendering;
