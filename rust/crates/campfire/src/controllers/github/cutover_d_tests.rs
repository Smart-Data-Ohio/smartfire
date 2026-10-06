//! Exact open WS15g declarations, using reference fixtures and production callers.
use super::{card_tests::Fresh, test_support::request};
use crate::controllers::presenters::Presenter;
use crate::integrations::{
    github::{
        accounts::{Account, AccountInput},
        pull_requests::PullRequest,
        tests::crypto,
        threads::PullRequestThread,
    },
    test_support::Route,
};
use campfire_db::{
    Agent, AgentApproval, AgentGrant, ChannelThread, Message, NewApproval, NewChannelThread,
    NewGrant, NewMessage, User, fixtures::identify as id,
};
use campfire_richtext::dom::{Dom, NodeId};
use rusqlite::params;
use serde_json::{Value, json};

fn parse_markup(html: &str) -> (Dom, NodeId) {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).unwrap();
    (dom, root)
}
fn class_nodes(dom: &Dom, root: NodeId, name: &str) -> Vec<NodeId> {
    dom.descendants(root)
        .into_iter()
        .filter(|&node| {
            dom.attr(node, "class")
                .is_some_and(|classes| classes.split_whitespace().any(|class| class == name))
        })
        .collect()
}
fn class_count(html: &str, name: &str) -> usize {
    let (dom, root) = parse_markup(html);
    class_nodes(&dom, root, name).len()
}

