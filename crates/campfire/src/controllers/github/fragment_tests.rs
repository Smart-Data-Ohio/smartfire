use crate::controllers::presenters::test_support::{BENDER, TestApp};

#[tokio::test]
async fn round2_profile_fragment_is_reachable_from_the_seed_page() {
    let app = TestApp::boot().await.expect("build default seed");
    let mut david = app.david();
    let page = david.get("/users/me/profile").await;
    assert_eq!(page.status, 200);
    assert!(
        page.text()
            .contains("aria-labelledby=\"github-connection-title\""),
        "GitHub profile fragment is absent"
    );
    assert!(page.text().contains("action=\"/github/connection\""));
    linked_page_states(&app, 127326141, "/users/me/profile").await;
}
#[tokio::test]
async fn round2_bot_fragment_is_reachable_from_the_seed_page() {
    let app = TestApp::boot().await.expect("build default seed");
    let mut david = app.david();
    let page = david.get(&format!("/account/bots/{BENDER}/edit")).await;
    assert_eq!(page.status, 200);
    assert!(
        page.text()
            .contains("aria-labelledby=\"github-connection-title\""),
        "GitHub bot fragment is absent"
    );
    assert!(page.text().contains(&format!(
        "action=\"/account/bots/{BENDER}/github_connection\""
    )));
    linked_page_states(&app, BENDER, &format!("/account/bots/{BENDER}/edit")).await;
}

