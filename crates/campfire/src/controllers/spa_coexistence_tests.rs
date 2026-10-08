//! Both UIs side by side (plan §5.0) over the seeded app: a person's UI choice, the classic pages
//! the SPA has ported redirecting there for people who use it, `?classic=1`, the profile's switch,
//! and the `SPA_ENABLED` / `SPA_DEFAULT` gates.

use axum::http::{Method, StatusCode};
use campfire_db::models::user::ui_preference::{self, UiPreference};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, Room};

use crate::controllers::presenters::test_support::{
    Browser, DAVID, JASON, KEVIN, Reply, Req, TestApp, seed_clock,
};

const ORIGIN: &str = "http://campfire.test";

async fn app(env: &[(&str, &str)]) -> Option<TestApp> {
    TestApp::boot_seed_with_env("default", seed_clock(), env).await
}

async fn enabled() -> Option<TestApp> {
    app(&[("SPA_ENABLED", "1")]).await
}

async fn choose(a: &TestApp, user: i64, preference: UiPreference) {
    a.db()
        .write(move |tx| ui_preference::store(tx, user, preference))
        .await
        .unwrap();
}

async fn stored(a: &TestApp, user: i64) -> Option<UiPreference> {
    a.db()
        .read(move |conn| ui_preference::stored(conn, user))
        .await
        .unwrap()
}

/// A room David belongs to.
async fn room(a: &TestApp) -> i64 {
    a.db()
        .read(|conn| {
            Ok(Room::for_user(conn, DAVID)?
                .first()
                .expect("David has rooms")
                .id)
        })
        .await
        .unwrap()
}

async fn message(a: &TestApp, room_id: i64, creator_id: i64, content: &str) -> Message {
    let content = content.to_string();
    a.db()
        .write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id,
                    creator_id,
                    markdown_source: Some(content),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap()
}

fn gap_pages(room: i64, message: i64) -> [(String, String, StatusCode); 10] {
    [
        (
            format!("/rooms/{room}/messages/{message}"),
            format!("/app/r/{room}/m/{message}"),
            StatusCode::OK,
        ),
        (
            format!("/rooms/{room}/messages/{message}/edit"),
            format!("/app/r/{room}/m/{message}"),
            StatusCode::OK,
        ),
        // The bare classic actions have no room_id; keep their existing 404 on opt-out.
        (
            format!("/messages/{message}"),
            format!("/app/m/{message}"),
            StatusCode::NOT_FOUND,
        ),
        (
            format!("/messages/{message}/edit"),
            format!("/app/m/{message}"),
            StatusCode::NOT_FOUND,
        ),
        (
            format!("/messages/{message}/boosts"),
            format!("/app/m/{message}"),
            StatusCode::OK,
        ),
        (
            format!("/messages/{message}/boosts/new"),
            format!("/app/m/{message}"),
            StatusCode::OK,
        ),
        (
            format!("/rooms/{room}/threads"),
            format!("/app/r/{room}/threads"),
            StatusCode::OK,
        ),
        (
            format!("/rooms/{room}/files"),
            format!("/app/r/{room}/files"),
            StatusCode::OK,
        ),
        (
            format!("/rooms/{room}/pins"),
            format!("/app/r/{room}/pins"),
            StatusCode::OK,
        ),
        (
            format!("/rooms/{room}/involvement"),
            format!("/app/r/{room}/notifications"),
            StatusCode::OK,
        ),
    ]
}

fn to(path: &str) -> String {
    format!("{ORIGIN}{path}")
}

fn redirected_to_spa(reply: &Reply) -> bool {
    reply
        .location()
        .is_some_and(|location| location.starts_with(&to("/app")))
}

/// `POST /app/ui_preference` as the profile's form sends it, with the session's token.
async fn post_ui(b: &mut Browser<'_>, pairs: &[(&str, &str)]) -> Reply {
    b.write(Req::new(Method::POST, "/app/ui_preference").form(pairs))
        .await
}

#[tokio::test]
async fn ported_pages_send_people_who_use_the_new_ui_to_the_spa() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
    let posted = message(&a, room, DAVID, "anchor").await;
    choose(&a, DAVID, UiPreference::Next).await;
    let mut david = a.sign_in(DAVID).await;
    for (classic, spa) in [
        ("/".to_string(), "/app/".to_string()),
        (format!("/rooms/{room}"), format!("/app/r/{room}")),
        (
            format!("/rooms/{room}/@12345"),
            format!("/app/r/{room}/m/12345"),
        ),
        (
            format!("/rooms/{room}/threads/77"),
            format!("/app/r/{room}/t/77"),
        ),
        (
            format!("/rooms/{room}?message_id={}", posted.id),
            format!("/app/r/{room}/m/{}", posted.id),
        ),
        (
            format!("/rooms/{room}/events"),
            format!("/app/r/{room}/events"),
        ),
    ] {
        let reply = david.get(&classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(reply.location(), Some(to(&spa).as_str()), "{classic}");
    }
    let head = david
        .send(Req::new(Method::HEAD, &format!("/rooms/{room}")))
        .await;
    assert_eq!(
        head.location(),
        Some(to(&format!("/app/r/{room}")).as_str())
    );

    // Jason hasn't chosen, so he stays on the classic page.
    let mut jason = a.sign_in(JASON).await;
    let page = jason.get(&format!("/rooms/{room}")).await;
    assert!(!redirected_to_spa(&page), "{:?}", page.location());
}