async fn fixture(routes: Vec<Route>) -> Fresh {
    clean_fixture(Fresh::with_routes(&json!({"mapping":false,"reference":false}), routes).await)
        .await
}
async fn clean_fixture(mut f: Fresh) -> Fresh {
    f.app.db.write(|tx| {
        tx.conn().execute_batch("DELETE FROM github_pull_request_threads WHERE github_pull_request_id=816; DELETE FROM github_pull_request_references WHERE github_pull_request_id=816; DELETE FROM channel_threads WHERE id=817; DELETE FROM messages WHERE id IN(818,828); DELETE FROM memberships WHERE room_id IN(815,825); DELETE FROM rooms WHERE id IN(815,825); DELETE FROM sessions WHERE user_id=811; DELETE FROM users WHERE id IN(811,812); DELETE FROM github_pull_requests WHERE id=816;")?;
        Ok(())
    }).await.unwrap();
    as_user(&mut f, id("david")).await;
    f
}
async fn as_user(f: &mut Fresh, user: i64) {
    let cookies =
        crate::controllers::presenters::test_support::sign_in_for_tests(&f.app, user).await;
    // `request` supplies its deterministic CSRF/session cookie; keep the issued
    // authentication and device cookies without a second Rails session key.
    f.cookie = cookies
        .split("; ")
        .filter(|cookie| !cookie.starts_with("_campfire_session="))
        .collect::<Vec<_>>()
        .join("; ");
}
async fn grant_sudo_access(f: &Fresh) -> Value {
    let (status, headers, _) = super::test_support::request_form(
        f,
        "POST",
        "/sudo",
        &[("password", "secret123456")],
        json!({}),
    )
    .await;
    assert!((300..400).contains(&status));
    super::test_support::response_session(f, &headers)
}
fn create_message(
    tx: &mut campfire_db::Tx<'_>,
    room: i64,
    creator: i64,
    key: &str,
    text: &str,
) -> campfire_db::Result<Message> {
    Message::create_markdown(
        tx,
        NewMessage {
            room_id: room,
            creator_id: creator,
            client_message_id: Some(key.into()),
            ..Default::default()
        },
        text,
    )
}
fn pr_message(
    tx: &mut campfire_db::Tx<'_>,
    number: i64,
    key: &str,
) -> campfire_db::Result<(Message, PullRequest)> {
    let m = create_message(
        tx,
        id("designers"),
        id("david"),
        key,
        &format!("review https://github.com/rails/rails/pull/{number}"),
    )?;
    let p = PullRequest::for_message(tx.conn(), m.id)?.remove(0);
    Ok((m, p))
}
fn filled_card(tx: &mut campfire_db::Tx<'_>, p: i64, private: bool) -> campfire_db::Result<()> {
    let earlier = tx.now().ago(jiff::SignedDuration::from_hours(1));
    tx.conn().execute("UPDATE github_pull_requests SET private=?,title='Add shiny things',author_login='dhh',author_avatar_url='https://avatars.example/dhh',state='open',base_branch='main',head_branch='shiny',head_sha='abc123',review_decision='approved',check_status='passing',html_url='https://github.com/'||owner||'/'||repo||'/pull/'||number,github_updated_at=?,payload='{}',fetched_at=?,fetch_error=NULL,fetch_requested_at=NULL WHERE id=?",params![private,earlier,tx.now(),p])?;
    Ok(())
}
fn discuss(
    tx: &mut campfire_db::Tx<'_>,
    m: &Message,
    p: &PullRequest,
) -> campfire_db::Result<ChannelThread> {
    let t = ChannelThread::create(
        tx,
        NewChannelThread {
            room_id: m.room_id,
            creator_id: id("david"),
            parent_message_id: Some(m.id),
            name: Some("PR chat".into()),
            ..Default::default()
        },
    )?;
    campfire_db::ThreadMembership::join(tx, t.id, id("david"))?;
    PullRequestThread::create(tx, p.id, m.room_id, t.id)?;
    Ok(t)
}
async fn key(f: &Fresh, message: i64) -> String {
    let app = f.app.clone();
    f.app
        .db
        .read(move |c| {
            let m = Message::find(c, message)?;
            Presenter::new(c, &app, None)
                .preload_search(std::slice::from_ref(&m))?
                .message_collection_cache_key(&m)
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn cutover_d_cache_no_pr_key_matches_original_slots() {
    let f = fixture(vec![]).await;
    let message = id("first");
    let expected = f
        .app
        .db
        .read(move |c| {
            let m = Message::find(c, message)?;
            Ok(format!(
                "{}/messages/{message}-{}",
                "",
                m.updated_at.jiff().strftime("%Y%m%d%H%M%S%6f")
            ))
        })
        .await
        .unwrap();
    let actual = key(&f, message).await;
    let record = expected.strip_prefix('/').unwrap();
    assert_eq!(actual, format!("{record}/////false/false////3")); // WS15g-025
}
#[tokio::test]
async fn cutover_d_cache_frozen_reply_counts_add_delete_and_zero_queries() {
    for delete in [false, true] {
        let f = fixture(vec![]).await;
        let (parent, thread, older) = f
            .app
            .db
            .write(move |tx| {
                let parent = Message::find(tx.conn(), id("third"))?;
                let mut thread = ChannelThread::create(
                    tx,
                    NewChannelThread {
                        room_id: parent.room_id,
                        creator_id: id("jz"),
                        parent_message_id: Some(parent.id),
                        name: Some("Keyed".into()),
                        ..Default::default()
                    },
                )?;
                let older = thread.post_message(
                    tx,
                    id("jz"),
                    NewMessage {
                        markdown_source: Some(if delete { "Older" } else { "One" }.into()),
                        ..Default::default()
                    },
                )?;
                if delete {
                    thread.post_message(
                        tx,
                        id("jz"),
                        NewMessage {
                            markdown_source: Some("Newer".into()),
                            ..Default::default()
                        },
                    )?;
                }
                Ok((parent.id, thread.id, older.id))
            })
            .await
            .unwrap();
        let before = key(&f, parent).await;
        f.app
            .db
            .write(move |tx| {
                if delete {
                    Message::find(tx.conn(), older)?.destroy(tx)?;
                } else {
                    ChannelThread::find(tx.conn(), thread)?.post_message(
                        tx,
                        id("jz"),
                        NewMessage {
                            markdown_source: Some("Two".into()),
                            ..Default::default()
                        },
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
        assert_ne!(
            before,
            key(&f, parent).await,
            "frozen reply-count dependency delete={delete}"
        ); // WS15g-026,027
    }
    let f = fixture(vec![]).await;
    let parent = f
        .app
        .db
        .write(|tx| {
            let m = Message::find(tx.conn(), id("third"))?;
            let mut t = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: m.room_id,
                    creator_id: id("jz"),
                    parent_message_id: Some(m.id),
                    name: Some("Keyed".into()),
                    ..Default::default()
                },
            )?;
            t.post_message(
                tx,
                id("jz"),
                NewMessage {
                    markdown_source: Some("One".into()),
                    ..Default::default()
                },
            )?;
            Ok(m.id)
        })
        .await
        .unwrap();
    let app = f.app.clone();
    f.app
        .db
        .read(move |c| {
            let m = Message::find(c, parent)?;
            let p = Presenter::new(c, &app, None).preload_search(std::slice::from_ref(&m))?;
            p.message_collection_cache_key(&m)?;
            c.flush_prepared_statement_cache();
            let reads = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let observed = reads.clone();
            c.authorizer(Some(move |ctx: rusqlite::hooks::AuthContext<'_>| {
                if matches!(ctx.action, rusqlite::hooks::AuthAction::Select) {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                rusqlite::hooks::Authorization::Allow
            }));
            let key = p.message_collection_cache_key(&m)?;
            c.authorizer(
                None::<fn(rusqlite::hooks::AuthContext<'_>) -> rusqlite::hooks::Authorization>,
            );
            assert!(key.split('/').any(|slot| slot == "1")); // WS15g-028 reply-count element
            assert_eq!(reads.load(std::sync::atomic::Ordering::SeqCst), 0); // WS15g-028
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn cutover_d_cache_streaming_steps_quotes_polls_and_system_notes() {
    let f = fixture(vec![]).await;
    let first = id("first");
    let before = key(&f, first).await;
    f.app
        .db
        .write(move |tx| {
            tx.conn()
                .execute("UPDATE messages SET streaming=1 WHERE id=?", [first])?;
            Ok(())
        })
        .await
        .unwrap();
    assert_ne!(before, key(&f, first).await); // WS15g-029
    let f = fixture(vec![]).await;
    let step_message = f
        .app
        .db
        .write(|tx| {
            Ok(create_message(
                tx,
                id("watercooler"),
                id("bender"),
                "steps-cache-key",
                "Working on it",
            )?
            .id)
        })
        .await
        .unwrap();
    let before = key(&f, step_message).await;
    let base_time = f.app.clock.now();
    f.clock.advance(jiff::SignedDuration::from_mins(1));
    f.app
        .db
        .write(move |tx| {
            campfire_db::AgentStep::create(
                tx,
                campfire_db::NewAgentStep {
                    agent_id: id("bender_agent"),
                    message_id: Some(step_message),
                    name: "Run tests".into(),
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    f.clock.set(base_time);
    assert_ne!(before, key(&f, step_message).await); // WS15g-030
    let f = fixture(vec![]).await;
    let (source, quote) = f
        .app
        .db
        .write(|tx| {
            let source = Message::create(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: id("david"),
                    body: Some("doomed source words".into()),
                    client_message_id: Some("key-doomed".into()),
                    ..Default::default()
                },
            )?;
            let quote = create_message(
                tx,
                id("designers"),
                id("david"),
                "key-quoting",
                &format!("quoting /rooms/{}/@{}", source.room_id, source.id),
            )?;
            Ok((source.id, quote.id))
        })
        .await
        .unwrap();
    let before = key(&f, quote).await;
    let base_time = f.app.clock.now();
    f.clock.advance(jiff::SignedDuration::from_mins(1));
    f.app
        .db
        .write(move |tx| Message::find(tx.conn(), source)?.destroy(tx))
        .await
        .unwrap();
    f.clock.set(base_time);
    assert_ne!(before, key(&f, quote).await); // WS15g-034
    let f = fixture(vec![]).await;
    let (question, poll) = f
        .app
        .db
        .write(|tx| {
            let m = create_message(
                tx,
                id("watercooler"),
                id("david"),
                "poll-cache-key",
                "Lunch?",
            )?;
            let p = campfire_db::Poll::create_for_message(
                tx,
                &m,
                campfire_db::NewPoll {
                    labels: vec!["Tacos".into(), "Pizza".into()],
                    ..Default::default()
                },
            )?;
            Ok((m.id, p.id))
        })
        .await
        .unwrap();
    let before = key(&f, question).await;
    let base_time = f.app.clock.now();
    f.clock.advance(jiff::SignedDuration::from_mins(1));
    f.app
        .db
        .write(move |tx| {
            let mut p = campfire_db::Poll::find(tx.conn(), poll)?;
            let option: i64 = tx.conn().query_row(
                "SELECT id FROM poll_options WHERE poll_id=? ORDER BY position LIMIT 1",
                [poll],
                |r| r.get(0),
            )?;
            p.cast_vote(tx, id("david"), &[option])
        })
        .await
        .unwrap();
    f.clock.set(base_time);
    let voted = key(&f, question).await;
    assert_ne!(before, voted); // WS15g-035 vote
    f.clock.advance(jiff::SignedDuration::from_mins(2));
    f.app
        .db
        .write(move |tx| campfire_db::Poll::find(tx.conn(), poll)?.cast_vote(tx, id("david"), &[]))
        .await
        .unwrap();
    f.clock.set(base_time);
    assert_ne!(voted, key(&f, question).await); // WS15g-035 retract
    let f = fixture(vec![]).await;
    let note = f
        .app
        .db
        .write(|tx| {
            Ok(Message::create_markdown(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: id("david"),
                    system_note: true,
                    client_message_id: Some("note-cache-key".into()),
                    ..Default::default()
                },
                "pinned a message",
            )?
            .id)
        })
        .await
        .unwrap();
    assert_eq!(key(&f, note).await.rsplit('/').nth(5), Some("true")); // WS15g-036 true
    assert_eq!(key(&f, first).await.rsplit('/').nth(5), Some("false")); // WS15g-036 false
}
#[tokio::test]
async fn cutover_d_cache_pins_and_newer_card_unpin() {
    for newer in [false, true] {
        let f = fixture(vec![]).await;
        let message = id("first");
        let pr = if newer {
            Some(f.app.db.write(move|tx|{
            let p=PullRequest::for_reference(tx,"smart-data-ohio","smartfire",43)?;
            tx.conn().execute("INSERT INTO github_pull_request_references(github_pull_request_id,message_id,created_at,updated_at) VALUES(?,?,?,?)",params![p.id,message,tx.now(),tx.now()])?;Ok(p.id)
        }).await.unwrap())
        } else {
            None
        };
        let before = key(&f, message).await;
        let base_time = f.app.clock.now();
        if !newer {
            f.clock.advance(jiff::SignedDuration::from_mins(1));
        }
        let pin = f
            .app
            .db
            .write(move |tx| {
                let m = Message::find(tx.conn(), message)?;
                Ok(campfire_db::MessagePin::pin(tx, &m, id("david"))?.unwrap())
            })
            .await
            .unwrap();
        f.clock.set(base_time);
        let pinned = key(&f, message).await;
        if !newer {
            assert_ne!(before, pinned);
        } // WS15g-037 pin
        f.clock.advance(jiff::SignedDuration::from_mins(1));
        f.app
            .db
            .write(move |tx| {
                if let Some(pr) = pr {
                    crate::integrations::github::pull_requests::update(
                        tx,
                        pr,
                        &[(
                            "title",
                            rusqlite::types::Value::Text("Updated title".into()),
                        )],
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
        if newer {
            f.clock.set(base_time);
        }
        let before_unpin = key(&f, message).await;
        f.app.db.write(move |tx| pin.unpin(tx)).await.unwrap();
        f.clock.set(base_time);
        if newer {
            assert_ne!(before_unpin, key(&f, message).await);
        }
        assert_ne!(
            pinned,
            key(&f, message).await,
            "newer referenced card={newer}"
        ); // WS15g-037 unpin,038
    }
}
#[tokio::test]
async fn cutover_d_cache_x_and_link_fetch_dependencies() {
    let f = fixture(vec![]).await;
    let message = id("first");
    let post=f.app.db.write(move|tx|{
        let p=crate::integrations::twitter::post::Post::for_reference(tx,"500",Some("https://x.com/jack/status/500"))?;
        tx.conn().execute("INSERT INTO twitter_post_references(twitter_post_id,message_id,created_at,updated_at) VALUES(?,?,?,?)",params![p.id,message,tx.now(),tx.now()])?;Ok(p.id)
    }).await.unwrap();
    let before = key(&f, message).await;
    let base_time = f.app.clock.now();
    f.clock.advance(jiff::SignedDuration::from_mins(1));
    f.app
        .db
        .write(move |tx| {
            let p = crate::integrations::twitter::post::Post::find(tx.conn(), post)?;
            p.save_card(
                tx,
                &crate::integrations::twitter::fetcher::Card {
                    url: "https://x.com/jack/status/500".into(),
                    text: Some("just setting up my twttr".into()),
                    author_handle: None,
                    author_name: None,
                    author_avatar_url: None,
                    posted_at: None,
                    replies: None,
                    reposts: None,
                    likes: None,
                    media: json!([]),
                    quote: None,
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    f.clock.set(base_time);
    assert_ne!(before, key(&f, message).await); // WS15g-039
    let f = fixture(vec![]).await;
    let embed=f.app.db.write(move|tx|{
        let e=crate::integrations::link_embed::store::Embed::for_reference(tx,"https://example.com/article")?;
        tx.conn().execute("INSERT INTO link_embed_references(link_embed_id,message_id,url,created_at,updated_at) VALUES(?,?,'https://example.com/article#intro',?,?)",params![e.id,message,tx.now(),tx.now()])?;Ok(e.id)
    }).await.unwrap();
    let before = key(&f, message).await;
    let base_time = f.app.clock.now();
    f.clock.advance(jiff::SignedDuration::from_mins(1));
    f.app
        .db
        .write(move |tx| {
            crate::integrations::link_embed::store::Embed::find(tx.conn(), embed)?.save_metadata(
                tx,
                &crate::integrations::link_embed::metadata_parser::Metadata {
                    title: Some("An article".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    f.clock.set(base_time);
    assert_ne!(before, key(&f, message).await); // WS15g-040
}
#[tokio::test]
async fn cutover_d_thread_card_discuss_button_and_link_are_room_scoped() {
    for mapped in [false, true] {
        let f = fixture(vec![]).await;
        let thread = f
            .app
            .db
            .write(move |tx| {
                let (m, p) = pr_message(
                    tx,
                    if mapped { 141 } else { 140 },
                    if mapped {
                        "threads-view-link"
                    } else {
                        "threads-view-button"
                    },
                )?;
                filled_card(tx, p.id, false)?;
                Ok(if mapped {
                    Some(discuss(tx, &m, &p)?.id)
                } else {
                    None
                })
            })
            .await
            .unwrap();
        let (status, _, html) = request(
            &f,
            "GET",
            &format!("/rooms/{}", id("designers")),
            Value::Null,
            json!({}),
        )
        .await;
        assert_eq!(status, 200); // WS15g-044,045
        let (dom, root) = parse_markup(&html);
        let cards = class_nodes(&dom, root, "github-pr-card");
        let links = cards
            .iter()
            .flat_map(|&card| class_nodes(&dom, card, "github-pr-card__discuss"))
            .filter(|&node| dom.local_name(node) == Some("a"))
            .collect::<std::collections::HashSet<_>>();
        let forms = class_nodes(&dom, root, "github-pr-card__discuss-form");
        if let Some(thread) = thread {
            let href = format!("/rooms/{}/threads/{thread}", id("designers"));
            assert_eq!(
                links
                    .iter()
                    .filter(|&&node| dom.attr(node, "href") == Some(href.as_str())
                        && dom.text_content(node) == "Discuss")
                    .count(),
                1
            ); // WS15g-045 link href+text+count
            assert_eq!(forms.len(), 0); // WS15g-045
        } else {
            assert_eq!(forms.len(), 1); // WS15g-044 form count
            let action = format!("/rooms/{}/github/pull_request_threads", id("designers"));
            let scoped_forms = forms
                .iter()
                .copied()
                .filter(|&node| dom.attr(node, "action") == Some(action.as_str()))
                .collect::<Vec<_>>();
            assert_eq!(scoped_forms.len(), 1); // WS15g-044 action
            assert_eq!(
                scoped_forms
                    .iter()
                    .flat_map(|&form| class_nodes(&dom, form, "github-pr-card__discuss"))
                    .filter(|&node| dom.local_name(node) == Some("button")
                        && dom.text_content(node) == "Discuss")
                    .count(),
                1
            ); // WS15g-044 nested button
            assert_eq!(links.len(), 0); // WS15g-044 no link
        }
    }
}
#[tokio::test]
async fn cutover_d_thread_files_loaded_exact_loading_ordinary_xss_and_private() {
    for case in ["loaded", "exact", "loading", "ordinary", "xss", "private"] {
        let f = fixture(vec![]).await;
        let thread=f.app.db.write(move|tx|{
            if case=="ordinary" {
                let m=create_message(tx,id("designers"),id("david"),"threads-view-ordinary","just chatting")?;
                return Ok(ChannelThread::create(tx,NewChannelThread{room_id:m.room_id,creator_id:id("david"),parent_message_id:Some(m.id),name:Some("Ordinary chat".into()),..Default::default()})?.id);
            }
            let (number,client)=match case {"loaded"=>(142,"threads-view-header"),"exact"=>(143,"threads-view-exact"),"loading"=>(144,"threads-view-loading"),"xss"=>(145,"threads-view-xss"),_=>(147,"threads-view-private-frame")};
            let (m,p)=pr_message(tx,number,client)?;filled_card(tx,p.id,case=="private")?;
            if case!="loading" {
                let files=match case {
                    "loaded"=>json!({"files":[{"filename":"app/models/user.rb","additions":10,"deletions":2,"status":"modified"},{"filename":"app/models/new.rb","additions":5,"deletions":0,"status":"added"}],"total_count":5}),
                    "xss"=>json!({"files":[{"filename":"<img src=x onerror=\"window.__prFilesXss = true\">","additions":1,"deletions":0,"status":"modified"}],"total_count":1}),
                    "private"=>json!({"files":[{"filename":"app/models/secret.rb","additions":3,"deletions":1,"status":"modified"}],"total_count":1}),
                    _=>json!({"files":[{"filename":"only.rb","additions":1,"deletions":0,"status":"modified"}],"total_count":1})};
                crate::integrations::github::pull_requests::update(tx,p.id,&[("changed_files",rusqlite::types::Value::Text(files.to_string())),("changed_files_fetched_at",rusqlite::types::Value::Text(tx.now().to_db()))])?;
            }
            let mut t=discuss(tx,&m,&p)?;
            if case=="loaded" {t.post_message(tx,id("david"),NewMessage{markdown_source:Some("first reply".into()),..Default::default()})?;}
            Ok(t.id)
        }).await.unwrap();
        let (status, _, html) = request(
            &f,
            "GET",
            &format!("/rooms/{}/threads/{thread}", id("designers")),
            Value::Null,
            json!({}),
        )
        .await;
        assert_eq!(status, 200, "{case}"); // WS15g-046,047,048,049,050,051
        if case == "ordinary" {
            assert_eq!(class_count(&html, "github-pr-thread-header"), 0);
            continue;
        } // WS15g-049
        let (dom, root) = parse_markup(&html);
        let headers = class_nodes(&dom, root, "github-pr-thread-header");
        let scoped = |name: &str| {
            headers
                .iter()
                .flat_map(|&header| class_nodes(&dom, header, name))
                .collect::<std::collections::HashSet<_>>()
        };
        match case {
            "loaded" => {
                assert_eq!(scoped("github-pr-card").len(), 1); // WS15g-046 card scope
                assert!(
                    scoped("github-pr-card__title")
                        .iter()
                        .any(|&node| dom.text_content(node) == "Add shiny things")
                ); // WS15g-046 title scope
                assert!(html.contains("class=\"github-pr-files__heading\">Files changed</h2>")); // WS15g-046 heading
                assert_eq!(class_count(&html, "github-pr-files__file"), 2); // WS15g-046 files
                assert!(html.contains("class=\"github-pr-files__path\">app/models/user.rb</span>")); // WS15g-046 path
                assert!(html.contains("class=\"github-pr-files__status\">Modified</span>")); // WS15g-046 modified
                assert!(html.contains("class=\"github-pr-files__status\">Added</span>")); // WS15g-046 added
                assert!(html.contains("class=\"github-pr-files__counts\">+10 −2</span>")); // WS15g-046 counts
                assert!(html.contains("and 3 more on GitHub")); // WS15g-046 more
                assert!(
                    html.find("github-pr-thread-header").unwrap()
                        < html.find("first reply").unwrap()
                );
            }
            "exact" => {
                assert_eq!(class_count(&html, "github-pr-files__file"), 1); // WS15g-047 file
                assert_eq!(class_count(&html, "github-pr-files__more"), 0); // WS15g-047 no more
            }
            "loading" => {
                assert_eq!(scoped("github-pr-card").len(), 1); // WS15g-048 card
                let mut dom = campfire_richtext::dom::Dom::new();
                let root = dom.parse_fragment(&html).unwrap();
                assert!(dom.descendants(root).into_iter().any(|node| {
                    dom.attr(node, "class").is_some_and(|classes| {
                        classes
                            .split_whitespace()
                            .any(|class| class == "github-pr-files__loading")
                    }) && dom.text_content(node).contains("Loading files")
                })); // WS15g-048 loading
                assert_eq!(class_count(&html, "github-pr-files__file"), 0); // WS15g-048 no file
            }
            "xss" => {
                let (dom, root) = parse_markup(&html);
                let paths = class_nodes(&dom, root, "github-pr-files__path");
                assert!(paths.iter().any(|&path| dom.text_content(path)
                    == "<img src=x onerror=\"window.__prFilesXss = true\">")); // WS15g-050 exact path text
                assert_eq!(
                    paths
                        .iter()
                        .flat_map(|&path| dom.descendants(path))
                        .filter(|&node| dom.local_name(node) == Some("img"))
                        .count(),
                    0
                ); // WS15g-050 all scoped paths have no img
            }
            "private" => {
                let pr=f.app.db.read(move|c|Ok(PullRequestThread::for_room_pr(c,id("designers"),c.query_row("SELECT github_pull_request_id FROM github_pull_request_threads WHERE channel_thread_id=?",[thread],|r|r.get::<_,i64>(0))?)?.unwrap().pull_request_id)).await.unwrap();
                assert_eq!(scoped("github-pr-card").len(), 0); // WS15g-051 scoped no card
                assert_eq!(scoped("github-pr-files").len(), 0); // WS15g-051 scoped no files
                assert_eq!(scoped("github-pr-card-frame").iter().filter(|&&node| {
                    dom.local_name(node) == Some("turbo-frame")
                        && dom.attr(node, "loading") == Some("lazy")
                        && dom.attr(node, "id") == Some(format!("card_for_thread_{thread}_github_pull_request_{pr}").as_str())
                        && dom.attr(node, "src") == Some(format!("/rooms/{}/github/pull_requests/{pr}/card?thread_id={thread}", id("designers")).as_str())
                }).count(), 1); // WS15g-051 exact lazy frame
                assert!(!html.contains("Add shiny things")); // WS15g-051 no title
                assert!(!html.contains("app/models/secret.rb")); // WS15g-051 no filename
            }
            _ => unreachable!(),
        }
    }
}
#[tokio::test]
async fn cutover_d_open_room_join_page_omits_card_and_offers_join() {
    let mut f = fixture(vec![]).await;
    f.app
        .db
        .write(|tx| {
            let m = create_message(
                tx,
                id("pets"),
                id("david"),
                "card-render-join-preview",
                "https://github.com/rails/rails/pull/129",
            )?;
            filled_card(
                tx,
                PullRequest::for_message(tx.conn(), m.id)?.remove(0).id,
                false,
            )
        })
        .await
        .unwrap();
    as_user(&mut f, id("kevin")).await;
    let (status, _, html) = request(
        &f,
        "GET",
        &format!("/rooms/{}", id("pets")),
        Value::Null,
        json!({}),
    )
    .await;
    assert_eq!(status, 200); // WS15g-043
    assert_eq!(class_count(&html, "github-pr-card"), 0); // WS15g-043
    assert!(html.contains("Join channel</button>")); // WS15g-043
}
#[tokio::test]
async fn cutover_d_profile_verified_login_rejects_edit_until_disconnected() {
    let f = fixture(vec![]).await;
    let sudo = grant_sudo_access(&f).await;
    f.app
        .db
        .write(|tx| {
            Account::create(
                tx,
                &crypto(),
                &AccountInput {
                    user_id: id("david"),
                    github_login: "octocat",
                    access_token: "x",
                    refresh_token: None,
                    token_expires_at: None,
                    token_source: "pat",
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let (_, _, html) = request(&f, "GET", "/users/me/profile", Value::Null, sudo.clone()).await;
    let input = html
        .split("<input")
        .find(|tag| {
            tag.split('>')
                .next()
                .unwrap()
                .contains("name=\"user[github_login]\"")
        })
        .unwrap()
        .split('>')
        .next()
        .unwrap();
    assert!(input.contains("disabled")); // WS15g-011 disabled input
    let (status, headers, _) = request(
        &f,
        "PUT",
        "/users/me/profile",
        json!({"user":{"github_login":"someone-else","name":"Dave"}}),
        sudo.clone(),
    )
    .await;
    assert_eq!(status, 302); // WS15g-011 redirect
    assert_eq!(headers["location"], "http://example.org/users/me/profile"); // WS15g-011 redirect path
    f.app
        .db
        .read(|c| {
            let u = User::find(c, id("david"))?;
            assert_eq!(
                c.query_row(
                    "SELECT github_login FROM users WHERE id=?",
                    [id("david")],
                    |r| r.get::<_, Option<String>>(0)
                )?
                .as_deref(),
                Some("octocat")
            ); // WS15g-011 login preserved
            assert_eq!(u.name, "Dave"); // WS15g-011 permitted name
            Ok(())
        })
        .await
        .unwrap();
    f.app
        .db
        .write(|tx| {
            let rejected = campfire_db::models::user::profile_settings::update(
                tx,
                id("david"),
                campfire_db::models::user::profile_settings::Changes {
                    github_login: Some("someone-else".into()),
                    ..Default::default()
                },
            );
            assert!(rejected.is_err()); // WS15g-011 model rejects
            let campfire_db::Error::RecordInvalid(e) = rejected.unwrap_err() else {
                panic!("expected profile validation")
            };
            assert!(
                e.on("github_login")
                    .contains(&"is set by your linked GitHub account")
            ); // WS15g-011 model error
            Ok(())
        })
        .await
        .unwrap();
    request(
        &f,
        "DELETE",
        "/github/connection",
        Value::Null,
        sudo.clone(),
    )
    .await;
    let (status, headers, _) = request(
        &f,
        "PUT",
        "/users/me/profile",
        json!({"user":{"github_login":"david-gh"}}),
        sudo.clone(),
    )
    .await;
    assert_eq!(status, 302); // WS15g-012 redirect
    assert_eq!(headers["location"], "http://example.org/users/me/profile"); // WS15g-012 path
    f.app
        .db
        .read(|c| {
            assert_eq!(
                c.query_row(
                    "SELECT github_login FROM users WHERE id=?",
                    [id("david")],
                    |r| r.get::<_, Option<String>>(0)
                )?
                .as_deref(),
                Some("david-gh")
            );
            Ok(())
        })
        .await
        .unwrap(); // WS15g-012 login
}
#[tokio::test]
async fn cutover_d_profile_connected_login_claim_conflict_preserves_model_guard() {
    use campfire_db::models::user::profile_settings::{Changes, update};
    let f = fixture(vec![]).await;
    f.app
        .db
        .write(|tx| {
            for user_id in [id("jz"), id("david")] {
                Account::create(
                    tx,
                    &crypto(),
                    &AccountInput {
                        user_id,
                        github_login: "Octocat",
                        access_token: "profile-token",
                        refresh_token: None,
                        token_expires_at: None,
                        token_source: "pat",
                    },
                )?;
            }
            assert_eq!(
                tx.conn().query_row(
                    "SELECT github_login FROM users WHERE id=?",
                    [id("david")],
                    |row| row.get::<_, Option<String>>(0)
                )?,
                None
            );
            assert!(
                Account::for_user(tx.conn(), id("david"))?
                    .unwrap()
                    .connected()
            );
            update(
                tx,
                id("david"),
                Changes {
                    github_login: Some("".into()),
                    ..Default::default()
                },
            )?;
            for login in ["someone-else", "previous-login"] {
                let error = update(
                    tx,
                    id("david"),
                    Changes {
                        github_login: Some(login.into()),
                        ..Default::default()
                    },
                )
                .unwrap_err();
                let campfire_db::Error::RecordInvalid(errors) = error else {
                    panic!("expected profile validation")
                };
                assert_eq!(
                    errors.on("github_login"),
                    vec!["is set by your linked GitHub account"]
                );
            }
            let error = update(
                tx,
                id("david"),
                Changes {
                    github_login: Some(" OCTOCAT ".into()),
                    ..Default::default()
                },
            )
            .unwrap_err();
            let campfire_db::Error::RecordInvalid(errors) = error else {
                panic!("expected uniqueness validation")
            };
            assert_eq!(
                errors.on("github_login"),
                vec!["is already linked to another user"]
            );
            assert_eq!(
                tx.conn().query_row(
                    "SELECT github_login FROM users WHERE id=?",
                    [id("david")],
                    |row| row.get::<_, Option<String>>(0)
                )?,
                None
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn cutover_d_profile_connected_whitespace_reason_ignores_manual_login_and_saves_name() {
    for reason in ["\t", "\u{a0}"] {
        let f = fixture(vec![]).await;
        let sudo = grant_sudo_access(&f).await;
        f.app
            .db
            .write(move |tx| {
                let account = Account::create(
                    tx,
                    &crypto(),
                    &AccountInput {
                        user_id: id("david"),
                        github_login: "octocat",
                        access_token: "profile-token",
                        refresh_token: None,
                        token_expires_at: None,
                        token_source: "pat",
                    },
                )?;
                Account::mark_disconnected(tx, account.id, reason)?;
                assert!(
                    Account::for_user(tx.conn(), id("david"))?
                        .unwrap()
                        .connected()
                );
                Ok(())
            })
            .await
            .unwrap();
        let (status, headers, _) = request(
            &f,
            "PUT",
            "/users/me/profile",
            json!({"user":{"github_login":"someone-else", "name":"Dave"}}),
            sudo.clone(),
        )
        .await;
        assert_eq!(status, 302, "reason={reason:?}");
        assert_eq!(headers["location"], "http://example.org/users/me/profile");
        f.app
            .db
            .read(|conn| {
                assert_eq!(User::find(conn, id("david"))?.name, "Dave");
                assert_eq!(
                    conn.query_row(
                        "SELECT github_login FROM users WHERE id=?",
                        [id("david")],
                        |row| row.get::<_, Option<String>>(0)
                    )?
                    .as_deref(),
                    Some("octocat")
                );
                Ok(())
            })
            .await
            .unwrap();
    }
}

async fn agent_fixture(f: &Fresh) -> i64 {
    f.app
        .db
        .write(|tx| {
            let agent = Agent::for_user(tx.conn(), id("bender"))?.unwrap();
            for capability in ["read_messages", "external_action"] {
                AgentGrant::create(
                    tx,
                    NewGrant {
                        agent_id: agent.id,
                        room_id: Some(id("watercooler")),
                        granted_by_id: id("david"),
                        capability: capability.into(),
                        ..Default::default()
                    },
                )?;
            }
            Account::create(
                tx,
                &crypto(),
                &AccountInput {
                    user_id: id("bender"),
                    github_login: "bender-machine",
                    access_token: "agent-token-abc",
                    refresh_token: None,
                    token_expires_at: None,
                    token_source: "pat",
                },
            )?;
            let m = create_message(
                tx,
                id("watercooler"),
                id("david"),
                "github-action-delivery-pr-12",
                "review https://github.com/rails/rails/pull/12",
            )?;
            let p = PullRequest::for_message(tx.conn(), m.id)?.remove(0);
            discuss(tx, &m, &p)?;
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            Ok(agent.id)
        })
        .await
        .unwrap()
}
async fn approval(f: &Fresh, action: &str, body: &str) -> i64 {
    let action = action.to_owned();
    let body = body.to_owned();
    f.app.db.write(move|tx|{
        let agent=Agent::for_user(tx.conn(),id("bender"))?.unwrap();
        let account=Account::for_user(tx.conn(),id("bender"))?.unwrap();
        let pr=PullRequest::for_message(tx.conn(),tx.conn().query_row("SELECT id FROM messages WHERE client_message_id='github-action-delivery-pr-12'",[],|r|r.get::<_,i64>(0))?)?.remove(0);
        let a=crate::integrations::github::actions::Action::from_payload(pr.id,crate::integrations::github::client::PullRequestKey{owner:pr.owner,repo:pr.repo,number:pr.number},&json!({"kind":"comment","body":body}));
        assert!(a.errors().is_empty()); // Rails build_approval action.valid?
        Ok(AgentApproval::create(tx,NewApproval{agent_id:agent.id,room_id:Some(id("watercooler")),action:action.clone(),summary:if action=="github.comment"{a.summary().unwrap()}else{"Ship it".into()},payload:(action=="github.comment").then(||a.payload().to_string()),github_account_id:Some(account.id),github_login:Some(account.github_login),..Default::default()})?.id)
    }).await.unwrap()
}
async fn decide(f: &Fresh, approval: i64, decision: &str) {
    let decision = decision.to_owned();
    f.app
        .db
        .write(move |tx| {
            let by = User::find(tx.conn(), id("david"))?;
            assert!(
                AgentApproval::find(tx.conn(), approval)?
                    .unwrap()
                    .decide(tx, &decision, &by, None)?
                    .is_empty()
            );
            Ok(())
        })
        .await
        .unwrap();
}
async fn run_action(f: &Fresh, approval: i64, webhooks: bool) {
    if !webhooks {
        f.app
            .db
            .write(|tx| {
                tx.conn().execute(
                    "DELETE FROM background_jobs WHERE job_class!='Github::PerformAgentActionJob'",
                    [],
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }
    let registry = crate::jobs::registry();
    let config = crate::queue::runner_config(&f.app.config);
    let (_, adhoc) = crate::queue::Jobs::new(&registry, &config).unwrap();
    let runner = crate::jobs::start(
        f.app.clone(),
        registry,
        adhoc,
        config,
        crate::jobs::periodic::Loops::new(crate::jobs::periodic::Intervals {
            periodic: None,
            huddle: None,
        }),
    );
    tokio::time::timeout(std::time::Duration::from_secs(10),async{loop{
        let done=f.app.db.read(move|c|{let complete=c.query_row("SELECT EXISTS(SELECT 1 FROM agent_events WHERE event_type='github_action_completed' AND agent_approval_id=?)",[approval],|r|r.get::<_,bool>(0))?;let remaining=c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class IN ('Github::PerformAgentActionJob','Agent::EventWebhookJob') AND status IN ('ready','running')",[],|r|r.get::<_,i64>(0))?;Ok(complete&&(!webhooks||remaining==0))}).await.unwrap();
        if done {break;}tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }}).await.unwrap();
    runner.shutdown(std::time::Duration::from_secs(2)).await;
}
async fn bearer(f: &Fresh, method: &str, path: &str, secret: &str) -> (u16, Value) {
    use tower::ServiceExt;
    let response = f
        .router
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(method)
                .uri(path)
                .header("Host", "example.org")
                .header("Authorization", format!("Bearer {secret}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
async fn completion(f: &Fresh, approval: i64) -> (i64, i64, String, Option<i64>, Value) {
    f.app.db.read(move|c|Ok(c.query_row("SELECT id,room_id,outcome,message_id,metadata FROM agent_events WHERE agent_approval_id=? AND event_type='github_action_completed' ORDER BY id DESC LIMIT 1",[approval],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,serde_json::from_str::<Value>(&r.get::<_,String>(4)?).unwrap())))?)).await.unwrap()
}
#[tokio::test]
async fn cutover_d_approval_job_complete_failure_poll_ack_readability_and_ledger() {
    for case in ["success", "refused", "left_room"] {
        let f = fixture(vec![
            Route::new(
                "POST",
                "api.github.com",
                "/repos/rails/rails/issues/12/comments",
                if case == "refused" { 403 } else { 201 },
            )
            .body(
                if case == "refused" {
                    json!({"message":"Resource not accessible by personal access token"})
                } else {
                    json!({"html_url":"https://github.com/rails/rails/pull/12#issuecomment-1"})
                }
                .to_string(),
            ),
        ])
        .await;
        let agent = agent_fixture(&f).await;
        let approval = approval(&f, "github.comment", "Nice").await;
        let before=f.app.db.read(move|c|Ok(c.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='github_action_completed'",[agent],|r|r.get::<_,i64>(0))?)).await.unwrap();
        if case == "left_room" {
            f.app
                .db
                .write(move |tx| {
                    AgentGrant::create(
                        tx,
                        NewGrant {
                            agent_id: agent,
                            room_id: Some(id("bender_and_kevin")),
                            granted_by_id: id("david"),
                            capability: "read_messages".into(),
                            ..Default::default()
                        },
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        decide(&f, approval, "approved").await;
        if case == "left_room" {
            f.app
                .db
                .write(|tx| {
                    campfire_db::Membership::find_by_room_and_user(
                        tx.conn(),
                        id("watercooler"),
                        id("bender"),
                    )?
                    .unwrap()
                    .destroy(tx)
                })
                .await
                .unwrap();
        }
        run_action(&f, approval, false).await;
        let event = completion(&f, approval).await;
        let after=f.app.db.read(move|c|Ok(c.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='github_action_completed'",[agent],|r|r.get::<_,i64>(0))?)).await.unwrap();
        assert_eq!(after, before + 1); // WS15g-002 completion count
        assert_eq!(event.2, "delivered"); // WS15g-002 outcome
        assert_eq!(event.1, id("watercooler")); // WS15g-002 room
        assert_eq!(event.3, None); // WS15g-002 message_id
        let expected = match case {
            "success" => {
                json!({"approval_id":approval,"action":"github.comment","status":"completed","url":"https://github.com/rails/rails/pull/12#issuecomment-1"})
            }
            "refused" => {
                json!({"approval_id":approval,"action":"github.comment","status":"failed","message":"GitHub refused: Resource not accessible by personal access token"})
            }
            _ => {
                json!({"approval_id":approval,"action":"github.comment","status":"failed","message":"Agent is no longer a member of the room"})
            }
        };
        assert_eq!(event.4, expected); // WS15g-002 complete metadata,003 failed metadata
        let (status, poll) = bearer(
            &f,
            "GET",
            "/agents/events?envelope=1",
            "bender-test-secret-1234",
        )
        .await;
        assert_eq!(status, 200); // WS15g-004,005
        let row = poll["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["event_type"] == "github_action_completed");
        assert!(row.is_some(), "completion absent from {poll}"); // WS15g-004,005 presence
        let row = row.unwrap();
        if case == "success" {
            assert_eq!(row["outcome"], "delivered"); // WS15g-004
            assert!(row["message"].is_null()); // WS15g-004
            assert_eq!(row["github_action"], expected); // WS15g-004
            assert_eq!(row["room"]["id"], id("watercooler")); // WS15g-004
            let event_id = event.0;
            f.app
                .db
                .read(move |c| {
                    assert!(
                        campfire_db::models::agent_event_access::readable_page(c, agent, 0, None)?
                            .iter()
                            .any(|e| e.id == event_id)
                    );
                    Ok(())
                })
                .await
                .unwrap(); // WS15g-008 own model readable
            let other = f
                .app
                .db
                .write(|tx| {
                    let bot = User::create_bot(tx, "Github Completion Other Bot", None)?;
                    let other = Agent::create(
                        tx,
                        campfire_db::NewAgent {
                            user_id: bot.id,
                            owner_id: Some(id("david")),
                            ..Default::default()
                        },
                    )?;
                    campfire_db::Membership::create_default(tx, id("watercooler"), bot.id)?;
                    AgentGrant::create(
                        tx,
                        NewGrant {
                            agent_id: other.id,
                            room_id: Some(id("watercooler")),
                            granted_by_id: id("david"),
                            capability: "read_messages".into(),
                            ..Default::default()
                        },
                    )?;
                    Ok(campfire_db::AgentCredential::create_with_secret(
                        tx,
                        other.id,
                        "delivery",
                        id("david"),
                        None,
                    )?
                    .1)
                })
                .await
                .unwrap();
            let (status, poll) = bearer(&f, "GET", "/agents/events?envelope=1", &other).await;
            assert_eq!(status, 200); // WS15g-008 other status
            assert!(
                poll["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|r| r["event_type"] != "github_action_completed")
            ); // WS15g-008 filtered empty
            let (status, _) = bearer(
                &f,
                "POST",
                &format!("/agents/events/{}/ack", event.0),
                &other,
            )
            .await;
            assert_eq!(status, 404); // WS15g-008 other ack
            assert_eq!(completion(&f, approval).await.2, "delivered"); // WS15g-008 unchanged
            let (status, body) = bearer(
                &f,
                "POST",
                &format!("/agents/events/{}/ack", event.0),
                "bender-test-secret-1234",
            )
            .await;
            assert_eq!(status, 200); // WS15g-006 status
            assert_eq!(body["outcome"], "acknowledged"); // WS15g-006 response
            assert_eq!(completion(&f, approval).await.2, "acknowledged"); // WS15g-006 DB
        } else if case == "left_room" {
            assert_eq!(row["github_action"]["status"], "failed"); // WS15g-005
            assert_eq!(
                row["github_action"]["message"],
                "Agent is no longer a member of the room"
            ); // WS15g-005
            assert!(
                !row["github_action"]
                    .as_object()
                    .unwrap()
                    .contains_key("url")
            ); // WS15g-005
        }
        let (status, _, html) = request(
            &f,
            "GET",
            &format!("/agents/{agent}/events"),
            Value::Null,
            json!({}),
        )
        .await;
        assert_eq!(status, 200); // WS15g-009,010
        if case == "success" {
            assert!(html.contains("github_action_completed")); // WS15g-009
            assert!(html.contains("GitHub github.comment: completed")); // WS15g-009
        } else if case == "left_room" {
            assert!(html.contains("GitHub github.comment: failed")); // WS15g-010
            assert!(html.contains("Agent is no longer a member of the room")); // WS15g-010
        }
    }
}
#[tokio::test]
async fn cutover_d_approval_enqueues_exact_argument_only_for_approved_github_action() {
    let f = fixture(vec![]).await;
    agent_fixture(&f).await;
    let approve = approval(&f, "github.comment", "Nice").await;
    let deny = approval(&f, "github.comment", "Nope").await;
    let other = approval(&f, "deploy", "Ship it").await;
    decide(&f, approve, "approved").await;
    let jobs=f.app.db.read(move|c|{Ok(c.prepare("SELECT arguments FROM background_jobs WHERE job_class='Github::PerformAgentActionJob' ORDER BY id")?.query_map([],|r|Ok(serde_json::from_str::<Value>(&r.get::<_,String>(0)?).unwrap()))?.collect::<rusqlite::Result<Vec<_>>>()?)}).await.unwrap();
    assert_eq!(jobs, vec![json!({"approval_id":approve})]); // WS15g-058 exact args
    decide(&f, deny, "denied").await;
    let denied=f.app.db.read(move|c|Ok(c.prepare("SELECT arguments FROM background_jobs WHERE job_class='Github::PerformAgentActionJob' ORDER BY id")?.query_map([],|r|Ok(serde_json::from_str::<Value>(&r.get::<_,String>(0)?).unwrap()))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap();
    assert_eq!(denied, jobs); // WS15g-058 denying adds none
    decide(&f, other, "approved").await;
    let non_github=f.app.db.read(move|c|Ok(c.prepare("SELECT arguments FROM background_jobs WHERE job_class='Github::PerformAgentActionJob' ORDER BY id")?.query_map([],|r|Ok(serde_json::from_str::<Value>(&r.get::<_,String>(0)?).unwrap()))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap();
    assert_eq!(non_github, jobs); // WS15g-059
}
#[derive(Clone, Default)]
struct Logs(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for Logs {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Logs {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}
async fn front_post(f: &Fresh, path: &str, body: Value) -> u16 {
    let mut values = grant_sudo_access(f).await;
    let raw = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, [7u8; 32]);
    values["_csrf_token"] = json!(raw);
    let request = axum::http::Request::builder()
        .method("POST")
        .uri(path)
        .header("Host", "example.org")
        .header("Cookie", super::test_support::session(f, &values))
        .header(
            "X-CSRF-Token",
            campfire_kit::csrf::mask(&[7u8; 32], [9u8; 32]),
        )
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(body.to_string()))
        .unwrap();
    let handler = std::sync::Arc::new(campfire_kit::front::Handler::new(
        &campfire_kit::front::FrontConfig::from_lookup(|key| {
            (key == "GZIP_COMPRESSION_ENABLED").then(|| "false".into())
        }),
        f.router.clone(),
    ));
    let response = handler
        .call(
            request,
            campfire_kit::front::ConnInfo {
                remote: "127.0.0.1:40101".parse().unwrap(),
                tls: false,
            },
        )
        .await;
    let status = response.status().as_u16();
    axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    status
}
#[tokio::test]
async fn cutover_d_request_logs_filter_link_comment_and_review_credentials() {
    use tracing::instrument::WithSubscriber;
    for kind in ["link", "comment", "request_review"] {
        let routes = vec![
            Route::new("GET", "api.github.com", "/user", 200).body(r#"{"login":"octocat"}"#),
            Route::new(
                "POST",
                "api.github.com",
                "/repos/rails/rails/issues/12/comments",
                201,
            )
            .body(r#"{"id":1}"#),
            Route::new(
                "POST",
                "api.github.com",
                "/repos/rails/rails/pulls/12/requested_reviewers",
                201,
            )
            .body(r#"{"id":12}"#),
        ];
        let f = clean_fixture(
            Fresh::with_routes(
                &json!({"mapping":false,"reference":false,"reader_token":"workspace-token"}),
                routes,
            )
            .await,
        )
        .await;
        let token = if kind == "link" {
            "github_pat_secret_xyz"
        } else {
            "user-token-secret-xyz"
        };
        let body = if kind == "link" {
            json!({"access_token":token})
        } else {
            f.app
                .db
                .write(move |tx| {
                    let (m, p) = pr_message(
                        tx,
                        12,
                        if kind == "comment" {
                            "write-comment-1"
                        } else {
                            "write-review-request-1"
                        },
                    )?;
                    discuss(tx, &m, &p)?;
                    Account::create(
                        tx,
                        &crypto(),
                        &AccountInput {
                            user_id: id("david"),
                            github_login: "david",
                            access_token: token,
                            refresh_token: None,
                            token_expires_at: None,
                            token_source: "pat",
                        },
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
            let pr = f
                .app
                .db
                .read(|c| {
                    Ok(c.query_row(
                        "SELECT id FROM github_pull_requests WHERE number=12",
                        [],
                        |r| r.get::<_, i64>(0),
                    )?)
                })
                .await
                .unwrap();
            if kind == "comment" {
                json!({"pull_request_id":pr,"body":"Nice work"})
            } else {
                json!({"pull_request_id":pr,"reviewers":"alice"})
            }
        };
        let path = if kind == "link" {
            "/github/connection".into()
        } else {
            format!(
                "/rooms/{}/github/{}",
                id("designers"),
                if kind == "comment" {
                    "pull_request_comments"
                } else {
                    "pull_request_review_requests"
                }
            )
        };
        let logs = Logs::default();
        let writer = logs.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        let status = front_post(&f, &path, body)
            .with_subscriber(subscriber)
            .await;
        if kind != "link" {
            assert_eq!(status, 200);
        } // WS15g-015,016 response
        assert!(
            logs.text().contains(&path),
            "missing request log: {}",
            logs.text()
        ); // WS15g-013,015,016 positive request log
        assert!(!logs.text().contains(token)); // WS15g-013,015,016 token absent
    }
    assert_eq!(
        crate::security::parameter_filter()
            .filter(&json!({"access_token":"github_pat_secret_xyz"}))["access_token"],
        "[FILTERED]"
    ); // WS15g-014
}
#[tokio::test]
async fn cutover_d_write_timeout_logs_warning_without_member_token() {
    use tracing::instrument::WithSubscriber;
    let mut route = Route::new(
        "POST",
        "api.github.com",
        "/repos/rails/rails/issues/12/comments",
        201,
    )
    .body("{}");
    route.delay = std::time::Duration::from_secs(11);
    let (_server, network) = crate::integrations::github::tests::fake(vec![route]).await;
    let client = crate::integrations::github::client::WriteClient::with_network(
        "user-token-123".into(),
        network,
    );
    let logs = Logs::default();
    let writer = logs.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let error = client
        .create_issue_comment(
            &crate::integrations::github::client::PullRequestKey {
                owner: "rails".into(),
                repo: "rails".into(),
                number: 12,
            },
            "hi",
        )
        .with_subscriber(subscriber)
        .await
        .unwrap_err();
    assert_eq!(
        error.kind,
        crate::integrations::github::client::ErrorKind::Other
    ); // WS15g-061 error class
    assert!(error.message.contains("Could not reach GitHub")); // WS15g-061 error message
    assert!(logs.text().contains("Github::WriteClient request failed")); // WS15g-061 warning
    assert!(!logs.text().contains("user-token-123")); // WS15g-061 privacy
}
#[tokio::test]
async fn cutover_d_mapping_race_recovers_unique_index_and_validation_losers_over_http() {
    for index in [true, false] {
        let f = fixture(vec![]).await;
        let (message, pr, before) = f
            .app
            .db
            .write(move |tx| {
                let (m, p) = pr_message(tx, 12, "discuss-card-1")?;
                let before = tx
                    .conn()
                    .prepare("SELECT id FROM channel_threads ORDER BY id")?
                    .query_map([], |r| r.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                crate::integrations::github::threads::test_creation_race::set(Box::new(
                    move |tx| {
                        let winner_parent = create_message(
                            tx,
                            m.room_id,
                            id("jz"),
                            "race-winner-parent",
                            "review https://github.com/rails/rails/pull/12",
                        )?;
                        let winner = ChannelThread::create(
                            tx,
                            NewChannelThread {
                                room_id: m.room_id,
                                creator_id: id("jz"),
                                parent_message_id: Some(winner_parent.id),
                                name: Some("Winning chat".into()),
                                ..Default::default()
                            },
                        )?;
                        campfire_db::ThreadMembership::join(tx, winner.id, id("jz"))?;
                        PullRequestThread::create(tx, p.id, m.room_id, winner.id)?;
                        Err(if index {
                            campfire_db::Error::Sqlite(rusqlite::Error::SqliteFailure(
                                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE),
                                Some("index_github_pr_threads_on_pr_and_room".into()),
                            ))
                        } else {
                            campfire_db::Error::RecordInvalid(campfire_db::Errors(vec![(
                                "github_pull_request_id",
                                "has already been taken".into(),
                            )]))
                        })
                    },
                ));
                Ok((m.id, p.id, before))
            })
            .await
            .unwrap();
        let (status, headers, body) = request(
            &f,
            "POST",
            &format!("/rooms/{}/github/pull_request_threads", id("designers")),
            json!({"pull_request_id":pr,"message_id":message}),
            json!({}),
        )
        .await;
        assert_eq!(status, 303, "index={index}: {body}");
        let mapping = f
            .app
            .db
            .read(move |c| {
                assert_eq!(
                    c.query_row(
                        "SELECT COUNT(*) FROM github_pull_request_threads",
                        [],
                        |r| r.get::<_, i64>(0)
                    )?,
                    1
                ); // WS15g-017,018 mapping delta and total
                let mapping = PullRequestThread::for_room_pr(c, id("designers"), pr)?.unwrap();
                let ids = c
                    .prepare("SELECT id FROM channel_threads ORDER BY id")?
                    .query_map([], |r| r.get::<_, i64>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                assert_eq!(
                    ids.into_iter()
                        .filter(|i| !before.contains(i))
                        .collect::<Vec<_>>(),
                    vec![mapping.channel_thread_id]
                ); // WS15g-017,018 loser removed
                Ok(mapping)
            })
            .await
            .unwrap();
        assert_eq!(status, 303); // WS15g-017,018 redirect
        assert_eq!(
            headers["location"],
            format!(
                "http://example.org/rooms/{}/threads/{}",
                id("designers"),
                mapping.channel_thread_id
            )
        ); // WS15g-017,018 winner redirect
    }
}
#[tokio::test]
async fn cutover_d_approval_completion_webhook_has_exact_agent_and_action() {
    use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer};
    let webhook = FakeServer::start(vec![
        Route::new("POST", "example.com", "/bender", 200)
            .body("{}")
            .header("content-type", "application/json"),
    ])
    .await;
    let net = crate::integrations::test_support::network(
        std::sync::Arc::new(FakeResolver::new([("example.com", vec!["93.184.216.34"])])),
        std::sync::Arc::new(MappingDialer {
            public: std::collections::HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: webhook.addr,
            dialed: Default::default(),
        }),
    );
    let mut f = clean_fixture(
        Fresh::with_networks(
            &json!({"mapping":false,"reference":false}),
            vec![
                Route::new(
                    "POST",
                    "api.github.com",
                    "/repos/rails/rails/issues/12/comments",
                    201,
                )
                .body(r#"{"html_url":"https://github.com/rails/rails/pull/12#issuecomment-1"}"#),
            ],
            net,
        )
        .await,
    )
    .await;
    as_user(&mut f, id("david")).await;
    let agent = agent_fixture(&f).await;
    let approval = approval(&f, "github.comment", "Nice").await;
    decide(&f, approval, "approved").await;
    run_action(&f, approval, true).await;
    let completion = completion(&f, approval).await;
    let received = webhook.received();
    let requests = received
        .iter()
        .filter(|request| {
            request.method == "POST"
                && request.header("Host") == Some("example.com")
                && request.target == "/bender"
        })
        .collect::<Vec<_>>();
    assert_eq!(requests.len(), 2); // WS15g-007 approval plus completion
    let matched = requests
        .iter()
        .filter(|r| {
            let body: Value = serde_json::from_slice(&r.body).unwrap();
            body["agent"]["delivery_id"] == completion.0
        })
        .collect::<Vec<_>>();
    assert_eq!(matched.len(), 1); // WS15g-007 exact delivery once
    let body: Value = serde_json::from_slice(&matched[0].body).unwrap();
    assert_eq!(matched[0].method, "POST"); // WS15g-007 POST actual stub
    assert_eq!(matched[0].target, "/bender"); // WS15g-007 webhook actual stub
    assert_eq!(body["agent"]["id"], agent); // WS15g-007 agent id
    assert_eq!(body["agent"]["name"], "Bender Bot"); // WS15g-007 agent name
    assert_eq!(body["agent"]["delivery_id"], completion.0); // WS15g-007 delivery id
    assert_eq!(
        body["github_action"],
        json!({"approval_id":approval,"action":"github.comment","status":"completed","url":"https://github.com/rails/rails/pull/12#issuecomment-1"})
    ); // WS15g-007 action
}
#[tokio::test]
async fn cutover_d_private_card_relink_retires_denial_and_transport_error_is_not_cached() {
    for transport in [false, true] {
        let mut first = Route::new(
            "GET",
            "api.github.com",
            "/repos/acme/secret",
            if transport { 200 } else { 404 },
        )
        .body("{}");
        if transport {
            first.delay = std::time::Duration::from_secs(11);
        }
        let f = fixture(vec![
            Route::new("GET", "api.github.com", "/user", 200).body(r#"{"login":"david"}"#),
            first,
        ])
        .await;
        let sudo = grant_sudo_access(&f).await;
        let (message,pr)=f.app.db.write(move|tx|{
            let m=create_message(tx,id("designers"),id("david"),"card-frame-1","review https://github.com/acme/secret/pull/7")?;let p=PullRequest::for_message(tx.conn(),m.id)?.remove(0);
            tx.conn().execute("UPDATE github_pull_requests SET private=1,title='Secret plans',author_login='alice',state='open',base_branch='main',head_branch='secret',review_decision='approved',check_status='passing',html_url='https://github.com/acme/secret/pull/7',github_updated_at=?,fetched_at=?,fetch_error=NULL,fetch_requested_at=NULL WHERE id=?",params![tx.now().ago(jiff::SignedDuration::from_hours(1)),tx.now(),p.id])?;
            if transport {Account::create(tx,&crypto(),&AccountInput{user_id:id("david"),github_login:"david",access_token:"user-token",refresh_token:None,token_expires_at:None,token_source:"pat"})?;}
            Ok((m.id,p.id))
        }).await.unwrap();
        if !transport {
            let (status, headers, _) = request(
                &f,
                "POST",
                "/github/connection",
                json!({"access_token":"alpha-link"}),
                sudo.clone(),
            )
            .await;
            assert_eq!(status, 302); // WS15g-023 first link redirect
            assert_eq!(headers["location"], "http://example.org/users/me/profile"); // WS15g-023
        }
        let path = format!(
            "/rooms/{}/github/pull_requests/{pr}/card?message_id={message}",
            id("designers")
        );
        let (status, _, html) = request(&f, "GET", &path, Value::Null, json!({})).await;
        assert_eq!(status, 200); // WS15g-023,024 empty-frame response
        assert_eq!(class_count(&html, "github-pr-card"), 0); // WS15g-023,024 no card
        if transport {
            assert!(!html.contains("Secret plans"));
        } // WS15g-024 no leaked title
        if !transport {
            f.clock.advance(jiff::SignedDuration::from_millis(100));
            let (status, headers, _) = request(
                &f,
                "POST",
                "/github/connection",
                json!({"access_token":"alpha-link"}),
                sudo.clone(),
            )
            .await;
            assert_eq!(status, 302); // WS15g-023 relink redirect
            assert_eq!(headers["location"], "http://example.org/users/me/profile"); // WS15g-023
        }
        f.server.replace_routes(vec![
            Route::new("GET", "api.github.com", "/repos/acme/secret", 200)
                .body(r#"{"private":true}"#),
        ]);
        let (status, _, html) = request(&f, "GET", &path, Value::Null, json!({})).await;
        assert_eq!(status, 200); // WS15g-023,024 recovered status
        assert!(html.contains("class=\"github-pr-card__title\">Secret plans</p>")); // WS15g-023,024 recovered card
        assert_eq!(
            f.server
                .received()
                .iter()
                .filter(|r| r.method == "GET" && r.target == "/repos/acme/secret")
                .count(),
            2
        ); // WS15g-023 exact two reads
    }
}
#[tokio::test]
async fn cutover_d_room_http_query_count_stays_flat_for_two_then_six_pr_messages() {
    let f = fixture(vec![]).await;
    let room = id("designers");
    f.app
        .db
        .write(|tx| {
            for i in 0..2 {
                let (_, p) = pr_message(tx, 200 + i, &format!("card-query-{}", 200 + i))?;
                filled_card(tx, p.id, false)?;
                tx.conn().execute("UPDATE github_pull_requests SET html_url='https://github.com/rails/rails/pull/123', fetched_at=? WHERE id=?",params![tx.now().ago(jiff::SignedDuration::from_mins(1)),p.id])?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/rooms/{room}");
    assert_eq!(
        request(&f, "GET", &path, Value::Null, json!({})).await.0,
        200
    ); // WS15g-042 warm
    let reads = f.app.db.capture_queries();
    let small = request(&f, "GET", &path, Value::Null, json!({})).await;
    f.app.db.stop_capturing_queries();
    let small_sql = reads.lock().unwrap().clone();
    let small_queries = small_sql.len();
    assert_eq!(small.0, 200); // WS15g-042 small
    f.app
        .db
        .write(|tx| {
            for i in 0..4 {
                let (_, p) = pr_message(tx, 300 + i, &format!("card-query-{}", 300 + i))?;
                filled_card(tx, p.id, false)?;
                tx.conn().execute("UPDATE github_pull_requests SET html_url='https://github.com/rails/rails/pull/123', fetched_at=? WHERE id=?",params![tx.now().ago(jiff::SignedDuration::from_mins(1)),p.id])?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let reads = f.app.db.capture_queries();
    let large = request(&f, "GET", &path, Value::Null, json!({})).await;
    f.app.db.stop_capturing_queries();
    let large_sql = reads.lock().unwrap().clone();
    let large_queries = large_sql.len();
    assert_eq!(large.0, 200); // WS15g-042 large
    assert_eq!(
        small_queries, large_queries,
        "room SELECTs with two/six PR messages"
    ); // WS15g-042 actual HTTP read count
}
async fn run_registered(f: &Fresh, classes: &[&str]) {
    let classes = classes.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let registry = crate::jobs::registry();
    let config = crate::queue::runner_config(&f.app.config);
    let (_, adhoc) = crate::queue::Jobs::new(&registry, &config).unwrap();
    let runner = crate::jobs::start(
        f.app.clone(),
        registry,
        adhoc,
        config,
        crate::jobs::periodic::Loops::new(crate::jobs::periodic::Intervals {
            periodic: None,
            huddle: None,
        }),
    );
    tokio::time::timeout(std::time::Duration::from_secs(10),async{loop{
        let list=classes.clone();let done=f.app.db.read(move|c|Ok(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class IN (SELECT value FROM json_each(?)) AND status IN ('ready','running')",[json!(list).to_string()],|r|r.get::<_,i64>(0))?==0)).await.unwrap();
        if done {break;}tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }}).await.unwrap();
    runner.shutdown(std::time::Duration::from_secs(2)).await;
}
fn subscription_payload(action: &str, reviewer: &str) -> Value {
    json!({"action":action,"sender":{"login":if action=="review_requested"{"bob"}else{"alice"}},"repository":{"full_name":"rails/rails","private":false},"pull_request":{"number":12,"title":"Fix login","html_url":"https://github.com/rails/rails/pull/12","merged":false,"merged_by":{"login":"alice"},"closed_at":"2026-09-17T12:00:00Z","base":{"repo":{"full_name":"rails/rails"}}},"requested_reviewer":{"login":reviewer}})
}
#[tokio::test]
async fn cutover_d_review_notification_registered_job_scopes_access_preference_and_thread_source() {
    for case in ["linked", "off", "thread"] {
        let f = fixture(vec![]).await;
        let (thread,before_messages,before_items)=f.app.db.write(move|tx|{
            crate::integrations::github::subscriptions::RepositorySubscription::create(tx,id("designers"),"rails","rails",json!(["opened","merged","closed","review_requested","review_submitted","checks_failed"]),Some(id("david")),false)?;
            campfire_db::models::user::profile_settings::update(tx,id("kevin"),campfire_db::models::user::profile_settings::Changes{github_login:Some("kevin-gh".into()),inbox_preferences:(case=="off").then(||json!({"github_review_requests":false})),..Default::default()})?;
            let thread=if case=="thread"{let (m,p)=pr_message(tx,12,"notifier-thread-review")?;Some(discuss(tx,&m,&p)?.id)}else{None};
            let before_messages=tx.conn().query_row("SELECT COUNT(*) FROM messages WHERE room_id=?",[id("designers")],|r|r.get::<_,i64>(0))?;
            let before_items=tx.conn().query_row(if case=="linked"{"SELECT COUNT(*) FROM activity_items WHERE event_type='pr_review_request' AND user_id=?"}else{"SELECT COUNT(*) FROM activity_items WHERE event_type='pr_review_request' AND ? IS NOT NULL"},[id("kevin")],|r|r.get::<_,i64>(0))?;
            tx.conn().execute("DELETE FROM background_jobs",[])?;
            tx.emit_after_commit(campfire_db::Event::job(&crate::integrations::github::jobs::DeliverSubscriptionEventJob{event:"pull_request".into(),payload:subscription_payload("review_requested",if case=="off"{"kevin-gh"}else{"Kevin-GH"})}));
            Ok((thread,before_messages,before_items))
        }).await.unwrap();
        run_registered(&f, &["Github::DeliverSubscriptionEventJob"]).await;
        let secrets = f.app.secrets.clone();
        f.app.db.write(move|tx|{
            let message=tx.conn().query_row("SELECT id,markdown_source,thread_id FROM messages WHERE room_id=? ORDER BY created_at DESC,id DESC LIMIT 1",[id("designers")],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<i64>>(2)?)))?;
            let count=tx.conn().query_row("SELECT COUNT(*) FROM messages WHERE room_id=?",[id("designers")],|r|r.get::<_,i64>(0))?;
            let items=tx.conn().query_row(if case=="linked"{"SELECT COUNT(*) FROM activity_items WHERE event_type='pr_review_request' AND user_id=?"}else{"SELECT COUNT(*) FROM activity_items WHERE event_type='pr_review_request' AND ? IS NOT NULL"},[id("kevin")],|r|r.get::<_,i64>(0))?;
            assert_eq!(count,before_messages+1); // WS15g-054 message delta
            if case=="off" {
                assert_eq!(items,before_items); // WS15g-054 scoped no review inbox
                let mention=Message::create(tx,NewMessage{room_id:id("designers"),creator_id:id("david"),body:Some(format!("Hey <action-text-attachment sgid=\"{}\" content-type=\"application/vnd.campfire.mention\"></action-text-attachment>", rails_compat::global_id::attachable_sgid(&secrets,&rails_compat::global_id::GlobalId::new("User",id("kevin"))))),client_message_id:Some("review-switch-neighbour".into()),..Default::default()})?;
                assert_eq!(campfire_db::ActivityItem::find_by_user_and_source(tx.conn(),id("kevin"),"Message",mention.id)?.unwrap().event_type,"mention"); // WS15g-054 neighboring mention
            } else {
                assert_eq!(items,before_items+1); // WS15g-053 scoped item delta
                assert_eq!(message.1,"**bob** requested a review from **Kevin-GH** on #12: Fix login\nhttps://github.com/rails/rails/pull/12"); // WS15g-053,055 body
                assert_eq!(message.2,thread); // WS15g-055 thread reply
                let item_id=tx.conn().query_row("SELECT id FROM activity_items WHERE user_id=? AND event_type='pr_review_request' ORDER BY id LIMIT 1",[id("kevin")],|r|r.get::<_,i64>(0))?;
                let item=campfire_db::ActivityItem::find(tx.conn(),item_id)?;
                assert_eq!((item.source_type.as_str(),item.source_id),("Message",message.0)); // WS15g-053,055 source
                let kevin=User::find(tx.conn(),id("kevin"))?;
                assert!(campfire_db::ActivityItem::accessible_to(tx.conn(),&kevin)?.iter().any(|i|i.id==item.id)); // WS15g-053,055 accessible
                if case=="linked" {
                    tx.conn().execute("DELETE FROM memberships WHERE user_id=? AND room_id=?",params![id("kevin"),id("designers")])?;
                    assert!(campfire_db::ActivityItem::find_accessible(tx.conn(),&kevin,item.id)?.is_none()); // WS15g-053 inaccessible after membership removal
                }
            }Ok(())
        }).await.unwrap();
    }
}
fn fetch_routes() -> Vec<Route> {
    vec![
        Route::new("GET","api.github.com","/repos/rails/rails/pulls/123",200).body(json!({"number":123,"title":"Add shiny things","state":"open","draft":false,"merged_at":null,"html_url":"https://github.com/rails/rails/pull/123","updated_at":"2026-09-15T12:00:00Z","user":{"login":"dhh","avatar_url":"https://avatars.example/dhh"},"base":{"ref":"main","repo":{"private":false}},"head":{"ref":"shiny","sha":"abc123"},"changed_files":1}).to_string()),
        Route::new("GET","api.github.com","/repos/rails/rails/pulls/123/reviews?per_page=100",200).body("[]"),
        Route::new("GET","api.github.com","/repos/rails/rails/commits/abc123/check-runs?per_page=100",200).body(r#"{"check_runs":[]}"#),
        Route::new("GET","api.github.com","/repos/rails/rails/commits/abc123/status",200).body(r#"{"state":"success","total_count":1}"#),
        Route::new("GET","api.github.com","/repos/rails/rails/pulls/123/files?per_page=100",200).body(r#"[{"filename":"app/models/user.rb","additions":3,"deletions":1,"status":"modified"}]"#)
    ]
}
async fn socket(
    f: &Fresh,
    model: &str,
    id: i64,
) -> (
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio::task::JoinHandle<()>,
) {
    use futures_util::SinkExt;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let listener = crate::test_support::bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = f.app.cable.router::<()>("/cable");
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{address}").parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", f.cookie.parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert_eq!(socket_frame(&mut socket).await, json!({"type":"welcome"}));
    let gid = rails_compat::global_id::GlobalId::new(model, id).to_param();
    let identifier=json!({"channel":"RoomMessagesChannel","signed_stream_name":rails_compat::turbo::signed_stream_name(&f.app.secrets,&[&gid,"messages"])}).to_string();
    socket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(
        socket_frame(&mut socket).await["type"],
        "confirm_subscription"
    );
    (socket, server)
}
async fn socket_frame(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Value {
    use futures_util::StreamExt;
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) =
                socket.next().await
            {
                let frame: Value = serde_json::from_str(&text).unwrap();
                if frame["type"] != "ping" {
                    return frame;
                }
            }
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn cutover_d_fetch_registered_job_replaces_mapped_header_and_is_silent_without_references() {
    for mapped in [true, false] {
        let mut f = clean_fixture(
            Fresh::with_routes(
                &json!({"mapping":false,"reference":false,"reader_token":"test-token"}),
                fetch_routes(),
            )
            .await,
        )
        .await;
        as_user(&mut f, id("david")).await;
        let (pr, thread) = f
            .app
            .db
            .write(move |tx| {
                let p = PullRequest::for_reference(tx, "rails", "rails", 123)?;
                let t = if mapped {
                    let m = create_message(
                        tx,
                        id("designers"),
                        id("david"),
                        "fetch-files-123",
                        "review https://github.com/rails/rails/pull/123",
                    )?;
                    Some(discuss(tx, &m, &p)?.id)
                } else {
                    None
                };
                tx.conn().execute("DELETE FROM background_jobs", [])?;
                Ok((p.id, t))
            })
            .await
            .unwrap();
        let (mut socket, serving) = socket(
            &f,
            if mapped {
                "ChannelThread"
            } else {
                "Rooms::Closed"
            },
            thread.unwrap_or(id("designers")),
        )
        .await;
        let publications = f.app.cable.capture_publications();
        f.app
            .db
            .write(move |tx| {
                tx.emit_after_commit(campfire_db::Event::job(
                    &crate::integrations::github::jobs::FetchPullRequestJob {
                        pull_request_id: pr,
                    },
                ));
                Ok(())
            })
            .await
            .unwrap();
        run_registered(&f, &["Github::FetchPullRequestJob"]).await;
        if let Some(thread) = thread {
            let target = format!("github_pr_header_channel_thread_{thread}");
            let frame = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    let frame = socket_frame(&mut socket).await;
                    if frame["message"]
                        .as_str()
                        .is_some_and(|html| html.contains(&target))
                    {
                        break frame;
                    }
                }
            })
            .await
            .unwrap();
            let html = frame["message"].as_str().unwrap();
            let (dom, root) = parse_markup(html);
            let header_streams = dom
                .descendants(root)
                .into_iter()
                .filter(|&node| {
                    dom.local_name(node) == Some("turbo-stream")
                        && dom.attr(node, "action") == Some("replace")
                        && dom.attr(node, "target") == Some(target.as_str())
                })
                .collect::<Vec<_>>();
            assert_eq!(header_streams.len(), 1, "{html}"); // WS15g-056 exact header replace
            let header_stream = header_streams[0];
            assert_eq!(class_nodes(&dom, header_stream, "github-pr-card").len(), 1); // WS15g-056 scoped card
            assert!(
                dom.text_content(class_nodes(&dom, header_stream, "github-pr-card__title")[0])
                    .contains("Add shiny things")
            ); // WS15g-056 scoped title
            assert!(
                dom.text_content(class_nodes(&dom, header_stream, "github-pr-files__path")[0])
                    .contains("app/models/user.rb")
            ); // WS15g-056 scoped path
        } else {
            f.app
                .db
                .read(move |c| {
                    assert_eq!(
                        PullRequest::find(c, pr)?.title.as_deref(),
                        Some("Add shiny things")
                    );
                    Ok(())
                })
                .await
                .unwrap();
            assert!(publications.take().is_empty()); // WS15g-057 no card/header broadcast on actual subscribed room
        }
        serving.abort();
    }
}
async fn webhook_post(f: &Fresh, body: Value, delivery: &str) -> u16 {
    use hmac::Mac;
    use tower::ServiceExt;
    let raw = body.to_string();
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(b"webhook-secret").unwrap();
    mac.update(raw.as_bytes());
    let signature = format!("sha256={:x}", mac.finalize().into_bytes());
    let response = f
        .router
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/github/webhooks")
                .header("Host", "example.org")
                .header("Content-Type", "application/json")
                .header("X-Hub-Signature-256", signature)
                .header("X-GitHub-Delivery", delivery)
                .header("X-GitHub-Event", "pull_request")
                .body(axum::body::Body::from(raw))
                .unwrap(),
        )
        .await
        .unwrap();
    response.status().as_u16()
}
#[tokio::test]
async fn cutover_d_webhook_signed_fetch_updates_and_broadcasts_once_to_its_room() {
    let mut f = clean_fixture(
        Fresh::with_routes(
            &json!({"reader_token":"test-token","webhook_secret":"webhook-secret"}),
            fetch_routes(),
        )
        .await,
    )
    .await;
    as_user(&mut f, id("david")).await;
    let pr = f
        .app
        .db
        .write(|tx| {
            let (m, p) = pr_message(tx, 123, "webhook-ref-1")?;
            let _ = m;
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            Ok(p.id)
        })
        .await
        .unwrap();
    let (_socket, serving) = socket(&f, "Rooms::Closed", id("designers")).await;
    let publications = f.app.cable.capture_publications();
    assert_eq!(webhook_post(&f,json!({"repository":{"full_name":"rails/rails"},"pull_request":{"number":123,"base":{"repo":{"full_name":"rails/rails"}}}}),"d-valid-fetch").await,200); // WS15g-020 status
    run_registered(&f, &["Github::FetchPullRequestJob"]).await;
    let room = rails_compat::global_id::GlobalId::new("Rooms::Closed", id("designers")).to_param()
        + ":messages";
    assert_eq!(
        publications
            .take()
            .iter()
            .filter(|(stream, _)| stream == &room)
            .count(),
        1
    ); // WS15g-020 scoped exact broadcast count
    f.app
        .db
        .read(move |c| {
            assert_eq!(
                PullRequest::find(c, pr)?.title.as_deref(),
                Some("Add shiny things")
            );
            Ok(())
        })
        .await
        .unwrap(); // WS15g-020 saved title
    serving.abort();
}
#[tokio::test]
async fn cutover_d_webhook_subscription_queues_posts_and_deduplicates_redelivery() {
    let f = clean_fixture(
        Fresh::with_routes(&json!({"webhook_secret":"webhook-secret"}), vec![]).await,
    )
    .await;
    let (before,job_before)=f.app.db.write(|tx|{
        pr_message(tx,123,"webhook-ref-1")?;
        crate::integrations::github::subscriptions::RepositorySubscription::create(tx,id("designers"),"rails","rails",json!(crate::integrations::github::subscriptions::DEFAULT_EVENTS),Some(id("david")),false)?;
        tx.conn().execute("DELETE FROM background_jobs",[])?;
        Ok((tx.conn().query_row("SELECT COUNT(*) FROM messages WHERE room_id=?",[id("designers")],|r|r.get::<_,i64>(0))?,tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::DeliverSubscriptionEventJob'",[],|r|r.get::<_,i64>(0))?))
    }).await.unwrap();
    let mut body = subscription_payload("opened", "carol");
    body.as_object_mut().unwrap().remove("requested_reviewer");
    body["repository"]
        .as_object_mut()
        .unwrap()
        .remove("private");
    body["pull_request"]
        .as_object_mut()
        .unwrap()
        .remove("merged_by");
    assert_eq!(webhook_post(&f, body.clone(), "d-subscription").await, 200); // WS15g-021,022 first status
    f.app.db.write(move|tx|{
        let jobs=tx.conn().prepare("SELECT arguments FROM background_jobs WHERE job_class='Github::DeliverSubscriptionEventJob'")?.query_map([],|r|Ok(serde_json::from_str::<Value>(&r.get::<_,String>(0)?).unwrap()))?.collect::<rusqlite::Result<Vec<_>>>()?;
        assert_eq!(jobs.len() as i64,job_before+1); // WS15g-021 enqueue delta
        tx.conn().execute("DELETE FROM background_jobs WHERE job_class!='Github::DeliverSubscriptionEventJob'",[])?;Ok(())
    }).await.unwrap();
    run_registered(&f, &["Github::DeliverSubscriptionEventJob"]).await;
    f.app.db.read(move|c|{
        assert_eq!(c.query_row("SELECT COUNT(*) FROM messages WHERE room_id=?",[id("designers")],|r|r.get::<_,i64>(0))?,before+1); // WS15g-021 room delta
        let message=Message::find(c,c.query_row("SELECT id FROM messages WHERE room_id=? ORDER BY created_at DESC,id DESC LIMIT 1",[id("designers")],|r|r.get::<_,i64>(0))?)?;
        let bot=User::find(c,message.creator_id)?;
        assert_eq!(bot.name,"GitHub"); // WS15g-021 creator
        assert!(bot.is_bot()&&bot.is_active()); // WS15g-021 active bot
        assert!(message.markdown_source.as_deref().unwrap().contains("https://github.com/rails/rails/pull/12")); // WS15g-021 body URL
        assert_eq!(PullRequest::for_message(c,message.id)?.iter().map(|p|p.number).collect::<Vec<_>>(),vec![12]); // WS15g-021 refs
        assert_eq!(c.query_row("SELECT COUNT(*) FROM messages WHERE room_id=? AND markdown_source LIKE '%opened pull request%'",[id("designers")],|r|r.get::<_,i64>(0))?,1); // WS15g-022 posted exactly once
        Ok(())
    }).await.unwrap();
    let jobs_before=f.app.db.read(|c|Ok(c.prepare("SELECT arguments FROM background_jobs WHERE job_class='Github::DeliverSubscriptionEventJob'")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap();
    assert_eq!(webhook_post(&f, body, "d-subscription").await, 200); // WS15g-022 replay status
    let jobs_after=f.app.db.read(|c|Ok(c.prepare("SELECT arguments FROM background_jobs WHERE job_class='Github::DeliverSubscriptionEventJob'")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap();
    assert_eq!(jobs_after, jobs_before); // WS15g-022 replay no jobs
}
