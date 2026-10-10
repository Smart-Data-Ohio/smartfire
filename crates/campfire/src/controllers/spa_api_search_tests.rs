//! S3 search on `/api/v1` (`campfire_api::search`): results with their chips, sections and
//! keyset paging, and the recent searches.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::json;

use super::api_tests::{ALL_PETS, app, get, json_body, parse};
use crate::controllers::presenters::test_support::{
    ALL_TALK, Browser, DAVID, DIRECT_DAVID_JASON, JASON, KEVIN, Reply, Req,
};

const DESIGNERS: i64 = 654632876;
const THREAD: i64 = 1;

async fn post(b: &mut Browser<'_>, room_id: i64, n: usize, source: &str) -> api::MessageDTO {
    let body = json!({"clientMessageId": format!("0199b3c4-search-{n}"), "markdownSource": source, "replyToMessageId": null, "replyNotifyAuthor": null});
    let reply = b
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{room_id}/messages"),
            &body,
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    parse(&reply)
}

fn validation(reply: &Reply) -> (String, Vec<String>) {
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { message, fields } = parse::<api::ApiErrorResponse>(reply).error
    else {
        panic!("{}", reply.text())
    };
    (message, fields.into_keys().collect())
}

#[tokio::test]
async fn search_finds_pages_and_chips() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut posted = Vec::new();
    for n in 0..45 {
        posted.push(
            post(
                &mut david,
                DESIGNERS,
                n,
                &format!("needlepaging number {n:02}"),
            )
            .await,
        );
    }
    // Some share a time, so the cursor's id breaks the tie.
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE messages SET created_at = '2026-01-01 12:00:00' WHERE markdown_source LIKE 'needlepaging number 1%'",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();

    let reply = david.send(get("/api/v1/search?q=needlepaging")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let first: api::SearchResults = parse(&reply);
    assert_eq!(first.query, "needlepaging");
    assert!(first.chips.is_empty() && first.sections.is_empty());
    assert_eq!(first.messages.len(), 40);
    // Oldest first within the page; the newest is the last one posted.
    assert_eq!(
        first.messages.last().map(|m| m.id),
        posted.last().map(|m| m.id)
    );
    assert!(first.users.iter().any(|user| user.id == DAVID));
    assert!(
        first
            .conversations
            .iter()
            .any(|name| name.room_id == DESIGNERS && name.thread_id.is_none())
    );
    let cursor = first.next_cursor.clone().expect("an older page");
    let second: api::SearchResults = parse(
        &david
            .send(get(&format!(
                "/api/v1/search?q=needlepaging&before={cursor}"
            )))
            .await,
    );
    assert_eq!(second.messages.len(), 5);
    assert_eq!(second.next_cursor, None);
    let mut all = first
        .messages
        .iter()
        .chain(&second.messages)
        .map(|m| m.id)
        .collect::<Vec<_>>();
    all.sort();
    all.dedup();
    assert_eq!(all.len(), 45, "each once");
    // Deleting the cursor's message doesn't break the next page.
    let oldest = first.messages[0].id;
    let reply = david
        .write(
            Req::new(Method::DELETE, &format!("/api/v1/messages/{oldest}"))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let again: api::SearchResults = parse(
        &david
            .send(get(&format!(
                "/api/v1/search?q=needlepaging&before={cursor}"
            )))
            .await,
    );
    assert_eq!(ids(&again), ids(&second));

    // Operators become chips; the query is collapsed.
    let reply = david
        .send(get(
            "/api/v1/search?q=needlepaging%20%20from:@david,%20has:BOGUS%20in:%23designers",
        ))
        .await;
    let filtered: api::SearchResults = parse(&reply);
    assert_eq!(
        filtered.query,
        "needlepaging from:@david, has:BOGUS in:#designers"
    );
    let chips = filtered
        .chips
        .iter()
        .map(|chip| (chip.operator, chip.value.as_str(), chip.label.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        chips,
        [
            (api::SearchOperator::From, "david", "from: david"),
            (api::SearchOperator::In, "designers", "in: designers"),
        ]
    );
    assert_eq!(
        filtered.chips[0].remove_query,
        "needlepaging has:BOGUS in:#designers"
    );
    // `has:BOGUS` stays in the text, so nothing matches it.
    assert!(filtered.messages.is_empty());

    // Others only find what's in their rooms; a blank query is an empty page.
    let mine = post(&mut david, ALL_PETS, 99, "needlepets only here").await;
    let kevins = post(&mut kevin, DESIGNERS, 98, "needlepets in designers").await;
    let theirs: api::SearchResults = parse(&kevin.send(get("/api/v1/search?q=needlepets")).await);
    assert_eq!(ids(&theirs), [kevins.id]);
    let found: api::SearchResults = parse(&david.send(get("/api/v1/search?q=needlepets")).await);
    assert_eq!(ids(&found), [mine.id, kevins.id]);
    let blank: api::SearchResults = parse(&david.send(get("/api/v1/search?q=%20%20")).await);
    assert_eq!(
        (
            blank.messages.len(),
            blank.next_cursor,
            blank.query.as_str()
        ),
        (0, None, "")
    );
    let reply = david
        .send(get("/api/v1/search?q=needlepaging&before=not-a-cursor"))
        .await;
    assert_eq!(validation(&reply).1, ["before"]);
}

#[tokio::test]
async fn the_first_page_lists_matching_work_threads() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE channel_threads SET work_status = 'in_progress' WHERE id = ?",
                [THREAD],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let results: api::SearchResults = parse(&david.send(get("/api/v1/search?q=launch")).await);
    let work = results
        .sections
        .iter()
        .find(|section| section.kind == api::SearchSectionKind::WorkThreads)
        .expect("the work threads section");
    let row = work.rows.iter().find(|row| row.id == THREAD).unwrap();
    assert_eq!(
        (
            row.room_id,
            row.room_kind,
            row.title.as_str(),
            row.work_status,
            row.cancelled
        ),
        (
            DESIGNERS,
            api::RoomKind::Closed,
            "Launch review",
            Some(api::WorkStatus::InProgress),
            false
        )
    );
    assert!(
        results
            .conversations
            .iter()
            .any(|name| (name.room_id, name.thread_id) == (DESIGNERS, Some(THREAD)))
    );
    // Operators alone have no sections.
    let results: api::SearchResults = parse(&david.send(get("/api/v1/search?q=is:thread")).await);
    assert!(results.sections.is_empty());
}

#[tokio::test]
async fn recent_searches_record_move_up_and_clear() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let record = |query: &str| {
        json_body(
            Method::POST,
            "/api/v1/search/recents",
            &json!({ "query": query }),
        )
    };
    let reply = david.write(record("  launch   plans ")).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let list: api::RecentSearchList = parse(&reply);
    assert_eq!(list.searches[0].query, "launch plans");
    for n in 0..11 {
        david.write(record(&format!("query {n}"))).await;
    }
    // Repeating one moves it to the top; the list keeps the newest 10.
    let list: api::RecentSearchList = parse(&david.write(record("query 3")).await);
    assert_eq!(list.searches.len(), 10);
    assert_eq!(list.searches[0].query, "query 3");
    assert!(
        list.searches
            .iter()
            .all(|search| search.query != "launch plans")
    );
    let listed: api::RecentSearchList = parse(&david.send(get("/api/v1/search/recents")).await);
    assert_eq!(listed, list);
    let theirs: api::RecentSearchList = parse(&kevin.send(get("/api/v1/search/recents")).await);
    assert!(
        theirs
            .searches
            .iter()
            .all(|search| !search.query.starts_with("query "))
    );

    let reply = david.write(record("   ")).await;
    assert_eq!(
        validation(&reply),
        (
            "Enter a word to search for.".to_owned(),
            vec!["query".to_owned()]
        )
    );

    let reply = david
        .write(
            Req::new(Method::DELETE, "/api/v1/search/recents").header("accept", "application/json"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    let listed: api::RecentSearchList = parse(&david.send(get("/api/v1/search/recents")).await);
    assert!(listed.searches.is_empty());
}

fn ids(results: &api::SearchResults) -> Vec<i64> {
    results.messages.iter().map(|m| m.id).collect()
}

async fn search(b: &mut Browser<'_>, q: &str) -> api::SearchResults {
    let reply = b
        .send(get(&format!(
            "/api/v1/search?q={}",
            url::form_urlencoded::byte_serialize(q.as_bytes()).collect::<String>()
        )))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{q}: {}", reply.text());
    parse(&reply)
}

#[tokio::test]
async fn search_keeps_to_the_viewers_rooms() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut jason = a.sign_in(JASON).await;

    // A closed room and a direct room Kevin isn't in.
    let private = post(&mut david, ALL_TALK, 1, "needleprivate talk").await;
    let direct = post(&mut david, DIRECT_DAVID_JASON, 2, "needledirect words").await;
    for (q, id) in [("needleprivate", private.id), ("needledirect", direct.id)] {
        assert!(search(&mut kevin, q).await.messages.is_empty(), "{q}");
        assert_eq!(ids(&search(&mut david, q).await), [id], "{q}");
    }
    // `in:` never names a direct room, whatever its people are called.
    for q in [
        "needledirect in:jason",
        "needledirect in:david",
        "needledirect in:j",
    ] {
        assert!(search(&mut david, q).await.messages.is_empty(), "{q}");
    }

    // A room Kevin left: his own message there is gone from his results, and so is its work
    // thread from the sections, while David still finds both.
    let left = post(&mut kevin, DESIGNERS, 3, "needleleft launch").await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE channel_threads SET work_status = 'in_progress' WHERE id = ?",
                [THREAD],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let work_rows = |results: &api::SearchResults| {
        results
            .sections
            .iter()
            .filter(|section| section.kind == api::SearchSectionKind::WorkThreads)
            .flat_map(|section| section.rows.iter().map(|row| row.id))
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&search(&mut kevin, "needleleft").await), [left.id]);
    assert!(work_rows(&search(&mut kevin, "launch").await).contains(&THREAD));
    a.db()
        .write(|tx| {
            campfire_db::Membership::find_by_room_and_user(tx.conn(), DESIGNERS, KEVIN)?
                .expect("Kevin is in Designers")
                .destroy(tx)
        })
        .await
        .unwrap();
    assert!(search(&mut kevin, "needleleft").await.messages.is_empty());
    assert!(!work_rows(&search(&mut kevin, "launch").await).contains(&THREAD));
    assert_eq!(ids(&search(&mut david, "needleleft").await), [left.id]);
    assert!(work_rows(&search(&mut david, "launch").await).contains(&THREAD));

    // An event in a room Kevin isn't in.
    let event: i64 = a
        .db()
        .write(|tx| {
            let now = tx.now();
            Ok(tx.conn().query_row(
                "INSERT INTO events (room_id, organizer_id, title, starts_at, time_zone, created_at, updated_at) VALUES (?, ?, 'Needleevent planning', '2030-01-01 12:00:00', 'UTC', ?, ?) RETURNING id",
                rusqlite::params![ALL_TALK, DAVID, now, now],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    let events = |results: &api::SearchResults| {
        results
            .sections
            .iter()
            .filter(|section| section.kind == api::SearchSectionKind::Events)
            .flat_map(|section| section.rows.iter().map(|row| row.id))
            .collect::<Vec<_>>()
    };
    assert_eq!(events(&search(&mut david, "needleevent").await), [event]);
    assert!(search(&mut kevin, "needleevent").await.sections.is_empty());

    // A deactivated person's messages are still found, as on the classic page.
    let theirs = post(&mut jason, ALL_TALK, 4, "needlegone farewell").await;
    a.db()
        .write(|tx| campfire_db::User::find(tx.conn(), JASON)?.deactivate(tx))
        .await
        .unwrap();
    let found = search(&mut david, "needlegone").await;
    assert_eq!(ids(&found), [theirs.id]);
    assert!(found.users.iter().any(|user| user.id == JASON));
}

#[tokio::test]
async fn hostile_and_oversized_queries_answer_cleanly() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    post(&mut david, DESIGNERS, 1, "needlehostile and near the end").await;
    // Full-text syntax is only ever words.
    for q in [
        "\"",
        "*",
        "AND",
        "NEAR(",
        "needlehostile AND",
        "needlehostile*",
        "\"needlehostile",
        "NEAR(needlehostile end)",
        "needlehostile OR x",
        "- ^ :",
        "100%",
        "a_b",
        "in:% from:_",
    ] {
        search(&mut david, q).await;
    }
    assert_eq!(
        search(&mut david, "needlehostile AND").await.messages.len(),
        1
    );

    // A date past the last instant is a bound after every message, on both pages.
    let found = search(&mut david, "needlehostile before:9999-12-31").await;
    assert_eq!(found.messages.len(), 1);
    for q in [
        "needlehostile after:9999-12-31",
        "needlehostile on:9999-12-31",
        "after:9999-12-31",
    ] {
        assert!(search(&mut david, q).await.messages.is_empty(), "{q}");
        let classic = david
            .classic_page(&format!(
                "/searches?q={}",
                url::form_urlencoded::byte_serialize(q.as_bytes()).collect::<String>()
            ))
            .await;
        assert_eq!(classic.status, StatusCode::OK, "{q}: {}", classic.text());
    }

    // Bounds: 500 characters, and 10 each of `from:` and `in:`.
    let at_most = format!("needlehostile {}", "x".repeat(500 - 14));
    search(&mut david, &at_most).await;
    let filters = |operator: &str, n: usize| {
        (0..n)
            .map(|i| format!("{operator}:name{i}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    search(
        &mut david,
        &format!("{} {}", filters("from", 10), filters("in", 10)),
    )
    .await;
    for q in [
        format!("{at_most}x"),
        filters("from", 11),
        filters("in", 11),
    ] {
        let reply = david
            .send(get(&format!(
                "/api/v1/search?q={}",
                url::form_urlencoded::byte_serialize(q.as_bytes()).collect::<String>()
            )))
            .await;
        assert_eq!(validation(&reply).1, ["q"], "{q}");
        let reply = david
            .write(json_body(
                Method::POST,
                "/api/v1/search/recents",
                &json!({ "query": q }),
            ))
            .await;
        assert_eq!(validation(&reply).1, ["query"], "{q}");
    }
}

#[tokio::test]
async fn typed_search_filters_preserve_ids_visibility_and_sort() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let first = post(&mut david, DESIGNERS, 200, "typedneedle first").await;
    let second = post(&mut david, DESIGNERS, 201, "typedneedle second").await;
    post(&mut kevin, DESIGNERS, 202, "typedneedle someone else").await;
    post(&mut david, ALL_TALK, 203, "typedneedle hidden").await;
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET name='Renamed person' WHERE id=?", [DAVID])?;
            tx.conn().execute(
                "UPDATE rooms SET name='Renamed channel' WHERE id=?",
                [DESIGNERS],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let results: api::SearchResults = parse(
        &david
            .send(get(&format!(
                "/api/v1/search?q=typedneedle&authorId={DAVID}&channelId={DESIGNERS}&sort=oldest"
            )))
            .await,
    );
    // The wire page is reverse display order, as with the existing newest-first pages.
    assert_eq!(ids(&results), [second.id, first.id]);
    assert_eq!(results.chips.len(), 3);
    let hidden: api::SearchResults = parse(
        &kevin
            .send(get(&format!(
                "/api/v1/search?q=typedneedle&authorId={DAVID}&channelId={ALL_TALK}&sort=relevance"
            )))
            .await,
    );
    assert!(hidden.messages.is_empty() && hidden.sections.is_empty());
    for params in [
        "authorId=bad",
        "channelId=-1",
        "sort=bogus",
        "has=bogus",
        "mentionsMe=bogus",
    ] {
        let reply = david
            .send(get(&format!("/api/v1/search?q=typedneedle&{params}")))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{params}: {}",
            reply.text()
        );
    }
}

#[tokio::test]
async fn search_mentions_use_persisted_user_identity_and_current_body() {
    let Some(a) = app(true).await else { return };
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mine = post(
        &mut david,
        DESIGNERS,
        210,
        &format!("typedmention <@{DAVID}> https://example.com"),
    )
    .await;
    post(
        &mut david,
        DESIGNERS,
        211,
        &format!("typedmention <@{KEVIN}>"),
    )
    .await;
    post(
        &mut david,
        ALL_TALK,
        212,
        &format!("typedmention <@{DAVID}>"),
    )
    .await;
    let results: api::SearchResults = parse(
        &david
            .send(get(
                "/api/v1/search?q=typedmention&mentionsMe=true&has=mention,link",
            ))
            .await,
    );
    assert_eq!(ids(&results), [mine.id]);
    let theirs: api::SearchResults = parse(
        &kevin
            .send(get("/api/v1/search?q=typedmention&mentionsMe=true"))
            .await,
    );
    assert_eq!(theirs.messages.len(), 1);
    assert_eq!(theirs.messages[0].room_id, DESIGNERS);
    a.db()
        .write(move |tx| {
            campfire_db::Message::find(tx.conn(), mine.id)?.update(
                tx,
                campfire_db::MessageChanges {
                    markdown_source: Some("typedmention removed".into()),
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let current: api::SearchResults = parse(
        &david
            .send(get(&format!(
                "/api/v1/search?q=typedmention&mentionsMe=true&channelId={DESIGNERS}"
            )))
            .await,
    );
    assert!(current.messages.is_empty());
}
