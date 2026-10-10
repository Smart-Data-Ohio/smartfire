//! The SPA is the only UI for signed-in people (cutover step 26), over the seeded app: every
//! classic page the SPA has ported is an alias that sends a signed-in navigation to its SPA screen.
//! There's no way back: a stored choice of the classic UI, `?classic=1`, the old
//! `POST /app/ui_preference` switch and the `SPA_ENABLED` / `SPA_DEFAULT` env vars change nothing.
//! Scripts, Turbo frames, bot keys, agent tokens and writes still get the classic responses.

use axum::http::{Method, StatusCode};
use campfire_db::models::user::ui_preference::{self, UiPreference};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, Room};

use crate::controllers::presenters::test_support::{
    Browser, DAVID, DIRECT_DAVID_JASON, JASON, KEVIN, Reply, Req, TestApp, seed_clock,
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

/// Message aliases and room tools: the classic path, its SPA screen, and what the classic page
/// itself answers to a script's fetch.
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
        // The bare classic actions have no room_id; the classic page answers 404.
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

/// `path` with a `classic` parameter in each spelling that once kept the classic page.
fn with_classic(path: &str) -> [String; 3] {
    let joiner = if path.contains('?') { '&' } else { '?' };
    ["classic=1", "classic=true", "classic"].map(|pair| format!("{path}{joiner}{pair}"))
}

#[tokio::test]
async fn ported_pages_send_everyone_to_the_spa() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
    let posted = message(&a, room, DAVID, "anchor").await;
    // Neither has chosen a UI.
    assert_eq!(stored(&a, DAVID).await, None);
    assert_eq!(stored(&a, JASON).await, None);
    for user in [DAVID, JASON] {
        let mut b = a.sign_in(user).await;
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
            let reply = b.get(&classic).await;
            assert_eq!(reply.status, StatusCode::FOUND, "{user} {classic}");
            assert_eq!(reply.location(), Some(to(&spa).as_str()), "{user} {classic}");
        }
        let head = b
            .send(Req::new(Method::HEAD, &format!("/rooms/{room}")))
            .await;
        assert_eq!(
            head.location(),
            Some(to(&format!("/app/r/{room}")).as_str()),
            "{user}"
        );
    }
}

/// A person who once chose the classic UI is sent to the SPA like everyone else.
#[tokio::test]
async fn a_stored_classic_choice_no_longer_keeps_the_classic_pages() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
    choose(&a, DAVID, UiPreference::Classic).await;
    let mut david = a.sign_in(DAVID).await;
    for (classic, spa) in [
        ("/".to_string(), "/app/".to_string()),
        (format!("/rooms/{room}"), format!("/app/r/{room}")),
        ("/users/me/profile".to_string(), "/app/settings".to_string()),
        (format!("/users/{JASON}"), format!("/app/people/{JASON}")),
        ("/account/edit".to_string(), "/app/admin".to_string()),
    ] {
        let reply = david.get(&classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(reply.location(), Some(to(&spa).as_str()), "{classic}");
    }
    assert_eq!(stored(&a, DAVID).await, Some(UiPreference::Classic));
}

/// `?classic=1` (or any other `classic` value) once kept a ported page classic. Now the redirect
/// happens anyway, and the parameter is dropped from the SPA URL.
#[tokio::test]
async fn a_classic_query_no_longer_keeps_the_classic_page() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
    let posted = message(&a, room, DAVID, "anchor").await;
    choose(&a, DAVID, UiPreference::Classic).await;
    let mut b = a.sign_in(DAVID).await;
    for (classic, spa) in [
        (format!("/rooms/{room}"), format!("/app/r/{room}")),
        (
            format!("/rooms/{room}?message_id={}", posted.id),
            format!("/app/r/{room}/m/{}", posted.id),
        ),
        ("/users/me/profile".to_string(), "/app/settings".to_string()),
        ("/work?state=agents".to_string(), "/app/work?state=agents".to_string()),
    ] {
        for path in with_classic(&classic).into_iter().chain([format!(
            "{classic}{}classic=0",
            if classic.contains('?') { '&' } else { '?' }
        )]) {
            let reply = b.get(&path).await;
            assert_eq!(reply.status, StatusCode::FOUND, "{path}: {}", reply.text());
            assert_eq!(reply.location(), Some(to(&spa).as_str()), "{path}");
        }
    }
}