// The composer opens the `/event` command's classic URL in a new tab. For someone on the new
// UI that lands on the SPA's new-event form with the command's query intact: Rails' sorted,
// form-encoded `event[...]` keys, of which the form's prefill (`newEventPrefill`) reads
// `event[title]` and `event[starts_at]` (an ISO time in UTC).
#[tokio::test]
async fn the_event_commands_link_opens_the_spa_form_with_its_prefill() {
    let a = enabled()
        .await
        .expect("the default frozen seed is required");
    let room = crate::controllers::presenters::test_support::ALL_TALK;
    choose(&a, DAVID, UiPreference::Next).await;
    let mut david = a.sign_in(DAVID).await;
    let reply = david
        .write(super::api_tests::json_body(
            Method::POST,
            &format!("/api/v1/rooms/{room}/slash_commands"),
            &serde_json::json!({"text": "/event Launch party tomorrow at 3pm", "threadId": null}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let campfire_api_types::SlashCommandResult::OpenUrl { url } = super::api_tests::parse(&reply)
    else {
        panic!("/event opens a URL: {}", reply.text())
    };
    let query = url
        .strip_prefix(&format!("/rooms/{room}/events/new?"))
        .unwrap_or_else(|| panic!("the classic new-event URL with a query: {url}"));

    let classic = david.get(&url).await;
    assert_eq!(classic.status, StatusCode::FOUND, "{url}");
    assert_eq!(
        classic.location(),
        Some(to(&format!("/app/r/{room}/events/new?{query}")).as_str())
    );
    let pairs: Vec<(&str, &str)> = query
        .split('&')
        .map(|pair| pair.split_once('=').expect("a key and a value"))
        .collect();
    let keys: Vec<&str> = pairs.iter().map(|(key, _)| *key).collect();
    assert_eq!(
        keys,
        [
            "event%5Bstarts_at%5D",
            "event%5Btime_zone%5D",
            "event%5Btitle%5D"
        ]
    );
    assert_eq!(pairs[2].1, "Launch+party");
    let starts_at = pairs[0].1.replace("%3A", ":");
    assert!(
        starts_at.len() == "2026-03-03T20:00:00Z".len()
            && starts_at.ends_with('Z')
            && starts_at.as_bytes()[10] == b'T',
        "{starts_at}"
    );
}

#[tokio::test]
async fn message_aliases_and_room_tools_redirect_to_the_new_ui() {
    let a = enabled()
        .await
        .expect("the default frozen seed is required");
    let room = crate::controllers::presenters::test_support::ALL_TALK;
    let message = message(&a, room, DAVID, "coexistence gap message").await;
    choose(&a, DAVID, UiPreference::Next).await;
    let mut david = a.sign_in(DAVID).await;
    for (classic, spa, _) in gap_pages(room, message.id) {
        let reply = david.get(&classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(reply.location(), Some(to(&spa).as_str()), "{classic}");
    }
}

#[tokio::test]
async fn message_aliases_and_room_tools_keep_the_classic_behavior_on_opt_out() {
    let a = enabled()
        .await
        .expect("the default frozen seed is required");
    let room = crate::controllers::presenters::test_support::ALL_TALK;
    let message = message(&a, room, DAVID, "coexistence gap message").await;
    let mut david = a.sign_in(DAVID).await;
    for preference in [UiPreference::Classic, UiPreference::Next] {
        choose(&a, DAVID, preference).await;
        for (classic, _, status) in gap_pages(room, message.id) {
            let path = if preference == UiPreference::Next {
                format!("{classic}?classic=1")
            } else {
                classic.clone()
            };
            let reply = david.get(&path).await;
            assert_eq!(reply.status, status, "{path}");
            assert!(!redirected_to_spa(&reply), "{path}");
            assert!(
                reply
                    .content_type()
                    .is_some_and(|kind| kind.starts_with("text/html")),
                "{path}"
            );
            if classic == format!("/rooms/{room}/messages/{}", message.id) {
                assert!(reply.text().contains("coexistence gap message"), "{path}");
            }
        }
    }
}

#[tokio::test]
async fn message_aliases_and_room_tools_never_reveal_another_rooms_content() {
    use campfire_api_types as api;

    use super::api_tests::{get, parse};
    use crate::controllers::presenters::test_support::{DIRECT_KEVIN_BENDER, KEVIN};

    const SECRET: &str = "private coexistence gap message";
    let a = enabled()
        .await
        .expect("the default frozen seed is required");
    let message = message(&a, DIRECT_KEVIN_BENDER, KEVIN, SECRET).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let member_read = kevin
        .send(get(&format!("/api/v1/messages/{}", message.id)))
        .await;
    assert_eq!(member_read.status, StatusCode::OK, "{}", member_read.text());
    let read: api::MessageRead = parse(&member_read);
    assert_eq!(read.message.id, message.id);
    assert_eq!(
        (read.conversation.room_id, read.conversation.thread_id),
        (DIRECT_KEVIN_BENDER, None)
    );
    assert!(read.message.body_html.contains(SECRET));

    let mut david = a.sign_in(DAVID).await;
    for preference in [UiPreference::Classic, UiPreference::Next] {
        choose(&a, DAVID, preference).await;
        for (classic, spa, _) in gap_pages(DIRECT_KEVIN_BENDER, message.id) {
            let reply = david.get(&classic).await;
            if preference == UiPreference::Next {
                assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
                assert_eq!(reply.location(), Some(to(&spa).as_str()), "{classic}");
            } else {
                assert_eq!(reply.status, StatusCode::NOT_FOUND, "{classic}");
            }
            assert!(!reply.text().contains(SECRET), "{classic}");
            let bypass = david.get(&format!("{classic}?classic=1")).await;
            assert_eq!(bypass.status, StatusCode::NOT_FOUND, "{classic}");
            assert!(!bypass.text().contains(SECRET), "{classic}");
        }
    }
    // After redirecting, the resolver and room tools still authorize their data reads.
    for path in [
        format!("/api/v1/messages/{}", message.id),
        "/api/v1/messages/999999999".to_string(),
        format!("/api/v1/rooms/{DIRECT_KEVIN_BENDER}"),
        format!("/api/v1/rooms/{DIRECT_KEVIN_BENDER}/threads"),
        format!("/api/v1/rooms/{DIRECT_KEVIN_BENDER}/files"),
        format!("/api/v1/rooms/{DIRECT_KEVIN_BENDER}/pins"),
    ] {
        let reply = david.send(get(&path)).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        let envelope: api::ApiErrorResponse = parse(&reply);
        assert!(
            matches!(envelope.error, api::ApiError::NotFound { .. }),
            "{path}"
        );
        assert!(!reply.text().contains(SECRET), "{path}");
    }
    let mut anonymous = a.anonymous();
    for (classic, _, _) in gap_pages(DIRECT_KEVIN_BENDER, message.id) {
        let reply = anonymous.get(&classic).await;
        assert_eq!(
            reply.location(),
            Some(to("/session/new").as_str()),
            "{classic}"
        );
        assert!(!reply.text().contains(SECRET), "{classic}");
    }
}

#[tokio::test]
async fn classic_1_keeps_them_on_the_classic_page() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
    choose(&a, DAVID, UiPreference::Next).await;
    let mut b = a.sign_in(DAVID).await;
    let page = b.get(&format!("/rooms/{room}?classic=1")).await;
    assert_eq!(
        (page.status, page.content_type()),
        (StatusCode::OK, Some("text/html; charset=utf-8"))
    );
    assert!(redirected_to_spa(
        &b.get(&format!("/rooms/{room}?classic=0")).await
    ));
}

#[tokio::test]
async fn only_html_navigations_of_ported_pages_redirect() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
    choose(&a, DAVID, UiPreference::Next).await;
    let mut b = a.sign_in(DAVID).await;
    let room_path = format!("/rooms/{room}");
    let requests = [
        // Script fetches: JSON, a Turbo frame, XHR.
        Req::new(Method::GET, &room_path).header("accept", "application/json"),
        Req::new(Method::GET, &format!("{room_path}.json")),
        Req::new(Method::GET, &room_path).header("turbo-frame", "messages"),
        Req::new(Method::GET, &room_path).header("x-requested-with", "XMLHttpRequest"),
        Req::new(Method::GET, &room_path).header("accept", "text/vnd.turbo-stream.html"),
        Req::new(Method::GET, &format!("{room_path}/events")).header("accept", "application/json"),
        // Unported pages, and paths a ported pattern doesn't cover.
        Req::new(Method::GET, "/users/7/profile"),
        Req::new(Method::GET, "/users/me/profile/edit"),
        Req::new(Method::GET, "/rooms/new"),
        Req::new(Method::GET, &format!("{room_path}/messages")),
    ];
    for request in requests {
        let label = format!("{} {:?}", request.path, request.headers);
        let reply = b.send(request).await;
        assert!(
            !redirected_to_spa(&reply),
            "{label}: {:?}",
            reply.location()
        );
    }

    // Signed out, a ported page asks for a sign-in as ever.
    let signed_out = a.anonymous().get(&room_path).await;
    assert_eq!(signed_out.location(), Some(to("/session/new").as_str()));
}

/// A classic action that leaves a notice for the next page keeps that page classic, so the
/// notice shows; the next visit, with the flash shown, goes to the SPA again.
#[tokio::test]
async fn a_pending_flash_keeps_the_classic_page() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
    let path = format!("/rooms/{room}");
    choose(&a, DAVID, UiPreference::Next).await;
    let mut b = a.sign_in(DAVID).await;
    let saved = b
        .write(Req::new(Method::PATCH, "/users/me/profile").form(&[("user[time_zone]", "")]))
        .await;
    assert_eq!(saved.status, StatusCode::FOUND);
    let shown = b.get(&path).await;
    assert_eq!(shown.status, StatusCode::OK, "{:?}", shown.location());
    assert!(redirected_to_spa(&b.get(&path).await));
}

/// A page fetched by a script with the session cookie (`fetch()`, `curl`): `Accept: */*` and no
/// `Sec-Fetch-Mode: navigate`, so it gets the page, not a redirect. Either navigation signal alone
/// is enough to redirect.
#[tokio::test]
async fn a_fetch_with_the_session_cookie_is_not_a_navigation() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
    choose(&a, DAVID, UiPreference::Next).await;
    let mut b = a.sign_in(DAVID).await;
    let path = format!("/rooms/{room}");
    for request in [
        Req::new(Method::GET, &path).header("accept", "*/*"),
        Req::new(Method::GET, &path)
            .header("accept", "*/*")
            .header("sec-fetch-mode", "cors"),
        Req::new(Method::HEAD, &path).header("accept", "*/*"),
    ] {
        let label = format!("{} {:?}", request.method, request.headers);
        let reply = b.send(request).await;
        assert!(
            !redirected_to_spa(&reply),
            "{label}: {:?}",
            reply.location()
        );
    }
    for request in [
        Req::new(Method::GET, &path)
            .header("accept", "*/*")
            .header("sec-fetch-mode", "navigate"),
        Req::new(Method::GET, &path).header("accept", "text/html"),
    ] {
        let label = format!("{:?}", request.headers);
        assert!(redirected_to_spa(&b.send(request).await), "{label}");
    }
}

