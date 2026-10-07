//! S3 search on `/api/v1` (`campfire_api::search`): results with their chips, sections and
//! keyset paging, and the recent searches.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::json;

use super::api_tests::{ALL_PETS, app, get, json_body, parse};
use crate::controllers::presenters::test_support::{Browser, DAVID, KEVIN, Reply, Req};

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
    let mut ids = first
        .messages
        .iter()
        .chain(&second.messages)
        .map(|m| m.id)
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 45, "each once");
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
    let ids =
        |results: &api::SearchResults| results.messages.iter().map(|m| m.id).collect::<Vec<_>>();
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
    let theirs: api::SearchResults = parse(&kevin.send(get("/api/v1/search?q=needlepets")).await);
    assert!(theirs.messages.iter().all(|m| m.id != mine.id));
    let found: api::SearchResults = parse(&david.send(get("/api/v1/search?q=needlepets")).await);
    assert_eq!(ids(&found), [mine.id]);
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