/// The old switch is gone: posting to it, with the session's token, isn't a success or a redirect
/// and stores nothing. The classic profile no longer offers it.
#[tokio::test]
async fn the_ui_switch_is_gone() {
    let Some(a) = enabled().await else { return };
    let mut b = a.sign_in(DAVID).await;
    for pairs in [
        &[("ui", "classic")][..],
        &[("ui", "classic"), ("return_to", "/rooms/5")][..],
        &[("ui", "next")][..],
    ] {
        let reply = b
            .write(Req::new(Method::POST, "/app/ui_preference").form(pairs))
            .await;
        assert!(
            !reply.status.is_success() && !reply.status.is_redirection(),
            "{pairs:?}: {:?} {:?}",
            reply.status,
            reply.location()
        );
        assert_eq!(stored(&a, DAVID).await, None, "{pairs:?} stored nothing");
    }
    let signed_out = a
        .anonymous()
        .send(Req::new(Method::POST, "/app/ui_preference").form(&[("ui", "classic")]))
        .await;
    assert!(
        !signed_out.status.is_success() && !signed_out.status.is_redirection(),
        "{:?}",
        signed_out.status
    );

    let profile = b.classic_page("/users/me/profile").await;
    assert_eq!(profile.status, StatusCode::OK, "{:?}", profile.location());
    let profile = profile.text();
    assert!(profile.contains(CLASSIC_SHELL), "the classic profile page");
    for gone in [
        r#"id="next_ui""#,
        "Switch to classic",
        "Try the new Smartfire",
        "/app/ui_preference",
    ] {
        assert!(!profile.contains(gone), "{gone}: {profile}");
    }
}

/// The env vars that once kept the classic UI are no longer read: an app started with
/// `SPA_ENABLED=0` and `SPA_DEFAULT=classic` sends everyone to the SPA.
#[tokio::test]
async fn spa_enabled_0_and_spa_default_classic_still_send_everyone_to_the_spa() {
    use crate::controllers::presenters::test_support::BENDER_KEY;

    let Some(a) = app(&[("SPA_ENABLED", "0"), ("SPA_DEFAULT", "classic")]).await else {
        return;
    };
    let room = room(&a).await;
    for user in [DAVID, JASON] {
        let mut b = a.sign_in(user).await;
        for (classic, spa) in [
            ("/".to_string(), "/app/".to_string()),
            (format!("/rooms/{room}"), format!("/app/r/{room}")),
            ("/users/me/profile/edit".to_string(), "/app/settings".to_string()),
        ] {
            let reply = b.get(&classic).await;
            assert_eq!(reply.status, StatusCode::FOUND, "{user} {classic}");
            assert_eq!(reply.location(), Some(to(&spa).as_str()), "{user} {classic}");
        }
        let shell = b.get("/app/").await;
        assert_eq!(shell.status, StatusCode::OK, "{user}");
        assert!(shell.text().contains(SPA_BOOT), "{user}");
    }
    // Only a signed-in session is sent on; the edit alias keeps its 404 for anyone else.
    profile_edit_is_not_found(&mut a.anonymous(), "").await;
    profile_edit_is_not_found(&mut a.anonymous(), &format!("?bot_key={BENDER_KEY}")).await;
}

