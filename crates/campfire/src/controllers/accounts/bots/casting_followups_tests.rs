//! PR196 Casting follow-ups boundaries, captured from pinned Rails by render_replay.sh.
use super::input_casts;
use crate::controllers::presenters::test_support::{
    BENDER, DAVID, Req, TestApp, with_fixed_render_secrets,
};
use campfire_db::Timestamp;
use campfire_kit::{Method, Param};
use campfire_presentation::time::Zone;
use serde_json::{Value, json};

fn inputs() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../test-support/agents_ui/casting_followups_inputs.json"
    ))
    .unwrap()
}
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/bot-ui-casting-followups.json"
    ))
    .unwrap()
}

#[test]
fn ws11ui_casting_followups_boundary_expiry_rendering_matches_rails() {
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
        "Casting follow-ups expiry render differential: {renders} renders; {roundtrips} roundtrips; {} mismatches",
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{:?}",
        &failures[..failures.len().min(8)]
    );
}

#[tokio::test]
async fn ws11ui_casting_followups_http_save_boundary_expiries_matches_rails() {
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
        let name = format!("Casting follow-ups boundary {index}");
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
        assert_eq!(saved.status.as_u16(), 201);
        assert!(saved.json()["secret"].as_str().is_some());
        let actual = json!({"stored":stored});
        let expected = json!({"stored":case["stored"]});
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
        "Casting follow-ups HTTP expiry save differential: {} compared; {} mismatches",
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
async fn ws11ui_casting_followups_sudo_replay_numeric_hidden_fields_match_rails() {
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
        // The replayed form's contract (action, method, fields, values), not its markup.
        let contract = |html: Option<&str>| {
            format!("{:?}", crate::form_contracts::forms(html.unwrap_or_default()))
        };
        let actual = json!({"challenge_status":challenged.status.as_u16(),"status":replay.status.as_u16(),"form":contract(form.as_deref())});
        let expected = json!({"challenge_status":case["challenge_status"],"status":case["status"],"form":contract(case["form"].as_str())});
        if actual != expected {
            failures.push(json!({"raw":raw,"actual":actual,"expected":expected}));
        }
    }
    println!(
        "Casting follow-ups sudo replay HTML differential: {} compared; {} mismatches",
        cases.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn ws11ui_casting_followups_integer_replay_size_counts_digits_without_quotes() {
    for case in oracle()["size"].as_array().unwrap() {
        let digits = case["digits"].as_u64().unwrap() as usize;
        let raw = "1".repeat(digits);
        let params = crate::server::json_params::json_body_params(
            &Method::POST,
            "/account/bots/1/github_connection",
            format!("{{\"unused\":{raw}}}").as_bytes(),
        )
        .unwrap();
        let stored = crate::concerns::session_keys::sudo_storable_params("POST", &params);
        assert_eq!(
            stored.is_some(),
            case["storable"].as_bool().unwrap(),
            "{digits} digits, Rails {} bytes",
            case["bytes"]
        );
    }
    // A quoted numeric string still pays its JSON quotes; only exact Integers omit them.
    let raw = "1".repeat(2036);
    let params = crate::server::json_params::json_body_params(
        &Method::POST,
        "/account/bots/1/github_connection",
        format!("{{\"unused\":\"{raw}\"}}").as_bytes(),
    )
    .unwrap();
    assert!(crate::concerns::session_keys::sudo_storable_params("POST", &params).is_none());
}

#[tokio::test]
async fn ws11ui_casting_followups_huge_integer_challenge_keeps_approved_302() {
    let t = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    for case in oracle()["size"].as_array().unwrap().iter().take(2) {
        assert_eq!(case["status"], 500, "pinned Rails cookie overflow");
        let raw = "1".repeat(case["digits"].as_u64().unwrap() as usize);
        let mut browser = t.david();
        let response = browser
            .write(
                Req::new(
                    Method::POST,
                    &format!("/account/bots/{BENDER}/github_connection?access_token=good"),
                )
                .header("content-type", "application/json")
                .body(format!("{{\"unused\":{raw}}}")),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            302,
            "approved fail-closed oversized-cookie handling"
        );
    }
}
