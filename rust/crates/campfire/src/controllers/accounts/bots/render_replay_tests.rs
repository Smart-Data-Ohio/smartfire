//! PR196 R4 boundaries, captured from pinned Rails by render_replay.sh.
use super::{github_connections, input_casts};
use crate::controllers::presenters::test_support::{
    BENDER, DAVID, Req, TestApp, with_fixed_render_secrets,
};
use campfire_db::Timestamp;
use campfire_kit::{Method, Param};
use campfire_views::time::Zone;
use serde_json::{Value, json};

fn inputs() -> Value {
    use std::io::Read;
    let bytes = include_bytes!(
        "../../../../../../reference-tools/views/agents_ui/render_replay_inputs.json.gz"
    );
    let mut text = String::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .read_to_string(&mut text)
        .unwrap();
    serde_json::from_str(&text).unwrap()
}
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-render-replay.json"
    ))
    .unwrap()
}

#[test]
fn pr196_r4_boundary_expiry_rendering_matches_rails() {
    let inputs = inputs();
    let oracle = oracle();
    let now = Timestamp::parse_db("2026-03-02 16:00:00").unwrap();
    let mut failures = Vec::new();
    let mut renders = 0;
    let mut roundtrips = 0;
    for case in oracle["expiry"].as_array().unwrap() {
        let input = Param::Str(
            inputs["expiry"][case["input_index"].as_u64().unwrap() as usize]
                .as_str()
                .unwrap()
                .into(),
        );
        let zone = Zone::for_user(case["zone"].as_str());
        let actual = match input_casts::datetime(Some(&input), &zone, now) {
            Ok(value) => {
                if let Some(ts) = value {
                    roundtrips += 1;
                    assert_eq!(Timestamp::parse_db(&ts.to_db()), Some(ts));
                    renders += 1;
                }
                let rendered = value.and_then(|ts| {
                    std::panic::catch_unwind(|| input_casts::extended_datetime(ts, &zone, true))
                        .ok()
                });
                json!({"stored":value.map(|ts|ts.to_db()), "render":rendered})
            }
            Err(error) => json!({"error":error.to_string()}),
        };
        let expected = if case.get("error").is_some() {
            json!({"error":case["error"]})
        } else {
            json!({"stored":case["stored"], "render":case["render"]})
        };
        if actual != expected {
            failures.push(json!({"case":case,"actual":actual}));
        }
    }
    println!(
        "R4 expiry render differential: {renders} renders; {roundtrips} roundtrips; {} mismatches",
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{:?}",
        &failures[..failures.len().min(8)]
    );
}

fn tokens(key: &str) {
    let inputs = inputs();
    let oracle = oracle();
    let mut failures = Vec::new();
    let cases = oracle[key].as_array().unwrap();
    for case in cases {
        let raw = inputs[key][case["input_index"].as_u64().unwrap() as usize]
            .as_str()
            .unwrap();
        let actual = match github_connections::json_body_params(
            &Method::POST,
            "/account/bots/1/github_connection",
            format!("{{\"access_token\":{raw}}}").as_bytes(),
        ) {
            Ok(params) => {
                json!({"string":input_casts::token_string(params.get("access_token").unwrap()).trim_matches(|c:char| c == '\0' || c.is_ascii_whitespace())})
            }
            Err(_) => json!({"error":true}),
        };
        let expected = if case.get("error").is_some() {
            json!({"error":true})
        } else {
            json!({"string":case["string"]})
        };
        if actual != expected {
            failures.push(json!({"raw":raw,"actual":actual,"expected":expected}));
        }
    }
    println!(
        "R4 {key} differential: {} compared; {} mismatches",
        cases.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{:?}",
        &failures[..failures.len().min(8)]
    );
}
#[test]
fn pr196_r4_extreme_exponents_match_rails() {
    tokens("tokens");
}
#[test]
fn pr196_r4_generated_exponent_boundaries_match_rails() {
    tokens("generated_tokens");
}

