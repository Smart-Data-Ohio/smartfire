//! Complete Rails recipient endpoint observations; real human/CSRF/membership checks.
use super::*;
use crate::controllers::presenters::test_support::{Browser, Reply};
fn observation(reply: &Reply) -> Value {
    let body = if reply.body.is_empty() {
        json!("")
    } else {
        serde_json::from_slice(&reply.body).unwrap_or_else(|_| json!(reply.text()))
    };
    json!({"status":reply.status.as_u16(),"body":body,"cache_control":reply.headers.get("cache-control").map(|v|v.to_str().unwrap())})
}
async fn get(browser: &mut Browser<'_>, path: &str) -> Reply {
    browser
        .send(Req::new(Method::GET, path).header("accept", "application/json"))
        .await
}
#[tokio::test]
async fn google_drive_recipient_requests_match_all_pinned_rails_observations() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_recipient_cases.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let (a, recorded) = app().await;
        let name = row["spec"]["name"].as_str().unwrap();
        let room = row["room_id"].as_i64().unwrap();
        let actor = row["actor_id"].as_i64().unwrap();
        a.booted.app.google.drive().install_picker(!matches!(
            name,
            "missing_key" | "missing_project" | "not_configured"
        ));
        let setup = row["setup"].clone();
        a.db().write(move |tx| {
            for (id,attrs) in setup["users"].as_object().unwrap() {
                let id=id.parse::<i64>().unwrap();
                for (key,value) in attrs.as_object().unwrap() {
                    assert!(matches!(key.as_str(),"status"|"name"|"email_address"));
                    let value=match value { Value::String(s)=>rusqlite::types::Value::Text(s.clone()),Value::Number(n)=>rusqlite::types::Value::Integer(n.as_i64().unwrap()),Value::Null=>rusqlite::types::Value::Null,_=>panic!("unexpected fixture attribute")};
                    tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"),rusqlite::params![value,id])?;
                }
            }
            if let Some(id)=setup["agent_user"].as_i64() {
                tx.conn().execute("INSERT INTO agents(user_id,owner_id,created_at,updated_at) VALUES(?,?,?,?)",rusqlite::params![id,DAVID,tx.now(),tx.now()])?;
            }
            Ok(())
        }).await.unwrap();
        let mut b = if matches!(
            name,
            "anonymous" | "anonymous_validate" | "bot_key" | "bot_key_validate"
        ) {
            a.anonymous()
        } else {
            a.sign_in(actor).await
        };
        let path = row["path"].as_str().unwrap();
        if name == "stale" {
            assert_eq!(
                observation(&get(&mut b, path.strip_suffix("/validate").unwrap()).await),
                row["before"],
                "{name}: preview"
            );
            let removed = row["setup"]["removed"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap())
                .collect::<Vec<_>>();
            a.db()
                .write(move |tx| {
                    for id in removed {
                        tx.conn().execute(
                            "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                            [room, id],
                        )?;
                    }
                    Ok(())
                })
                .await
                .unwrap();
        }
        if name.starts_with("throttle") {
            let base = path.strip_suffix("/validate").unwrap_or(path);
            let mut before = vec![];
            for _ in 0..60 {
                before.push(observation(&get(&mut b, base).await));
            }
            assert_eq!(
                json!(before),
                row["before"],
                "{name}: sixty allowed requests"
            );
        }
        let reply = if row["spec"]["method"] == "post" {
            let request = json_req(Method::POST, path, row["params"].clone());
            if name == "no_csrf" {
                b.send(request).await
            } else {
                b.write(request).await
            }
        } else {
            get(&mut b, path).await
        };
        assert_eq!(
            observation(&reply),
            row["result"],
            "{name}: complete recipient response"
        );
        if name.starts_with("throttle") {
            let mut other = a.sign_in(DAVID).await;
            assert_eq!(
                observation(&get(&mut other, path.strip_suffix("/validate").unwrap_or(path)).await),
                row["after"],
                "{name}: independent user budget"
            );
        }
        assert!(
            recorded.calls.lock().unwrap().is_empty(),
            "{name}: recipients never use Google HTTP"
        );
    }
    println!(
        "Pinned Rails Drive recipients: {} real request scenarios; 0 skipped; no Google HTTP",
        oracle["rows"].as_array().unwrap().len()
    );
}
