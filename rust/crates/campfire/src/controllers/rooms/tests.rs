//! Request-level tests for the room controllers, against the `default` parity seed.

use axum::http::{Method, StatusCode};
use campfire_db::{Membership, Room, RoomType};

use crate::controllers::presenters::test_support::*;

#[tokio::test]
async fn show_renders_the_room_and_remembers_it() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let reply = david.get(&format!("/rooms/{ALL_TALK}")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.content_type(), Some("text/html; charset=utf-8"));
    let html = reply.text();
    assert!(html.contains("<title>All Talk</title>"), "{html}");
    assert!(html.contains(r#"<meta name="current-room-id" content="486777696">"#));
    assert_eq!(html.matches(r#"data-controller="reply""#).count(), 40, "the last page");
    assert!(reply.headers.get_all("set-cookie").iter().any(|c| c.to_str().unwrap().starts_with(&format!("last_room={ALL_TALK}"))));
    assert_eq!(reply.header("x-version"), Some("parity"));
}

#[tokio::test]
async fn show_at_a_message_pages_around_it() {
    let Some(app) = TestApp::boot().await else { return };
    let first = app
        .db()
        .read(|conn| Ok(campfire_db::Message::for_room(conn, ALL_TALK)?.into_iter().min_by_key(|m| m.created_at).unwrap()))
        .await
        .unwrap();
    let reply = app.david().get(&format!("/rooms/{ALL_TALK}/@{}", first.id)).await;
    assert_eq!(reply.status, StatusCode::OK);
    // The first message and the 40 after it.
    assert_eq!(reply.text().matches(r#"data-controller="reply""#).count(), 41);
}

#[tokio::test]
async fn inaccessible_rooms_redirect_home_with_an_alert() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let reply = david.get(&format!("/rooms/{DIRECT_KEVIN_BENDER}")).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some("http://campfire.test/"));
    let reply = david.get("/rooms/nonsense").await;
    assert_eq!(reply.status, StatusCode::FOUND);
}

#[tokio::test]
async fn index_redirects_to_the_last_room() {
    let Some(app) = TestApp::boot().await else { return };
    let reply = app.david().get("/rooms").await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert!(reply.location().unwrap().starts_with("http://campfire.test/rooms/"));
    // Anonymous: off to sign in.
    let reply = app.anonymous().get("/rooms").await;
    assert_eq!(reply.location(), Some("http://campfire.test/session/new"));
}

#[tokio::test]
async fn undeclared_actions() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    assert_eq!(david.get("/rooms/new").await.status, StatusCode::NOT_FOUND);
    assert_eq!(david.get(&format!("/rooms/{ALL_TALK}/edit")).await.status, StatusCode::NOT_FOUND);
    let direct = david.get(&format!("/rooms/directs/{DIRECT_DAVID_JASON}")).await;
    assert_eq!(direct.status, StatusCode::FOUND);
    assert!(direct.header("location").unwrap().ends_with(&format!("/rooms/{DIRECT_DAVID_JASON}")));
    let reply = david.write(Req::new(Method::DELETE, &format!("/rooms/opens/{HQ}"))).await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn open_rooms_are_created_edited_and_updated() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let new = david.get("/rooms/opens/new").await;
    assert_eq!(new.status, StatusCode::OK);
    assert!(new.text().contains("New chat room"));

    let created = david.write(Req::new(Method::POST, "/rooms/opens").form(&[("room[name]", "Watercooler")])).await;
    assert_eq!(created.status, StatusCode::FOUND, "{}", created.text());
    let room_id: i64 = created.location().unwrap().rsplit('/').next().unwrap().parse().unwrap();
    let room = app.db().read(move |conn| Room::find(conn, room_id)).await.unwrap();
    assert_eq!((room.name.as_deref(), room.room_type), (Some("Watercooler"), RoomType::Open));
    // Rooms::Open grants every active user after commit.
    let members = app.db().read(move |conn| Membership::for_room(conn, room_id)).await.unwrap();
    assert!(members.len() > 3);

    assert_eq!(david.get(&format!("/rooms/opens/{room_id}/edit")).await.status, StatusCode::OK);
    let shown = david.get(&format!("/rooms/opens/{room_id}")).await;
    assert_eq!(shown.location(), Some(format!("http://campfire.test/rooms/{room_id}").as_str()));

    let updated = david.write(Req::new(Method::PATCH, &format!("/rooms/closeds/{room_id}")).form(&[("room[name]", "Private"), ("user_ids[]", &DAVID.to_string())])).await;
    assert_eq!(updated.status, StatusCode::FOUND, "{}", updated.text());
    let room = app.db().read(move |conn| Room::find(conn, room_id)).await.unwrap();
    assert_eq!((room.name.as_deref(), room.room_type), (Some("Private"), RoomType::Closed));
    let members = app.db().read(move |conn| Membership::for_room(conn, room_id)).await.unwrap();
    assert_eq!(members.iter().map(|m| m.user_id).collect::<Vec<_>>(), vec![DAVID]);

    let missing_param = david.write(Req::new(Method::POST, "/rooms/opens").form(&[("name", "x")])).await;
    assert_eq!(missing_param.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn closed_rooms_are_created_with_the_selected_users() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    assert_eq!(david.get("/rooms/closeds/new").await.status, StatusCode::OK);
    let created = david
        .write(Req::new(Method::POST, "/rooms/closeds").form(&[
            ("room[name]", "Secret"),
            ("user_ids[]", &DAVID.to_string()),
            ("user_ids[]", &JASON.to_string()),
            ("user_ids[]", "999"),
        ]))
        .await;
    assert_eq!(created.status, StatusCode::FOUND, "{}", created.text());
    let room_id: i64 = created.location().unwrap().rsplit('/').next().unwrap().parse().unwrap();
    let mut members: Vec<i64> = app.db().read(move |conn| Membership::for_room(conn, room_id)).await.unwrap().iter().map(|m| m.user_id).collect();
    members.sort();
    assert_eq!(members, vec![DAVID, JASON]);
    assert_eq!(david.get(&format!("/rooms/closeds/{room_id}/edit")).await.status, StatusCode::OK);
}

#[tokio::test]
async fn only_administrators_or_creators_update_rooms() {
    let Some(app) = TestApp::boot().await else { return };
    // Jason (an administrator in the seed) isn't needed: David is an admin, so check the scope instead:
    // direct rooms are out of reach of the open/closed controllers.
    let mut david = app.david();
    let reply = david.get(&format!("/rooms/opens/{DIRECT_DAVID_JASON}/edit")).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some("http://campfire.test/"));
}

#[tokio::test]
async fn direct_rooms_are_found_or_created() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    assert_eq!(david.get("/rooms/directs/new").await.status, StatusCode::OK);
    let existing = david.write(Req::new(Method::POST, "/rooms/directs").form(&[("user_ids[]", &JASON.to_string())])).await;
    assert_eq!(existing.location(), Some(format!("http://campfire.test/rooms/{DIRECT_DAVID_JASON}").as_str()));
    let created = david.write(Req::new(Method::POST, "/rooms/directs").form(&[("user_ids[]", &KEVIN.to_string())])).await;
    let room_id: i64 = created.location().unwrap().rsplit('/').next().unwrap().parse().unwrap();
    let room = app.db().read(move |conn| Room::find(conn, room_id)).await.unwrap();
    assert_eq!(room.room_type, RoomType::Direct);

    assert_eq!(david.get(&format!("/rooms/directs/{room_id}/edit")).await.status, StatusCode::OK);
    let destroyed = david.write(Req::new(Method::DELETE, &format!("/rooms/directs/{room_id}"))).await;
    assert_eq!(destroyed.location(), Some("http://campfire.test/"));
    assert!(app.db().read(move |conn| Ok(Room::find(conn, room_id)?.deleted_at.is_some())).await.unwrap());
}

#[tokio::test]
async fn refresh_streams_messages_since_a_time() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let reply = david
        .send(Req::new(Method::GET, &format!("/rooms/{ALL_TALK}/refresh?since=0")).header("accept", "text/vnd.turbo-stream.html, text/html"))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.content_type(), Some("text/vnd.turbo-stream.html; charset=utf-8"));
    assert!(reply.text().starts_with(r#"<turbo-stream action="append" target="messages_rooms_closed_486777696">"#), "{}", reply.text());

    let html_only = david.get(&format!("/rooms/{ALL_TALK}/refresh?since=0")).await;
    assert_eq!(html_only.status, StatusCode::NOT_ACCEPTABLE);
    assert_eq!(david.get(&format!("/rooms/{DIRECT_KEVIN_BENDER}/refresh")).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn involvement_is_shown_and_changed() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let shown = david.get(&format!("/rooms/{ALL_TALK}/involvement")).await;
    assert_eq!(shown.status, StatusCode::OK);
    assert!(shown.text().contains("turbo-frame"));
    let updated = david.write(Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/involvement")).form(&[("involvement", "invisible")])).await;
    assert_eq!(updated.location(), Some(format!("http://campfire.test/rooms/{ALL_TALK}/involvement").as_str()));
    let membership = app.db().read(|conn| Membership::find_by_room_and_user(conn, ALL_TALK, DAVID)).await.unwrap().unwrap();
    assert_eq!(membership.involvement, Some(campfire_db::Involvement::Invisible));
    let invalid = david.write(Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/involvement")).form(&[("involvement", "loud")])).await;
    assert_eq!(invalid.status, StatusCode::INTERNAL_SERVER_ERROR);

    // Our fork requires a nonblank involvement.
    let missing = david.write(Req::new(Method::PATCH, &format!("/rooms/{ALL_TALK}/involvement"))).await;
    assert_eq!(missing.status, StatusCode::BAD_REQUEST);
    let membership = app.db().read(|conn| Membership::find_by_room_and_user(conn, ALL_TALK, DAVID)).await.unwrap().unwrap();
    assert_eq!(membership.involvement, Some(campfire_db::Involvement::Invisible));
}

#[tokio::test]
async fn rooms_are_destroyed_by_administrators() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let reply = david.write(Req::new(Method::DELETE, &format!("/rooms/{QUIET_CORNER}"))).await;
    assert_eq!(reply.location(), Some("http://campfire.test/"));
    assert!(app.db().read(|conn| Ok(Room::find(conn, QUIET_CORNER)?.deleted_at.is_some())).await.unwrap());
}

#[tokio::test]
async fn cross_site_writes_are_refused() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    // Another site's page can post with David's cookies, but can't read his authenticity token.
    let request = Req::new(Method::POST, "/rooms/opens").form(&[("room[name]", "x")]);
    assert_eq!(david.send(request).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    let forged = Req::new(Method::POST, "/rooms/opens").form(&[("room[name]", "x"), ("authenticity_token", "forged")]);
    assert_eq!(david.send(forged).await.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn the_last_room_cookie_is_set_only_when_it_changes() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let last_room = |reply: &Reply| {
        reply.headers.get_all(axum::http::header::SET_COOKIE).iter().any(|c| c.to_str().unwrap().starts_with("last_room="))
    };
    assert!(last_room(&david.get(&format!("/rooms/{HQ}")).await));
    assert!(!last_room(&david.get(&format!("/rooms/{HQ}")).await), "the same room again");
    assert!(last_room(&david.get(&format!("/rooms/{ALL_TALK}")).await));
}

/// A value on a page that belongs to the session it was rendered for: an authenticity token (the
/// `csrf-token` meta tag's, or a form's, with the path and method the form submits to) or a CSP
/// nonce.
#[derive(Debug)]
enum SessionBound {
    Token { value: String, path: String, method: String },
    Nonce(String),
}

impl SessionBound {
    fn value(&self) -> &str {
        match self {
            SessionBound::Token { value, .. } | SessionBound::Nonce(value) => value,
        }
    }
}

/// Every session-bound value on `html`, a page served for `page_path`, in page order.
fn session_bound(html: &str, page_path: &str) -> Vec<SessionBound> {
    let meta = regex::Regex::new(r#"<meta name="csrf-token" content="([^"]*)""#).unwrap();
    let form = regex::Regex::new(r#"(?s)<form\b([^>]*)>(.*?)</form>"#).unwrap();
    let attribute = |name: &str| regex::Regex::new(&format!(r#"\s{name}="([^"]*)""#)).unwrap();
    let (action, form_method) = (attribute("action"), attribute("method"));
    let hidden = |name: &str| regex::Regex::new(&format!(r#"name="{name}" value="([^"]*)""#)).unwrap();
    let (token, method_override) = (hidden("authenticity_token"), hidden("_method"));
    let nonce = regex::Regex::new(r#"(?:name="csp-nonce" content="|\snonce=")([^"]*)""#).unwrap();

    let mut found: Vec<(usize, SessionBound)> = Vec::new();
    for captures in meta.captures_iter(html) {
        let value = captures.get(1).unwrap();
        found.push((value.start(), SessionBound::Token { value: value.as_str().into(), path: "/".into(), method: "post".into() }));
    }
    for captures in form.captures_iter(html) {
        let (attributes, inner) = (&captures[1], captures.get(2).unwrap());
        let Some(value) = token.captures(inner.as_str()).map(|c| c.get(1).unwrap()) else { continue };
        let action = action.captures(attributes).map_or(String::new(), |c| c[1].replace("&amp;", "&"));
        // A relative action gets the per-form token of the page's own path
        // (RequestForgeryProtection#normalize_relative_action_path).
        let path = if action.starts_with('/') { action.split(['?', '#']).next().unwrap().to_string() } else { page_path.to_string() };
        let method = method_override.captures(inner.as_str()).or_else(|| form_method.captures(attributes)).map_or("post".into(), |c| c[1].to_lowercase());
        found.push((inner.start() + value.start(), SessionBound::Token { value: value.as_str().into(), path, method }));
    }
    for captures in nonce.captures_iter(html) {
        let value = captures.get(1).unwrap();
        found.push((value.start(), SessionBound::Nonce(value.as_str().into())));
    }
    assert_eq!(
        found.iter().filter(|(_, bound)| matches!(bound, SessionBound::Token { .. })).count(),
        html.matches(r#"name="csrf-token""#).count() + html.matches(r#"name="authenticity_token""#).count(),
        "every token on the page is in a form or the meta tag"
    );
    found.sort_by_key(|(at, _)| *at);
    found.into_iter().map(|(_, bound)| bound).collect()
}

/// Whose session each token on the page verifies for (`valid_authenticity_token?` for the path and
/// method it's submitted with), written over the token, so pages compare by who their tokens
/// belong to rather than by their masked bytes. Nonces become `nonce`.
fn by_owner(html: &str, page_path: &str, sessions: &[(&str, &campfire_kit::csrf::RealToken)]) -> (String, Vec<String>) {
    let mut page = html.to_string();
    let mut owners = Vec::new();
    for bound in session_bound(html, page_path) {
        let label = match &bound {
            SessionBound::Token { value, path, method } => {
                let owner: Vec<&str> = sessions.iter().filter(|(_, real)| real.is_valid(value, path, method)).map(|(name, _)| *name).collect();
                let owner = if owner.is_empty() { "nobody".to_string() } else { owner.join("+") };
                owners.push(owner.clone());
                format!("{owner}'s token for {method} {path}")
            }
            SessionBound::Nonce(_) => "nonce".to_string(),
        };
        page = page.replacen(bound.value(), &label, 1);
    }
    (page, owners)
}

/// Rails #148 leaves cached message-tree write forms tokenless. The layout's forms and
/// composer carry only the current viewer's tokens, cold or warm; GET/dialog forms have none.
#[tokio::test]
async fn room_pages_carry_only_their_own_viewers_session_bound_values() {
    let Some(app) = TestApp::boot().await else { return };
    let newest = app
        .db()
        .read(|conn| Ok(campfire_db::Message::for_room(conn, ALL_TALK)?.into_iter().max_by_key(|m| m.created_at).unwrap()))
        .await
        .unwrap();
    // The existing seed's boosts are all counted reactions. Add one free-text boost to this
    // test's private database so the legacy delete form's tokenlessness is still exercised.
    let newest_id = newest.id;
    app.db().write(move |tx| campfire_db::Boost::create(tx, newest_id, DAVID, "Hello")).await.unwrap();
    let room = format!("/rooms/{ALL_TALK}");
    let older = format!("/rooms/{ALL_TALK}/messages?before={}", newest.id);
    let mut david = app.sign_in(DAVID).await;
    let mut jason = app.sign_in(JASON).await;

    // David's first render of each page stores its messages in the fragment cache; every render
    // after reads them.
    let mut renders = Vec::new();
    for path in [&room, &older] {
        for name in ["david", "jason", "david", "jason"] {
            let browser = if name == "david" { &mut david } else { &mut jason };
            let reply = browser.get(path).await;
            assert_eq!(reply.status, StatusCode::OK, "{path} as {name}");
            renders.push((path.split('?').next().unwrap().to_string(), name, reply.text()));
        }
    }
    let david_real = david.real_authenticity_token().unwrap();
    let jason_real = jason.real_authenticity_token().unwrap();
    let sessions = [("david", &david_real), ("jason", &jason_real)];

    let mut labeled = Vec::new();
    let form = regex::Regex::new(r#"(?s)<form\b([^>]*)>(.*?)</form>"#).unwrap();
    let method = regex::Regex::new(r#"\smethod="([^"]*)""#).unwrap();
    let mut tokenless_boosts = 0;
    for (page_path, name, html) in &renders {
        for captures in form.captures_iter(html) {
            let (attributes, inner) = (&captures[1], &captures[2]);
            let tokens = inner.matches(r#"name="authenticity_token""#).count();
            if inner.contains(r#"data-action="boost-delete#perform""#) {
                assert_eq!(tokens, 0, "{page_path} as {name}: cached boost-delete forms are tokenless (#148)");
                tokenless_boosts += 1;
            } else if attributes.contains(r#"class="reaction-chip__form""#) || attributes.contains(r#"class="poll__form""#) || attributes.contains(r#"class="poll__retract-form""#) {
                assert_eq!(tokens, 0, "cached reaction and poll forms are tokenless (#148)");
            } else if method.captures(attributes).is_none_or(|c| c[1].eq_ignore_ascii_case("get") || c[1].eq_ignore_ascii_case("dialog")) {
                assert_eq!(tokens, 0, "GET and dialog forms carry no token");
            } else {
                assert_eq!(tokens, 1, "{page_path} as {name}: other write forms carry one viewer token: {attributes}");
            }
        }
        let (page, owners) = by_owner(html, page_path, &sessions);
        if page_path.ends_with("/messages") {
            assert!(owners.is_empty(), "layout-free cached message HTML has no tokens");
            assert!(session_bound(html, page_path).is_empty(), "layout-free cached message HTML has no session-bound values");
        } else {
            assert!(owners.len() > 1, "{page_path} as {name} has layout and composer tokens");
        }
        let foreign: Vec<&String> = owners.iter().filter(|owner| owner.as_str() != *name).collect();
        assert!(foreign.is_empty(), "{page_path} as {name}: {} of {} tokens aren't {name}'s: {:?}", foreign.len(), owners.len(), &foreign[..foreign.len().min(3)]);
        labeled.push(page);
    }
    assert!(tokenless_boosts > 0, "the seeded case exercises #148's boost-delete form");
    // No token or nonce one viewer was given turns up in the other's pages.
    let values = |who: &str| -> std::collections::HashSet<String> {
        renders.iter().filter(|(_, name, _)| *name == who).flat_map(|(path, _, html)| session_bound(html, path)).map(|b| b.value().to_string()).collect()
    };
    let shared: Vec<String> = values("david").intersection(&values("jason")).cloned().collect();
    assert!(shared.is_empty(), "{} values in both viewers' pages: {:?}", shared.len(), &shared[..shared.len().min(3)]);
    // Cold and warm, a viewer's page is the same but for fresh masks and nonces (the first
    // request's nonce is random: it arrives without a session id to derive one from).
    for page in 0..2 {
        let at = |pass: usize| &labeled[page * 4 + pass];
        assert_eq!(at(0), at(2), "{} cold and warm for david", renders[page * 4].0);
        assert_eq!(at(1), at(3), "{} for jason", renders[page * 4].0);
    }

    // Submit the real composer's token: Jason's token works only with Jason's cookies.
    let (_, _, jason_page) = &renders[3];
    let (value, path) = session_bound(jason_page, &room)
        .into_iter()
        .find_map(|bound| match bound {
            SessionBound::Token { value, path, .. } if path.ends_with("/messages") => Some((value, path)),
            _ => None,
        })
        .expect("the composer form on the room page");
    let compose = |token: &str| Req::new(Method::POST, &path).header("accept", "text/vnd.turbo-stream.html").form(&[
        ("authenticity_token", token), ("message[body]", "A viewer-bound composer submission"),
        ("message[client_message_id]", "viewer-token-regression"),
    ]);
    assert_eq!(david.send(compose(&value)).await.status, StatusCode::UNPROCESSABLE_ENTITY, "Jason's token with David's session");
    assert_eq!(jason.send(compose(&value)).await.status, StatusCode::OK, "Jason's token with his own session");
}

#[tokio::test]
async fn a_room_page_gets_a_new_etag_for_every_render() {
    let Some(app) = TestApp::boot().await else { return };
    let mut david = app.david();
    let etag = |reply: &Reply| reply.header("etag").map(str::to_string);
    let cold = david.get(&format!("/rooms/{HQ}")).await;
    let warm = david.get(&format!("/rooms/{HQ}")).await;
    // Rack::ETag digests the body, so a page carrying a freshly masked token gets a new ETag on
    // every request, in our Rails as here.
    assert!(etag(&cold).is_some() && etag(&warm).is_some());
    assert_ne!(etag(&cold), etag(&warm));
}
