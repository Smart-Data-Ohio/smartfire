//! Named Rails controller ports plus real middleware, transaction and presentation checks.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread};
async fn app() -> TestApp {
    TestApp::boot_with_test_clock(std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    )))
    .await
    .expect("WS8bm2 requires default seed")
}
async fn slash(b: &mut Browser<'_>, room: i64, text: &str) -> Reply {
    b.write(
        Req::new(Method::POST, &format!("/rooms/{room}/slash_commands"))
            .json(serde_json::json!({"text":text})),
    )
    .await
}
#[tokio::test]
async fn shrug_posts_through_the_dispatcher() {
    let app = app().await;
    let r = slash(&mut app.david(), ALL_TALK, "/shrug ship it").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["status"], "posted");
    let id = r.json()["message_id"].as_i64().unwrap();
    let m = app.db().read(move |c| Message::find(c, id)).await.unwrap();
    assert_eq!(m.markdown_source.as_deref(), Some("ship it ¯\\_(ツ)_/¯"));
}
#[tokio::test]
async fn unknown_commands_answer_an_error_without_posting() {
    let app = app().await;
    let r = slash(&mut app.david(), ALL_TALK, "/frobnicate").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["status"], "error");
    assert!(
        r.json()["message"]
            .as_str()
            .unwrap()
            .contains("Unknown command")
    );
}
#[tokio::test]
async fn status_answers_ephemeral_confirmation() {
    let app = app().await;
    let r = slash(&mut app.david(), ALL_TALK, "/status 🚂 On a train").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["status"], "ephemeral");
    assert_eq!(
        app.db()
            .read(|c| Ok(c.query_row(
                "SELECT custom_status_emoji FROM users WHERE id=?",
                [DAVID],
                |r| r.get::<_, String>(0)
            )?))
            .await
            .unwrap(),
        "🚂"
    );
}
#[tokio::test]
async fn event_answers_an_open_url() {
    let app = app().await;
    let r = slash(&mut app.david(), ALL_TALK, "/event Launch party").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["status"], "open_url");
    assert!(r.json()["url"].as_str().unwrap().contains("Launch"));
}
#[tokio::test]
async fn bare_event_opens_the_blank_form() {
    let app = app().await;
    let r = slash(&mut app.david(), ALL_TALK, "/event").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["url"], format!("/rooms/{ALL_TALK}/events/new"));
}
#[tokio::test]
async fn poll_answers_open_poll() {
    let app = app().await;
    let r = slash(&mut app.david(), ALL_TALK, "/poll").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json(), serde_json::json!({"status":"open_poll"}));
}
#[tokio::test]
async fn thread_commands_dispatch_with_the_thread() {
    let app = app().await;
    let t = app
        .db()
        .write(|tx| {
            ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    name: Some("Side chat".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let r = app
        .david()
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/slash_commands"))
                .json(serde_json::json!({"text":"/poll","thread_id":t.id})),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()["status"], "error");
    assert!(
        r.json()["message"]
            .as_str()
            .unwrap()
            .contains("not in threads")
    );
}
#[tokio::test]
async fn non_members_get_404() {
    let app = app().await;
    assert_eq!(
        slash(&mut app.david(), DIRECT_KEVIN_BENDER, "/shrug hi")
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn bots_are_forbidden() {
    let app = app().await;
    assert_eq!(
        slash(&mut app.sign_in(BENDER).await, ALL_TALK, "/shrug hi")
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn agent_commands_invoke_through_the_room_and_persist_the_rails_event() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/agent_command.json"
    )).unwrap();
    let app = app().await.without_job_runner().await;
    app.db().write(|tx| {
        let agent_id = tx.conn().query_row("SELECT id FROM agents WHERE user_id=?", [BENDER], |r| r.get(0))?;
        campfire_db::AgentSlashCommand::create(tx, campfire_db::NewAgentSlashCommand {
            agent_id, room_id: ALL_TALK, name: "deploy".into(), ..Default::default()
        })?;
        Ok(())
    }).await.unwrap();
    let response = slash(&mut app.david(), ALL_TALK, "/deploy staging").await;
    assert_eq!(response.status.as_u16(), oracle["status"].as_u64().unwrap() as u16);
    assert_eq!(response.text(), oracle["body"].as_str().unwrap());
    let event = app.db().read(|conn| {
        Ok(conn.query_row("SELECT agent_id,room_id,actor_id,event_type,outcome,metadata FROM agent_events WHERE event_type='slash_command' ORDER BY id DESC LIMIT 1", [], |r| {
            Ok(serde_json::json!({"agent_id":r.get::<_,i64>(0)?,"room_id":r.get::<_,i64>(1)?,"actor_id":r.get::<_,i64>(2)?,"event_type":r.get::<_,String>(3)?,"outcome":r.get::<_,String>(4)?,"metadata":serde_json::from_str::<serde_json::Value>(&r.get::<_,String>(5)?).unwrap()}))
        })?)
    }).await.unwrap();
    assert_eq!(event, oracle["event"]);
}
#[tokio::test]
async fn slash_writes_require_csrf_and_scope_threads() {
    let app = app().await;
    let r = app
        .david()
        .send(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/slash_commands"))
                .json(serde_json::json!({"text":"/shrug hi"})),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    let r = app
        .david()
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/slash_commands"))
                .json(serde_json::json!({"text":"/shrug hi","thread_id":-1})),
        )
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn autocomplete_lists_commands_and_denies_other_rooms() {
    let app = app().await;
    let r = app
        .david()
        .get(&format!(
            "/autocompletable/slash_commands.json?room_id={ALL_TALK}"
        ))
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json().as_array().unwrap().len(), 10);
    assert_eq!(
        app.david()
            .get(&format!(
                "/autocompletable/slash_commands.json?room_id={DIRECT_KEVIN_BENDER}"
            ))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn icon_autocomplete_lists_ranked_brands_and_emoji() {
    let app = app().await;
    let r = app.david().get("/autocompletable/icons.json?q=open").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()[0]["name"], "openai");
    assert!(
        r.json()
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "emoji")
    );
}
#[tokio::test]
async fn user_autocomplete_has_markdown_names_and_unique_tokens() {
    let app = app().await;
    let r = app
        .david()
        .get(&format!(
            "/autocompletable/users.json?room_id={ALL_TALK}&query=da"
        ))
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()[0]["markdown_display_name"], "David");
    assert_eq!(r.json()[0]["mention_token"], "@[David]");
}

trait JsonRequest {
    fn json(self, value: serde_json::Value) -> Self;
}
impl JsonRequest for Req {
    fn json(self, value: serde_json::Value) -> Self {
        self.header("content-type", "application/json")
            .header("accept", "application/json")
            .body(value.to_string())
    }
}

async fn icons(b: &mut Browser<'_>, query: &str) -> Reply {
    b.get(&format!("/autocompletable/icons.json?q={}", encode(query)))
        .await
}
async fn custom(app: &TestApp, name: &str, title: &str) {
    let name = name.to_owned();
    let title = title.to_owned();
    app.db().write(move|tx| {
        tx.conn().execute("INSERT INTO workspace_icons(name,title,creator_id,created_at,updated_at) VALUES(?,?,?,?,?)",(&name,&title,DAVID,tx.now(),tx.now()))?;Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn icons_require_authentication_like_the_users_endpoint() {
    let app = app().await;
    let r = icons(&mut app.anonymous(), "open").await;
    assert_eq!(r.status, StatusCode::FOUND);
    assert!(r.location().unwrap().ends_with("/session/new"));
}
#[tokio::test]
async fn icons_return_mixed_brand_and_emoji_matches_with_their_payloads() {
    let app = app().await;
    let r = icons(&mut app.david(), "open").await;
    assert_eq!(r.status, StatusCode::OK);
    let rows = r.json();
    let first = &rows[0];
    assert_eq!(first["name"], "openai");
    assert_eq!(first["title"], "OpenAI");
    assert!(
        first["image"]
            .as_str()
            .unwrap()
            .starts_with("/assets/icons/brands/openai-")
    );
    assert!(first.get("character").is_none());
    let e = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "emoji")
        .unwrap();
    assert!(e.get("image").is_none());
    assert!(!e["character"].as_str().unwrap().is_empty());
}
#[tokio::test]
async fn icons_order_prefix_matches_first_and_limit_the_results() {
    let app = app().await;
    let r = icons(&mut app.david(), "fire").await;
    assert_eq!(r.status, StatusCode::OK);
    let rows = r.json();
    assert_eq!(rows[0]["name"], "fire");
    assert!(
        rows.as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "heart_on_fire")
    );
    assert!(rows.as_array().unwrap().len() <= 8);
}
#[tokio::test]
async fn icons_return_no_matches_for_blank_or_unknown_queries() {
    let app = app().await;
    for q in ["", "zzz_no_such_icon"] {
        assert_eq!(
            icons(&mut app.david(), q).await.json(),
            serde_json::json!([])
        );
    }
}
#[tokio::test]
async fn icons_return_workspace_icons_with_their_stable_image_url() {
    let app = app().await;
    custom(&app, "acme", "Acme Corp").await;
    let r = icons(&mut app.david(), "acme").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()[0]["image"], "/icons/acme");
    assert!(r.json()[0].get("character").is_none());
}
#[tokio::test]
async fn icons_list_every_workspace_icon_for_the_picker_custom_tab() {
    let app = app().await;
    custom(&app, "acme", "Acme Corp").await;
    custom(&app, "globex", "Globex").await;
    let r = app
        .david()
        .get("/autocompletable/icons.json?custom=0")
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.json()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["acme", "globex"]
    );
}
async fn commands(b: &mut Browser<'_>, extra: &str) -> Reply {
    b.get(&format!(
        "/autocompletable/slash_commands.json?room_id={ALL_TALK}{extra}"
    ))
    .await
}
#[tokio::test]
async fn picker_lists_built_ins_with_metadata() {
    let app = app().await;
    let r = commands(&mut app.david(), "").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.json()
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "huddle", "event", "poll", "remind", "status", "dnd", "ooo", "shrug", "me", "play"
        ]
    );
}
#[tokio::test]
async fn picker_exposes_takes_arguments_for_immediate_and_argument_commands() {
    let app = app().await;
    let r = commands(&mut app.david(), "").await;
    let rows = r.json();
    let c = |name| {
        rows.as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap()
    };
    for name in ["huddle", "event", "poll"] {
        assert_eq!(c(name)["takes_arguments"], false);
    }
    for name in ["remind", "shrug"] {
        assert_eq!(c(name)["takes_arguments"], true);
    }
    assert_eq!(c("remind")["arg_hint"], "<when> <text>");
}
async fn registration(app: &TestApp, takes: bool, description: Option<&str>) -> i64 {
    let description = description.map(str::to_owned);
    app.db().write(move|tx| {
        let agent:i64=tx.conn().query_row("SELECT id FROM agents LIMIT 1",[],|r|r.get(0))?;
        tx.conn().execute("INSERT INTO agent_slash_commands(agent_id,room_id,name,description,takes_arguments,created_at,updated_at) VALUES(?,?,'deploy',?,?,?,?)",(agent,ALL_TALK,description,takes,tx.now(),tx.now()))?;Ok(agent)
    }).await.unwrap()
}
#[tokio::test]
async fn picker_includes_the_rooms_agent_commands() {
    let app = app().await;
    let agent = registration(&app, true, Some("Ship it")).await;
    let expected=app.db().read(move|c|Ok(c.query_row("SELECT users.name FROM agents JOIN users ON users.id=agents.user_id WHERE agents.id=?",[agent],|r|r.get::<_,String>(0))?)).await.unwrap();
    let r = commands(&mut app.david(), "").await;
    assert_eq!(r.status, StatusCode::OK);
    let rows = r.json();
    let deploy = rows.as_array().unwrap().last().unwrap();
    assert_eq!(deploy["description"], "Ship it");
    assert_eq!(deploy["agent"], expected);
    assert_eq!(deploy["takes_arguments"], true);
}
#[tokio::test]
async fn picker_agent_commands_registered_without_arguments_run_immediately() {
    let app = app().await;
    registration(&app, false, None).await;
    let r = commands(&mut app.david(), "").await;
    assert_eq!(r.status, StatusCode::OK);
    let rows = r.json();
    let deploy = rows.as_array().unwrap().last().unwrap();
    assert_eq!(deploy["takes_arguments"], false);
    assert_eq!(deploy["description"], "Custom command");
}
#[tokio::test]
async fn picker_thread_conversations_hide_root_only_commands() {
    let app = app().await;
    let t = app
        .db()
        .write(|tx| {
            ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let r = commands(&mut app.david(), &format!("&thread_id={}", t.id)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        !r.json()
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "poll")
    );
}
#[tokio::test]
async fn picker_filters_by_query() {
    let app = app().await;
    let r = commands(&mut app.david(), "&query=STA").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        r.json()
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "status")
    );
    assert!(
        !r.json()
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "poll")
    );
}
#[tokio::test]
async fn picker_non_members_get_404() {
    let app = app().await;
    assert_eq!(
        app.david()
            .get(&format!(
                "/autocompletable/slash_commands.json?room_id={DIRECT_KEVIN_BENDER}"
            ))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}
async fn users(b: &mut Browser<'_>, extra: &str) -> Reply {
    b.get(&format!("/autocompletable/users.json?query=da{extra}"))
        .await
}
#[tokio::test]
async fn users_search_returns_matching_users() {
    let app = app().await;
    let r = users(&mut app.david(), "").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()[0]["name"], "David");
}
#[tokio::test]
async fn users_search_results_escape_html_in_names() {
    let app = app().await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET name='David <script>alert(123)</script>' WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let r = users(&mut app.david(), "").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.json()[0]["name"],
        "David &lt;script&gt;alert(123)&lt;/script&gt;"
    );
    assert_eq!(
        r.json()[0]["markdown_display_name"],
        "David <script>alert(123)</script>"
    );
}
#[tokio::test]
async fn users_room_search_returns_matching_users() {
    let app = app().await;
    let r = users(&mut app.david(), &format!("&room_id={ALL_TALK}")).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json()[0]["mention_token"], "@[David]");
}
#[tokio::test]
async fn users_room_search_omits_the_markdown_token_for_duplicate_display_names() {
    let app = app().await;
    app.db()
        .write(|tx| {
            let u = campfire_db::User::create(
                tx,
                campfire_db::NewUser {
                    name: "David".into(),
                    ..Default::default()
                },
            )?;
            let room = campfire_db::Room::find(tx.conn(), ALL_TALK)?;
            room.grant_to(tx, &[u.id])
        })
        .await
        .unwrap();
    let r = users(&mut app.david(), &format!("&room_id={ALL_TALK}")).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json().as_array().unwrap().len(), 2);
    assert!(
        r.json()
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r.get("mention_token").is_none())
    );
}
#[tokio::test]
async fn users_room_search_is_scoped_by_membership() {
    let app = app().await;
    assert_eq!(
        users(
            &mut app.sign_in(KEVIN).await,
            &format!("&room_id={ALL_TALK}")
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
}

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../vectors/messaging/slash.json")).unwrap()
}
#[tokio::test]
async fn slash_and_picker_responses_match_pinned_rails_exact_bytes() {
    let app = app().await;
    let fixture = oracle();
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET time_zone='America/New_York' WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    for case in fixture["play"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap().to_owned();
        let client = case["client_message_id"].as_str().unwrap().to_owned();
        app.db()
            .write(move |tx| {
                Message::create(
                    tx,
                    campfire_db::NewMessage {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        markdown_source: Some(source),
                        client_message_id: Some(client),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
    }
    let mut browser = app.david();
    for step in fixture["steps"].as_array().unwrap() {
        let r = browser
            .write(
                Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/slash_commands"))
                    .json(serde_json::json!({"text":step["text"]})),
            )
            .await;
        assert_eq!(
            r.status.as_u16(),
            step["status"].as_u64().unwrap() as u16,
            "{}",
            step["text"]
        );
        assert_eq!(r.text(), step["body"].as_str().unwrap(), "{}", step["text"]);
    }
    custom(&app, "acme", "Acme").await;
    custom(&app, "globex", "Globex").await;
    let agent = registration(&app, true, Some("Ship it")).await;
    app.db().write(move|tx|{tx.conn().execute("INSERT INTO agent_slash_commands(agent_id,room_id,name,takes_arguments,created_at,updated_at) VALUES(?,?,'zero',0,?,?)",(agent,ALL_TALK,tx.now(),tx.now()))?;Ok(())}).await.unwrap();
    let t = app
        .db()
        .write(|tx| {
            ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    name: Some("Side chat".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    assert_eq!(t.id, fixture["thread_id"].as_i64().unwrap());
    for step in fixture["pickers"].as_array().unwrap() {
        let r = browser.get(step["path"].as_str().unwrap()).await;
        assert_eq!(
            r.status.as_u16(),
            step["status"].as_u64().unwrap() as u16,
            "{}",
            step["path"]
        );
        assert_eq!(r.text(), step["body"].as_str().unwrap(), "{}", step["path"]);
        for (header, field) in [("x-total-count", "total"), ("link", "link")] {
            assert_eq!(
                r.headers.get(header).map(|h| h.to_str().unwrap()),
                step[field].as_str(),
                "{}",
                step["path"]
            );
        }
    }
    for step in fixture["formats"].as_array().unwrap() {
        let r = browser.get(step["path"].as_str().unwrap()).await;
        assert_eq!(
            r.status.as_u16(),
            step["status"].as_u64().unwrap() as u16,
            "{}",
            step["path"]
        );
        assert_eq!(
            r.headers["content-type"]
                .to_str()
                .unwrap()
                .split(';')
                .next()
                .unwrap(),
            step["content_type"].as_str().unwrap()
        );
    }
}
#[tokio::test]
async fn play_presentation_matches_every_rails_sound_and_unknown_inputs() {
    use askama::Template;
    let app = app().await;
    let state = app.booted.app.clone();
    for case in oracle()["play"].as_array().unwrap() {
        let source = case["source"].as_str().unwrap().to_owned();
        let client = case["client_message_id"].as_str().unwrap().to_owned();
        let m = app
            .db()
            .write(move |tx| {
                Message::create(
                    tx,
                    campfire_db::NewMessage {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        markdown_source: Some(source),
                        client_message_id: Some(client),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
        let actual = app
            .db()
            .read({
                let state = state.clone();
                move |conn| {
                    let p = crate::controllers::presenters::Presenter::new(
                        conn,
                        &state,
                        Some("campfire.test".into()),
                    );
                    let view = p.message(&m)?;
                    Ok(crate::controllers::presenters::page::render_detached_at(
                        &state,
                        None,
                        "http://campfire.test",
                        |ctx| {
                            campfire_views::messages::PresentationPartial {
                                ctx,
                                message: &view,
                            }
                            .render()
                            .unwrap()
                        },
                    ))
                }
            })
            .await
            .unwrap();
        assert_eq!(actual, case["html"].as_str().unwrap(), "{}", case["source"]);
    }
}
#[tokio::test]
async fn slash_remind_rolls_back_post_save_and_index_when_job_insert_fails() {
    let app = app().await;
    let count = || {
        app.db().read(|conn| {
            Ok((
                conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
                conn.query_row("SELECT COUNT(*) FROM saved_items", [], |r| {
                    r.get::<_, i64>(0)
                })?,
                conn.query_row("SELECT COUNT(*) FROM message_search_index", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            ))
        })
    };
    let before = count().await.unwrap();
    app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER slash_reject_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'job rejected'); END;")?;Ok(())}).await.unwrap();
    assert_eq!(
        slash(
            &mut app.david(),
            ALL_TALK,
            "/remind in 20 minutes review deploy"
        )
        .await
        .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(count().await.unwrap(), before);
}
#[tokio::test]
async fn slash_post_reaches_a_real_websocket_once_without_session_values() {
    use crate::channels::tests::support::{Client, bind_listener, identifier};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let app = app().await;
    let listener = bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("host", "campfire.test".parse().unwrap());
    request
        .headers_mut()
        .insert("origin", "http://campfire.test".parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let room = app
        .db()
        .read(|conn| campfire_db::Room::find(conn, ALL_TALK))
        .await
        .unwrap();
    let stream = rails_compat::turbo::signed_stream_name(
        &app.booted.app.secrets,
        &[&crate::channels::room_gid(&room).to_param(), "messages"],
    );
    client
        .confirm(&identifier(
            serde_json::json!({"channel":"RoomMessagesChannel","signed_stream_name":stream}),
        ))
        .await;
    assert_eq!(
        slash(&mut app.david(), ALL_TALK, "/shrug Socket slash example")
            .await
            .status,
        StatusCode::OK
    );
    let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
    let html = frame["message"].as_str().unwrap();
    assert!(html.starts_with("<turbo-stream action=\"append\""));
    assert!(html.contains("Socket slash example"));
    assert!(html.contains("http://campfire.test/"));
    assert!(!html.contains("authenticity_token") && !html.contains("nonce=\""));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), client.next_text())
            .await
            .is_err()
    );
    server.abort();
}
#[tokio::test]
async fn user_tokens_check_duplicates_outside_the_page_and_omit_invalid_names() {
    let app = app().await;
    app.db()
        .write(|tx| {
            let room = campfire_db::Room::find(tx.conn(), ALL_TALK)?;
            for _ in 0..24 {
                let u = campfire_db::User::create(
                    tx,
                    campfire_db::NewUser {
                        name: "David".into(),
                        ..Default::default()
                    },
                )?;
                room.grant_to(tx, &[u.id])?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let r = users(&mut app.david(), &format!("&room_id={ALL_TALK}&page=1")).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json().as_array().unwrap().len(), 20);
    assert_eq!(r.headers["x-total-count"], "25");
    assert!(r.headers.contains_key("link"));
    assert!(
        r.json()
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r.get("mention_token").is_none())
    );
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET name='David [Test]' WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    let r = app
        .david()
        .get(&format!(
            "/autocompletable/users.json?room_id={ALL_TALK}&query=%5BTest%5D"
        ))
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json().as_array().unwrap().len(), 1);
    assert!(r.json()[0].get("mention_token").is_none());
}
