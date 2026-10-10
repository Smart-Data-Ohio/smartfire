//! The board API follows the classic board relation and write authority, and every writer
//! publishes the JSON twin of the classic board row on the room's sync topic.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use campfire_db::{BoardTagAssignment, ChannelThread, NewBoardTagAssignment};
use serde_json::{Value, json};

use super::api_tests::{Sync, app, get, json_body, parse, serve, tag};
use crate::controllers::presenters::test_support::{
    BENDER, Browser, DAVID, HQ, JASON, KEVIN, Reply, Req, TestApp, seed_clock,
};

const BOARD: i64 = 699448332;
const DESIGNERS: i64 = 654632876;
const PLANNED: i64 = 4;
const BLOCKED: i64 = 6;
const DONE: i64 = 7;
const AGENT: i64 = 773018776;

#[tokio::test]
async fn spa_api_board_catalog_crud_propagates_and_broadcasts() {
    let a = app(true).await.expect("the frozen default seed");
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut sync = Sync::connect(addr, &david.cookie_header(), &[format!("room:{BOARD}")]).await;
    sync.welcome().await;
    let path = format!("/api/v1/rooms/{BOARD}/board/tags");
    let reply = david.write(json_body(Method::POST, &path, &json!({"name":" Rust ","emoji":"🦀"}))).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let catalog: Value = parse(&reply);
    assert_eq!(catalog["tags"][0]["name"], "Rust");
    assert_eq!(catalog["tags"][0]["emoji"], "🦀");
    let id = catalog["tags"][0]["id"].as_i64().unwrap();
    sync.until(|event| matches!(&event.payload, api::SyncPayload::BoardAutomationsChanged(change) if change.room_id == BOARD), |_| false).await;
    let reply = david.write(json_body(Method::PATCH, &format!("{path}/{id}"), &json!({"name":"Release Notes","emoji":null}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    sync.until(|event| matches!(&event.payload, api::SyncPayload::BoardAutomationsChanged(change) if change.room_id == BOARD), |_| false).await;
    assert_eq!(a.db().read(|conn| ChannelThread::find(conn, PLANNED)?.tag_names(conn)).await.unwrap(), ["Release Notes", "release"]);
    let reply = david.send(get(&format!("/api/v1/rooms/{BOARD}/board"))).await;
    let listing: Value = parse(&reply);
    assert_eq!(listing["tags"][0]["name"], "Release Notes");
    assert_eq!(listing["tagsRequired"], false);
    assert_eq!(listing["defaultBoardTagId"], Value::Null);
    assert!(listing["tagCounts"].as_array().unwrap().iter().any(|row| row["name"] == "release"));
    let reply = david.write(json_body(Method::DELETE, &format!("{path}/{id}"), &json!({}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    sync.until(|event| matches!(&event.payload, api::SyncPayload::BoardAutomationsChanged(change) if change.room_id == BOARD), |_| false).await;
    assert_eq!(a.db().read(|conn| ChannelThread::find(conn, PLANNED)?.tag_names(conn)).await.unwrap(), ["release"]);
    server.abort();
}

#[tokio::test]
async fn spa_api_board_catalog_permissions_limits_and_reorder() {
    let a = app(true).await.expect("the frozen default seed");
    sql(&a, format!("UPDATE users SET role=0 WHERE id={JASON}; UPDATE rooms SET creator_id={JASON} WHERE id={BOARD}")).await;
    a.db().write(|tx| campfire_db::Membership::create_default(tx, BOARD, KEVIN)).await.unwrap();
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let path = format!("/api/v1/rooms/{BOARD}/board/tags");
    let reply = kevin.write(json_body(Method::POST, &path, &json!({"name":"denied","emoji":null}))).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
    for (name, message) in [(" ".to_string(), "can't be blank"), ("é".repeat(21), "is too long (maximum is 20 characters)")] {
        let reply = jason.write(json_body(Method::POST, &path, &json!({"name":name,"emoji":null}))).await;
        validation(&reply, "name", message);
    }
    let mut ids = Vec::new();
    for i in 0..20 {
        let reply = jason.write(json_body(Method::POST, &path, &json!({"name":format!("tag-{i}"),"emoji":null}))).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let catalog: Value = parse(&reply);
        ids.push(catalog["tags"][i]["id"].as_i64().unwrap());
    }
    let reply = david.write(json_body(Method::POST, &path, &json!({"name":"overflow","emoji":null}))).await;
    validation(&reply, "tags", "are limited to 20 per board");
    let reply = david.write(json_body(Method::PATCH, &format!("{path}/{}", ids[1]), &json!({"name":" TAG-0 ","emoji":null}))).await;
    validation(&reply, "name", "has already been taken");
    let reply = david.write(json_body(Method::PUT, &format!("{path}/order"), &json!({"tagIds":[ids[0],ids[0]]}))).await;
    validation(&reply, "tagIds", "must contain every board tag exactly once");
    ids.reverse();
    let reply = david.write(json_body(Method::PUT, &format!("{path}/order"), &json!({"tagIds":ids}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let catalog: Value = parse(&reply);
    assert_eq!(catalog["tags"].as_array().unwrap().iter().map(|tag| tag["id"].as_i64().unwrap()).collect::<Vec<_>>(), ids);
    for (method, suffix, body) in [(Method::PATCH, format!("/{}",ids[0]), json!({"name":"no","emoji":null})), (Method::DELETE, format!("/{}",ids[0]), json!({})), (Method::PUT, "/order".into(), json!({"tagIds":ids}))] {
        let reply = kevin.write(json_body(method, &format!("{path}{suffix}"), &body)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.text());
    }
    let reply = david.write(json_body(Method::POST, &format!("/api/v1/rooms/{HQ}/board/tags"), &json!({"name":"no","emoji":null}))).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let reply = kevin.write(json_body(Method::PATCH, &format!("/api/v1/rooms/{BOARD}/board"), &json!({"tagsRequired":true,"defaultBoardTagId":null}))).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let reply = kevin.send(get(&path)).await;
    assert_eq!(reply.status, StatusCode::OK);
    sql(&a, format!("DELETE FROM memberships WHERE user_id={KEVIN} AND room_id={BOARD}")).await;
    let reply = kevin.send(get(&path)).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let other = a.db().write(|tx| campfire_db::Room::create_for(tx, campfire_db::RoomType::Board, Some("Other board"), DAVID, &[DAVID])).await.unwrap();
    let other_path = format!("/api/v1/rooms/{}/board", other.id);
    let reply = david.write(json_body(Method::PATCH, &format!("{other_path}/tags/{}", ids[0]), &json!({"name":"stolen","emoji":null}))).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let reply = david.write(json_body(Method::PATCH, &other_path, &json!({"tagsRequired":true,"defaultBoardTagId":ids[0]}))).await;
    validation(&reply, "defaultBoardTagId", "must belong to this board");
}

#[tokio::test]
async fn spa_api_board_catalog_required_default_create_edit_and_legacy() {
    let a = app(true).await.expect("the frozen default seed");
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{BOARD}/board");
    let reply = david.write(json_body(Method::POST, &format!("{path}/tags"), &json!({"name":"Bug Report","emoji":null}))).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let catalog: Value = parse(&reply);
    let id = catalog["tags"][0]["id"].as_i64().unwrap();
    let reply = david.write(json_body(Method::PATCH, &path, &json!({"tagsRequired":true,"defaultBoardTagId":null}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let create = format!("/api/v1/rooms/{BOARD}/posts");
    for tags in [json!([]),json!(["legacy"])] {
        let mut body = new_post("Required tag"); body["tags"] = tags;
        let reply = david.write(json_body(Method::POST, &create, &body)).await;
        validation(&reply, "tags", "must include a catalog tag");
    }
    let mut body = new_post("Curated tag"); body["tags"] = json!([" bug report ","legacy"]);
    let reply = david.write(json_body(Method::POST, &create, &body)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!(detail.thread.work.unwrap().tags, ["Bug Report", "legacy"]);
    let edit = format!("/api/v1/threads/{}/work",detail.thread.id);
    let reply = david.write(json_body(Method::PATCH, &edit, &json!({"tags":[]}))).await;
    validation(&reply, "tags", "must include a catalog tag");
    // A title-only edit of an existing free-text post remains possible.
    let reply = david.write(json_body(Method::PATCH, &format!("/api/v1/threads/{PLANNED}"), &json!({"name":"Legacy post"}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let reply = david.write(json_body(Method::PATCH, &path, &json!({"tagsRequired":true,"defaultBoardTagId":id}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let reply = david.write(json_body(Method::POST, &create, &new_post("Default tag"))).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!(detail.thread.work.unwrap().tags, ["Bug Report"]);
    let reply = david.write(json_body(Method::PATCH, &edit, &json!({"tags":["legacy"]}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!(detail.thread.work.unwrap().tags, ["Bug Report", "legacy"]);
    let reply = david.write(json_body(Method::PATCH, &path, &json!({"tagsRequired":true,"defaultBoardTagId":-1}))).await;
    validation(&reply, "defaultBoardTagId", "must belong to this board");
    let reply = david.write(json_body(Method::DELETE, &format!("{path}/tags/{id}"), &json!({}))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let catalog: Value = parse(&reply);
    assert_eq!(catalog["defaultBoardTagId"], Value::Null);
    assert_eq!(catalog["tagsRequired"], true);
    let reply = david.write(json_body(Method::POST, &create, &new_post("Deleted default"))).await;
    validation(&reply, "tags", "must include a catalog tag");
}

#[tokio::test]
async fn spa_api_board_catalog_rechecks_membership_at_writer_boundary() {
    let a = app(true).await.expect("the frozen default seed");
    let mut david = a.sign_in(DAVID).await;
    let db = a.db().clone();
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let blocker = tokio::spawn(async move {
        db.write(move |tx| {
            entered.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(10)).unwrap();
            tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?", [BOARD,DAVID])?;
            Ok(())
        }).await.unwrap();
    });
    ready.await.unwrap();
    let mut request = Box::pin(david.write(json_body(Method::POST, &format!("/api/v1/rooms/{BOARD}/board/tags"), &json!({"name":"stale","emoji":null}))));
    tokio::select! {
        reply = &mut request => panic!("writer was held: {}", reply.text()),
        _ = wait_for_queued_writes(&a, 1) => (),
    }
    release.send(()).unwrap();
    blocker.await.unwrap();
    let reply = request.await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.text());
    assert_eq!(a.db().read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM board_tags WHERE room_id=?", [BOARD], |row| row.get::<_,i64>(0))?)).await.unwrap(), 0);
}

async fn sql(a: &TestApp, statements: impl Into<String>) {
    let statements = statements.into();
    a.db()
        .write(move |tx| {
            tx.conn().execute_batch(&statements)?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn grant_bender(a: &TestApp) {
    a.db()
        .write(|tx| {
            for capability in ["post_messages", "manage_threads", "read_messages"] {
                tx.conn().execute(
                    "INSERT INTO agent_grants (agent_id, room_id, capability, granted_by_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                    rusqlite::params![AGENT, BOARD, capability, DAVID, tx.now(), tx.now()],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
}

async fn deny_bender_posting(a: &TestApp) {
    // Even a revoked grant opts an agent out of the legacy no-grants allowance.
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "INSERT INTO agent_grants (agent_id, room_id, capability, granted_by_id, revoked_at, created_at, updated_at) VALUES (?, ?, 'post_messages', ?, ?, ?, ?)",
                rusqlite::params![AGENT, BOARD, DAVID, tx.now(), tx.now(), tx.now()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

async fn hold_writer(a: &TestApp) -> (std::sync::mpsc::Sender<()>, tokio::task::JoinHandle<()>) {
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let db = a.db().clone();
    let blocker = tokio::spawn(async move {
        db.write(move |_| {
            entered.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(10)).unwrap();
            Ok(())
        })
        .await
        .unwrap();
    });
    ready.await.unwrap();
    (release, blocker)
}

async fn wait_for_queued_writes(a: &TestApp, count: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while a.db().queued_writes() < count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("both HTTP writes queued behind the held writer");
}

async fn listing(browser: &mut Browser<'_>, query: &str) -> api::BoardListing {
    let reply = browser
        .send(get(&format!("/api/v1/rooms/{BOARD}/board{query}")))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    parse(&reply)
}

fn ids(list: &api::BoardListing) -> Vec<i64> {
    list.posts.iter().map(|row| row.thread.id).collect()
}

fn new_post(name: &str) -> Value {
    json!({"name": name, "status": "planned", "ownerId": null, "tags": [], "message": null})
}

fn brief(client_id: &str, source: &str) -> Value {
    json!({"clientMessageId": client_id, "markdownSource": source})
}

fn validation(reply: &Reply, field: &str, message: &str) {
    assert_eq!(
        (reply.status, tag(reply)),
        (StatusCode::UNPROCESSABLE_ENTITY, "Validation".into()),
        "{}",
        reply.text()
    );
    let api::ApiError::Validation { fields, .. } = parse::<api::ApiErrorResponse>(reply).error
    else {
        unreachable!()
    };
    assert_eq!(fields.get(field), Some(&vec![message.to_string()]));
}

fn created(thread_id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::ThreadCreated(thread) if thread.id == thread_id)
}

fn updated(thread_id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| matches!(&event.payload, api::SyncPayload::ThreadUpdated(thread) if thread.id == thread_id)
}

fn topics(thread_id: i64) -> Vec<String> {
    vec![
        format!("room:{BOARD}"),
        format!("thread:{thread_id}"),
        format!("thread:{DONE}"),
    ]
}

/// Drain the ring to a message on a different post after all deferred publications settled.
/// This catches duplicate publications without relying on a quiet-time timeout.
async fn no_more(
    a: &TestApp,
    browser: &mut Browser<'_>,
    sync: &mut Sync,
    forbidden: impl Fn(&api::SyncEvent) -> bool,
    marker: &str,
) {
    tokio::time::timeout(
        Duration::from_secs(5),
        a.booted.app.broadcasts.settle_sync(),
    )
    .await
    .expect("deferred publications settled");
    let reply = browser
        .write(json_body(
            Method::POST,
            &format!("/api/v1/threads/{DONE}/messages"),
            &brief(marker, marker),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    sync.until(
        |event| {
            matches!(&event.payload, api::SyncPayload::MessageCreated(message)
                if message.client_message_id == marker)
        },
        forbidden,
    )
    .await;
}

async fn both_updates(sync: &mut Sync, thread_id: i64) -> Vec<api::Thread> {
    let mut seen = Vec::<String>::new();
    let mut threads = Vec::new();
    while seen.len() < 2 {
        let event = sync.until(updated(thread_id), |_| false).await;
        assert!(
            !seen.contains(&event.topic),
            "duplicate thread.updated on {}",
            event.topic
        );
        assert!(
            [format!("room:{BOARD}"), format!("thread:{thread_id}")].contains(&event.topic),
            "unexpected topic {}",
            event.topic
        );
        seen.push(event.topic);
        let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
            unreachable!()
        };
        threads.push(thread);
    }
    threads
}

#[tokio::test]
async fn spa_api_board_listing_filters_orders_and_projects_work_and_memberships() {
    let a = app(true).await.expect("the frozen default seed");
    sql(
        &a,
        "DELETE FROM thread_tags WHERE channel_thread_id IN (4,5,6,7);
         INSERT INTO thread_tags(channel_thread_id,name,created_at,updated_at) VALUES
         (4,'api','2026-03-02 15:00:00','2026-03-02 15:00:00'),
         (5,'api','2026-03-02 15:00:00','2026-03-02 15:00:00'),
         (5,'beta','2026-03-02 15:00:00','2026-03-02 15:00:00'),
         (6,'api','2026-03-02 15:00:00','2026-03-02 15:00:00'),
         (6,'release','2026-03-02 15:00:00','2026-03-02 15:00:00'),
         (7,'zeta','2026-03-02 15:00:00','2026-03-02 15:00:00');",
    )
    .await;
    let mut david = a.sign_in(DAVID).await;
    let open = listing(&mut david, "").await;
    assert_eq!(
        (
            open.room_id,
            open.status,
            open.owner.as_str(),
            open.tag.as_str(),
            open.page
        ),
        (BOARD, api::BoardStatusFilter::Open, "anyone", "", 1)
    );
    assert_eq!(ids(&open), [6, 5, 4]);
    assert!(open.any_posts && !open.has_more && open.can_administer);
    assert_eq!(open.digest, None);
    assert_eq!(
        open.owner_options,
        [
            api::BoardOwnerOption {
                user_id: BENDER,
                agent: true
            },
            api::BoardOwnerOption {
                user_id: DAVID,
                agent: false
            },
            api::BoardOwnerOption {
                user_id: JASON,
                agent: false
            },
        ]
    );
    assert_eq!(
        open.tag_counts
            .iter()
            .map(|tag| (tag.name.as_str(), tag.count))
            .collect::<Vec<_>>(),
        [("api", 3), ("beta", 1), ("release", 1), ("zeta", 1)]
    );
    for row in &open.posts {
        assert_eq!(row.thread.parent_message_id, None);
        assert_eq!(
            row.membership.as_ref().map(|member| member.thread_id),
            Some(row.thread.id)
        );
        assert_eq!(
            row.membership.as_ref().unwrap().involvement,
            api::ThreadInvolvement::Mentions
        );
        let work = row
            .thread
            .work
            .as_ref()
            .expect("board posts always have work");
        assert!(!work.tags.is_empty());
        assert!(work.tags.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            open.users
                .iter()
                .any(|user| user.id == row.thread.creator_id)
        );
    }
    for option in &open.owner_options {
        assert!(open.users.iter().any(|user| user.id == option.user_id));
    }
    let mut user_ids = open.users.iter().map(|user| user.id).collect::<Vec<_>>();
    user_ids.sort_unstable();
    user_ids.dedup();
    assert_eq!(user_ids.len(), open.users.len(), "users appear once");

    for (query, wanted, status, owner, tag_name) in [
        (
            "?status=all",
            vec![7, 6, 5, 4],
            api::BoardStatusFilter::All,
            "anyone",
            "",
        ),
        (
            "?status=done",
            vec![7],
            api::BoardStatusFilter::Done,
            "anyone",
            "",
        ),
        (
            "?status=unknown&owner=nobody",
            vec![6, 5, 4],
            api::BoardStatusFilter::Open,
            "anyone",
            "",
        ),
        (
            "?owner=me",
            vec![5, 4],
            api::BoardStatusFilter::Open,
            "me",
            "",
        ),
        (
            "?status=all&owner=agents",
            vec![6],
            api::BoardStatusFilter::All,
            "agents",
            "",
        ),
        (
            "?status=all&owner=127326141",
            vec![7, 5, 4],
            api::BoardStatusFilter::All,
            "127326141",
            "",
        ),
        (
            "?status=all&tag=%20ApI%20",
            vec![6, 5, 4],
            api::BoardStatusFilter::All,
            "anyone",
            "api",
        ),
        (
            "?status=all&owner=me&tag=beta",
            vec![5],
            api::BoardStatusFilter::All,
            "me",
            "beta",
        ),
        (
            "?tag=absent",
            vec![],
            api::BoardStatusFilter::Open,
            "anyone",
            "absent",
        ),
    ] {
        let filtered = listing(&mut david, query).await;
        assert_eq!(ids(&filtered), wanted, "{query}");
        assert_eq!(
            (
                filtered.status,
                filtered.owner.as_str(),
                filtered.tag.as_str()
            ),
            (status, owner, tag_name),
            "{query}"
        );
        assert!(filtered.any_posts, "an empty filter is not an empty board");
        assert_eq!(
            filtered.tag_counts, open.tag_counts,
            "counts ignore filters"
        );
        assert_eq!(
            filtered.owner_options, open.owner_options,
            "options ignore filters"
        );
    }
    let mut jason = a.sign_in(JASON).await;
    let others = listing(&mut jason, "?status=all").await;
    assert!(others.posts.iter().all(|post| post.membership.is_none()));
    // The Agents relation retains an unavailable owner even though its picker is inactive.
    sql(&a, format!("UPDATE users SET status=1 WHERE id={BENDER}")).await;
    let agents = listing(&mut david, "?owner=agents").await;
    assert_eq!(ids(&agents), [BLOCKED]);
    assert!(!agents.posts[0].thread.work.as_ref().unwrap().owner_active);
    assert!(
        !agents
            .owner_options
            .iter()
            .any(|option| option.user_id == BENDER)
    );
}

#[tokio::test]
async fn spa_api_board_paging_is_cumulative_clamped_and_probes_one_more() {
    let a = app(true).await.expect("the frozen default seed");
    a.db()
        .write(|tx| {
            for id in 10_000..=11_000 {
                tx.conn().execute(
                    "INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,last_activity_at,created_at,updated_at) VALUES(?,?,?,'Paged post','planned',?,?,?)",
                    rusqlite::params![id, BOARD, DAVID, tx.now(), tx.now(), tx.now()],
                )?;
                tx.conn().execute(
                    "INSERT INTO thread_tags(channel_thread_id,name,created_at,updated_at) VALUES(?,'paged',?,?)",
                    rusqlite::params![id, tx.now(), tx.now()],
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let first = listing(&mut david, "?tag=paged").await;
    assert_eq!(
        (first.page, first.posts.len(), first.has_more),
        (1, 50, true)
    );
    assert_eq!(ids(&first), (10_951..=11_000).rev().collect::<Vec<_>>());
    let second = listing(&mut david, "?tag=paged&page=2").await;
    assert_eq!(
        (second.page, second.posts.len(), second.has_more),
        (2, 100, true)
    );
    assert_eq!(&second.posts[..50], &first.posts);
    for (raw, page) in [
        ("0", 1),
        ("-3", 1),
        ("nonsense", 1),
        ("2tail", 2),
        ("2_0tail", 20),
        ("99999999999999999999999", 20),
    ] {
        let window = listing(&mut david, &format!("?tag=paged&page={raw}")).await;
        assert_eq!(
            (window.page, window.posts.len()),
            (page, page as usize * 50),
            "page={raw}"
        );
        assert!(window.has_more, "even the capped window has a probe row");
    }
    sql(
        &a,
        "DELETE FROM thread_tags WHERE name='paged' AND channel_thread_id<=10900",
    )
    .await;
    let last = listing(&mut david, "?tag=paged&page=2").await;
    assert_eq!((last.posts.len(), last.has_more), (100, false));
    assert!(listing(&mut david, "?tag=paged").await.has_more);
    sql(
        &a,
        "DELETE FROM thread_tags WHERE name='paged' AND channel_thread_id<=10950",
    )
    .await;
    let exact = listing(&mut david, "?tag=paged").await;
    assert_eq!((exact.posts.len(), exact.has_more), (50, false));
}

#[tokio::test]
async fn spa_api_board_listing_batches_tags_owners_and_memberships_at_ten_and_a_hundred_posts() {
    let a = app(true)
        .await
        .expect("the frozen default seed")
        .without_job_runner()
        .await;
    grant_bender(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{BOARD}/board?tag=batched&page=2");
    let mut reads = Vec::new();
    for (start, end) in [(10_000, 10_010), (10_010, 10_100)] {
        a.db().write(move |tx| {
            for id in start..end {
                let owner = if id % 2 == 0 { DAVID } else { BENDER };
                tx.conn().execute(
                    "INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,last_activity_at,created_at,updated_at) VALUES(?,?,?,'Batched post','planned',?,?,?,?)",
                    rusqlite::params![id, BOARD, owner, owner, tx.now(), tx.now(), tx.now()],
                )?;
                tx.conn().execute(
                    "INSERT INTO thread_tags(channel_thread_id,name,created_at,updated_at) VALUES(?,'batched',?,?)",
                    rusqlite::params![id, tx.now(), tx.now()],
                )?;
                if id % 2 == 0 {
                    tx.conn().execute(
                        "INSERT INTO thread_memberships(thread_id,user_id,involvement,joined_at,created_at,updated_at) VALUES(?,?,'mentions',?,?,?)",
                        rusqlite::params![id, DAVID, tx.now(), tx.now(), tx.now()],
                    )?;
                }
            }
            Ok(())
        }).await.unwrap();
        assert_eq!(david.send(get(&path)).await.status, StatusCode::OK);
        let log = a.db().capture_read_queries();
        let reply = david.send(get(&path)).await;
        a.db().stop_capturing_read_queries();
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let list: api::BoardListing = parse(&reply);
        assert_eq!(list.posts.len(), (end - 10_000) as usize);
        assert!(!list.has_more);
        for row in &list.posts {
            let work = row.thread.work.as_ref().unwrap();
            assert_eq!(work.tags, ["batched"]);
            assert!(work.owner_active);
            assert_eq!(
                work.owner.as_ref().map(|owner| owner.id),
                Some(row.thread.creator_id)
            );
            assert_eq!(row.membership.is_some(), row.thread.id % 2 == 0);
        }
        reads.push(log.lock().unwrap().len());
    }
    assert_eq!(
        reads[0], reads[1],
        "board listing SELECTs must stay flat for 10/100 posts: {reads:?}"
    );
}

#[tokio::test]
async fn spa_api_board_empty_digest_and_administration_follow_classic_facts() {
    let a = app(true).await.expect("the frozen default seed");
    sql(
        &a,
        format!("UPDATE users SET role=0 WHERE id IN ({DAVID},{JASON})"),
    )
    .await;
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    assert!(
        listing(&mut david, "").await.can_administer,
        "creator need not be an admin"
    );
    assert!(!listing(&mut jason, "").await.can_administer);
    sql(&a, format!("UPDATE users SET role=1 WHERE id={JASON}")).await;
    assert!(listing(&mut jason, "").await.can_administer);

    sql(
        &a,
        format!("INSERT INTO messages(id,room_id,creator_id,client_message_id,markdown_source,system_note,created_at,updated_at) VALUES
         (900001001,{BOARD},{DAVID},'older-digest','Older digest',1,'2026-03-01 15:00:00','2026-03-01 15:00:00'),
         (900001002,{BOARD},{DAVID},'latest-digest','Latest stale work',1,'2026-03-02 15:00:00','2026-03-02 15:00:00');
         INSERT INTO action_text_rich_texts(record_type,record_id,name,body,created_at,updated_at) VALUES
         ('Message',900001001,'body','<p>Older digest</p>','2026-03-01 15:00:00','2026-03-01 15:00:00'),
         ('Message',900001002,'body','<p>Latest stale work</p>','2026-03-02 15:00:00','2026-03-02 15:00:00');
         INSERT INTO board_stale_digests(room_id,digest_on,message_id,created_at,updated_at) VALUES
         ({BOARD},'2026-03-01',900001001,'2026-03-01 15:00:00','2026-03-01 15:00:00'),
         ({BOARD},'2026-03-02',900001002,'2026-03-02 15:00:00','2026-03-02 15:00:00');"),
    )
    .await;
    assert_eq!(
        listing(&mut david, "").await.digest,
        Some(api::BoardDigest {
            date: "2026-03-02".into(),
            text: "Latest stale work".into()
        })
    );
    // A failed newer claim is the latest record, so classic doesn't display an older digest.
    sql(&a, format!("INSERT INTO board_stale_digests(room_id,digest_on,created_at,updated_at) VALUES({BOARD},'2026-03-03','2026-03-03 15:00:00','2026-03-03 15:00:00')")).await;
    assert_eq!(listing(&mut david, "").await.digest, None);

    // Keep a real board/member, with no posts at all, separate from an unmatched filter.
    sql(
        &a,
        format!("UPDATE rooms SET type='Rooms::Board' WHERE id={HQ}"),
    )
    .await;
    let reply = david.send(get(&format!("/api/v1/rooms/{HQ}/board"))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let empty: api::BoardListing = parse(&reply);
    assert!(empty.posts.is_empty() && !empty.any_posts && !empty.has_more);
    assert!(empty.tag_counts.is_empty());
}

#[tokio::test]
async fn spa_api_board_endpoints_require_an_alive_board_and_human_member() {
    let a = app(true).await.expect("the frozen default seed");
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    for room in [DESIGNERS, 999999] {
        for suffix in ["board", "posts/new"] {
            let reply = david
                .send(get(&format!("/api/v1/rooms/{room}/{suffix}")))
                .await;
            assert_eq!(
                (reply.status, tag(&reply)),
                (StatusCode::NOT_FOUND, "NotFound".into())
            );
        }
        let reply = david
            .write(json_body(
                Method::POST,
                &format!("/api/v1/rooms/{room}/posts"),
                &new_post("Wrong room"),
            ))
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::NOT_FOUND, "NotFound".into())
        );
    }
    for suffix in ["board", "posts/new"] {
        let reply = kevin
            .send(get(&format!("/api/v1/rooms/{BOARD}/{suffix}")))
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::NOT_FOUND, "NotFound".into())
        );
    }
    let reply = kevin
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{BOARD}/posts"),
            &new_post("No access"),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );

    crate::controllers::agent_http_tests::initialize(&a).await;
    let authorization = format!("Bearer {}", crate::controllers::agent_http_tests::SECRET);
    for suffix in ["board", "posts/new"] {
        let reply = a
            .anonymous()
            .send(
                get(&format!("/api/v1/rooms/{BOARD}/{suffix}"))
                    .header("authorization", &authorization),
            )
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::FORBIDDEN, "Forbidden".into())
        );
    }
    let reply = a
        .anonymous()
        .send(
            json_body(
                Method::POST,
                &format!("/api/v1/rooms/{BOARD}/posts"),
                &new_post("Agent token"),
            )
            .header("authorization", &authorization),
        )
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::FORBIDDEN, "Forbidden".into())
    );
    let bot_key = a
        .db()
        .write(|tx| campfire_db::User::find(tx.conn(), BENDER)?.reset_bot_key(tx))
        .await
        .unwrap();
    for suffix in ["board", "posts/new"] {
        let reply = a
            .anonymous()
            .send(get(&format!(
                "/api/v1/rooms/{BOARD}/{suffix}?bot_key={bot_key}"
            )))
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::FORBIDDEN, "Forbidden".into())
        );
    }

    let reply = a
        .anonymous()
        .send(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{BOARD}/posts?bot_key={bot_key}"),
            &new_post("Bot key"),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::FORBIDDEN, "Forbidden".into())
    );

    sql(
        &a,
        format!("UPDATE rooms SET deleted_at='2026-03-02 16:00:00' WHERE id={BOARD}"),
    )
    .await;
    for suffix in ["board", "posts/new"] {
        let reply = david
            .send(get(&format!("/api/v1/rooms/{BOARD}/{suffix}")))
            .await;
        assert_eq!(
            (reply.status, tag(&reply)),
            (StatusCode::NOT_FOUND, "NotFound".into())
        );
    }
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{BOARD}/posts"),
            &new_post("Deleted room"),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::NOT_FOUND, "NotFound".into())
    );
}

#[tokio::test]
async fn spa_api_board_new_form_groups_eligible_owners_and_suggests_board_tags() {
    let a = app(true).await.expect("the frozen default seed");
    crate::controllers::agent_http_tests::initialize(&a).await;
    deny_bender_posting(&a).await;
    sql(&a, format!("INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES({BOARD},{KEVIN},'2026-03-02 15:00:00','2026-03-02 15:00:00'); UPDATE users SET status=1 WHERE id={KEVIN}")).await;
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{BOARD}/posts/new");
    let form: api::BoardPostForm = parse(&david.send(get(&path)).await);
    assert_eq!(
        form.owner_candidates
            .iter()
            .map(|owner| owner.user_id)
            .collect::<Vec<_>>(),
        [DAVID, JASON]
    );
    assert_eq!(form.tag_suggestions, ["release", "rust"]);
    assert_eq!(form.users.len(), 2);
    grant_bender(&a).await;
    let reply = david.send(get(&path)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let form: api::BoardPostForm = parse(&reply);
    assert_eq!(
        form.owner_candidates
            .iter()
            .map(|owner| owner.user_id)
            .collect::<Vec<_>>(),
        [DAVID, JASON, BENDER],
        "humans before agents, each group by name"
    );
    assert_eq!(form.users.len(), 3);
    assert!(
        form.users
            .iter()
            .find(|user| user.id == BENDER)
            .unwrap()
            .agent
            .is_some()
    );
    for owner in &form.owner_candidates {
        assert!(form.users.iter().any(|user| user.id == owner.user_id));
    }
}

#[tokio::test]
async fn spa_api_board_creation_validates_classic_fields_and_changes_nothing_on_failure() {
    let a = app(true).await.expect("the frozen default seed");
    crate::controllers::agent_http_tests::initialize(&a).await;
    deny_bender_posting(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{BOARD}/posts");
    let before = a
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM channel_threads", [], |row| {
                    row.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    for (field, value, message) in [
        ("name", json!("  "), "can't be blank"),
        (
            "name",
            json!("a".repeat(101)),
            "is too long (maximum is 100 characters)",
        ),
        (
            "tags",
            json!(["a", "b", "c", "d", "e", "f"]),
            "are limited to 5 per post",
        ),
        (
            "tags",
            json!(["a".repeat(31)]),
            "must be at most 30 characters",
        ),
        (
            "tags",
            json!(["bad_tag"]),
            "use lowercase letters, digits, and hyphens",
        ),
        (
            "ownerId",
            json!(KEVIN),
            "must be an active human member of the parent room",
        ),
        (
            "ownerId",
            json!(BENDER),
            "must be an active agent member of the parent room with permission to post",
        ),
        (
            "message",
            brief("overlong-brief", &"a".repeat(50_001)),
            "is too long (maximum is 50000 characters)",
        ),
        (
            "message",
            json!({"clientMessageId": "existing-reply-target", "markdownSource": "Brief", "replyToMessageId": 935962049}),
            "replyToMessageId isn't a message on this timeline",
        ),
        (
            "message",
            json!({"clientMessageId": "missing-reply-target", "markdownSource": "Brief", "replyToMessageId": 999999999}),
            "replyToMessageId isn't a message on this timeline",
        ),
    ] {
        let mut body = new_post("Invalid draft");
        body[field] = value;
        let reply = david.write(json_body(Method::POST, &path, &body)).await;
        validation(&reply, field, message);
    }
    let after = a
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT COUNT(*) FROM channel_threads", [], |row| {
                    row.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(
        after, before,
        "even an invalid opener rolls back its new post"
    );
    assert!(
        a.db()
            .read(|conn| campfire_db::Message::find_duplicate(conn, BOARD, DAVID, "overlong-brief"))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn spa_api_board_creation_normalizes_tags_records_assignment_and_opener_and_retries_once() {
    let a = app(true).await.expect("the frozen default seed");
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let mut sync = Sync::connect(
        addr,
        &jason.cookie_header(),
        &[format!("room:{BOARD}"), format!("thread:{DONE}")],
    )
    .await;
    sync.welcome().await;
    let body = json!({"name": "Ship the release", "status": "blocked", "ownerId": JASON, "tags": [" Rust ", "api", "API", "", "rust"], "message": brief("board-opener-1", "## Context\n\nFix **this** first")});
    let path = format!("/api/v1/rooms/{BOARD}/posts");
    let reply = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    let id = detail.thread.id;
    assert_eq!(
        (
            detail.thread.room_id,
            detail.thread.creator_id,
            detail.thread.parent_message_id
        ),
        (BOARD, DAVID, None)
    );
    assert_eq!(detail.thread.name, "Ship the release");
    assert_eq!(detail.thread.reply_count, 1);
    assert_eq!(
        detail.membership.as_ref().map(|member| member.thread_id),
        Some(id)
    );
    assert_eq!(detail.parent_message, None);
    let facts = detail.thread.work.as_ref().unwrap();
    assert_eq!(
        (
            facts.status,
            facts.owner.as_ref().map(|owner| owner.id),
            facts.tags.as_slice()
        ),
        (
            api::WorkStatus::Blocked,
            Some(JASON),
            ["api".to_string(), "rust".to_string()].as_slice()
        )
    );
    let (messages, history) = a
        .db()
        .read(move |conn| {
            Ok((
                campfire_db::Message::in_thread(conn, id)?,
                campfire_db::WorkThreadEvent::for_thread(conn, id)?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].board_post_opener);
    assert_eq!(messages[0].client_message_id, "board-opener-1");
    assert_eq!(
        messages[0].markdown_source.as_deref(),
        Some("## Context\n\nFix **this** first")
    );
    assert!(
        history
            .iter()
            .any(|entry| entry.to_owner_id == Some(JASON) && entry.from_owner_id.is_none())
    );
    let event = sync.until(created(id), |_| false).await;
    assert_eq!(event.topic, format!("room:{BOARD}"));
    let api::SyncPayload::ThreadCreated(thread) = event.payload else {
        unreachable!()
    };
    assert_eq!(thread.work.as_ref().unwrap().tags, ["api", "rust"]);
    assert_eq!(thread.reply_count, 1);

    let retry = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(retry.status, StatusCode::OK, "{}", retry.text());
    let retried: api::ThreadDetail = parse(&retry);
    assert_eq!(retried.thread.id, id);
    assert_eq!(
        a.db()
            .read(move |conn| Ok(campfire_db::Message::in_thread(conn, id)?.len()))
            .await
            .unwrap(),
        1
    );
    no_more(
        &a,
        &mut david,
        &mut sync,
        created(id),
        "board-create-marker",
    )
    .await;
    server.abort();
}

#[tokio::test]
async fn spa_api_board_overlapping_creation_retries_commit_one_post_and_opener() {
    let a = app(true).await.expect("the frozen default seed");
    let (addr, server) = serve(&a).await;
    let mut first = a.sign_in(DAVID).await;
    let mut second = a.sign_in(DAVID).await;
    first.authenticity_token().await;
    second.authenticity_token().await;
    let jason = a.sign_in(JASON).await;
    let mut sync = Sync::connect(
        addr,
        &jason.cookie_header(),
        &[format!("room:{BOARD}"), format!("thread:{DONE}")],
    )
    .await;
    sync.welcome().await;
    let mut body = new_post("Concurrent board creation");
    body["message"] = brief("concurrent-board-opener", "A single brief");
    let path = format!("/api/v1/rooms/{BOARD}/posts");
    let (release, blocker) = hold_writer(&a).await;
    let (left, right, ()) = tokio::join!(
        first.write(json_body(Method::POST, &path, &body)),
        second.write(json_body(Method::POST, &path, &body)),
        async {
            wait_for_queued_writes(&a, 2).await;
            release.send(()).unwrap();
            blocker.await.unwrap();
        },
    );
    let mut statuses = [left.status, right.status];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::CREATED],
        "left: {}; right: {}",
        left.text(),
        right.text()
    );
    let left: api::ThreadDetail = parse(&left);
    let right: api::ThreadDetail = parse(&right);
    assert_eq!(left.thread.id, right.thread.id);
    let id = left.thread.id;
    let (posts, messages) = a
        .db()
        .read(move |conn| {
            Ok((
                conn.query_row(
                    "SELECT COUNT(*) FROM channel_threads WHERE name='Concurrent board creation'",
                    [],
                    |row| row.get::<_, i64>(0),
                )?,
                campfire_db::Message::in_thread(conn, id)?,
            ))
        })
        .await
        .unwrap();
    assert_eq!((posts, messages.len()), (1, 1));
    assert_eq!(messages[0].client_message_id, "concurrent-board-opener");
    let event = sync.until(created(id), |_| false).await;
    assert_eq!(event.topic, format!("room:{BOARD}"));
    no_more(
        &a,
        &mut first,
        &mut sync,
        created(id),
        "board-concurrent-create-marker",
    )
    .await;
    server.abort();
}

#[tokio::test]
async fn spa_api_board_creation_allows_agent_owner_and_blank_brief_creates_no_message() {
    let a = app(true).await.expect("the frozen default seed");
    crate::controllers::agent_http_tests::initialize(&a).await;
    grant_bender(&a).await;
    let mut jason = a.sign_in(JASON).await;
    let mut body = new_post("An agent can pick this up");
    body["ownerId"] = json!(BENDER);
    body["message"] = brief("blank-board-brief", "  \n\t");
    let reply = jason
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{BOARD}/posts"),
            &body,
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!(detail.thread.reply_count, 0);
    assert_eq!(
        detail
            .thread
            .work
            .as_ref()
            .unwrap()
            .owner
            .as_ref()
            .map(|owner| owner.id),
        Some(BENDER)
    );
    let id = detail.thread.id;
    assert!(
        a.db()
            .read(move |conn| campfire_db::Message::in_thread(conn, id))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn spa_api_board_creation_runs_tag_assignment_after_commit() {
    let a = app(true).await.expect("the frozen default seed");
    a.db()
        .write(|tx| {
            BoardTagAssignment::create(
                tx,
                NewBoardTagAssignment {
                    room_id: BOARD,
                    tag: "route-me".into(),
                    assignee_id: JASON,
                    created_by_id: DAVID,
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let mut sync = Sync::connect(
        addr,
        &jason.cookie_header(),
        &[format!("room:{BOARD}"), format!("thread:{DONE}")],
    )
    .await;
    sync.welcome().await;
    let mut body = new_post("Automatically routed");
    body["tags"] = json!(["route-me"]);
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{BOARD}/posts"),
            &body,
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert_eq!(
        detail
            .thread
            .work
            .as_ref()
            .unwrap()
            .owner
            .as_ref()
            .map(|owner| owner.id),
        Some(JASON)
    );
    assert!(detail.work.as_ref().unwrap().history.iter().any(|entry| {
        entry.kind == api::WorkHistoryKind::Assignment
            && entry.to_owner.as_ref().and_then(|owner| owner.user_id) == Some(JASON)
            && entry.from_owner.is_none()
    }));
    let id = detail.thread.id;
    let history = a
        .db()
        .read(move |conn| campfire_db::WorkThreadEvent::for_thread(conn, id))
        .await
        .unwrap();
    assert!(
        history
            .iter()
            .any(|entry| entry.metadata["note"] == "Auto-assigned by board tag rule")
    );
    let (mut saw_created, mut saw_updated) = (false, false);
    while !saw_created || !saw_updated {
        let event = sync
            .until(|event| created(id)(event) || updated(id)(event), |_| false)
            .await;
        assert_eq!(event.topic, format!("room:{BOARD}"));
        match event.payload {
            api::SyncPayload::ThreadCreated(_) => {
                assert!(
                    !saw_created,
                    "creation publishes once even with tag assignment"
                );
                saw_created = true;
            }
            api::SyncPayload::ThreadUpdated(thread) => {
                assert!(
                    !saw_updated,
                    "the separate assignment commit publishes once"
                );
                assert_eq!(
                    thread
                        .work
                        .as_ref()
                        .unwrap()
                        .owner
                        .as_ref()
                        .map(|owner| owner.id),
                    Some(JASON)
                );
                saw_updated = true;
            }
            _ => unreachable!(),
        }
    }
    no_more(
        &a,
        &mut david,
        &mut sync,
        |event| created(id)(event) || updated(id)(event),
        "board-create-assignment-marker",
    )
    .await;
    server.abort();
}

#[tokio::test]
async fn spa_api_board_tags_replace_clear_validate_and_publish_on_room_and_thread_once() {
    let a = app(true).await.expect("the frozen default seed");
    sql(&a, format!("UPDATE users SET role=0 WHERE id={JASON}")).await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    let mut sync = Sync::connect(addr, &jason.cookie_header(), &topics(PLANNED)).await;
    sync.welcome().await;
    let path = format!("/api/v1/threads/{PLANNED}/work");
    let reply = jason
        .write(json_body(
            Method::PATCH,
            &path,
            &json!({"tags": ["stolen"]}),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::FORBIDDEN, "Forbidden".into())
    );
    for (tags, message) in [
        (
            json!(["a", "b", "c", "d", "e", "f"]),
            "are limited to 5 per post",
        ),
        (json!(["a".repeat(31)]), "must be at most 30 characters"),
        (
            json!(["no spaces"]),
            "use lowercase letters, digits, and hyphens",
        ),
    ] {
        let reply = david
            .write(json_body(Method::PATCH, &path, &json!({"tags": tags})))
            .await;
        validation(&reply, "tags", message);
    }
    assert_eq!(
        a.db()
            .read(|conn| ChannelThread::find(conn, PLANNED)?.tag_names(conn))
            .await
            .unwrap(),
        ["release", "rust"]
    );
    for (index, tags, expected) in [
        (
            0,
            json!([" Beta ", "alpha", "ALPHA", ""]),
            vec!["alpha", "beta"],
        ),
        (1, json!([]), vec![]),
    ] {
        let reply = david
            .write(json_body(Method::PATCH, &path, &json!({"tags": tags})))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        let detail: api::ThreadDetail = parse(&reply);
        assert_eq!(detail.thread.work.as_ref().unwrap().tags, expected);
        for thread in both_updates(&mut sync, PLANNED).await {
            assert_eq!(thread.work.as_ref().unwrap().tags, expected);
        }
        no_more(
            &a,
            &mut david,
            &mut sync,
            updated(PLANNED),
            &format!("board-tags-{index}"),
        )
        .await;
    }
    // Replacing an already empty set emits no update.
    let reply = david
        .write(json_body(Method::PATCH, &path, &json!({"tags": []})))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    no_more(
        &a,
        &mut david,
        &mut sync,
        updated(PLANNED),
        "board-tags-noop",
    )
    .await;
    server.abort();
}

#[tokio::test]
async fn spa_api_board_ordinary_thread_tags_follow_classic_settings_authority_and_stay_out_of_work_facts()
 {
    let a = app(true).await.expect("the frozen default seed");
    sql(&a, format!("UPDATE users SET role=0 WHERE id={KEVIN}; UPDATE channel_threads SET work_status='planned',work_owner_id={KEVIN} WHERE id=1")).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let classic = format!("/rooms/{DESIGNERS}/threads/1.json");
    let reply = david
        .write(json_body(
            Method::PATCH,
            &classic,
            &json!({"thread": {"tags": "Classic, api"}}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(
        a.db()
            .read(|conn| ChannelThread::find(conn, 1)?.tag_names(conn))
            .await
            .unwrap(),
        ["api", "classic"]
    );
    let api_path = "/api/v1/threads/1/work";
    let reply = kevin
        .write(json_body(
            Method::PATCH,
            api_path,
            &json!({"tags": ["owner"]}),
        ))
        .await;
    assert_eq!(
        (reply.status, tag(&reply)),
        (StatusCode::FORBIDDEN, "Forbidden".into()),
        "an ordinary work owner cannot edit thread settings"
    );
    let reply = david
        .write(json_body(
            Method::PATCH,
            api_path,
            &json!({"tags": [" API ", "modern"]}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let detail: api::ThreadDetail = parse(&reply);
    assert!(detail.thread.work.as_ref().unwrap().tags.is_empty());
    assert_eq!(
        a.db()
            .read(|conn| ChannelThread::find(conn, 1)?.tag_names(conn))
            .await
            .unwrap(),
        ["api", "modern"]
    );
    let work: api::WorkList = parse(&david.send(get("/api/v1/work?state=all")).await);
    assert!(
        work.threads
            .iter()
            .find(|row| row.thread.id == 1)
            .unwrap()
            .thread
            .work
            .as_ref()
            .unwrap()
            .tags
            .is_empty()
    );
}

#[tokio::test]
async fn spa_api_board_reply_publishes_updated_reply_count_and_activity_on_room_topic() {
    let a = app(true).await.expect("the frozen default seed");
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let mut sync = Sync::connect(addr, &jason.cookie_header(), &topics(PLANNED)).await;
    sync.welcome().await;
    let before: api::ThreadDetail =
        parse(&david.send(get(&format!("/api/v1/threads/{PLANNED}"))).await);
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/threads/{PLANNED}/messages"),
            &brief("board-reply-1", "An update from the discussion"),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let message: api::MessageDTO = parse(&reply);
    assert_eq!(message.thread_id, Some(PLANNED));
    let event = sync
        .until(
            |event| event.topic == format!("room:{BOARD}") && updated(PLANNED)(event),
            |_| false,
        )
        .await;
    let api::SyncPayload::ThreadUpdated(thread) = event.payload else {
        unreachable!()
    };
    assert_eq!(thread.reply_count, before.thread.reply_count + 1);
    assert!(thread.last_activity_at > before.thread.last_activity_at);
    let current: api::ThreadDetail =
        parse(&david.send(get(&format!("/api/v1/threads/{PLANNED}"))).await);
    assert_eq!(thread.last_activity_at, current.thread.last_activity_at);
    assert_eq!(thread.reply_count, current.thread.reply_count);
    assert_eq!(thread.work.as_ref().unwrap().tags, ["release", "rust"]);
    no_more(
        &a,
        &mut david,
        &mut sync,
        |event| event.topic == format!("room:{BOARD}") && updated(PLANNED)(event),
        "board-reply-marker",
    )
    .await;
    server.abort();
}

#[tokio::test]
async fn spa_api_board_classic_html_json_agent_and_mcp_creation_publish_created_once() {
    let a = app(true).await.expect("the frozen default seed");
    crate::controllers::agent_http_tests::initialize(&a).await;
    grant_bender(&a).await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let mut sync = Sync::connect(
        addr,
        &jason.cookie_header(),
        &[format!("room:{BOARD}"), format!("thread:{DONE}")],
    )
    .await;
    sync.welcome().await;
    for (index, name) in [
        "Classic HTML post",
        "Classic JSON post",
        "Agent REST post",
        "MCP post",
    ]
    .into_iter()
    .enumerate()
    {
        let reply = match index {
            0 => david.write(Req::new(Method::POST, &format!("/rooms/{BOARD}/threads")).header("accept", "text/html").form(&[("thread[name]", name), ("thread[tags]", "classic, rust")])).await,
            1 => david.write(json_body(Method::POST, &format!("/rooms/{BOARD}/threads.json"), &json!({"thread": {"name": name, "tags": "classic, rust"}}))).await,
            2 => a.anonymous().send(json_body(Method::POST, &format!("/rooms/{BOARD}/agents/posts"), &json!({"title": name, "tags": ["agent", "rust"]})).header("authorization", &format!("Bearer {}", crate::controllers::agent_http_tests::SECRET))).await,
            3 => a.anonymous().send(json_body(Method::POST, "/agents/mcp", &json!({"jsonrpc": "2.0", "id": 13, "method": "tools/call", "params": {"name": "create_board_post", "arguments": {"room_id": BOARD, "title": name, "tags": ["agent", "rust"]}}})).header("authorization", &format!("Bearer {}", crate::controllers::agent_http_tests::SECRET))).await,
            _ => unreachable!(),
        };
        if index == 0 {
            assert!(reply.status.is_redirection(), "{}", reply.text());
        } else {
            assert_eq!(
                reply.status,
                if index == 3 {
                    StatusCode::OK
                } else {
                    StatusCode::CREATED
                },
                "{}",
                reply.text()
            );
        }
        let wanted = name.to_string();
        let post = a
            .db()
            .read(move |conn| {
                Ok(ChannelThread::for_room(conn, BOARD)?
                    .into_iter()
                    .find(|post| post.name == wanted)
                    .expect("created post"))
            })
            .await
            .unwrap();
        let event = sync.until(created(post.id), |_| false).await;
        assert_eq!(event.topic, format!("room:{BOARD}"));
        let api::SyncPayload::ThreadCreated(thread) = event.payload else {
            unreachable!()
        };
        assert_eq!(thread.name, name);
        assert_eq!(thread.creator_id, if index < 2 { DAVID } else { BENDER });
        assert_eq!(
            thread.work.as_ref().unwrap().tags,
            if index < 2 {
                vec!["classic", "rust"]
            } else {
                vec!["agent", "rust"]
            }
        );
        no_more(
            &a,
            &mut david,
            &mut sync,
            created(post.id),
            &format!("board-other-create-{index}"),
        )
        .await;
    }
    server.abort();
}

#[tokio::test]
async fn spa_api_board_classic_and_agent_tag_writes_publish_new_tags_once() {
    let a = app(true).await.expect("the frozen default seed");
    crate::controllers::agent_http_tests::initialize(&a).await;
    grant_bender(&a).await;
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let jason = a.sign_in(JASON).await;
    let mut sync = Sync::connect(addr, &jason.cookie_header(), &topics(BLOCKED)).await;
    sync.welcome().await;
    let reply = david
        .write(json_body(
            Method::PATCH,
            &format!("/rooms/{BOARD}/threads/{BLOCKED}.json"),
            &json!({"thread": {"tags": "Classic, api"}}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    for thread in both_updates(&mut sync, BLOCKED).await {
        assert_eq!(thread.work.as_ref().unwrap().tags, ["api", "classic"]);
    }
    no_more(
        &a,
        &mut david,
        &mut sync,
        updated(BLOCKED),
        "board-classic-tags-marker",
    )
    .await;
    let reply = a
        .anonymous()
        .send(
            json_body(
                Method::PATCH,
                &format!("/agents/work/{BLOCKED}"),
                &json!({"tags": ["Agent", "api"]}),
            )
            .header(
                "authorization",
                &format!("Bearer {}", crate::controllers::agent_http_tests::SECRET),
            ),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    for thread in both_updates(&mut sync, BLOCKED).await {
        assert_eq!(thread.work.as_ref().unwrap().tags, ["agent", "api"]);
    }
    no_more(
        &a,
        &mut david,
        &mut sync,
        updated(BLOCKED),
        "board-agent-tags-marker",
    )
    .await;
    server.abort();
}

#[tokio::test]
async fn spa_api_board_briefless_creation_retries_by_client_post_id() {
    let a = app(true).await.expect("the frozen default seed");
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    let path = format!("/api/v1/rooms/{BOARD}/posts");
    let mut body = new_post("Retry me without a brief");
    body["clientPostId"] = json!("0192a3b4-0000-7000-8000-00000000b0a1");
    let reply = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let id = parse::<api::ThreadDetail>(&reply).thread.id;

    let retry = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(retry.status, StatusCode::OK, "{}", retry.text());
    assert_eq!(parse::<api::ThreadDetail>(&retry).thread.id, id);
    let posts = a
        .db()
        .read(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM channel_threads WHERE name='Retry me without a brief'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(Into::into)
        })
        .await
        .unwrap();
    assert_eq!(posts, 1);

    // The key is the creator's: another member's post with it is their own.
    let other = jason.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(other.status, StatusCode::CREATED, "{}", other.text());
    assert_ne!(parse::<api::ThreadDetail>(&other).thread.id, id);

    // Once the post is gone, the key makes a new one.
    let deleted = david
        .write(
            Req::new(Method::DELETE, &format!("/api/v1/threads/{id}"))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.text());
    let again = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(again.status, StatusCode::CREATED, "{}", again.text());
    assert_ne!(parse::<api::ThreadDetail>(&again).thread.id, id);

    body["clientPostId"] = json!("x".repeat(256));
    validation(
        &david.write(json_body(Method::POST, &path, &body)).await,
        "clientPostId",
        "is too long (maximum is 255 characters)",
    );
}

#[tokio::test]
async fn spa_api_board_briefless_retry_after_a_restart_answers_the_first_post() {
    let a = app(true).await.expect("the frozen default seed");
    let path = format!("/api/v1/rooms/{BOARD}/posts");
    let mut body = new_post("Lost reply before a restart");
    body["clientPostId"] = json!("0192a3b4-0000-7000-8000-00000000b0a3");
    let id = {
        let mut david = a.sign_in(DAVID).await;
        let reply = david.write(json_body(Method::POST, &path, &body)).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        parse::<api::ThreadDetail>(&reply).thread.id
    };

    let a = a.restart(seed_clock(), &[("SPA_ENABLED", "1")]).await;
    let mut david = a.sign_in(DAVID).await;
    let retry = david.write(json_body(Method::POST, &path, &body)).await;
    assert_eq!(retry.status, StatusCode::OK, "{}", retry.text());
    assert_eq!(parse::<api::ThreadDetail>(&retry).thread.id, id);
}

#[tokio::test]
async fn spa_api_board_creation_whose_commit_fails_leaves_its_key_free() {
    let a = app(true).await.expect("the frozen default seed");
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{BOARD}/posts");
    // A deferred foreign key fails the COMMIT itself, after the post was inserted.
    a.db()
        .write(|tx| {
            tx.conn().execute_batch(
                "CREATE TABLE commit_breaker_parents(id INTEGER PRIMARY KEY);
                 CREATE TABLE commit_breakers(parent_id INTEGER REFERENCES commit_breaker_parents(id) DEFERRABLE INITIALLY DEFERRED);
                 CREATE TRIGGER break_commit AFTER INSERT ON channel_threads WHEN NEW.name = 'Lost at commit'
                 BEGIN INSERT INTO commit_breakers VALUES (-1); END;",
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut lost = new_post("Lost at commit");
    lost["clientPostId"] = json!("0192a3b4-0000-7000-8000-00000000b0a2");
    let failed = david.write(json_body(Method::POST, &path, &lost)).await;
    assert!(!failed.status.is_success(), "{}", failed.text());
    a.db()
        .write(|tx| {
            tx.conn().execute_batch("DROP TRIGGER break_commit")?;
            Ok(())
        })
        .await
        .unwrap();

    // The next post may take the rolled-back id; the retry must not answer with it.
    let other = david.write(json_body(Method::POST, &path, &new_post("Takes the id"))).await;
    assert_eq!(other.status, StatusCode::CREATED, "{}", other.text());
    let other = parse::<api::ThreadDetail>(&other).thread;
    let retry = david.write(json_body(Method::POST, &path, &lost)).await;
    assert_eq!(retry.status, StatusCode::CREATED, "{}", retry.text());
    let retried = parse::<api::ThreadDetail>(&retry).thread;
    assert_ne!(retried.id, other.id);
    assert_eq!(retried.name, "Lost at commit");
}

#[tokio::test]
async fn spa_api_board_rows_count_every_message_as_classic() {
    let a = app(true).await.expect("the frozen default seed");
    let mut david = a.sign_in(DAVID).await;
    let before = listing(&mut david, "?status=all").await;
    let row = |list: &api::BoardListing| {
        list.posts
            .iter()
            .find(|row| row.thread.id == PLANNED)
            .map(|row| (row.thread.reply_count, row.thread.work.as_ref().unwrap().message_count))
            .unwrap()
    };
    let (replies, messages) = row(&before);
    sql(
        &a,
        format!("INSERT INTO messages(room_id,thread_id,creator_id,client_message_id,markdown_source,system_note,streaming,created_at,updated_at) VALUES
         ({BOARD},{PLANNED},{DAVID},'board-count-note','A system note',1,0,'2026-03-01 15:00:00','2026-03-01 15:00:00'),
         ({BOARD},{PLANNED},{DAVID},'board-count-stream','Streaming',0,1,'2026-03-01 15:00:00','2026-03-01 15:00:00');"),
    )
    .await;
    let after = listing(&mut david, "?status=all").await;
    assert_eq!(row(&after), (replies, messages + 2));
    let detail: api::ThreadDetail =
        parse(&david.send(get(&format!("/api/v1/threads/{PLANNED}"))).await);
    assert_eq!(detail.thread.work.unwrap().message_count, messages + 2);
}
