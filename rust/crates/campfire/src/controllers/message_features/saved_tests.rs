//! Request ports of `test/controllers/saved_items_controller_test.rb`.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Message, NewMessage, SavedItem};
use serde_json::{Value, json};
use std::sync::Arc;

async fn fixture() -> (TestApp, i64, Arc<campfire_kit::clock::FrozenClock>) {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let app = TestApp::boot_with_test_clock(clock.clone())
        .await
        .expect("WS8bm2 requires default seed").without_job_runner().await;
    let message = app
        .db()
        .write(|tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Saved <message> & example".into()),
                    client_message_id: Some("saved-example".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    (app, message.id, clock)
}
fn req(method: Method, path: &str, value: Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&value).unwrap())
}
async fn save(browser: &mut Browser<'_>, message: i64, remind: Option<&str>) -> Reply {
    browser
        .write(req(
            Method::POST,
            "/saved",
            json!({"message_id": message, "saved_item": {"remind_at": remind}}),
        ))
        .await
}
async fn item(app: &TestApp, message: i64) -> SavedItem {
    app.db()
        .read(move |conn| Ok(SavedItem::find_by_user_and_message(conn, DAVID, message)?.unwrap()))
        .await
        .unwrap()
}

#[tokio::test]
async fn index_lists_saved_messages_with_status_filters() {
    let (app, message, _) = fixture().await;
    let (open, done) = app
        .db()
        .write(move |tx| {
            let open = SavedItem::save_for(tx, DAVID, message, None)?;
            let other = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Done body".into()),
                    ..Default::default()
                },
            )?;
            let done = SavedItem::create(
                tx,
                campfire_db::NewSavedItem {
                    user_id: DAVID,
                    message_id: other.id,
                    status: Some("done".into()),
                    ..Default::default()
                },
            )?;
            SavedItem::save_for(tx, JASON, message, None)?;
            Ok((open.id, done.id))
        })
        .await
        .unwrap();
    let mut browser = app.david();
    for (filter, has_open, has_done) in [
        ("all", true, true),
        ("in_progress", true, false),
        ("done", false, true),
        ("bogus", true, true),
    ] {
        let response = browser.get(&format!("/saved?status={filter}")).await;
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(
            response
                .text()
                .contains(&format!("id=\"saved_item_{open}\"")),
            has_open
        );
        assert_eq!(
            response
                .text()
                .contains(&format!("id=\"saved_item_{done}\"")),
            has_done
        );
        assert_eq!(response.header("cache-control"), Some("no-store"));
        assert_eq!(response.header("pragma"), Some("no-cache"));
    }
}
#[tokio::test]
async fn index_hides_items_whose_room_access_was_lost() {
    let (app, message, _) = fixture().await;
    let mut browser = app.david();
    assert_eq!(
        save(&mut browser, message, None).await.status,
        StatusCode::CREATED
    );
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                (DAVID, ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let response = browser.get("/saved").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(!response.text().contains(&format!(
        "id=\"saved_item_{}\"",
        item(&app, message).await.id
    )));
    assert_eq!(item(&app, message).await.status, "in_progress");
}
#[tokio::test]
async fn create_saves_with_a_reminder() {
    let (app, message, _) = fixture().await;
    let response = save(&mut app.david(), message, Some("2026-03-02T17:00:00.123Z")).await;
    assert_eq!(response.status, StatusCode::CREATED);
    let body = response.json();
    assert_eq!(body["remind_at"], "2026-03-02T17:00:00.123Z");
    assert_eq!(
        body["url"],
        format!("http://campfire.test/saved/{}.json", body["id"])
    );
    assert_eq!(body["status"], "in_progress");
    assert_eq!(
        item(&app, message).await.remind_at.unwrap().to_db(),
        "2026-03-02 17:00:00.123000"
    );
}
#[tokio::test]
async fn create_without_a_reminder_leaves_remind_at_blank() {
    let (app, message, _) = fixture().await;
    let response = save(&mut app.david(), message, None).await;
    assert_eq!(response.status, StatusCode::CREATED);
    assert!(response.json()["remind_at"].is_null());
}
#[tokio::test]
async fn create_again_updates_the_reminder_instead_of_duplicating() {
    let (app, message, _) = fixture().await;
    let mut browser = app.david();
    let first = save(&mut browser, message, None).await;
    assert_eq!(first.status, StatusCode::CREATED);
    let next = save(&mut browser, message, Some("2026-03-02T18:00:00Z")).await;
    assert_eq!(next.status, StatusCode::CREATED);
    assert_eq!(first.json()["id"], next.json()["id"]);
    assert_eq!(next.json()["remind_at"], "2026-03-02T18:00:00.000Z");
}
#[tokio::test]
async fn create_again_after_a_fired_reminder_rearms_it() {
    let (app, message, clock) = fixture().await;
    let mut browser = app.david();
    let first = save(&mut browser, message, Some("2026-03-02T16:01:00Z")).await;
    assert_eq!(first.status, StatusCode::CREATED);
    let id = first.json()["id"].as_i64().unwrap();
    clock.set("2026-03-02T16:02:00Z".parse().unwrap());
    app.db()
        .write(move |tx| SavedItem::dispatch_reminder(tx, id, tx.now()))
        .await
        .unwrap();
    assert!(item(&app, message).await.reminded_at.is_some());
    assert_eq!(
        save(&mut browser, message, Some("2026-03-02T18:00:00Z"))
            .await
            .status,
        StatusCode::CREATED
    );
    assert!(item(&app, message).await.reminded_at.is_none());
}
#[tokio::test]
async fn create_rejects_past_and_unparseable_reminders() {
    let (app, message, _) = fixture().await;
    let mut browser = app.david();
    for raw in ["2026-03-02T15:59:00Z", "not a time", "2026-99-99T00:00"] {
        assert_eq!(
            save(&mut browser, message, Some(raw)).await.status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert!(
        app.db()
            .read(move |conn| SavedItem::find_by_user_and_message(conn, DAVID, message))
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn create_is_404_for_a_message_the_user_cannot_see() {
    let (app, message, _) = fixture().await;
    assert_eq!(
        save(&mut app.sign_in(KEVIN).await, message, None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn update_marks_done_and_reopens() {
    let (app, message, _) = fixture().await;
    let mut browser = app.david();
    let saved = save(&mut browser, message, None).await;
    assert_eq!(saved.status, StatusCode::CREATED);
    let id = saved.json()["id"].as_i64().unwrap();
    for status in ["done", "in_progress"] {
        let response = browser
            .write(req(
                Method::PATCH,
                &format!("/saved/{id}"),
                json!({"saved_item":{"status":status}}),
            ))
            .await;
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.json()["status"], status);
    }
}
#[tokio::test]
async fn update_rejects_an_invalid_status() {
    let (app, message, _) = fixture().await;
    let saved = app
        .db()
        .write(move |tx| SavedItem::save_for(tx, DAVID, message, None))
        .await
        .unwrap();
    let response = app
        .david()
        .write(req(
            Method::PATCH,
            &format!("/saved/{}", saved.id),
            json!({"saved_item":{"status":"archived"}}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response.json(),
        json!({"error":"Status must be in progress or done"})
    );
    assert!(item(&app, message).await.in_progress());
}
#[tokio::test]
async fn update_and_destroy_are_404_for_hidden_or_foreign_items() {
    let (app, message, _) = fixture().await;
    let (hidden, foreign) = app
        .db()
        .write(move |tx| {
            let hidden = SavedItem::save_for(tx, DAVID, message, None)?;
            let foreign = SavedItem::save_for(tx, JASON, message, None)?;
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                (DAVID, ALL_TALK),
            )?;
            Ok((hidden.id, foreign.id))
        })
        .await
        .unwrap();
    let mut browser = app.david();
    for id in [hidden, foreign] {
        for method in [Method::PATCH, Method::DELETE] {
            assert_eq!(
                browser
                    .write(req(
                        method,
                        &format!("/saved/{id}"),
                        json!({"saved_item":{"status":"done"}})
                    ))
                    .await
                    .status,
                StatusCode::NOT_FOUND
            );
        }
    }
    assert!(item(&app, message).await.in_progress());
}
#[tokio::test]
async fn destroy_removes_the_item() {
    let (app, message, _) = fixture().await;
    let saved = app
        .db()
        .write(move |tx| SavedItem::save_for(tx, DAVID, message, None))
        .await
        .unwrap();
    let response = app
        .david()
        .write(req(
            Method::DELETE,
            &format!("/saved/{}", saved.id),
            json!({}),
        ))
        .await;
    assert_eq!(response.status, StatusCode::NO_CONTENT);
    assert!(response.body.is_empty());
    assert!(
        app.db()
            .read(move |conn| SavedItem::find_by_id(conn, saved.id))
            .await
            .unwrap()
            .is_none()
    );
}
fn oracle() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/messaging/saved.json")).unwrap()
}
#[tokio::test]
async fn saved_http_matches_rails_exact_json_and_no_store_headers() {
    let (app, message, _) = fixture().await;
    assert_eq!(message, oracle()["message_id"]);
    let mut browser = app.david();
    for step in oracle()["steps"].as_array().unwrap() {
        let response = browser
            .write(req(
                step["method"]
                    .as_str()
                    .unwrap()
                    .to_uppercase()
                    .parse()
                    .unwrap(),
                step["path"].as_str().unwrap(),
                step["input"].clone(),
            ))
            .await;
        assert_eq!(
            response.status.as_u16(),
            step["status"].as_u64().unwrap() as u16
        );
        assert_eq!(response.text(), step["body"].as_str().unwrap());
        assert_eq!(
            response.header("cache-control"),
            step["cache_control"].as_str()
        );
        assert_eq!(response.header("pragma"), step["pragma"].as_str());
    }
}
pub(super) fn zoned<'a>(
    ctx: &campfire_views::ViewContext<'a>,
    zone: &str,
) -> campfire_views::ViewContext<'a> {
    campfire_views::ViewContext {
        current_user: None,
        account: ctx.account.clone(),
        flash_notice: None,
        flash_alert: None,
        platform: ctx.platform.clone(),
        vapid_public_key: ctx.vapid_public_key.clone(),
        asset_path: ctx.asset_path,
        importmap_tags: ctx.importmap_tags,
        stylesheet_tags: ctx.stylesheet_tags,
        custom_styles: None,
        cable_url: ctx.cable_url.clone(),
        base_url: ctx.base_url.clone(),
        request_url: ctx.request_url.clone(),
        referrer: None,
        last_room_visited_id: None,
        app_version: ctx.app_version.clone(),
        signed_stream_name: ctx.signed_stream_name,
        time_zone: campfire_views::time::Zone::for_user(Some(zone)),
        chrome: ctx.chrome.clone(),
    }
}
#[tokio::test]
async fn saved_partials_match_rails_for_reminders_statuses_zones_and_empty_page() {
    use askama::Template;
    let (app, message, _) = fixture().await;
    let item = app
        .db()
        .write(move |tx| SavedItem::save_for(tx, DAVID, message, None))
        .await
        .unwrap();
    for case in oracle()["html"].as_array().unwrap() {
        let case = case.clone();
        let expected = case["html"].as_str().unwrap().to_owned();
        let runtime = app.booted.app.clone();
        let item = item.clone();
        let actual = app
            .db()
            .read(move |conn| {
                let state = &case["state"];
                let mut item = item;
                item.status = state["status"].as_str().unwrap().into();
                item.remind_at = state["remind_at"]
                    .as_str()
                    .map(|raw| campfire_db::Timestamp::from_jiff(raw.parse().unwrap()));
                item.reminded_at = state["reminded_at"]
                    .as_str()
                    .map(|raw| campfire_db::Timestamp::from_jiff(raw.parse().unwrap()));
                let mut presenter = crate::controllers::presenters::Presenter::new(conn, &runtime, None);
                presenter.render_zone = campfire_views::time::Zone::lookup(case["zone"].as_str().unwrap()).unwrap();
                let view = crate::controllers::saved_items::view(
                    &presenter,
                    conn,
                    &campfire_db::User::find(conn, DAVID)?,
                    &item,
                )?;
                let account = campfire_db::Account::first(conn)?;
                Ok(crate::controllers::presenters::page::render_detached_at(
                    &runtime,
                    account.as_ref(),
                    "http://campfire.test",
                    |ctx| {
                        let ctx = zoned(ctx, case["zone"].as_str().unwrap());
                        campfire_views::saved_items::ItemPartial {
                            ctx: &ctx,
                            item: &view,
                            status_filter: "all",
                        }
                        .render()
                        .unwrap()
                    },
                ))
            })
            .await
            .unwrap();
        assert_eq!(actual, expected);
    }
    let actual = crate::controllers::presenters::page::render_detached_at(
        &app.booted.app,
        None,
        "http://campfire.test",
        |ctx| {
            campfire_views::saved_items::Index {
                ctx,
                items: &[],
                status_filter: "all",
            }
            .as_content()
            .render()
            .unwrap()
        },
    );
    assert_eq!(actual, oracle()["empty"].as_str().unwrap());
}
#[tokio::test]
async fn saved_mutations_require_csrf_and_turbo_redirects_keep_the_filter() {
    let (app, message, _) = fixture().await;
    let mut browser = app.david();
    assert_eq!(
        browser
            .send(req(Method::POST, "/saved", json!({"message_id":message})))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let id = save(&mut browser, message, None).await.json()["id"]
        .as_i64()
        .unwrap();
    for method in [Method::PATCH, Method::DELETE] {
        assert_eq!(
            browser
                .send(req(
                    method,
                    &format!("/saved/{id}"),
                    json!({"saved_item":{"status":"done"}})
                ))
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let response = browser
        .write(
            Req::new(Method::PATCH, &format!("/saved/{id}?status=done"))
                .header("accept", "text/vnd.turbo-stream.html")
                .form(&[("saved_item[status]", "done")]),
        )
        .await;
    assert_eq!(response.status, StatusCode::SEE_OTHER);
    assert_eq!(
        response.location(),
        Some("http://campfire.test/saved?status=done")
    );
}
#[tokio::test]
async fn reminder_dispatch_rolls_back_failed_jobs_and_refires_the_same_inbox_item() {
    let (app, message, clock) = fixture().await;
    let mut browser = app.david();
    let response = save(&mut browser, message, Some("2026-03-02T16:01:00Z")).await;
    assert_eq!(response.status, StatusCode::CREATED);
    let id = response.json()["id"].as_i64().unwrap();
    app.db().write(|tx| { tx.conn().execute_batch("CREATE TRIGGER reject_saved_push BEFORE INSERT ON background_jobs WHEN NEW.job_class='SavedItem::ReminderPushJob' BEGIN SELECT RAISE(ABORT,'reject reminder job'); END;")?; Ok(()) }).await.unwrap();
    clock.set("2026-03-02T16:02:00Z".parse().unwrap());
    crate::jobs::periodic::saved_item_reminders(app.db())
        .await
        .unwrap();
    assert!(item(&app, message).await.reminded_at.is_none());
    let count: i64 = app
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM activity_items WHERE source_type='SavedItem' AND source_id=?",
                [id],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 0);
    app.db()
        .write(|tx| {
            tx.conn().execute_batch("DROP TRIGGER reject_saved_push")?;
            Ok(())
        })
        .await
        .unwrap();
    crate::jobs::periodic::saved_item_reminders(app.db())
        .await
        .unwrap();
    crate::jobs::periodic::saved_item_reminders(app.db())
        .await
        .unwrap();
    assert!(item(&app, message).await.reminded_at.is_some());
    let first: i64 = app
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT id FROM activity_items WHERE source_type='SavedItem' AND source_id=?",
                [id],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(
        save(&mut browser, message, Some("2026-03-02T16:03:00Z"))
            .await
            .status,
        StatusCode::CREATED
    );
    clock.set("2026-03-02T16:04:00Z".parse().unwrap());
    crate::jobs::periodic::saved_item_reminders(app.db())
        .await
        .unwrap();
    let next: i64 = app
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT id FROM activity_items WHERE source_type='SavedItem' AND source_id=?",
                [id],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(first, next);
}

#[tokio::test]
async fn review_regression_status_patch_preserves_dispatch_claim() {
    let (app, message, clock) = fixture().await;
    let mut browser = app.david();
    let response = save(&mut browser, message, Some("2026-03-02T16:01:00Z")).await;
    assert_eq!(response.status, StatusCode::CREATED);
    let id = response.json()["id"].as_i64().unwrap();
    let token = browser.authenticity_token().await;
    clock.set("2026-03-02T16:02:00Z".parse().unwrap());
    let db = app.db().clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let blocker = tokio::spawn(async move {
        db.write(move |tx| {
            assert!(SavedItem::dispatch_reminder(tx, id, tx.now())?);
            started.send(()).unwrap();
            wait.recv()
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap()
    });
    ready.await.unwrap();
    let path = format!("/saved/{id}");
    let request = browser.send(
        req(
            Method::PATCH,
            &path,
            json!({"saved_item":{"status":"done"}}),
        )
        .header(campfire_kit::csrf::HEADER, &token),
    );
    let release = async {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while app.db().queued_writes() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("status write queued after its stale read");
        release.send(()).unwrap();
    };
    let (response, ()) = tokio::join!(request, release);
    blocker.await.unwrap();
    assert_eq!(response.status, StatusCode::OK);
    let row = item(&app, message).await;
    println!(
        "REVIEW_RACE after HTTP patch: status={} reminded_at={:?}",
        row.status, row.reminded_at
    );
    let refired = app
        .db()
        .write(move |tx| SavedItem::dispatch_reminder(tx, id, tx.now()))
        .await
        .unwrap();
    let jobs: i64 = app
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM background_jobs WHERE job_class='SavedItem::ReminderPushJob'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    println!(
        "REVIEW_RACE repeated_dispatch={} durable_push_jobs={}",
        refired, jobs
    );
    assert!(
        !refired,
        "status-only PATCH erased the claim and dispatched twice"
    );
    assert_eq!(
        row.reminded_at,
        Some(campfire_db::Timestamp::from_jiff(
            race_oracle()["claimed"].as_str().unwrap().parse().unwrap()
        ))
    );
    assert_eq!(jobs, 1);
    assert_eq!(response.json()["reminded_at"], race_oracle()["after_status_response"]["reminded_at"]);
    assert_eq!(response.json()["remind_at"], race_oracle()["after_status_response"]["remind_at"]);
}

#[tokio::test]
async fn review_regression_status_patch_preserves_concurrent_reschedule() {
    let (app, message, clock) = fixture().await;
    let mut browser = app.david();
    let response = save(&mut browser, message, Some("2026-03-02T16:01:00Z")).await;
    assert_eq!(response.status, StatusCode::CREATED);
    let id = response.json()["id"].as_i64().unwrap();
    let token = browser.authenticity_token().await;
    clock.set("2026-03-02T16:02:00Z".parse().unwrap());
    let db = app.db().clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let blocker = tokio::spawn(async move {
        db.write(move |tx| {
            let mut fresh = SavedItem::find(tx.conn(), id)?;
            fresh.update(
                tx,
                campfire_db::SavedItemChanges {
                    remind_at: Some(Some(campfire_db::Timestamp::from_jiff(
                        "2026-03-02T18:00:00Z".parse().unwrap(),
                    ))),
                    ..Default::default()
                },
            )?;
            started.send(()).unwrap();
            wait.recv()
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap()
    });
    ready.await.unwrap();
    let path = format!("/saved/{id}");
    let request = browser.send(
        req(
            Method::PATCH,
            &path,
            json!({"saved_item":{"status":"done"}}),
        )
        .header(campfire_kit::csrf::HEADER, &token),
    );
    let release = async {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while app.db().queued_writes() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("status write queued after its stale read");
        release.send(()).unwrap();
    };
    let (response, ()) = tokio::join!(request, release);
    blocker.await.unwrap();
    assert_eq!(response.status, StatusCode::OK);
    let row = item(&app, message).await;
    println!(
        "REVIEW_RACE after HTTP patch: status={} reminded_at={:?}",
        row.status, row.reminded_at
    );
    assert_eq!(
        row.remind_at,
        Some(campfire_db::Timestamp::from_jiff(
            "2026-03-02T18:00:00Z".parse().unwrap()
        )),
        "status PATCH overwrote the concurrent reminder schedule"
    );
    assert_eq!(response.json()["remind_at"], race_oracle()["after_reschedule_response"]["remind_at"]);
    assert_eq!(response.json()["reminded_at"], race_oracle()["after_reschedule_response"]["reminded_at"]);
}

fn race_oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/review_saved_race.json"
    ))
    .unwrap()
}