async fn linked_page_states(app: &TestApp, user_id: i64, path: &str) {
    use crate::integrations::github::accounts::{Account, AccountInput};
    let crypto = rails_compat::ar_encryption::ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(move |tx| {
            Account::create(
                tx,
                &crypto,
                &AccountInput {
                    user_id,
                    github_login: "parity-user",
                    access_token: "fragment-fixture-token",
                    refresh_token: None,
                    token_expires_at: None,
                    token_source: "pat",
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut browser = app.david();
    let page = browser.get(path).await;
    assert_eq!(page.status, 200);
    assert!(page.text().contains("Connected as parity-user"));
    assert!(page.text().contains("Disconnect GitHub"));
    assert!(!page.text().contains("fragment-fixture-token"));
    app.db().write(move|tx| {
        tx.conn().execute("UPDATE github_connected_accounts SET disconnected_reason='Revoked <grant>&' WHERE user_id=?",[user_id])?;
        Ok(())
    }).await.unwrap();
    let page = browser.get(path).await;
    assert_eq!(page.status, 200);
    assert!(
        page.text()
            .contains("GitHub rejected the connection (Revoked &lt;grant&gt;&amp;)")
    );
    assert!(page.text().contains("Reconnect GitHub"));
    assert!(!page.text().contains("fragment-fixture-token"));
}

struct Tokens;
impl campfire_views::helpers::request_forgery::AuthenticityTokens for Tokens {
    fn global(&self) -> String {
        "GLOBAL".into()
    }
    fn for_form(&self, action: &str, method: &str) -> String {
        format!("{method}:{action}")
    }
}
#[tokio::test]
async fn round2_seed_fragments_match_rails_bytes_and_keep_private_cards_lazy() {
    use crate::controllers::presenters::{github, page::render_detached_at};
    use campfire_views::{
        github as view,
        helpers::request_forgery::{RequestSecrets, rendering_with},
    };
    let app = TestApp::boot().await.expect("build default seed");
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/github_seed_fragments.json"
    ))
    .unwrap();
    let thread_id = vectors["thread"]["thread_id"].as_i64().unwrap();
    let expected = vectors["thread"]["header"].as_str().unwrap().to_owned();
    let state = app.booted.app.clone();
    app.db()
        .read(move |conn| {
            let thread = campfire_db::ChannelThread::find(conn, thread_id)?;
            let html = render_detached_at(&state, None, "http://campfire.test", |ctx| {
                github::thread_header(conn, ctx, &thread)
            })?;
            assert_eq!(
                html, expected,
                "seed thread header through the page adapter"
            );
            Ok(())
        })
        .await
        .unwrap();
    let data = view::write_actions::WriteActions {
        thread_id,
        room_id: vectors["thread"]["room_id"].as_i64().unwrap(),
        pull_request_id: vectors["thread"]["pull_request_id"].as_i64().unwrap(),
        ..Default::default()
    };
    assert_eq!(
        data.render(),
        vectors["thread"]["write_frame"].as_str().unwrap()
    );
    let mut david = app.david();
    let response = david
        .get(&format!(
            "/rooms/{}/github/pull_request_write_actions/{}",
            data.room_id, data.pull_request_id
        ))
        .await;
    assert_eq!(response.status, 200);
    assert_eq!(
        response.text(),
        vectors["thread"]["write_frame"].as_str().unwrap()
    );
    for case in vectors["connections"].as_array().unwrap() {
        let mut data: view::connections::Connection =
            serde_json::from_value(case["data"].clone()).unwrap();
        data.app_configured = case["app_configured"].as_bool().unwrap();
        rendering_with(
            RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: None,
            },
            || {
                assert_eq!(
                    view::connections::profile(&data),
                    case["profile"].as_str().unwrap(),
                    "profile {}",
                    case["name"]
                );
                assert_eq!(
                    view::connections::bot(&data, BENDER, case["administrator"].as_bool().unwrap()),
                    case["bot"].as_str().unwrap(),
                    "bot {}",
                    case["name"]
                );
            },
        );
    }
    let id = vectors["thread"]["pull_request_id"].as_i64().unwrap();
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE github_pull_requests SET private=1, title='Private <secret>' WHERE id=?",
                [id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let state = app.booted.app.clone();
    app.db()
        .read(move |conn| {
            let thread = campfire_db::ChannelThread::find(conn, thread_id)?;
            let html = render_detached_at(&state, None, "http://campfire.test", |ctx| {
                github::thread_header(conn, ctx, &thread)
            })?;
            assert!(!html.contains("Private") && !html.contains("Port the launch checklist"));
            assert!(html.contains(&format!("thread_id={thread_id}")));
            assert!(html.contains("github_write_actions_channel_thread_8"));
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn round2_connection_fragments_load_real_credentials_without_rendering_tokens() {
    use crate::{
        controllers::presenters::github,
        integrations::github::accounts::{Account, AccountInput},
    };
    use campfire_views::{
        github::connections,
        helpers::request_forgery::{RequestSecrets, rendering_with},
    };
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/github_seed_fragments.json"
    ))
    .unwrap();
    for configured in [false, true] {
        let client = crate::integrations::github::client::AppClient::new(
            configured.then(|| "fixture-client".into()),
            configured.then(|| "fixture-secret".into()),
        );
        let app = TestApp::boot_with_github_app(client)
            .await
            .expect("build default seed");
        for case in vectors["connections"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["app_configured"] == configured)
        {
            let input = case["data"].clone();
            let crypto = rails_compat::ar_encryption::ArEncryption::new(&app.booted.app.secrets);
            app.db().write(move|tx| {
                for id in [127326141,BENDER] {
                    tx.conn().execute("DELETE FROM github_connected_accounts WHERE user_id=?",[id])?;
                    if input["linked"]==true {
                        let account=Account::create(tx,&crypto,&AccountInput{user_id:id,github_login:input["login"].as_str().unwrap(),access_token:"fragment-fixture-token",refresh_token:None,token_expires_at:None,token_source:if input["app_token"]==true {"app"} else {"pat"}})?;
                        if let Some(reason)=input["reason"].as_str(){tx.conn().execute("UPDATE github_connected_accounts SET disconnected_reason=? WHERE id=?",rusqlite::params![reason,account.id])?;}
                    }
                }
                Ok(())
            }).await.unwrap();
            let human = github::connection(&app.booted.app, 127326141)
                .await
                .unwrap();
            let bot = github::connection(&app.booted.app, BENDER).await.unwrap();
            rendering_with(
                RequestSecrets {
                    tokens: Box::new(Tokens),
                    csp_nonce: None,
                },
                || {
                    let profile = connections::profile(&human);
                    let bot =
                        connections::bot(&bot, BENDER, case["administrator"].as_bool().unwrap());
                    assert_eq!(
                        profile,
                        case["profile"].as_str().unwrap(),
                        "profile data caller {}",
                        case["name"]
                    );
                    assert_eq!(
                        bot,
                        case["bot"].as_str().unwrap(),
                        "bot data caller {}",
                        case["name"]
                    );
                    assert!(
                        !profile.contains("fragment-fixture-token")
                            && !bot.contains("fragment-fixture-token")
                    );
                },
            );
        }
    }
}

use campfire_views::rendering::*;
