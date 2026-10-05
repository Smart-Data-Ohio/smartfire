//! A tools-only clock/fixture bridge. Production has no control file or route.
use crate::{app::{App, boot_with_services}, config::Config};
use campfire_db::{Message, NewMessage, Room, Timestamp};
use campfire_kit::{Clock, FrozenClock};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::{Arc, RwLock}, time::Duration};
use tower::ServiceExt;

const HUDDLE_ENVIRONMENT: [&str; 5] = [
    "LIVEKIT_URL", "LIVEKIT_INTERNAL_URL", "LIVEKIT_API_KEY",
    "LIVEKIT_API_SECRET", "LIVEKIT_GATEWAY_SECRET",
];

async fn broadcast(app: &App, room: &Room, message: &Message) {
    use crate::controllers::presenters::{Presenter, page::{self, Rendered}};
    let (app, room, message) = (app.clone(), room.clone(), message.clone());
    app.db.clone().read(move |conn| {
        let presenter = Presenter::new(conn, &app, None);
        let view = presenter.message(&message)?;
        let account = campfire_db::Account::first(conn)?;
        let html = page::render_detached(&app, account.as_ref(), |ctx| campfire_views::messages::message(ctx, &view));
        let partials = Rendered { message: Some(html), ..Rendered::default() };
        app.broadcasts.message_create(conn, &room, &message, &partials, &*app.db.env().rich_text)?;
        Ok(())
    }).await.unwrap();
}

async fn create_message(app: &App, v: &Value) -> Value {
    let room_id = v["room"].as_i64().unwrap();
    let creator_id = v["creator"].as_i64().unwrap();
    let body = v["body"].as_str().unwrap().to_owned();
    let key = v["key"].as_str().unwrap().to_owned();
    let message = app.db.write(move |tx| Message::create(tx, NewMessage {room_id,creator_id,body:Some(body),client_message_id:Some(key),..Default::default()})).await.unwrap();
    let room = app.db.read(move |conn| Room::find(conn, room_id)).await.unwrap();
    broadcast(app, &room, &message).await;
    json!({"id":message.id})
}

