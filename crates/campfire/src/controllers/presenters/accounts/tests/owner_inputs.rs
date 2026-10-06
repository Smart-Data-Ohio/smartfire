//! HTTP adapters over WS11's raw inputs; expected casts/errors come from pinned Rails.
use super::*;
use serde_json::{Value, json};

fn inputs() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../../vectors/bot-input-contract.json"
    ))
    .unwrap()
}
async fn json_edit(browser: &mut Browser<'_>, path: &str, body: Value) -> Reply {
    browser
        .request(
            Method::PATCH,
            path,
            &[("accept", "text/html")],
            Some(("application/json", body.to_string())),
        )
        .await
}
async fn snapshot(test: &Test, bot: i64) -> Value {
    test.booted
        .app
        .db
        .read(move |conn| {
            let user = campfire_db::User::find(conn, bot)?;
            let agent = campfire_db::Agent::for_user(conn, bot)?.unwrap();
            let audits: i64 = conn.query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action='agent.update'",
                [],
                |r| r.get(0),
            )?;
            Ok(json!([
                user.name,
                user.icon_name,
                user.updated_at.to_db(),
                agent.provider,
                agent.daily_message_cap,
                agent.daily_board_post_cap,
                agent.daily_external_action_cap,
                agent.updated_at.to_db(),
                audits
            ]))
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn raw_cap_http_inputs_match_all_81_rails_casts_errors_and_strong_parameters() {
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let mut admin = test.browser("198.51.100.241");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/account/bots/{bot}");
    for case in inputs()["caps"].as_array().unwrap() {
        test.booted.app.db.write(move |tx| {
            tx.conn().execute("UPDATE users SET name='Input baseline',icon_name=NULL WHERE id=?",[bot])?;
            tx.conn().execute("UPDATE agents SET provider='Before input',daily_message_cap=7,daily_board_post_cap=7,daily_external_action_cap=7 WHERE user_id=?",[bot])?;
            Ok(())
        }).await.unwrap();
        let before = snapshot(&test, bot).await;
        let field = case["field"].as_str().unwrap();
        let mut agent = json!({"provider":"Submitted provider"});
        agent[field] = case["input"].clone();
        let response = json_edit(
            &mut admin,
            &path,
            json!({"user":{"name":"Submitted bot","icon_name":"openai"},"agent":agent}),
        )
        .await;
        let invalid = !case["errors"].as_array().unwrap().is_empty();
        let out_of_range = case["permitted"] == true
            && !case["value"].is_null()
            && case["value"].as_i64().is_none()
            && !invalid;
        if invalid || out_of_range {
            assert_eq!(
                response.status,
                if out_of_range {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else {
                    StatusCode::UNPROCESSABLE_ENTITY
                },
                "{case}: {}",
                response.text()
            );
            let after = snapshot(&test, bot).await;
            if out_of_range {
                // Pinned HTTP oracle: invalid? succeeds, update_bot commits,
                // then Agent#save! raises RangeError before either audit.
                let boundaries: Value = serde_json::from_str(include_str!(
                    "../../../../../../../vectors/bot-ui-mutation-boundaries.json"
                ))
                .unwrap();
                let fault = boundaries["rows"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["cap_field"] == field)
                    .unwrap();
                assert_eq!(after[0], fault["bot"]["name"]);
                assert_eq!(after[1], fault["bot"]["icon_name"]);
                assert_eq!(
                    after.as_array().unwrap()[3..],
                    before.as_array().unwrap()[3..],
                    "failed agent save changed agent or audits: {case}"
                );
                assert!(fault["audits"].as_array().unwrap().is_empty());
                assert_eq!(
                    response.text(),
                    boundaries["errors"]["500"]["body"].as_str().unwrap()
                );
            } else {
                assert_eq!(after, before, "invalid input wrote data: {case}");
            }
            if invalid {
                let label = match field {
                    "daily_message_cap" => "Daily message cap",
                    "daily_board_post_cap" => "Daily board post cap",
                    _ => "Daily external action cap",
                };
                for error in case["errors"].as_array().unwrap() {
                    assert!(
                        response
                            .text()
                            .contains(&format!("{label} {}", error.as_str().unwrap())),
                        "{case}"
                    );
                }
                let raw = match &case["input"] {
                    Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                assert!(
                    response.text().contains(&format!(
                        "value=\"{raw}\" name=\"agent[{field}]\" id=\"agent_{field}\""
                    )),
                    "raw value lost: {case}: {}",
                    response.text()
                );
                assert!(
                    response.text().contains("value=\"Input baseline\""),
                    "Rails doesn't assign bot params when the agent is invalid"
                );
            }
        } else {
            assert_redirect(&response, "http://campfire.test/account/bots");
            let after = snapshot(&test, bot).await;
            let offset = match field {
                "daily_message_cap" => 4,
                "daily_board_post_cap" => 5,
                _ => 6,
            };
            assert_eq!(
                after[offset],
                if case["permitted"] == true {
                    case["value"].clone()
                } else {
                    json!(7)
                },
                "{case}"
            );
            assert_eq!(after[0], "Submitted bot");
            assert_eq!(after[1], "openai");
        }
    }
}
#[tokio::test]
async fn icon_http_updates_match_all_21_rails_normalizations_and_reject_atomically() {
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let mut admin = test.browser("198.51.100.242");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/account/bots/{bot}");
    for case in inputs()["icons"].as_array().unwrap() {
        test.booted
            .app
            .db
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE users SET name='Input baseline',icon_name='robot' WHERE id=?",
                    [bot],
                )?;
                tx.conn().execute(
                    "UPDATE agents SET provider='Before input' WHERE user_id=?",
                    [bot],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let before = snapshot(&test, bot).await;
        let response=json_edit(&mut admin,&path,json!({"user":{"name":"Submitted bot","icon_name":case["input"]},"agent":{"provider":"Submitted provider"}})).await;
        if !case["errors"].as_array().unwrap().is_empty() {
            assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY, "{case}");
            assert!(
                response.text().contains("Icon name is not a known icon"),
                "{}",
                response.text()
            );
            assert_eq!(
                snapshot(&test, bot).await,
                before,
                "bad icon persisted agent, bot or audit: {case}"
            );
            assert!(
                response.text().contains("value=\"Submitted bot\""),
                "rejected bot assignment must survive in the form"
            );
        } else {
            assert_redirect(&response, "http://campfire.test/account/bots");
            let after = snapshot(&test, bot).await;
            let expected = if case["permitted"] == true {
                case["value"].clone()
            } else {
                json!("robot")
            };
            assert_eq!(after[1], expected, "{case}");
            if before[1] != expected {
                test.booted.app.db.read(move |conn| {
                    let details:Value=conn.query_row("SELECT details FROM audit_logs WHERE action='agent.update' ORDER BY id DESC LIMIT 1",[],|r|r.get(0))?;
                    assert_eq!(details["icon_name"],json!({"before":"robot","after":expected}));Ok(())
                }).await.unwrap();
            }
        }
    }
}
#[tokio::test]
async fn bot_creation_uses_owner_icon_validation_and_never_inserts_unknown_icons() {
    let test = boot_seed("default").await.expect("default seed");
    let mut admin = test.browser("198.51.100.243");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    let before = test
        .booted
        .app
        .db
        .read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    let rejected = admin
        .form(
            "post",
            "/account/bots",
            &[
                ("user[name]", "Rejected icon"),
                ("user[icon_name]", ":notanicon:"),
            ],
        )
        .await;
    assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(rejected.text().contains("Icon name is not a known icon"));
    assert!(rejected.text().contains("value=\":notanicon:\""));
    test.booted
        .app
        .db
        .read(move |conn| {
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0))?,
                before
            );
            Ok(())
        })
        .await
        .unwrap();
    let created = admin
        .form(
            "post",
            "/account/bots",
            &[
                ("user[name]", "Created icon"),
                ("user[icon_name]", " ::: OpenAI ::: "),
            ],
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(created.headers["cache-control"], "no-store");
    test.booted
        .app
        .db
        .read(|conn| {
            let id = conn.query_row("SELECT id FROM users WHERE name='Created icon'", [], |r| {
                r.get(0)
            })?;
            let user = campfire_db::User::find(conn, id)?;
            assert_eq!(user.icon_name.as_deref(), Some("openai"));
            assert!(campfire_db::Agent::for_user(conn, user.id)?.is_some());
            Ok(())
        })
        .await
        .unwrap();
}
