//! The retained join forms and the SPA's JSON join endpoints over the same frozen seed: the same
//! rows, invite uses, session and cookies, and the same refusals.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::User;
use campfire_db::models::workspace_invite::{InviteExpiry, WorkspaceInvite};
use serde_json::{Value, json};

const AVATAR: &[u8] = include_bytes!("../../../../../fixtures/files/workspace_icons/square_64.png");

fn join_code() -> String {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../../vectors/users_joining.json")).unwrap();
    vectors["join"].as_str().unwrap().to_owned()
}

async fn app() -> TestApp {
    TestApp::boot_frozen()
        .await
        .expect("seed required")
        .without_job_runner()
        .await
}

async fn invite(app: &TestApp, uses: Option<i64>) -> (WorkspaceInvite, String) {
    app.db()
        .write(move |tx| WorkspaceInvite::create(tx, DAVID, InviteExpiry::OneHour, uses))
        .await
        .unwrap()
}

/// The retained form's fields, or the JSON endpoint's.
fn signup(json: bool, path: &str, name: &str, email: &str) -> Req {
    let fields: [(&str, &str); 4] = if json {
        [
            ("name", name),
            ("emailAddress", email),
            ("password", "secret123456"),
            ("role", "administrator"),
        ]
    } else {
        [
            ("user[name]", name),
            ("user[email_address]", email),
            ("user[password]", "secret123456"),
            ("user[role]", "administrator"),
        ]
    };
    let avatar = if json { "avatar" } else { "user[avatar]" };
    let request = Req::new(Method::POST, path).multipart(&fields, (avatar, "me.png", "image/png", AVATAR));
    if json { request.header("accept", "application/json") } else { request }
}

fn json_get(path: &str) -> Req {
    Req::new(Method::GET, path).header("accept", "application/json")
}

fn cookie_contract(reply: &Reply) -> Vec<String> {
    let mut cookies = reply
        .headers
        .get_all("set-cookie")
        .iter()
        .map(|value| {
            let value = value.to_str().unwrap();
            let (pair, attributes) = value.split_once(';').unwrap_or((value, ""));
            let (name, token) = pair.split_once('=').unwrap();
            format!("{name}={} ;{attributes}", if token.is_empty() { "deleted" } else { "set" })
        })
        .collect::<Vec<_>>();
    cookies.sort();
    cookies
}

/// Everything a signup wrote for `email`.
async fn enrollment(app: &TestApp, email: &'static str) -> Value {
    app.db()
        .read(move |c| {
            let row = c.query_row(
                "SELECT id,name,email_address,role,status,password_digest IS NOT NULL FROM users WHERE email_address=?",
                [email],
                |r| Ok((r.get::<_, i64>(0)?, json!([r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, i64>(3)?, r.get::<_, i64>(4)?, r.get::<_, bool>(5)?]))),
            )?;
            let (id, user) = row;
            let rooms = c
                .prepare("SELECT room_id,involvement FROM memberships WHERE user_id=? ORDER BY room_id")?
                .query_map([id], |r| Ok(json!([r.get::<_, i64>(0)?, r.get::<_, String>(1)?])))?
                .collect::<Result<Vec<_>, _>>()?;
            let sessions = c
                .prepare("SELECT user_agent,ip_address,two_factor_verified_at FROM sessions WHERE user_id=?")?
                .query_map([id], |r| Ok(json!([r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?])))?
                .collect::<Result<Vec<_>, _>>()?;
            let avatar = c
                .prepare("SELECT b.filename,b.content_type,b.byte_size FROM active_storage_attachments a JOIN active_storage_blobs b ON b.id=a.blob_id WHERE a.record_type='User' AND a.record_id=? AND a.name='avatar'")?
                .query_map([id], |r| Ok(json!([r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, i64>(2)?])))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"id":id,"user":user,"rooms":rooms,"sessions":sessions,"avatar":avatar,"users":User::count(c)?}))
        })
        .await
        .unwrap()
}

/// The new browser's cookie authenticates the new user (who still has to set up two-step sign-in).
async fn assert_signed_in_as(browser: &mut Browser<'_>, id: i64) {
    let page = browser.get("/two_factor_setup").await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.text().contains(&format!("name=\"current-user-id\" content=\"{id}\"")));
}