#[tokio::test]
#[ignore = "utility: external injected-clock browser host; invoked by run_ledger_browser_assertions.py"]
async fn original_ledger_injected_clock_host() {
    assert_eq!(std::env::var("WS11UI_LEDGER_HOST").as_deref(), Ok("1"));
    let mut config = Config::from_env().unwrap();
    assert_eq!(config.environment, "test", "original system-test renderer environment");
    // Original config/environments/test.rb: perform_caching=false/null_store.
    // A zero-byte real FragmentCache retains no value (every entry has overhead).
    config.fragment_cache_bytes = 0;
    let root = config.storage.database.parent().unwrap().to_path_buf();
    let clock = Arc::new(FrozenClock::new(std::env::var("CAMPFIRE_FROZEN_TIME").unwrap().parse().unwrap()));
    let mut booted = boot_with_services(config, clock.clone(), crate::integrations::net::Network::system(),
        crate::jobs::periodic::Intervals { periodic: None, huddle: None }).await.unwrap();
    booted.jobs.stop(Duration::from_secs(1)).await;
    // Same busy and transparent items as the original, fetched only after UI opt-in.
    let recorded = crate::app::google_api_tests::Recorded::new(vec![(200, serde_json::to_vec(&json!({"items":[
        {"status":"confirmed","start":{"dateTime":"2026-03-02T15:55:00Z"},"end":{"dateTime":"2026-03-02T16:05:00Z"}},
        {"status":"confirmed","start":{"dateTime":"2026-03-02T15:54:00Z"},"end":{"dateTime":"2026-03-02T15:56:00Z"},"transparency":"transparent"}
    ]})).unwrap())]);
    booted.app.google.install_api(crate::integrations::google::api::Api::new(crate::app::google_api_tests::config(), recorded.clone()));
    let current = Arc::new(RwLock::new((booted.app.clone(), booted.fixture_kit.clone(), booted.router.clone())));
    let fixtures = current.clone();
    let mut environment = HUDDLE_ENVIRONMENT.into_iter()
        .map(|name| (name.to_owned(), std::env::var(name).ok())).collect::<BTreeMap<_,_>>();
    tokio::spawn(async move {
        let mut sequence = 0;
        loop {
            let request = root.join("ledger-request.json");
            if let Ok(bytes) = std::fs::read(&request) {
                let v: Value = serde_json::from_slice(&bytes).unwrap();
                if v["sequence"].as_u64().unwrap() > sequence {
                    sequence = v["sequence"].as_u64().unwrap();
                    let user = v["user"].as_i64().unwrap_or(127326141);
                    let app = fixtures.read().unwrap_or_else(|p| p.into_inner()).0.clone();
                    let answer = match v["action"].as_str().unwrap() {
                        "reset-fixtures" => {
                            app.fragment_cache.clear();
                            fixtures.read().unwrap_or_else(|p| p.into_inner()).1.rate_limits().clear();
                            json!({"reset":true})
                        }
                        "refresh" => {
                            let enabled = app.db.read(move |c| Ok(c.query_row("SELECT meeting_status_enabled FROM users WHERE id=?", [user], |r| r.get::<_,bool>(0))?)).await.unwrap();
                            assert!(enabled, "original opt-in persisted before executing refresh");
                            // perform_enqueued_jobs executes the refresh emitted by the UI
                            // save. Require that actual job and consume its actual argument.
                            let queued = app.db.read(|c| Ok(campfire_jobs::inspect::all(c)?.into_iter().filter(|j| j.class=="Calendar::MeetingRefreshJob").collect::<Vec<_>>())).await.unwrap();
                            assert_eq!(queued.len(),1,"original opt-in enqueues exactly one refresh");
                            let job=&queued[0];
                            assert_eq!(job.arguments["user_id"].as_i64(),Some(user));
                            let result = crate::integrations::google::meeting_refresh::refresh(&app, job.arguments["user_id"].as_i64().unwrap(), Timestamp::from_jiff(clock.now())).await.unwrap();
                            assert_eq!(result, crate::integrations::google::meeting_refresh::Result::Ok);
                            let calls=recorded.calls.lock().unwrap().clone();
                            assert_eq!(calls.len(),1,"original Google events.list stub");
                            assert_eq!(calls[0]["method"],"GET");
                            let request=url::Url::parse(&format!("https://www.googleapis.com{}",calls[0]["path"].as_str().unwrap())).unwrap();
                            assert_eq!(request.path(),"/calendar/v3/calendars/primary/events");
                            let query=request.query_pairs().collect::<std::collections::BTreeMap<_,_>>();
                            assert_eq!(query.get("singleEvents").map(|s|s.as_ref()),Some("true"));
                            assert_eq!(query.get("fields").map(|s|s.as_ref()),Some("items(eventType,start,end,status,transparency,attendees(self,responseStatus)),nextPageToken"));
                            let id=job.id;
                            app.db.write(move |tx| {tx.conn().execute("DELETE FROM background_jobs WHERE id=?",[id])?;Ok(())}).await.unwrap();
                            campfire_db::models::calendar_dispatch::dispatch_meetings(&app.db, Timestamp::from_jiff(clock.now())).await.unwrap();
                            json!({"enabled":enabled})
                        }
                        "advance" => {
                            clock.advance(jiff::SignedDuration::from_secs(360));
                            campfire_db::models::calendar_dispatch::dispatch_meetings(&app.db, Timestamp::from_jiff(clock.now())).await.unwrap();
                            json!({"clock":clock.now().to_string()})
                        }
                        "message" => create_message(&app,&v).await,
                        "messages" => {
                            let mut messages=Vec::new();
                            for message in v["messages"].as_array().unwrap() {
                                messages.push(create_message(&app,message).await);
                            }
                            json!({"messages":messages})
                        }
                        "huddle-configured" => json!({"configured":app.config.huddle.configured()}),
                        "environment" => {
                            let values = v["values"].as_object().expect("original Huddle environment is a map");
                            for (name, value) in values {
                                assert!(HUDDLE_ENVIRONMENT.contains(&name.as_str()), "environment key belongs to the original Huddle helper");
                                assert!(value.is_null() || value.is_string(), "environment value is a string or null");
                            }
                            let previous = values.keys().map(|name| (name.clone(), environment[name].clone()))
                                .collect::<BTreeMap<_,_>>();
                            for (name, value) in values {
                                environment.insert(name.clone(), value.as_str().map(str::to_owned));
                            }
                            let next = app.fixture_huddle_config(|name| environment.get(name).cloned().flatten());
                            let kit = fixtures.read().unwrap_or_else(|p| p.into_inner()).1.clone();
                            let (kit, router) = crate::app::fixture_router(&next, &kit);
                            let configured = next.config.huddle.configured();
                            // The DB sink and every active websocket retain their original
                            // services. Model broadcasts now render using this actual config.
                            next.jobs.set_fixture_app(&next);
                            *fixtures.write().unwrap_or_else(|p| p.into_inner()) = (next, kit, router);
                            json!({"previous":previous,"configured":configured})
                        }
                        "forgery-off" | "forgery-on" => {
                            let disabled=v["action"]=="forgery-off";
                            super::FORGERY_DISABLED.store(disabled,std::sync::atomic::Ordering::SeqCst);
                            json!({"disabled":disabled})
                        }
                        "group-huddle" => {
                            let users:Vec<i64>=v["users"].as_array().unwrap().iter().map(|x|x.as_i64().unwrap()).collect();
                            let session=v["session"].as_i64().unwrap();
                            let config=campfire_db::models::room_delete::HuddleConfig {api_secret:app.config.huddle.api_secret.clone(),admin_configured:app.config.huddle.admin_configured()};
                            let room=app.db.write(move |tx| {
                                let room=Room::find_or_create_direct_for(tx,&users,127326141)?;
                                let member=campfire_db::Membership::find_by_room_and_user(tx.conn(),room.id,127326141)?.unwrap();
                                campfire_db::models::huddle_grant::HuddleGrant::issue(tx,session,member.id,room.id,&config)?;
                                Ok(room)
                            }).await.unwrap();
                            json!({"room":room.id})
                        }
                        "broadcast" => {
                            let id = v["message"].as_i64().unwrap();
                            let (room,message) = app.db.read(move |c| {let m=Message::find(c,id)?;Ok((Room::find(c,m.room_id)?,m))}).await.unwrap();
                            broadcast(&app,&room,&message).await;
                            json!({"id":id})
                        }
                        action => panic!("unrecognized original fixture action {action}"),
                    };
                    let answer = json!({"sequence":sequence,"result":answer});
                    let tmp = root.join("ledger-response.tmp");
                    std::fs::write(&tmp, answer.to_string()).unwrap();
                    std::fs::rename(tmp, root.join("ledger-response.json")).unwrap();
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    });
    // Each new HTTP request gets the current real router. Established Cable
    // sockets stay on the same Cable server throughout the helper's ENV block.
    let router = axum::Router::new().fallback_service(tower::service_fn(move |request: axum::extract::Request| {
        let router = current.read().unwrap_or_else(|p| p.into_inner()).2.clone();
        async move { router.oneshot(request).await }
    }));
    campfire_kit::front::serve(campfire_kit::front::FrontConfig::from_env(), router, campfire_kit::server::shutdown_signal()).await.unwrap();
}
