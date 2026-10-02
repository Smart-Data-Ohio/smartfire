//! Pinned HTTP commit/fault boundaries; no controller or domain method is mocked.
use crate::controllers::presenters::test_support::{DAVID, Req, TestApp};
use axum::http::Method;
use campfire_db::{Agent, AgentKind, NewAgent, User};
use serde_json::{Value, json};

#[tokio::test]
async fn ws11ui_bot_mutation_independent_saves_match_pinned_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-mutation-boundaries.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in oracle["rows"].as_array().unwrap() {
        let t = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let mut browser = t.david();
        browser.grant_sudo().await;
        let name = case["name"].as_str().unwrap().to_owned();
        let operation = case["operation"].as_str().unwrap();
        let bot = if operation == "create" {
            None
        } else {
            let legacy = operation.starts_with("legacy");
            let initial = name.clone();
            Some(
                t.db()
                    .write(move |tx| {
                        let bot = User::create_bot(tx, &initial, Some("https://old.example/hook"))?;
                        if !legacy {
                            Agent::create(
                                tx,
                                NewAgent {
                                    user_id: bot.id,
                                    owner_id: Some(DAVID),
                                    kind: AgentKind::Workspace,
                                    ..Default::default()
                                },
                            )?;
                        }
                        Ok(bot.id)
                    })
                    .await
                    .unwrap(),
            )
        };
        let before = t
            .db()
            .read(|conn| {
                Ok(
                    conn.query_row("SELECT COALESCE(MAX(id),0) FROM audit_logs", [], |r| {
                        r.get::<_, i64>(0)
                    })?,
                )
            })
            .await
            .unwrap();
        if let Some(table) = case["table"].as_str() {
            let event = case["event"].as_str().unwrap();
            let predicate = case["predicate"]
                .as_str()
                .map(|p| format!("WHEN {p}"))
                .unwrap_or_default();
            let sql = format!(
                "CREATE TRIGGER reject_bot_ui BEFORE {event} ON {table} {predicate} BEGIN SELECT RAISE(ABORT,'fixture bot boundary rejection'); END;"
            );
            t.db()
                .write(move |tx| {
                    tx.conn().execute_batch(&sql)?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        let path = bot
            .map(|id| format!("/account/bots/{id}"))
            .unwrap_or_else(|| "/account/bots".into());
        let response = match operation {
            "create" => {
                browser
                    .write(Req::new(Method::POST, &path).form(&[
                        ("user[name]", &name),
                        ("user[webhook_url]", "https://new.example/hook"),
                    ]))
                    .await
            }
            "update" => {
                browser
                    .write(Req::new(Method::PATCH, &path).form(&[
                        ("user[name]", &format!("{name} changed")),
                        ("user[webhook_url]", "https://new.example/hook"),
                        ("agent[provider]", "Changed"),
                    ]))
                    .await
            }
            "remove" => browser.write(Req::new(Method::DELETE, &path)).await,
            "legacy" => {
                browser
                    .send(Req::new(Method::GET, &format!("{path}/credentials")))
                    .await
            }
            "legacy_post" => {
                browser
                    .write(Req::new(Method::POST, &format!("{path}/credentials")))
                    .await
            }
            _ => unreachable!(),
        };
        if let Some(error) = oracle["errors"].get(response.status.as_u16().to_string()) {
            assert_eq!(response.text(), error["body"].as_str().unwrap());
            assert_eq!(response.content_type(), error["content_type"].as_str());
        }
        let actual=t.db().read(move|conn|{
            use rusqlite::OptionalExtension;
            let id=match bot {Some(id)=>Some(id),None=>conn.query_row("SELECT id FROM users WHERE name=?",[name],|r|r.get::<_,i64>(0)).optional()?};
            let user=id.map(|id|User::find(conn,id)).transpose()?;
            let agent=match id {Some(id)=>Agent::for_user(conn,id)?,None=>None};
            let user=user.map(|u|Ok::<_,campfire_db::Error>(json!({"name":u.name,"status":u.status.name(),"webhook_url":u.webhook_url(conn)?}))).transpose()?;
            let agent=agent.map(|a|json!({"kind":a.kind.name(),"owner_id":a.owner_id,"provider":a.provider}));
            let mut statement=conn.prepare("SELECT action FROM audit_logs WHERE id>? ORDER BY id")?;
            let audits=statement.query_map([before],|r|r.get::<_,String>(0))?.collect::<Result<Vec<_>,_>>()?;
            Ok(json!({"bot":user,"agent":agent,"audits":audits}))
        }).await.unwrap();
        let expected = json!({"bot":case["bot"],"agent":case["agent"],"audits":case["audits"]});
        if actual != expected || response.status.as_u16() != case["status"].as_u64().unwrap() as u16
        {
            failures.push(format!(
                "{}: HTTP {}; {actual}; Rails {expected}",
                case["name"], response.status
            ));
        }
    }
    println!("Bot mutation differential: 13 HTTP fault boundaries");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