#[tokio::test]
async fn pr196_r4_http_save_then_list_boundary_expiries_matches_rails() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = t.david();
    browser.grant_sudo().await;
    let path = format!("/account/bots/{BENDER}/credentials");
    let mut failures = Vec::new();
    let oracle = oracle();
    let cases = oracle["save_list"].as_array().unwrap();
    for (index, case) in cases.iter().enumerate() {
        let zone = case["zone"].as_str().unwrap().to_owned();
        t.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE users SET time_zone=? WHERE id=?",
                    rusqlite::params![zone, DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let name = format!("R4 boundary {index}");
        let saved = browser
            .write(
                Req::new(Method::POST, &path)
                    .header("content-type", "application/json")
                    .body(
                        json!({"agent_credential":{"name":name,"expires_at":case["input"]}})
                            .to_string(),
                    ),
            )
            .await;
        let row_name = name.clone();
        let stored = t
            .db()
            .read(move |conn| {
                Ok(conn.query_row(
                    "SELECT expires_at FROM agent_credentials WHERE name=?",
                    [row_name],
                    |r| r.get::<_, String>(0),
                )?)
            })
            .await
            .unwrap();
        let listed = browser.get(&path).await;
        let body = listed.text();
        let tag = body
            .rsplit_once("· expires ")
            .and_then(|(_, tail)| tail.find("</time>").map(|end| tail[..end + 7].to_owned()));
        let actual = json!({"save_status":saved.status.as_u16(),"list_status":listed.status.as_u16(),"stored":stored,"expiry_tag":tag});
        let expected = json!({"save_status":case["save_status"],"list_status":case["list_status"],"stored":case["stored"],"expiry_tag":case["expiry_tag"]});
        if actual != expected {
            failures.push(json!({"case":case,"actual":actual}));
        }
        t.db()
            .write(move |tx| {
                tx.conn()
                    .execute("DELETE FROM agent_credentials WHERE name=?", [name])?;
                Ok(())
            })
            .await
            .unwrap();
    }
    println!(
        "R4 HTTP expiry save/list differential: {} compared; {} mismatches",
        cases.len(),
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{:?}",
        &failures[..failures.len().min(8)]
    );
}

#[tokio::test]
async fn pr196_r4_sudo_replay_numeric_hidden_fields_match_rails() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let path = format!("/account/bots/{BENDER}/github_connection?access_token=good");
    let mut failures = Vec::new();
    let oracle = oracle();
    let cases = oracle["sudo"].as_array().unwrap();
    for case in cases {
        let mut browser = t.david();
        let raw = case["raw"].as_str().unwrap();
        let challenged = browser
            .write(
                Req::new(Method::POST, &path)
                    .header("content-type", "application/json")
                    .body(format!("{{\"unused\":{raw}}}")),
            )
            .await;
        let replay = with_fixed_render_secrets(
            browser.write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")])),
        )
        .await;
        let body = replay.text();
        let form = body
            .find("data-controller=\"auto-submit\"")
            .and_then(|marker| body[..marker].rfind("<form "))
            .and_then(|start| {
                body[start..]
                    .find("</form>")
                    .map(|end| body[start..start + end + 7].to_owned())
            });
        let actual = json!({"challenge_status":challenged.status.as_u16(),"status":replay.status.as_u16(),"form":form});
        let expected = json!({"challenge_status":case["challenge_status"],"status":case["status"],"form":case["form"]});
        if actual != expected {
            failures.push(json!({"raw":raw,"actual":actual,"expected":expected}));
        }
    }
    println!(
        "R4 sudo replay HTML differential: {} compared; {} mismatches",
        cases.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:?}");
}

#[tokio::test]
async fn pr196_r4_overridden_routes_match_rails_and_keep_create_parser_scoped() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let mut browser = t.david();
    browser.grant_sudo().await;
    let path = format!("/account/bots/{BENDER}/github_connection");
    let mut failures = Vec::new();
    for case in oracle()["methods"].as_array().unwrap() {
        let method = case["override"].as_str().unwrap();
        let effective =
            Method::from_bytes(case["effective_method"].as_str().unwrap().as_bytes()).unwrap();
        let response = browser
            .write(
                Req::new(Method::POST, &path)
                    .header("content-type", "application/json")
                    .header("x-http-method-override", method)
                    .body("{\"access_token\":\"\"}"),
            )
            .await;
        let selected = github_connections::scoped_json_body_params(
            &effective,
            &path,
            b"{\"access_token\":1e999}",
        )
        .is_some();
        let expected_selected = case["endpoint"]["action"] == "create";
        if response.status.as_u16() != case["status"].as_u64().unwrap() as u16
            || selected != expected_selected
        {
            failures
                .push(json!({"case":case,"status":response.status.as_u16(),"selected":selected}));
        }
    }
    println!(
        "R4 effective method Rails differential: 7 compared; {} mismatches",
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:?}");
}