// The composer opens the `/event` command's classic URL in a new tab. That lands on the SPA's
// new-event form with the command's query intact: Rails' sorted, form-encoded `event[...]` keys,
// of which the form's prefill (`newEventPrefill`) reads `event[title]` and `event[starts_at]` (an
// ISO time in UTC).
#[tokio::test]
async fn the_event_commands_link_opens_the_spa_form_with_its_prefill() {
    let a = enabled()
        .await
        .expect("the default frozen seed is required");
    let room = crate::controllers::presenters::test_support::ALL_TALK;
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
async fn board_lists_posts_and_new_posts_redirect_to_the_spa() {
    const BOARD: i64 = 699448332;
    const POST: i64 = 4;
    let a = enabled()
        .await
        .expect("the default frozen seed is required");
    let mut david = a.sign_in(DAVID).await;
    for (classic, spa) in [
        (format!("/rooms/{BOARD}"), format!("/app/r/{BOARD}")),
        (
            format!("/rooms/{BOARD}/threads/{POST}"),
            format!("/app/r/{BOARD}/t/{POST}"),
        ),
        (
            format!("/rooms/{BOARD}/threads/new"),
            format!("/app/r/{BOARD}/posts/new"),
        ),
    ] {
        let reply = david.get(&classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(reply.location(), Some(to(&spa).as_str()), "{classic}");
        let bypass = david.get(&format!("{classic}?classic=1")).await;
        assert_eq!(bypass.status, StatusCode::FOUND, "{classic}?classic=1");
        assert_eq!(bypass.location(), Some(to(&spa).as_str()), "{classic}?classic=1");
    }
    let filter = "status=blocked&owner=me&tag=Release&page=2";
    let reply = david.get(&format!("/rooms/{BOARD}?{filter}")).await;
    assert_eq!(
        reply.location(),
        Some(to(&format!("/app/r/{BOARD}?{filter}")).as_str())
    );
    let reply = david.get("/work").await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some(to("/app/work").as_str()));
    // The work filter carries over, and the handoff page's room is the SPA's to resolve.
    for (classic, spa) in [
        (
            "/work?state=agents".to_string(),
            "/app/work?state=agents".to_string(),
        ),
        (
            format!("/threads/{POST}/work/handoff/new"),
            format!("/app/t/{POST}/handoff"),
        ),
    ] {
        let reply = david.get(&classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(reply.location(), Some(to(&spa).as_str()), "{classic}");
    }
}

#[tokio::test]
async fn message_aliases_and_room_tools_redirect_to_the_new_ui() {
    let a = enabled()
        .await
        .expect("the default frozen seed is required");
    let room = crate::controllers::presenters::test_support::ALL_TALK;
    let message = message(&a, room, DAVID, "coexistence gap message").await;
    let mut david = a.sign_in(DAVID).await;
    for (classic, spa, _) in gap_pages(room, message.id) {
        let reply = david.get(&classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(reply.location(), Some(to(&spa).as_str()), "{classic}");
    }
}

/// A stored classic choice and `?classic=1` both still redirect the aliases. A script's fetch of
/// the classic page gets what it always got.
#[tokio::test]
async fn message_aliases_and_room_tools_redirect_whatever_was_chosen() {
    let a = enabled()
        .await
        .expect("the default frozen seed is required");
    let room = crate::controllers::presenters::test_support::ALL_TALK;
    let message = message(&a, room, DAVID, "coexistence gap message").await;
    choose(&a, DAVID, UiPreference::Classic).await;
    let mut david = a.sign_in(DAVID).await;
    for (classic, spa, status) in gap_pages(room, message.id) {
        for path in [classic.clone(), format!("{classic}?classic=1")] {
            let reply = david.get(&path).await;
            assert_eq!(reply.status, StatusCode::FOUND, "{path}");
            assert_eq!(reply.location(), Some(to(&spa).as_str()), "{path}");
        }
        let fetched = david.classic_page(&classic).await;
        assert_eq!(fetched.status, status, "{classic}");
        assert!(!redirected_to_spa(&fetched), "{classic}");
        if classic == format!("/rooms/{room}/messages/{}", message.id) {
            assert!(fetched.text().contains("coexistence gap message"), "{classic}");
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
    for (classic, spa, _) in gap_pages(DIRECT_KEVIN_BENDER, message.id) {
        for path in [classic.clone(), format!("{classic}?classic=1")] {
            let reply = david.get(&path).await;
            assert_eq!(reply.status, StatusCode::FOUND, "{path}");
            assert_eq!(reply.location(), Some(to(&spa).as_str()), "{path}");
            assert!(!reply.text().contains(SECRET), "{path}");
        }
        // The classic page itself, as a script fetches it, still refuses a non-member.
        let fetched = david.classic_page(&classic).await;
        assert_eq!(fetched.status, StatusCode::NOT_FOUND, "{classic}");
        assert!(!fetched.text().contains(SECRET), "{classic}");
    }
    // The room's permalink doesn't confirm a message David can't read: it stays a query on the
    // plain room route, `classic` dropped.
    for path in [
        format!("/rooms/{DIRECT_KEVIN_BENDER}?message_id={}", message.id),
        format!(
            "/rooms/{DIRECT_KEVIN_BENDER}?message_id={}&classic=1",
            message.id
        ),
    ] {
        let reply = david.get(&path).await;
        assert_eq!(
            reply.location(),
            Some(
                to(&format!(
                    "/app/r/{DIRECT_KEVIN_BENDER}?message_id={}",
                    message.id
                ))
                .as_str()
            ),
            "{path}"
        );
        assert!(!reply.text().contains(SECRET), "{path}");
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
        for path in [classic.clone(), format!("{classic}?classic=1")] {
            let reply = anonymous.get(&path).await;
            assert_eq!(
                reply.location(),
                Some(to("/session/new").as_str()),
                "{path}"
            );
            assert!(!reply.text().contains(SECRET), "{path}");
        }
    }
}

#[tokio::test]
async fn only_html_navigations_of_ported_pages_redirect() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
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

#[tokio::test]
async fn legacy_bot_and_numeric_profile_aliases_open_the_spa() {
    let Some(a) = enabled().await else { return };
    let bot = a.db().write(|tx| {
        let bot = campfire_db::User::create_bot(tx, "Legacy Bot", None)?;
        assert!(campfire_db::Agent::for_user(tx.conn(), bot.id)?.is_none());
        Ok(bot.id)
    }).await.unwrap();
    let mut b = a.sign_in(DAVID).await;
    for (id, destination) in [
        (DAVID, "/app/settings".to_string()),
        (JASON, format!("/app/people/{JASON}")),
        (bot, format!("/app/people/{bot}")),
    ] {
        for suffix in ["", "/profile", "/profile/edit"] {
            let path = format!("/users/{id}{suffix}?source=profile");
            let reply = b.get(&path).await;
            assert_eq!(reply.status, StatusCode::FOUND, "{path}: {}", reply.text());
            assert_eq!(reply.location(), Some(to(&format!("{destination}?source=profile")).as_str()), "{path}");
        }
    }
    assert_eq!(b.get("/users/me/profile/edit").await.location(), Some(to("/app/settings").as_str()));
    // `?classic=1` no longer keeps the classic answer (the edit alias's 404, the bot's page).
    assert_eq!(b.get("/users/me/profile/edit?classic=1").await.location(), Some(to("/app/settings").as_str()));
    assert_eq!(b.get(&format!("/users/{bot}?classic=1")).await.location(), Some(to(&format!("/app/people/{bot}")).as_str()));
}

async fn profile_edit_is_not_found(browser: &mut Browser<'_>, query: &str) {
    for user in ["me".to_string(), DAVID.to_string(), JASON.to_string()] {
        let path = format!("/users/{user}/profile/edit{query}");
        let reply = browser.get(&path).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}: {}", reply.text());
        assert_eq!(reply.location(), None, "{path}");
    }
}

#[tokio::test]
async fn signed_out_profile_edit_keeps_classic_not_found() {
    let Some(a) = enabled().await else { return };
    let mut browser = a.anonymous();
    profile_edit_is_not_found(&mut browser, "?classic=1").await;
    profile_edit_is_not_found(&mut browser, "").await;
}

#[tokio::test]
async fn bot_key_profile_edit_keeps_classic_not_found() {
    use crate::controllers::presenters::test_support::BENDER_KEY;

    let Some(a) = enabled().await else { return };
    let mut browser = a.anonymous();
    profile_edit_is_not_found(&mut browser, &format!("?classic=1&bot_key={BENDER_KEY}")).await;
    profile_edit_is_not_found(&mut browser, &format!("?bot_key={BENDER_KEY}")).await;
}

/// A classic action that leaves a notice for the next page no longer keeps that page classic: the
/// notice rides the redirect, and the SPA shell's boot JSON shows it once.
#[tokio::test]
async fn a_pending_flash_follows_the_redirect_into_the_spa_shell() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
    let path = format!("/rooms/{room}");
    let spa = format!("/app/r/{room}");
    let mut b = a.sign_in(DAVID).await;
    save_profile(&mut b).await;
    let flashed = b.get(&path).await;
    assert_eq!(flashed.status, StatusCode::FOUND);
    assert_eq!(flashed.location(), Some(to(&spa).as_str()));
    assert_shell_flash(&mut b, &spa).await;
}

/// The profile save's classic form post, which leaves `notice: "✓"` for the next page.
async fn save_profile(b: &mut Browser<'_>) {
    let saved = b
        .write(Req::new(Method::PATCH, "/users/me/profile").form(&[("user[time_zone]", "")]))
        .await;
    assert_eq!(saved.status, StatusCode::FOUND);
}

/// The shell's boot JSON in a page.
fn boot_json(page: &str) -> serde_json::Value {
    let pattern = regex::Regex::new(
        r#"(?s)<script type="application/json" id="boot"[^>]*>(.*?)</script>"#,
    )
    .unwrap();
    serde_json::from_str(&pattern.captures(page).expect("the shell's boot JSON")[1]).unwrap()
}

/// The SPA shell at `spa` shows the profile notice in its boot JSON, then not on the next load.
async fn assert_shell_flash(b: &mut Browser<'_>, spa: &str) {
    let shell = b.get(spa).await;
    assert_eq!(shell.status, StatusCode::OK, "{spa}: {:?}", shell.location());
    assert_eq!(
        boot_json(&shell.text())["flash"],
        serde_json::json!({"kind": "notice", "message": "✓"}),
        "{spa}"
    );
    let again = b.get(spa).await;
    assert_eq!(again.status, StatusCode::OK, "{spa}");
    assert!(boot_json(&again.text())["flash"].is_null(), "{spa} showed the notice once");
}

/// A page fetched by a script with the session cookie (`fetch()`, `curl`): `Accept: */*` and no
/// `Sec-Fetch-Mode: navigate`, so it gets the page, not a redirect. Either navigation signal alone
/// is enough to redirect.
#[tokio::test]
async fn a_fetch_with_the_session_cookie_is_not_a_navigation() {
    let Some(a) = enabled().await else { return };
    let room = room(&a).await;
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
/// any `POST` to one, get what they always got.
#[tokio::test]
async fn keys_tokens_and_posts_are_never_redirected() {
    use crate::controllers::presenters::test_support::BENDER_KEY;
    use campfire_db::{AgentCredential, NewCredential};
    use sha2::{Digest, Sha256};

    const SECRET: &str = "coexistence-agent-credential";
    let Some(a) = enabled().await else { return };
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

/// Settings and another person's profile alias, followed to the page they open: the SPA screen,
/// whatever was chosen and with or without `?classic=1`. A direct message has no SPA settings, so
/// its alias opens the conversation.
#[tokio::test]
async fn settings_and_profile_aliases_follow_through_to_the_spa() {
    let Some(a) = enabled().await else { return };
    let closed = WATERCOOLER;
    let board = 699448332i64;
    let direct = 186869642i64;
    for preference in [None, Some(UiPreference::Classic)] {
        if let Some(preference) = preference {
            choose(&a, DAVID, preference).await;
        }
        let mut b = a.sign_in(DAVID).await;
        for query in ["", "?classic=1"] {
            let label = format!("{preference:?} {query}");
            let (path, page) = follow(&mut b, &format!("/rooms/{closed}/settings{query}")).await;
            assert_eq!(path, format!("/app/r/{closed}/settings"), "{label}");
            assert_page(&path, &page, false);
            let (path, page) = follow(&mut b, &format!("/rooms/{board}/settings{query}")).await;
            assert_eq!(path, format!("/app/r/{board}/settings"), "{label}");
            assert_page(&path, &page, false);
            let (path, page) = follow(&mut b, &format!("/rooms/{direct}/settings{query}")).await;
            assert_eq!(path, format!("/app/r/{direct}"), "{label}");
            assert_page(&path, &page, false);
            let (path, page) = follow(&mut b, &format!("/users/{JASON}/profile{query}")).await;
            assert_eq!(path, format!("/app/people/{JASON}"), "{label}");
            assert_page(&path, &page, false);
        }
    }
}

/// An XHR or a Turbo frame keeps the settings and own-profile aliases classic. A pending flash no
/// longer does: the notice rides the redirect into the SPA shell.
#[tokio::test]
async fn settings_and_profile_aliases_stay_classic_for_xhr_and_turbo_frames_but_not_a_flash() {
    let Some(a) = enabled().await else { return };
    let closed = WATERCOOLER;
    let mut b = a.sign_in(DAVID).await;
    let settings = format!("/rooms/{closed}/settings");
    let edit = format!("/rooms/closeds/{closed}/edit");
    let profile = format!("/users/{DAVID}/profile");
    let guards = [
        ("xhr", "x-requested-with", "XMLHttpRequest"),
        ("turbo", "turbo-frame", "alias"),
    ];
    for (label, name, value) in guards {
        for path in [settings.clone(), format!("{settings}?classic=1")] {
            let settings_reply = b
                .send(Req::new(Method::GET, &path).header(name, value))
                .await;
            assert_eq!(
                settings_reply.location(),
                Some(to(&edit).as_str()),
                "{label} {path}"
            );
        }
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

    save_profile(&mut b).await;
    let flashed = b.get(&settings).await;
    let spa_settings = format!("/app/r/{closed}/settings");
    assert_eq!(flashed.location(), Some(to(&spa_settings).as_str()));
    assert_shell_flash(&mut b, &spa_settings).await;

    save_profile(&mut b).await;
    let flashed_profile = b.get(&profile).await;
    assert_eq!(flashed_profile.location(), Some(to("/app/settings").as_str()));
    assert_shell_flash(&mut b, "/app/settings").await;
}

/// Classic pages and fragments the SPA has no screen of its own for, opened as a page: the SPA
/// state that does the same job. A script's fetch, an XHR, a Turbo frame and JSON get what they
/// always got.
fn navigation_aliases(thread_room: i64, thread: i64) -> [(String, String); 9] {
    let direct = DIRECT_DAVID_JASON;
    [
        // The sidebar's New message picker.
        ("/rooms/directs/new".into(), "/app/rooms/new/direct".into()),
        // The conversation, whose header has "Add people" and "Rename conversation".
        (format!("/rooms/directs/{direct}/edit"), format!("/app/r/{direct}")),
        (format!("/rooms/{direct}/settings"), format!("/app/r/{direct}")),
        (format!("/users/{JASON}/card"), format!("/app/people/{JASON}")),
        (format!("/users/{DAVID}/card"), "/app/settings".into()),
        (
            format!("/rooms/{thread_room}/threads/{thread}/content"),
            format!("/app/r/{thread_room}/t/{thread}"),
        ),
        (
            format!("/rooms/{thread_room}/threads/{thread}/messages"),
            format!("/app/r/{thread_room}/t/{thread}"),
        ),
        ("/account/users".into(), "/app/admin/people".into()),
        ("/account/users?page=2".into(), "/app/admin/people?page=2".into()),
    ]
}

#[tokio::test]
async fn direct_message_card_thread_and_people_list_pages_open_their_spa_state() {
    let Some(a) = enabled().await else { return };
    let (thread_room, thread) = (DESIGNERS_ROOM, LAUNCH_THREAD);
    let mut b = a.sign_in(DAVID).await;
    for (classic, spa) in navigation_aliases(thread_room, thread) {
        let reply = b.get(&classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(reply.location(), Some(to(&spa).as_str()), "{classic}");
        for path in with_classic(&classic) {
            assert_eq!(b.get(&path).await.location(), Some(to(&spa).as_str()), "{path}");
        }
        let navigated = b
            .send(
                Req::new(Method::GET, &classic)
                    .header("accept", "*/*")
                    .header("sec-fetch-mode", "navigate")
                    .header("sec-fetch-dest", "document"),
            )
            .await;
        assert_eq!(navigated.location(), Some(to(&spa).as_str()), "navigate {classic}");
        // The SPA's service worker forwards a navigation with `fetch(request)`: still
        // `Sec-Fetch-Mode: navigate`, but `Sec-Fetch-Dest: empty`.
        for accept in ["*/*", "text/html,application/xhtml+xml"] {
            let forwarded = b
                .send(
                    Req::new(Method::GET, &classic)
                        .header("accept", accept)
                        .header("sec-fetch-mode", "navigate")
                        .header("sec-fetch-dest", "empty"),
                )
                .await;
            assert_eq!(forwarded.status, StatusCode::FOUND, "worker {accept} {classic}");
            assert_eq!(
                forwarded.location(),
                Some(to(&spa).as_str()),
                "worker {accept} {classic}"
            );
        }
        let (path, page) = follow(&mut b, &classic).await;
        assert_eq!(path, spa, "{classic}");
        assert_page(&path, &page, false);
    }
}

/// A thread's content opens at a reply with `message_id`; the SPA's thread route reads it as `m`.
#[tokio::test]
async fn the_thread_content_reply_anchor_becomes_the_spa_m() {
    let Some(a) = enabled().await else { return };
    let (room, thread) = (DESIGNERS_ROOM, LAUNCH_THREAD);
    let mut b = a.sign_in(DAVID).await;
    let base = format!("/rooms/{room}/threads/{thread}/content");
    for (query, spa) in [
        ("?message_id=123", format!("/app/r/{room}/t/{thread}?m=123")),
        ("?message_id=123&classic=1", format!("/app/r/{room}/t/{thread}?m=123")),
        ("?message_id=latest", format!("/app/r/{room}/t/{thread}")),
    ] {
        let reply = b.get(&format!("{base}{query}")).await;
        assert_eq!(reply.location(), Some(to(&spa).as_str()), "{query}");
    }
}

#[tokio::test]
async fn the_new_aliases_leave_scripts_frames_and_json_alone() {
    let Some(a) = enabled().await else { return };
    let (thread_room, thread) = (DESIGNERS_ROOM, LAUNCH_THREAD);
    let mut b = a.sign_in(DAVID).await;
    for (classic, _) in navigation_aliases(thread_room, thread) {
        if classic.split('?').next() == Some("/account/users") { continue; }
        let requests = [
            ("xhr", Req::new(Method::GET, &classic).header("x-requested-with", "XMLHttpRequest")),
            ("turbo", Req::new(Method::GET, &classic).header("turbo-frame", "alias")),
            ("json", Req::new(Method::GET, &classic).header("accept", "application/json")),
            // A script's fetch of the HTML: Fetch Metadata says it isn't a navigation.
            (
                "fetch",
                Req::new(Method::GET, &classic)
                    .header("accept", "text/html")
                    .header("sec-fetch-mode", "cors")
                    .header("sec-fetch-dest", "empty"),
            ),
            (
                "iframe",
                Req::new(Method::GET, &classic)
                    .header("accept", "text/html")
                    .header("sec-fetch-mode", "navigate")
                    .header("sec-fetch-dest", "iframe"),
            ),
            (
                "frame",
                Req::new(Method::GET, &classic)
                    .header("accept", "text/html")
                    .header("sec-fetch-mode", "navigate")
                    .header("sec-fetch-dest", "frame"),
            ),
            (
                "same-origin fetch",
                Req::new(Method::GET, &classic)
                    .header("accept", "text/html")
                    .header("sec-fetch-mode", "same-origin")
                    .header("sec-fetch-dest", "empty"),
            ),
            (
                "no-cors",
                Req::new(Method::GET, &classic)
                    .header("accept", "text/html")
                    .header("sec-fetch-mode", "no-cors")
                    .header("sec-fetch-dest", "script"),
            ),
        ];
        for (label, request) in requests {
            let reply = b.send(request).await;
            assert!(!redirected_to_spa(&reply), "{label} {classic}: {:?}", reply.location());
        }
    }
}

/// The router takes a `.html` suffix for the page itself (`/rooms/7.html` is `/rooms/7`), so the
/// suffix is an alias too. Other formats keep their classic answer.
#[tokio::test]
async fn an_html_suffix_is_the_same_classic_alias() {
    let Some(a) = enabled().await else { return };
    let room = WATERCOOLER;
    let mut b = a.sign_in(DAVID).await;
    for (classic, spa) in [
        (format!("/rooms/{room}.html"), format!("/app/r/{room}")),
        (format!("/rooms/{room}.html?thread={LAUNCH_THREAD}&x=1"), format!("/app/r/{room}?thread={LAUNCH_THREAD}&x=1")),
        ("/users/me/profile.html".into(), "/app/settings".into()),
        (format!("/users/{JASON}.html"), format!("/app/people/{JASON}")),
        ("/searches.html?q=fire".into(), "/app/search?q=fire".into()),
        ("/rooms/directs/new.html".into(), "/app/rooms/new/direct".into()),
        (format!("/rooms/{room}/threads.html"), format!("/app/r/{room}/threads")),
    ] {
        let reply = b.get(&classic).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{classic}");
        assert_eq!(reply.location(), Some(to(&spa).as_str()), "{classic}");
    }
    for other in [
        format!("/rooms/{room}.json"),
        format!("/users/{JASON}.json"),
        "/searches.json?q=fire".to_string(),
    ] {
        let reply = b.get(&other).await;
        assert!(!redirected_to_spa(&reply), "{other}: {:?}", reply.location());
    }
}

/// Production config parsing, untouched: with `SPA_ENABLED` absent or off, a signed-in browser
/// still gets the SPA, and an old URL still sends it there.
#[tokio::test]
async fn production_config_serves_the_spa_with_spa_enabled_absent_or_off() {
    let envs: [&[(&str, &str)]; 3] = [
        &[],
        &[("SPA_ENABLED", "false")],
        &[("SPA_ENABLED", "0"), ("SPA_DEFAULT", "classic")],
    ];
    for env in envs {
        let Some(a) = TestApp::boot_seed_with_production_env("default", seed_clock(), env).await
        else {
            return;
        };
        assert!(a.booted.app.config.spa_enabled, "{env:?}");
        let mut b = a.sign_in(DAVID).await;
        let shell = b.get("/app/").await;
        assert_eq!(shell.status, StatusCode::OK, "{env:?}");
        assert!(shell.text().contains(SPA_BOOT), "{env:?}");
        let reply = b.get(&format!("/rooms/{WATERCOOLER}")).await;
        assert_eq!(reply.location(), Some(to(&format!("/app/r/{WATERCOOLER}")).as_str()), "{env:?}");
        let reply = b.get("/users/me/profile").await;
        assert_eq!(reply.location(), Some(to("/app/settings").as_str()), "{env:?}");
    }
}

#[path = "spa_coexistence_tests/auth_return.rs"]
mod auth_return;