/// Only a browser session is redirected: a bot key or an agent token reading a ported page, and
/// any `POST` to one, get what they always got, even with everyone defaulted to the new UI.
#[tokio::test]
async fn keys_tokens_and_posts_are_never_redirected() {
    use crate::controllers::presenters::test_support::BENDER_KEY;
    use campfire_db::{AgentCredential, NewCredential};
    use sha2::{Digest, Sha256};

    const SECRET: &str = "coexistence-agent-credential";
    let Some(a) = app(&[("SPA_ENABLED", "1"), ("SPA_DEFAULT", "next")]).await else {
        return;
    };
    let room = room(&a).await;
    a.db()
        .write(|tx| {
            let agent: i64 =
                tx.conn()
                    .query_row("SELECT id FROM agents ORDER BY id LIMIT 1", [], |row| {
                        row.get(0)
                    })?;
            let digest = format!("{:x}", Sha256::digest(SECRET));
            AgentCredential::create(
                tx,
                NewCredential {
                    agent_id: agent,
                    created_by_id: DAVID,
                    name: "coexistence".into(),
                    token_last_four: digest[..4].into(),
                    token_digest: digest,
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let bearer = ["Bearer", SECRET].join(" ");
    let mut requests = Vec::new();
    for path in ["/".to_string(), format!("/rooms/{room}")] {
        for method in [Method::GET, Method::HEAD] {
            let keyed = format!("{path}?bot_key={BENDER_KEY}");
            requests.push(Req::new(method.clone(), &keyed).header("accept", "text/html"));
            requests.push(
                Req::new(method, &path)
                    .header("accept", "text/html")
                    .header("sec-fetch-mode", "navigate")
                    .header("authorization", &bearer),
            );
        }
        requests.push(
            Req::new(Method::POST, &path)
                .header("accept", "text/html")
                .header("authorization", &bearer),
        );
    }
    for request in requests {
        let label = format!("{} {} {:?}", request.method, request.path, request.headers);
        let reply = a.anonymous().send(request).await;
        assert!(
            !redirected_to_spa(&reply),
            "{label}: {:?}",
            reply.location()
        );
    }

    // A session's POST to a ported path isn't redirected either.
    let mut david = a.sign_in(DAVID).await;
    for path in ["/".to_string(), format!("/rooms/{room}")] {
        let reply = david
            .write(Req::new(Method::POST, &path).header("accept", "text/html"))
            .await;
        assert!(
            !redirected_to_spa(&reply),
            "POST {path}: {:?}",
            reply.location()
        );
    }
}

#[tokio::test]
async fn the_admin_pages_redirect_but_their_saves_stay_classic() {
    let Some(a) = enabled().await else { return };
    choose(&a, DAVID, UiPreference::Next).await;
    let mut david = a.sign_in(DAVID).await;
    for (classic, spa) in [
        ("/account/edit", "/app/admin"),
        ("/account/icons", "/app/admin/icons"),
        ("/account/custom_styles/edit", "/app/admin/styles"),
        ("/account/audit_log", "/app/admin/audit-log"),
        ("/account/integrations_health", "/app/admin/integrations"),
    ] {
        let reply = david.get(classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(reply.location(), Some(to(spa).as_str()), "{classic}");
    }

    // The classic form's save runs as before and answers with the classic redirect.
    let save = david
        .write(
            Req::new(Method::PATCH, "/account/custom_styles")
                .header("accept", "text/html")
                .form(&[("account[custom_styles]", "body { color: red; }")]),
        )
        .await;
    assert!(save.status.is_redirection(), "{:?}", save.status);
    assert!(!redirected_to_spa(&save), "{:?}", save.location());
}

#[tokio::test]
async fn the_bot_pages_redirect_but_their_saves_stay_classic() {
    let Some(a) = enabled().await else { return };
    choose(&a, DAVID, UiPreference::Next).await;
    let mut david = a.sign_in(DAVID).await;
    let bender = crate::controllers::presenters::test_support::BENDER;
    for (classic, spa) in [
        ("/account/bots".to_string(), "/app/admin/bots".to_string()),
        ("/account/bots/new".into(), "/app/admin/bots/new".into()),
        (
            format!("/account/bots/{bender}/edit"),
            format!("/app/admin/bots/{bender}"),
        ),
        (
            format!("/account/bots/{bender}/credentials"),
            format!("/app/admin/bots/{bender}/credentials"),
        ),
        (
            format!("/account/bots/{bender}/grants"),
            format!("/app/admin/bots/{bender}/grants"),
        ),
    ] {
        let reply = david.get(&classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(reply.location(), Some(to(&spa).as_str()), "{classic}");
    }

    // The classic form's save runs as before and answers with the classic redirect.
    let save = david
        .write(
            Req::new(Method::PATCH, &format!("/account/bots/{bender}"))
                .header("accept", "text/html")
                .form(&[("user[name]", "Bender")]),
        )
        .await;
    assert!(save.status.is_redirection(), "{:?}", save.status);
    assert!(!redirected_to_spa(&save), "{:?}", save.location());
}

#[tokio::test]
async fn spa_default_next_moves_everyone_who_has_not_chosen_classic() {
    let Some(a) = app(&[("SPA_ENABLED", "1"), ("SPA_DEFAULT", "next")]).await else {
        return;
    };
    let room = room(&a).await;
    let path = format!("/rooms/{room}");
    assert_eq!(stored(&a, JASON).await, None);
    let mut jason = a.sign_in(JASON).await;
    assert_eq!(
        jason.get(&path).await.location(),
        Some(to(&format!("/app/r/{room}")).as_str())
    );

    choose(&a, DAVID, UiPreference::Classic).await;
    let mut david = a.sign_in(DAVID).await;
    assert!(!redirected_to_spa(&david.get(&path).await));
    let profile = david.get("/users/me/profile").await.text();
    assert!(profile.contains("Try the new Smartfire"));
}

#[tokio::test]
async fn nothing_changes_without_spa_enabled() {
    for env in [&[][..], &[("SPA_DEFAULT", "next")][..]] {
        let Some(a) = app(env).await else { return };
        let room = room(&a).await;
        choose(&a, DAVID, UiPreference::Next).await;
        let mut b = a.sign_in(DAVID).await;
        for path in ["/".to_string(), format!("/rooms/{room}")] {
            assert!(!redirected_to_spa(&b.get(&path).await), "{env:?} {path}");
        }
        let profile = b.get("/users/me/profile").await;
        assert_eq!(profile.status, StatusCode::OK);
        assert!(
            !profile.text().contains("next_ui") && !profile.text().contains("/app/ui_preference"),
            "{env:?}"
        );
        let post = post_ui(&mut b, &[("ui", "classic")]).await;
        assert_eq!(post.status, StatusCode::NOT_FOUND, "{env:?}");
        assert_eq!(stored(&a, DAVID).await, Some(UiPreference::Next));
    }
}

#[tokio::test]
async fn the_profile_switch_opts_in_and_back_out() {
    let Some(a) = enabled().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let profile = b.get("/users/me/profile").await.text();
    assert!(profile.contains(r#"<div class="flex flex-wrap align-center gap pad-block-half" id="next_ui"><form class="button_to" data-turbo="false" method="post" action="/app/ui_preference"><button class="btn btn--reversed" type="submit">Try the new Smartfire</button>"#), "{profile}");
    assert!(
        profile.contains(r#"<input type="hidden" name="ui" value="next" /></form>"#),
        "{profile}"
    );

    let opted_in = post_ui(&mut b, &[("ui", "next")]).await;
    assert_eq!(opted_in.status, StatusCode::SEE_OTHER);
    assert_eq!(opted_in.location(), Some(to("/app/").as_str()));
    assert_eq!(stored(&a, DAVID).await, Some(UiPreference::Next));

    // The profile is the SPA's settings now; `?classic=1` keeps it here, with the way back.
    assert_eq!(
        b.get("/users/me/profile").await.location(),
        Some(to("/app/settings").as_str())
    );
    let profile = b.get("/users/me/profile?classic=1").await;
    assert_eq!(profile.status, StatusCode::OK);
    let profile = profile.text();
    assert!(
        profile.contains(r#"<a class="btn btn--reversed" href="/app/">Open the new Smartfire</a>"#),
        "{profile}"
    );
    assert!(
        profile.contains("Switch to classic")
            && profile.contains(r#"name="return_to" value="/users/me/profile""#),
        "{profile}"
    );

    let opted_out = post_ui(
        &mut b,
        &[("ui", "classic"), ("return_to", "/app/r/5/t/9?m=3")],
    )
    .await;
    assert_eq!(opted_out.status, StatusCode::SEE_OTHER);
    assert_eq!(
        opted_out.location(),
        Some(to("/rooms/5/threads/9?m=3").as_str())
    );
    assert_eq!(stored(&a, DAVID).await, Some(UiPreference::Classic));
}

#[tokio::test]
async fn switching_to_classic_returns_only_to_local_pages() {
    let Some(a) = enabled().await else { return };
    let mut b = a.sign_in(DAVID).await;
    for (return_to, landing) in [
        (None, "/"),
        (Some("/app/"), "/"),
        (Some("/app"), "/"),
        (Some("/app/r/12"), "/rooms/12"),
        (Some("/app/r/12/m/34"), "/rooms/12/@34"),
        (Some("/app/activity"), "/activity"),
        (Some("/app/nowhere"), "/"),
        (Some("/users/me/profile"), "/users/me/profile"),
        (Some("/rooms/3?classic=1"), "/rooms/3?classic=1"),
        (Some("//evil.example/x"), "/"),
        (Some("/\\evil.example"), "/"),
        (Some("https://evil.example/"), "/"),
        (Some("javascript:alert(1)"), "/"),
        (Some(""), "/"),
    ] {
        let mut pairs = vec![("ui", "classic")];
        pairs.extend(return_to.map(|path| ("return_to", path)));
        let reply = post_ui(&mut b, &pairs).await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{return_to:?}");
        assert_eq!(
            reply.location(),
            Some(to(landing).as_str()),
            "{return_to:?}"
        );
    }
}

#[tokio::test]
async fn the_switch_needs_a_known_ui_and_the_session_token() {
    let Some(a) = enabled().await else { return };
    let mut b = a.sign_in(DAVID).await;
    for pairs in [&[("ui", "Next")][..], &[("ui", "")][..], &[][..]] {
        assert_eq!(
            post_ui(&mut b, pairs).await.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{pairs:?}"
        );
    }
    let forged = b
        .send(Req::new(Method::POST, "/app/ui_preference").form(&[("ui", "next")]))
        .await;
    assert_ne!(forged.status, StatusCode::SEE_OTHER);
    assert_eq!(stored(&a, DAVID).await, None, "neither changed anything");

    let signed_out = a
        .anonymous()
        .send(Req::new(Method::POST, "/app/ui_preference").form(&[("ui", "next")]))
        .await;
    assert_ne!(signed_out.status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn the_shell_loads_in_full_from_a_turbo_visit() {
    let Some(a) = enabled().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let shell = b.get("/app/").await.text();
    assert!(
        shell.contains(r#"<meta name="turbo-visit-control" content="reload" />"#),
        "{shell}"
    );
}

const WATERCOOLER: i64 = 486777696;
const DESIGNERS_ROOM: i64 = 654632876;
const LAUNCH_THREAD: i64 = 1;
const BOARD_THREAD: i64 = 4;
/// The release board. Thread 4 lives here. Kevin is not a member; David is.
const RELEASE_BOARD: i64 = 699448332;
const SPA_BOOT: &str = "<script type=\"application/json\" id=\"boot\"";
const CLASSIC_SHELL: &str = "data-controller=\"local-time lightbox";

/// Follow redirects on `start` until a page, and return that page's path and body.
async fn follow(b: &mut Browser<'_>, start: &str) -> (String, Reply) {
    let mut path = start.to_string();
    for _ in 0..6 {
        let reply = b.get(&path).await;
        if !reply.status.is_redirection() {
            return (path, reply);
        }
        path = reply
            .location()
            .unwrap_or("")
            .trim_start_matches(ORIGIN)
            .to_string();
    }
    panic!("redirects from {start} did not settle");
}

fn assert_page(path: &str, reply: &Reply, classic: bool) {
    assert_eq!(
        reply.status,
        StatusCode::OK,
        "{path}: {:?}",
        reply.location()
    );
    let body = reply.text();
    if classic {
        assert!(body.contains(CLASSIC_SHELL), "{path} stayed classic");
        assert!(!body.contains(SPA_BOOT), "{path} bounced into the SPA");
    } else {
        assert!(body.contains(SPA_BOOT), "{path} opened the SPA");
    }
}

/// A thread or message that isn't in the room, or that the viewer can't see, or that's gone,
/// stays on the plain room route.
#[tokio::test]
async fn room_notification_links_refuse_foreign_inaccessible_and_deleted_ids() {
    let Some(a) = enabled().await else { return };
    choose(&a, DAVID, UiPreference::Next).await;
    choose(&a, KEVIN, UiPreference::Next).await;
    let mut david = a.sign_in(DAVID).await;
    let owned = david
        .get(&format!("/rooms/{DESIGNERS_ROOM}?thread={LAUNCH_THREAD}"))
        .await;
    assert_eq!(
        owned.location(),
        Some(to(&format!("/app/r/{DESIGNERS_ROOM}/t/{LAUNCH_THREAD}")).as_str())
    );
    let cross_thread = david
        .get(&format!("/rooms/{WATERCOOLER}?thread={LAUNCH_THREAD}"))
        .await;
    assert_eq!(
        cross_thread.location(),
        Some(to(&format!("/app/r/{WATERCOOLER}?thread={LAUNCH_THREAD}")).as_str())
    );
    let designers_message = 935961888i64;
    let cross_message = david
        .get(&format!(
            "/rooms/{WATERCOOLER}?message_id={designers_message}"
        ))
        .await;
    assert_eq!(
        cross_message.location(),
        Some(
            to(&format!(
                "/app/r/{WATERCOOLER}?message_id={designers_message}"
            ))
            .as_str()
        )
    );
    let mut kevin = a.sign_in(KEVIN).await;
    let inaccessible = kevin
        .get(&format!("/rooms/{DESIGNERS_ROOM}?thread={BOARD_THREAD}"))
        .await;
    assert_eq!(
        inaccessible.location(),
        Some(to(&format!("/app/r/{DESIGNERS_ROOM}?thread={BOARD_THREAD}")).as_str())
    );
    // The thread belongs to this room. David can open it. Kevin cannot, because he
    // isn't a member: dropping the membership check would send him to the thread.
    let member = david
        .get(&format!("/rooms/{RELEASE_BOARD}?thread={BOARD_THREAD}"))
        .await;
    assert_eq!(
        member.location(),
        Some(to(&format!("/app/r/{RELEASE_BOARD}/t/{BOARD_THREAD}")).as_str())
    );
    let non_member = kevin
        .get(&format!("/rooms/{RELEASE_BOARD}?thread={BOARD_THREAD}"))
        .await;
    assert_eq!(
        non_member.location(),
        Some(to(&format!("/app/r/{RELEASE_BOARD}?thread={BOARD_THREAD}")).as_str())
    );
    let (thread_id, message_id) = a
        .db()
        .write(|tx| {
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: WATERCOOLER,
                    creator_id: DAVID,
                    name: Some("Gone".into()),
                    ..Default::default()
                },
            )?;
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: WATERCOOLER,
                    creator_id: DAVID,
                    markdown_source: Some("Gone".into()),
                    ..Default::default()
                },
            )?;
            let (thread_id, message_id) = (thread.id, message.id);
            tx.conn()
                .execute("DELETE FROM channel_threads WHERE id=?", [thread_id])?;
            tx.conn()
                .execute("DELETE FROM messages WHERE id=?", [message_id])?;
            Ok((thread_id, message_id))
        })
        .await
        .unwrap();
    let deleted_thread = david
        .get(&format!("/rooms/{WATERCOOLER}?thread={thread_id}"))
        .await;
    assert_eq!(
        deleted_thread.location(),
        Some(to(&format!("/app/r/{WATERCOOLER}?thread={thread_id}")).as_str())
    );
    let deleted_message = david
        .get(&format!("/rooms/{WATERCOOLER}?message_id={message_id}"))
        .await;
    assert_eq!(
        deleted_message.location(),
        Some(to(&format!("/app/r/{WATERCOOLER}?message_id={message_id}")).as_str())
    );
}

/// Settings and another person's profile alias, followed to the page each UI actually opens.
#[tokio::test]
async fn settings_and_profile_aliases_follow_through_for_each_ui() {
    let Some(a) = enabled().await else { return };
    let closed = WATERCOOLER;
    let board = 699448332i64;
    let direct = 186869642i64;
    choose(&a, DAVID, UiPreference::Next).await;
    let mut next = a.sign_in(DAVID).await;
    let (path, page) = follow(&mut next, &format!("/rooms/{closed}/settings")).await;
    assert_eq!(path, format!("/app/r/{closed}/settings"));
    assert_page(&path, &page, false);
    let (path, page) = follow(&mut next, &format!("/rooms/{board}/settings")).await;
    assert_eq!(path, format!("/rooms/boards/{board}/edit?classic=1"));
    assert_page(&path, &page, true);
    let (path, page) = follow(&mut next, &format!("/rooms/{direct}/settings")).await;
    assert_eq!(path, format!("/rooms/directs/{direct}/edit?classic=1"));
    assert_page(&path, &page, true);
    let (path, page) = follow(&mut next, &format!("/rooms/{closed}/settings?classic=1")).await;
    assert_eq!(path, format!("/rooms/closeds/{closed}/edit?classic=1"));
    assert_page(&path, &page, true);
    let (path, page) = follow(&mut next, &format!("/users/{JASON}/profile")).await;
    assert_eq!(path, format!("/app/people/{JASON}"));
    assert_page(&path, &page, false);
    let (path, page) = follow(&mut next, &format!("/users/{JASON}/profile?classic=1")).await;
    assert_eq!(path, format!("/users/{JASON}?classic=1"));
    assert_page(&path, &page, true);

    choose(&a, DAVID, UiPreference::Classic).await;
    let mut classic = a.sign_in(DAVID).await;
    let (path, page) = follow(&mut classic, &format!("/rooms/{closed}/settings")).await;
    assert_eq!(path, format!("/rooms/closeds/{closed}/edit"));
    assert_page(&path, &page, true);
    let (path, page) = follow(&mut classic, &format!("/users/{JASON}/profile")).await;
    assert_eq!(path, format!("/users/{JASON}"));
    assert_page(&path, &page, true);
    assert!(
        page.text().contains("Jason"),
        "the person page, not the viewer's profile"
    );
}

/// A pending flash, an XHR, or a Turbo frame keeps the settings and own-profile aliases
/// classic. The same URLs go to the SPA on an ordinary navigation once nothing is waiting.
#[tokio::test]
async fn settings_and_profile_aliases_stay_classic_for_flash_xhr_and_turbo_frames() {
    let Some(a) = enabled().await else { return };
    let closed = WATERCOOLER;
    choose(&a, DAVID, UiPreference::Next).await;
    let mut b = a.sign_in(DAVID).await;
    let settings = format!("/rooms/{closed}/settings");
    let edit = format!("/rooms/closeds/{closed}/edit?classic=1");
    let profile = format!("/users/{DAVID}/profile");
    let guards = [
        ("xhr", "x-requested-with", "XMLHttpRequest"),
        ("turbo", "turbo-frame", "alias"),
    ];
    for (label, name, value) in guards {
        let settings_reply = b
            .send(Req::new(Method::GET, &settings).header(name, value))
            .await;
        assert_eq!(
            settings_reply.location(),
            Some(to(&edit).as_str()),
            "{label}"
        );
        let profile_reply = b
            .send(Req::new(Method::GET, &profile).header(name, value))
            .await;
        assert_eq!(
            profile_reply.status,
            StatusCode::OK,
            "{label}: {:?}",
            profile_reply.location()
        );
        assert!(!redirected_to_spa(&profile_reply), "{label}");
        let body = profile_reply.text();
        assert!(!body.contains(SPA_BOOT), "{label} opened the SPA");
        // A Turbo frame renders the frame layout, not the application shell.
        if label == "turbo" {
            assert!(body.contains("<turbo-frame"), "{label}: {body}");
        } else {
            assert!(
                body.contains(CLASSIC_SHELL),
                "{label} rendered the classic profile"
            );
        }
    }

    let saved = b
        .write(Req::new(Method::PATCH, "/users/me/profile").form(&[("user[time_zone]", "")]))
        .await;
    assert_eq!(saved.status, StatusCode::FOUND);
    let flashed = b.get(&settings).await;
    assert_eq!(flashed.location(), Some(to(&edit).as_str()));
    let shown = b.get(&edit).await;
    assert_eq!(shown.status, StatusCode::OK, "{:?}", shown.location());
    assert!(shown.text().contains(CLASSIC_SHELL));
    assert!(redirected_to_spa(&b.get(&settings).await));

    let saved = b
        .write(Req::new(Method::PATCH, "/users/me/profile").form(&[("user[time_zone]", "")]))
        .await;
    assert_eq!(saved.status, StatusCode::FOUND);
    let flashed_profile = b.get(&profile).await;
    assert_eq!(
        flashed_profile.status,
        StatusCode::OK,
        "{:?}",
        flashed_profile.location()
    );
    assert!(flashed_profile.text().contains(CLASSIC_SHELL));
    assert_eq!(
        b.get(&profile).await.location(),
        Some(to("/app/settings").as_str())
    );
}

#[path = "spa_coexistence_tests/auth_return.rs"]
mod auth_return;