#[tokio::test]
async fn join_code_html_and_json_enroll_identically() {
    let code = join_code();
    let mut outcomes = Vec::new();
    for json in [false, true] {
        let app = app().await;
        let mut browser = app.anonymous();
        let path = if json { format!("/api/v1/join/{code}") } else { format!("/join/{code}") };
        let page = if json { browser.send(json_get(&path)).await } else { browser.get(&path).await };
        assert_eq!(page.status, StatusCode::OK);
        let reply = browser.write(signup(json, &path, "New Person", "new@37signals.com")).await;
        let location = if json {
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            assert_eq!(reply.json()["kind"], "signedIn");
            reply.json()["location"].as_str().unwrap().to_owned()
        } else {
            assert_eq!(reply.status, StatusCode::FOUND);
            reply.location().unwrap().to_owned()
        };
        let facts = enrollment(&app, "new@37signals.com").await;
        assert_signed_in_as(&mut browser, facts["id"].as_i64().unwrap()).await;
        outcomes.push(json!({"location":location,"cookies":cookie_contract(&reply),"facts":facts}));
    }
    assert_eq!(outcomes[0], outcomes[1]);
    assert_eq!(outcomes[0]["location"], "http://campfire.test/");
    // A member, never the requested administrator, with the avatar and a session.
    assert_eq!(outcomes[0]["facts"]["user"][2], 0);
    assert_eq!(outcomes[0]["facts"]["avatar"][0][0], "me.png");
    assert_eq!(outcomes[0]["facts"]["sessions"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn invite_html_and_json_enroll_identically_and_count_one_use() {
    let mut outcomes = Vec::new();
    for json in [false, true] {
        let app = app().await;
        let (invite, token) = invite(&app, Some(5)).await;
        let mut browser = app.anonymous();
        let path = if json { format!("/api/v1/invite/{token}") } else { format!("/invite/{token}") };
        let reply = browser.write(signup(json, &path, "Invited", "invited@example.com")).await;
        let location = if json {
            assert_eq!(reply.json(), json!({"kind":"signedIn","location":"http://campfire.test/"}));
            "http://campfire.test/".to_owned()
        } else {
            reply.location().unwrap().to_owned()
        };
        let facts = enrollment(&app, "invited@example.com").await;
        assert_signed_in_as(&mut browser, facts["id"].as_i64().unwrap()).await;
        let uses = app.db().read(move |c| WorkspaceInvite::find(c, invite.id)).await.unwrap().unwrap().uses;
        outcomes.push(json!({"location":location,"cookies":cookie_contract(&reply),"facts":facts,"uses":uses}));
    }
    assert_eq!(outcomes[0], outcomes[1]);
    assert_eq!(outcomes[0]["uses"], 1);
}

#[tokio::test]
async fn json_join_pages_describe_the_workspace_and_keep_the_retained_gates() {
    let app = app().await;
    let code = join_code();
    let mut browser = app.anonymous();
    let (account_name, help) = app
        .db()
        .read(|c| {
            Ok((
                campfire_db::Account::first(c)?.unwrap().name,
                crate::controllers::presenters::accounts::help_contact(c)?.unwrap(),
            ))
        })
        .await
        .unwrap();
    let page = browser.send(json_get(&format!("/api/v1/join/{code}"))).await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(page.headers["cache-control"], "no-store");
    assert_eq!(
        page.json(),
        json!({"kind":"join","workspace":{"name":account_name,"logoUrl":null,"description":""},"helpContact":{"name":help.name,"emailAddress":help.email_address}})
    );
    let (_, token) = invite(&app, None).await;
    let page = browser.send(json_get(&format!("/api/v1/invite/{token}"))).await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(page.json()["kind"], "join");

    // A wrong join code: the retained empty 404, as an empty refusal; nothing is created.
    let before = app.db().read(User::count).await.unwrap();
    let wrong = browser.send(json_get("/api/v1/join/wrong-code")).await;
    assert_eq!((wrong.status, wrong.json()), (StatusCode::NOT_FOUND, json!({"kind":"error","fieldErrors":{}})));
    let wrong = browser.write(signup(true, "/api/v1/join/wrong-code", "Must not save", "nobody@example.com")).await;
    assert_eq!((wrong.status, wrong.json()), (StatusCode::NOT_FOUND, json!({"kind":"error","fieldErrors":{}})));
    assert_eq!(app.db().read(User::count).await.unwrap(), before);

    // The SPA shells keep the retained statuses.
    assert_eq!(browser.get(&format!("/app/join/{code}")).await.status, StatusCode::OK);
    assert_eq!(browser.get("/app/join/wrong-code").await.status, StatusCode::NOT_FOUND);
    assert_eq!(browser.get(&format!("/app/invite/{token}")).await.status, StatusCode::OK);
    assert_eq!(browser.get("/app/invite/unknown").await.status, StatusCode::NOT_FOUND);
    let shell = browser.get(&format!("/app/join/{code}")).await;
    assert!(shell.text().contains(r#"id="boot""#));
    assert!(shell.text().contains(r#""kind":"signedOut""#));

    // Signed in, every one of them goes to the root, as the retained pages do.
    let mut david = app.david();
    for path in [format!("/join/{code}"), format!("/app/join/{code}"), format!("/app/invite/{token}")] {
        assert_eq!(david.get(&path).await.location(), Some("http://campfire.test/"), "{path}");
    }
    for path in [format!("/api/v1/join/{code}"), format!("/api/v1/invite/{token}")] {
        assert_eq!(david.send(json_get(&path)).await.json(), json!({"kind":"navigate","location":"http://campfire.test/"}), "{path}");
        let post = david.write(signup(true, &path, "Signed in", "signed-in@example.com")).await;
        assert_eq!(post.json(), json!({"kind":"navigate","location":"http://campfire.test/"}), "{path}");
    }
    assert_eq!(app.db().read(User::count).await.unwrap(), before);
}

#[tokio::test]
async fn json_invite_refusals_match_the_retained_reasons_and_statuses() {
    let app = app().await;
    let before = app.db().read(User::count).await.unwrap();
    for (state, refusal, reason, status) in [
        ("expired", "expired", "It has expired.", StatusCode::GONE),
        ("exhausted", "exhausted", "All its uses have been taken.", StatusCode::GONE),
        ("revoked", "revoked", "It has been revoked.", StatusCode::GONE),
        ("unknown", "unknown", "The invite could not be found.", StatusCode::NOT_FOUND),
    ] {
        let (invite, token) = invite(&app, Some(1)).await;
        let id = invite.id;
        app.db()
            .write(move |tx| {
                match state {
                    "expired" => {
                        tx.conn().execute("UPDATE workspace_invites SET expires_at=? WHERE id=?", rusqlite::params![tx.now(), id])?;
                    }
                    "exhausted" => {
                        tx.conn().execute("UPDATE workspace_invites SET uses=max_uses WHERE id=?", [id])?;
                    }
                    "revoked" => {
                        WorkspaceInvite::revoke(tx, id)?;
                    }
                    _ => {}
                }
                Ok(())
            })
            .await
            .unwrap();
        let token = if state == "unknown" { "unknown".to_owned() } else { token };
        let mut browser = app.anonymous();
        let html = browser.get(&format!("/invite/{token}")).await;
        assert_eq!(html.status, status);
        assert!(html.text().contains(reason), "{state}");
        let path = format!("/api/v1/invite/{token}");
        for reply in [
            browser.send(json_get(&path)).await,
            browser.write(signup(true, &path, "Must not create", "dead@example.com")).await,
        ] {
            assert_eq!(reply.status, status, "{state}");
            assert_eq!(reply.json()["kind"], "inviteInvalid");
            assert_eq!(reply.json()["refusal"], refusal);
            assert_eq!(reply.json()["reason"], reason);
            // As the retained page, without the workspace's description.
            assert_eq!(reply.json()["workspace"]["description"], "");
        }
        assert_eq!(browser.get(&format!("/app/invite/{token}")).await.status, status, "{state}");
    }
    assert_eq!(app.db().read(User::count).await.unwrap(), before);
}

#[tokio::test]
async fn json_join_refusals_keep_the_retained_duplicate_redirect_body_and_forgery_checks() {
    let app = app().await;
    let code = join_code();
    let (invite, token) = invite(&app, Some(1)).await;
    let before = app.db().read(User::count).await.unwrap();
    let mut browser = app.anonymous();
    for path in [format!("/api/v1/join/{code}"), format!("/api/v1/invite/{token}")] {
        // An existing address goes to sign-in with it filled in; the invite keeps its use.
        let duplicate = browser.write(signup(true, &path, "Another David", "david@37signals.com")).await;
        assert_eq!(
            (duplicate.status, duplicate.json()),
            (StatusCode::OK, json!({"kind":"navigate","location":"http://campfire.test/session/new?email_address=david%4037signals.com"}))
        );
        // A body without the join fields.
        let malformed = browser
            .write(Req::new(Method::POST, &path).header("accept", "application/json").multipart(&[("name", "Half")], ("avatar", "me.png", "image/png", AVATAR)))
            .await;
        assert_eq!(
            (malformed.status, malformed.json()),
            (StatusCode::UNPROCESSABLE_ENTITY, json!({"kind":"error","fieldErrors":{"base":["The request body isn't valid."]}}))
        );
        // Without the authenticity token, as the retained form.
        let forged = browser.send(signup(true, &path, "Forged", "forged@example.com")).await;
        assert_eq!(forged.status, StatusCode::UNPROCESSABLE_ENTITY);
        let forged = browser
            .send(signup(false, &path.replace("/api/v1/join/", "/join/").replace("/api/v1/invite/", "/invite/"), "Forged", "forged@example.com"))
            .await;
        assert_eq!(forged.status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    assert_eq!(app.db().read(User::count).await.unwrap(), before);
    assert_eq!(app.db().read(move |c| WorkspaceInvite::find(c, invite.id)).await.unwrap().unwrap().uses, 0);

    // A JSON body joins too, without an avatar, and its authenticity token can ride in the body.
    let token_value = browser.authenticity_token().await;
    let reply = browser
        .send(
            Req::new(Method::POST, &format!("/api/v1/join/{code}"))
                .header("accept", "application/json")
                .header("content-type", "application/json")
                .body(json!({"name":"Body","emailAddress":"body@example.com","password":"secret123456","authenticity_token":token_value}).to_string()),
        )
        .await;
    assert_eq!(reply.json(), json!({"kind":"signedIn","location":"http://campfire.test/"}));
    let facts = enrollment(&app, "body@example.com").await;
    assert_eq!(facts["avatar"], json!([]));
    assert_eq!(facts["users"], before + 1);
}
