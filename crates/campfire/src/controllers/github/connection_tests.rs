use super::{
    card_tests::Fresh,
    test_support::{request, response_session, sudo},
};
use crate::integrations::{
    github::{
        accounts::{Account, AccountInput},
        tests::crypto,
    },
    test_support::Route,
};
use serde_json::{Value, json};
#[tokio::test]
async fn github_connections_security_enforces_sudo_admin_active_bot_and_single_use_state() {
    let fresh = Fresh::with_routes(
        &json!({"app_configured":true}),
        vec![Route::new("GET", "api.github.com", "/user", 200).body(r#"{"login":"octocat"}"#)],
    )
    .await;
    for (method, path) in [
        ("POST", "/github/connection"),
        ("DELETE", "/github/connection"),
        ("GET", "/github/app/connect"),
    ] {
        let (status, headers, _) = request(
            &fresh,
            method,
            path,
            json!({"access_token":"fixture-pasted"}),
            json!({}),
        )
        .await;
        assert_eq!(status, 302);
        assert_eq!(headers["location"], "http://example.org/sudo/new");
    }
    let (status, headers, _) = request(
        &fresh,
        "GET",
        "/github/app/callback?state=bogus&code=code",
        Value::Null,
        sudo(),
    )
    .await;
    assert_eq!(status, 302);
    assert_eq!(
        response_session(&fresh, &headers)["flash"]["flashes"]["alert"],
        "GitHub connection expired. Try again."
    );
    assert!(fresh.server.received().is_empty());
    let member = Fresh::with_routes(&json!({"role":0}), vec![]).await;
    for method in ["POST", "DELETE"] {
        assert_eq!(
            request(
                &member,
                method,
                "/account/bots/812/github_connection",
                json!({"access_token":"fixture-pasted"}),
                sudo()
            )
            .await
            .0,
            403
        );
    }
    assert_eq!(
        request(
            &fresh,
            "POST",
            "/account/bots/812/github_connection",
            json!({"access_token":"fixture-pasted"}),
            sudo()
        )
        .await
        .0,
        404
    );
}
#[tokio::test]
async fn github_connections_http_identity_flash_revocation_and_audits_match_rails() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/github_connections_http.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let configured = case["unconfigured"] != true;
        let mut routes = vec![
            Route::new(
                "GET",
                "api.github.com",
                "/user",
                case["user_status"].as_u64().unwrap_or(200) as u16,
            )
            .body(r#"{"login":"octocat"}"#),
            Route::new(
                "DELETE",
                "api.github.com",
                "/applications/fixture-client/token",
                204,
            ),
            Route::new(
                "DELETE",
                "api.github.com",
                "/applications/fixture-client/grant",
                204,
            ),
        ];
        let oauth = if case["refresh_error"] == true {
            r#"{}"#
        } else if case["oauth_error"] == true {
            r#"{"error":"denied"}"#
        } else if case["callback"] == true {
            r#"{"access_token":"fixture-app-token","refresh_token":"fixture-refresh","expires_in":28800}"#
        } else {
            r#"{"access_token":"fixture-fresh-token","refresh_token":"fixture-fresh-refresh","expires_in":28800}"#
        };
        routes.push(
            Route::new(
                "POST",
                "github.com",
                "/login/oauth/access_token",
                if case["refresh_error"] == true {
                    500
                } else {
                    200
                },
            )
            .body(oauth),
        );
        let fresh = Fresh::with_routes(&json!({"app_configured":configured}), routes).await;
        let input = case.clone();
        fresh.app.db.write(move|tx|{
   let now=tx.now();tx.conn().execute("INSERT INTO users(id,name,role,created_at,updated_at) VALUES(813,'Machine',2,?,?)",rusqlite::params![now,now])?;
   tx.conn().execute("INSERT INTO agents(id,user_id,owner_id,created_at,updated_at) VALUES(881,813,811,?,?)",rusqlite::params![now,now])?;
   tx.conn().execute("UPDATE users SET github_login=? WHERE id=811",[if input["name"]=="profile_replace"{Some("someone-else")}else{None}])?;
   tx.conn().execute("UPDATE users SET github_login=? WHERE id=812",[input["claimant"].as_str()])?;
   if let Some(source)=input["preset"].as_str(){let a=Account::create(tx,&crypto(),&AccountInput{user_id:if input["bot"]==true{813}else{811},github_login:"old-login",access_token:"fixture-old-token",refresh_token:(source=="app").then_some("fixture-old-refresh"),token_expires_at:(source=="app").then(||now.since(jiff::SignedDuration::from_secs(if input["expired"]==true{-60}else{3600}))),token_source:source})?;if input["initially_disconnected"]==true{Account::mark_disconnected(tx,a.id,"Disconnected")?;}}
   if input["claimant_verified"]==true{Account::create(tx,&crypto(),&AccountInput{user_id:812,github_login:"octocat",access_token:"fixture-other",refresh_token:None,token_expires_at:None,token_source:"pat"})?;}
   Ok(())
  }).await.unwrap();
        let bot = case["bot"] == true;
        let callback = case["callback"] == true;
        let mut values = sudo();
        values["github_app_oauth_state"] = json!(if case["wrong_session"] == true {
            "wrong"
        } else {
            "fixture-state"
        });
        let mut path = if bot {
            "/account/bots/813/github_connection"
        } else {
            "/github/connection"
        }
        .to_owned();
        if callback {
            let state = if case["bad_state"] == true {
                "bogus".into()
            } else {
                crate::integrations::github::oauth::sign_state(&fresh.app.secrets, "fixture-state")
            };
            let mut query = url::form_urlencoded::Serializer::new(String::new());
            query
                .append_pair("code", "fixture-code")
                .append_pair("state", &state);
            if let Some(error) = case["error"].as_str() {
                query.append_pair("error", error);
            }
            path = format!("/github/app/callback?{}", query.finish());
        }
        let (status, headers, body) = request(
            &fresh,
            if callback {
                "GET"
            } else {
                case["method"].as_str().unwrap_or("POST")
            },
            &path,
            case["request_body"].clone(),
            values,
        )
        .await;
        assert_eq!(status, case["status"], "{} {body}", case["name"]);
        assert_eq!(
            headers.get("location").and_then(|v| v.to_str().ok()),
            case["location"].as_str(),
            "{}",
            case["name"]
        );
        assert_eq!(
            response_session(&fresh, &headers)
                .get("flash")
                .and_then(|v| v.get("flashes"))
                .cloned()
                .unwrap_or(json!({})),
            case["flash"],
            "{}",
            case["name"]
        );
        if callback && configured {
            assert!(
                response_session(&fresh, &headers)
                    .get("github_app_oauth_state")
                    .is_none()
            );
        }
        let (account,profile,claimant,audits)=fresh.app.db.read(move|conn|{
   let a=Account::for_user(conn,if bot{813}else{811})?;
   let value=if let Some(a)=a{let (access,refresh)=conn.query_row("SELECT access_token,refresh_token FROM github_connected_accounts WHERE id=?",[a.id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?)))?;json!({"github_login":a.github_login,"token_source":a.token_source,"disconnected_reason":a.disconnected_reason,"last_error":a.last_error,"access_token":crypto().decrypt(&access).unwrap(),"refresh_token":refresh.map(|r|crypto().decrypt(&r).unwrap()),"token_expires_at":a.token_expires_at.map(|t|t.jiff().to_string())})}else{Value::Null};
   let p=conn.query_row("SELECT github_login FROM users WHERE id=811",[],|r|r.get::<_,Option<String>>(0))?;let q=conn.query_row("SELECT github_login FROM users WHERE id=812",[],|r|r.get::<_,Option<String>>(0))?;
   let audits=conn.prepare("SELECT action,actor_id,target_type,target_id,details FROM audit_logs ORDER BY id")?.query_map([],|r|Ok(json!({"action":r.get::<_,String>(0)?,"actor_id":r.get::<_,i64>(1)?,"target_type":r.get::<_,String>(2)?,"target_id":r.get::<_,i64>(3)?,"details":serde_json::from_str::<Value>(&r.get::<_,String>(4)?).unwrap()})))?.collect::<rusqlite::Result<Vec<_>>>()?;
   Ok((value,p,q,audits))
  }).await.unwrap();
        assert_eq!(account, case["account"], "{}", case["name"]);
        assert_eq!(json!(profile), case["profile"], "{}", case["name"]);
        assert_eq!(json!(claimant), case["claimant_login"], "{}", case["name"]);
        assert_eq!(json!(audits), case["audit"], "{}", case["name"]);
        let actual = fresh.server.received();
        assert_eq!(
            actual.len(),
            case["requests"].as_array().unwrap().len(),
            "{}",
            case["name"]
        );
        for (actual, expected) in actual.iter().zip(case["requests"].as_array().unwrap()) {
            assert_eq!(actual.method, expected[0]);
            assert_eq!(actual.target, expected[1]);
            if actual.method == "GET" {
                assert!(
                    actual
                        .headers
                        .iter()
                        .any(|(k, v)| k.eq_ignore_ascii_case("authorization")
                            && v == &format!("Bearer {}", expected[2].as_str().unwrap()))
                );
            }
            if actual.method == "DELETE" {
                assert_eq!(
                    serde_json::from_slice::<Value>(&actual.body).unwrap()["access_token"],
                    expected[2]
                );
            }
        }
    }
}
#[tokio::test]
async fn github_connections_oauth_state_round_trips_and_is_consumed_before_error_or_exchange() {
    let routes=vec![Route::new("POST","github.com","/login/oauth/access_token",200).body(r#"{"access_token":"fixture-app","refresh_token":"fixture-refresh","expires_in":28800}"#),Route::new("GET","api.github.com","/user",200).body(r#"{"login":"octocat"}"#)];
    let fresh = Fresh::with_routes(&json!({"app_configured":true}), routes).await;
    let (status, headers, _) =
        request(&fresh, "GET", "/github/app/connect", Value::Null, sudo()).await;
    assert_eq!(status, 302);
    let url = url::Url::parse(headers["location"].to_str().unwrap()).unwrap();
    assert_eq!(url.host_str(), Some("github.com"));
    let query: url::form_urlencoded::Parse<'_> =
        url::form_urlencoded::parse(url.query().unwrap().as_bytes());
    let query: std::collections::HashMap<_, _> = query.into_owned().collect();
    assert_eq!(query["scope"], "");
    assert_eq!(
        query["redirect_uri"],
        "http://example.org/github/app/callback"
    );
    let values = response_session(&fresh, &headers);
    let raw = values["github_app_oauth_state"].as_str().unwrap();
    assert_eq!(raw.len(), 32);
    assert!(crate::integrations::github::oauth::valid_state(
        &fresh.app.secrets,
        &query["state"],
        Some(&json!(raw)),
        fresh.app.clock.now()
    ));
    let path = format!(
        "/github/app/callback?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("state", &query["state"])
            .append_pair("code", "fixture-code")
            .finish()
    );
    let (status, headers, _) = request(&fresh, "GET", &path, Value::Null, values).await;
    assert_eq!(status, 302);
    let consumed = response_session(&fresh, &headers);
    assert!(consumed.get("github_app_oauth_state").is_none());
    assert_eq!(fresh.server.received().len(), 2);
    let (_, headers, _) = request(&fresh, "GET", &path, Value::Null, consumed).await;
    assert_eq!(
        response_session(&fresh, &headers)["flash"]["flashes"]["alert"],
        "GitHub connection expired. Try again."
    );
    assert_eq!(fresh.server.received().len(), 2);
    for path in ["/github/app/connect", "/github/app/callback"] {
        let unconfigured = Fresh::new(&json!({})).await;
        assert_eq!(
            request(&unconfigured, "GET", path, Value::Null, sudo())
                .await
                .0,
            404
        );
        assert!(unconfigured.server.received().is_empty());
    }
}
#[tokio::test]
async fn github_connections_link_audit_failure_rolls_back_credentials_and_verified_login() {
    let fresh = Fresh::with_routes(
        &json!({}),
        vec![Route::new("GET", "api.github.com", "/user", 200).body(r#"{"login":"octocat"}"#)],
    )
    .await;
    fresh.app.db.write(|tx|{tx.conn().execute_batch("CREATE TRIGGER reject_github_audit BEFORE INSERT ON audit_logs WHEN NEW.action='github.account.connect' BEGIN SELECT RAISE(ABORT,'audit rejected'); END;")?;Ok(())}).await.unwrap();
    assert_eq!(
        request(
            &fresh,
            "POST",
            "/github/connection",
            json!({"access_token":"fixture-pasted"}),
            sudo()
        )
        .await
        .0,
        500
    );
    fresh
        .app
        .db
        .read(|conn| {
            assert!(Account::for_user(conn, 811)?.is_none());
            assert!(
                conn.query_row("SELECT github_login FROM users WHERE id=811", [], |r| {
                    r.get::<_, Option<String>>(0)
                })?
                .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
}

mod ui_return;
