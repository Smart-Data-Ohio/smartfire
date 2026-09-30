//! Named request ports of test/controllers/searches_controller_test.rb.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Message, NewMessage, Search};
async fn app() -> TestApp {
    TestApp::boot().await.expect("WS8bm2 requires default seed")
}
async fn message(app: &TestApp, user: i64, text: &str) -> Message {
    let text = text.to_owned();
    app.db()
        .write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: user,
                    markdown_source: Some(text),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap()
}
async fn find(browser: &mut Browser<'_>, q: &str) -> Reply {
    browser.get(&campfire_views::searches::search_path(q)).await
}
fn count(reply: &Reply) -> usize {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    reply.text().matches("class=\"message\"").count()
}
async fn history(app: &TestApp) -> Vec<Search> {
    app.db()
        .read(|c| Search::ordered_for_user(c, DAVID))
        .await
        .unwrap()
}
async fn record(app: &TestApp, q: &str) {
    let q = q.to_owned();
    app.db()
        .write(move |tx| Search::record(tx, DAVID, &q))
        .await
        .unwrap();
}
#[tokio::test]
async fn index_initial_view() {
    let app = app().await;
    assert_eq!(count(&app.david().get("/searches").await), 0);
}
#[tokio::test]
async fn finding_reachable_messages() {
    let app = app().await;
    message(&app, DAVID, "needlealpha Hello world!").await;
    let r = find(&mut app.david(), "needlealpha").await;
    assert_eq!(count(&r), 1);
    assert!(r.text().contains("Hello world!"));
}
#[tokio::test]
async fn unreachable_messages_are_not_found() {
    let app = app().await;
    message(&app, DAVID, "needlealpha Hello world!").await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                (DAVID, ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(count(&find(&mut app.david(), "needlealpha").await), 0);
}
#[tokio::test]
async fn operator_words_are_searched_literally_instead_of_raising() {
    let app = app().await;
    message(&app, DAVID, "needlealpha cats and dogs").await;
    let mut b = app.david();
    assert_eq!(find(&mut b, "NOT").await.status, StatusCode::OK);
    assert_eq!(count(&find(&mut b, "needlealpha cats AND").await), 1);
}
#[tokio::test]
async fn a_leading_operator_returns_200() {
    let app = app().await;
    assert_eq!(
        find(&mut app.david(), "OR bar").await.status,
        StatusCode::OK
    );
}
#[tokio::test]
async fn a_boolean_looking_query_does_not_exclude_terms() {
    let app = app().await;
    message(&app, DAVID, "needlealpha foo bar").await;
    message(&app, DAVID, "needlealpha foo not bar").await;
    let r = find(&mut app.david(), "needlealpha foo NOT bar").await;
    assert_eq!(count(&r), 1);
    assert!(r.text().contains("foo not bar"));
}
#[tokio::test]
async fn a_quote_character_returns_200_with_sensible_results() {
    let app = app().await;
    message(&app, DAVID, "needlealpha").await;
    assert_eq!(count(&find(&mut app.david(), "\"needlealpha\"").await), 1);
}
#[tokio::test]
async fn create_does_not_run_the_search() {
    let app = app().await;
    app.db()
        .write(|tx| {
            tx.conn().execute_batch("DROP TABLE message_search_index")?;
            Ok(())
        })
        .await
        .unwrap();
    let r = app
        .david()
        .write(Req::new(Method::POST, "/searches").form(&[("q", "NOT")]))
        .await;
    assert_eq!(r.location(), Some("http://campfire.test/searches?q=NOT"));
    assert!(history(&app).await.iter().any(|s| s.query == "NOT"));
}
#[tokio::test]
async fn clear_does_not_run_the_search() {
    let app = app().await;
    app.db()
        .write(|tx| {
            tx.conn().execute_batch("DROP TABLE message_search_index")?;
            Ok(())
        })
        .await
        .unwrap();
    let r = app
        .david()
        .write(Req::new(Method::DELETE, "/searches/clear").form(&[("q", "NOT")]))
        .await;
    assert_eq!(r.location(), Some("http://campfire.test/searches"));
}
#[tokio::test]
async fn clear_answers_turbo_with_streams_that_empty_header_and_page_recents() {
    let app = app().await;
    record(&app, "hello").await;
    let r = app
        .david()
        .write(
            Req::new(Method::DELETE, "/searches/clear")
                .header("accept", "text/vnd.turbo-stream.html"),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.headers["content-type"],
        "text/vnd.turbo-stream.html; charset=utf-8"
    );
    for t in ["global-search-recents", "search-recents"] {
        assert!(
            r.text()
                .contains(&format!("action=\"update\" target=\"{t}\""))
        );
    }
    assert!(r.text().contains("No recent searches yet."));
    assert!(history(&app).await.is_empty());
}
#[tokio::test]
async fn clear_leaves_recents_alone_for_unsupported_formats() {
    let app = app().await;
    record(&app, "hello").await;
    assert_eq!(
        app.david()
            .write(Req::new(Method::DELETE, "/searches/clear").header("accept", "application/json"))
            .await
            .status,
        StatusCode::NOT_ACCEPTABLE
    );
    assert!(history(&app).await.iter().any(|s| s.query == "hello"));
}
#[tokio::test]
async fn the_header_renders_at_most_ten_recents() {
    let app = app().await;
    app.db().write(|tx|{Search::destroy_all_for_user(tx,DAVID)?;for i in 0..12{tx.conn().execute("INSERT INTO searches(user_id,query,dedup_key,created_at,updated_at)VALUES(?,?,?,?,?)",(DAVID,format!("old {i}"),format!("old {i}"),tx.now(),tx.now()))?;}Ok(())}).await.unwrap();
    let r = app.david().get(&format!("/rooms/{ALL_TALK}")).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.text().matches("role=\"option\"").count(), 10);
}
#[tokio::test]
async fn clear_without_turbo_returns_to_the_page_it_came_from() {
    let app = app().await;
    let r = app
        .david()
        .write(
            Req::new(Method::DELETE, "/searches/clear")
                .header("referer", "http://campfire.test/saved"),
        )
        .await;
    assert_eq!(r.location(), Some("http://campfire.test/saved"));
}
#[tokio::test]
async fn index_renders_html_for_turbo_without_an_older_cursor() {
    let app = app().await;
    message(&app, DAVID, "needlealpha").await;
    let r = app
        .david()
        .send(
            Req::new(Method::GET, "/searches?q=needlealpha")
                .header("accept", "text/vnd.turbo-stream.html, text/html"),
        )
        .await;
    assert_eq!(count(&r), 1);
    assert!(
        r.headers["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(r.text().contains("value=\"needlealpha\""));
}
#[tokio::test]
async fn the_header_search_field_is_empty_outside_search() {
    let app = app().await;
    record(&app, "needlealpha").await;
    let r = app.david().get("/saved?q=shouldnotprefill").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.text().contains("id=\"global-search-input\""));
    assert!(!r.text().contains("value=\"shouldnotprefill\""));
    assert!(r.text().contains("data-query=\"needlealpha\""));
}
#[tokio::test]
async fn the_search_page_without_a_query_lists_recents() {
    let app = app().await;
    record(&app, "needlealpha").await;
    let r = app.david().get("/searches").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.text().contains("searches__recents"));
    assert!(!r.text().contains("message-area--empty"));
    assert!(!r.text().contains("composer__input"));
}
#[tokio::test]
async fn a_query_with_no_results_shows_an_empty_state() {
    let app = app().await;
    let r = find(&mut app.david(), "zebra stripes tuxedo").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.text().contains("class=\"searches__empty\""));
}
async fn paging_fixture(app: &TestApp) -> Vec<i64> {
    let mut ids = vec![];
    for i in 0..45 {
        ids.push(
            message(app, DAVID, &format!("needlepaging number {i:02}"))
                .await
                .id,
        );
    }
    app.db().write(|tx|{tx.conn().execute("UPDATE messages SET created_at='2026-01-01 12:00:00' WHERE markdown_source LIKE 'needlepaging%'",[])?;Ok(())}).await.unwrap();
    ids
}
#[tokio::test]
async fn results_page_through_load_older_results_with_same_timestamp() {
    let app = app().await;
    let ids = paging_fixture(&app).await;
    let mut b = app.david();
    let r = find(&mut b, "needlepaging").await;
    assert_eq!(count(&r), 40);
    assert!(r.text().contains("needlepaging number 44"));
    assert!(!r.text().contains("needlepaging number 00"));
    assert!(r.text().contains("Load older results"));
    let r = b
        .send(
            Req::new(
                Method::GET,
                &format!("/searches?q=needlepaging&before={}", ids[5]),
            )
            .header("accept", "text/vnd.turbo-stream.html"),
        )
        .await;
    assert_eq!(count(&r), 5);
    assert!(
        r.text()
            .contains("action=\"prepend\" target=\"search-results\"")
    );
    assert!(r.text().contains("needlepaging number 00"));
    assert!(r.text().contains("needlepaging number 04"));
    assert!(!r.text().contains("needlepaging number 44"));
    assert!(
        r.text()
            .contains("action=\"remove\" target=\"load_older_results\"")
    );
}
#[tokio::test]
async fn an_older_window_renders_as_a_page_without_javascript() {
    let app = app().await;
    let ids = paging_fixture(&app).await;
    let r = app
        .david()
        .get(&format!("/searches?q=needlepaging&before={}", ids[5]))
        .await;
    assert_eq!(count(&r), 5);
    assert!(!r.text().contains("Load older results"));
}
#[tokio::test]
async fn create_saves_the_search_term() {
    let app = app().await;
    let r = app
        .david()
        .write(Req::new(Method::POST, "/searches").form(&[("q", " from:@jz   has:file launch ")]))
        .await;
    assert_eq!(
        r.location(),
        Some("http://campfire.test/searches?q=from%3A%40jz+has%3Afile+launch")
    );
    assert!(
        history(&app)
            .await
            .iter()
            .any(|s| s.query == "from:@jz has:file launch")
    );
}
#[tokio::test]
async fn create_without_searchable_words_redirects_with_notice_and_records_nothing() {
    let app = app().await;
    let before = history(&app).await.len();
    let mut b = app.david();
    for q in ["???", "🙂", ""] {
        let r = b
            .write(Req::new(Method::POST, "/searches").form(&[("q", q)]))
            .await;
        assert_eq!(r.location(), Some("http://campfire.test/searches"));
        assert_eq!(history(&app).await.len(), before);
        assert!(
            b.get("/searches")
                .await
                .text()
                .contains("Enter a word to search for.")
        );
    }
}
#[tokio::test]
async fn clear_search_history() {
    let app = app().await;
    record(&app, "hello").await;
    app.david()
        .write(Req::new(Method::DELETE, "/searches/clear"))
        .await;
    assert!(history(&app).await.is_empty());
}
#[tokio::test]
async fn from_narrows_results_to_that_author() {
    let app = app().await;
    message(&app, DAVID, "needleauthor alpha").await;
    let jz = app
        .db()
        .read(|c| {
            c.query_row("SELECT id FROM users WHERE name='JZ'", [], |r| {
                r.get::<_, i64>(0)
            })
            .map_err(Into::into)
        })
        .await
        .unwrap();
    message(&app, jz, "needleauthor alpha").await;
    assert_eq!(
        count(&find(&mut app.david(), "from:@JZ needleauthor alpha").await),
        1
    );
}
#[tokio::test]
async fn in_narrows_results_to_that_room() {
    let app = app().await;
    message(&app, DAVID, "needleroom alpha").await;
    let mut b = app.david();
    assert_eq!(
        count(&find(&mut b, "in:#Designers needleroom alpha").await),
        0
    );
    assert_eq!(count(&find(&mut b, "in:#Talk needleroom alpha").await), 1);
}
#[tokio::test]
async fn in_a_room_the_user_is_not_in_returns_nothing() {
    let app = app().await;
    message(&app, DAVID, "needlehidden alpha").await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                (DAVID, ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        count(&find(&mut app.david(), "in:#Talk needlehidden alpha").await),
        0
    );
}
#[tokio::test]
async fn has_pin_narrows_results_to_pinned_messages() {
    let app = app().await;
    let m = message(&app, DAVID, "needlepin alpha").await;
    message(&app, DAVID, "needlepin alpha").await;
    app.db()
        .write(move |tx| campfire_db::MessagePin::pin(tx, &m, DAVID).map(|_| ()))
        .await
        .unwrap();
    assert_eq!(
        count(&find(&mut app.david(), "has:pin needlepin alpha").await),
        1
    );
}
#[tokio::test]
async fn has_link_narrows_results_to_messages_carrying_a_link() {
    let app = app().await;
    message(&app, DAVID, "needlelink alpha at https://example.com/alpha").await;
    message(&app, DAVID, "needlelink alpha").await;
    assert_eq!(
        count(&find(&mut app.david(), "has:link needlelink alpha").await),
        1
    );
}
#[tokio::test]
async fn filter_only_queries_list_without_text_and_show_chips() {
    let app = app().await;
    let r = find(&mut app.david(), "from:@JZ").await;
    assert!(count(&r) > 0);
    assert!(r.text().contains("from: JZ"));
}
#[tokio::test]
async fn chips_link_back_without_their_operator() {
    let app = app().await;
    let r = find(&mut app.david(), "from:@jz has:file launch").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.text().matches("class=\"search-filter-chip\"").count(), 2);
    assert!(r.text().contains("q=has%3Afile+launch"));
    assert!(r.text().contains("Remove from: jz filter"));
}
#[tokio::test]
async fn operator_values_cannot_inject_sql_or_fts_syntax() {
    let app = app().await;
    let mut b = app.david();
    for q in [
        "\" OR 1=1 --",
        "from:@\" OR \"1\"=\"1 hello",
        "in:#% hello",
        "on:2026-13-45 hello",
    ] {
        assert_eq!(find(&mut b, q).await.status, StatusCode::OK);
    }
    assert_eq!(count(&find(&mut b, "in:#% hello").await), 0);
}
#[tokio::test]
async fn search_mutations_require_csrf() {
    let app = app().await;
    let mut b = app.david();
    for method in [Method::POST, Method::DELETE] {
        let path = if method == Method::POST {
            "/searches"
        } else {
            "/searches/clear"
        };
        assert_eq!(
            b.send(Req::new(method, path).form(&[("q", "hello")]))
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
}
#[tokio::test]
async fn an_unreachable_cursor_is_404() {
    let app = app().await;
    let m = message(&app, DAVID, "needlecursor").await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                (DAVID, ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        app.david()
            .get(&format!("/searches?q=needlecursor&before={}", m.id))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../vectors/messaging/search.json")).unwrap()
}
#[tokio::test]
async fn search_parser_chips_and_empty_page_match_pinned_rails_bytes() {
    use askama::Template;
    let app = app().await;
    for case in oracle()["parsed"].as_array().unwrap() {
        let q = campfire_db::search_query::SearchQuery::parse(case["raw"].as_str().unwrap());
        assert_eq!(q.text, case["text"].as_str().unwrap());
        assert_eq!(
            serde_json::to_value(q.match_expression()).unwrap(),
            case["expression"]
        );
        assert_eq!(q.blank_query(), case["blank"].as_bool().unwrap());
        assert_eq!(q.filters(), case["filters"].as_bool().unwrap());
        assert_eq!(serde_json::to_value(q.chips).unwrap(), case["chips"]);
    }
    for case in oracle()["partials"].as_array().unwrap() {
        let chips = campfire_db::search_query::SearchQuery::parse(case["raw"].as_str().unwrap())
            .chips
            .into_iter()
            .map(|c| campfire_views::searches::Chip {
                label: c.label,
                remove_query: c.remove_query,
            })
            .collect::<Vec<_>>();
        let actual = crate::controllers::presenters::page::render_detached_at(
            &app.booted.app,
            None,
            "http://campfire.test",
            |ctx| {
                campfire_views::searches::Filters { ctx, chips: &chips }
                    .render()
                    .unwrap()
            },
        );
        assert_eq!(actual, case["html"].as_str().unwrap());
    }
    let index = campfire_views::searches::IndexView {
        query: None,
        q: None,
        messages: vec![],
        recent_searches: vec![],
        return_to_room: None,
        has_more: false,
        oldest_id: None,
        chips: vec![],
        sections: vec![],
    };
    let actual = crate::controllers::presenters::page::render_detached_at(
        &app.booted.app,
        None,
        "http://campfire.test",
        |ctx| {
            campfire_views::searches::Index { ctx, index: &index }
                .as_content()
                .render()
                .unwrap()
        },
    );
    assert_eq!(actual, oracle()["empty"].as_str().unwrap());
    let actual = crate::controllers::presenters::page::render_detached_at(
        &app.booted.app,
        None,
        "http://campfire.test",
        |ctx| campfire_views::searches::Clear { ctx }.render().unwrap(),
    );
    assert_eq!(actual, oracle()["clear"].as_str().unwrap());
}
#[tokio::test]
async fn on_narrows_results_to_that_day_in_the_users_zone_including_dst_and_missing_days() {
    let app = app().await;
    let mut b = app.david();
    for case in oracle()["dates"].as_array().unwrap() {
        let zone = case["zone"].as_str().unwrap().to_owned();
        app.db()
            .write(move |tx| {
                tx.conn()
                    .execute("UPDATE users SET time_zone=? WHERE id=?", (zone, DAVID))?;
                Ok(())
            })
            .await
            .unwrap();
        let tag = case["tag"].as_str().unwrap();
        for (i, time) in case["times"].as_array().unwrap().iter().enumerate() {
            let m = message(&app, DAVID, &format!("{tag} sample{i}")).await;
            let time = campfire_db::Timestamp::from_jiff(time.as_str().unwrap().parse().unwrap());
            app.db()
                .write(move |tx| {
                    tx.conn()
                        .execute("UPDATE messages SET created_at=? WHERE id=?", (time, m.id))?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        for op in ["on", "before", "after"] {
            let r = find(
                &mut b,
                &format!("{op}:{} {tag}", case["day"].as_str().unwrap()),
            )
            .await;
            let expected = case["results"][op].as_array().unwrap();
            assert_eq!(
                count(&r),
                expected.len(),
                "{op} {}: {}",
                case["zone"],
                r.text()
            );
            for i in 0..8 {
                assert_eq!(
                    r.text().contains(&format!("{tag} sample{i}")),
                    expected.iter().any(|x| x.as_u64() == Some(i)),
                    "{op}: {} sample{i}",
                    case["zone"]
                );
            }
        }
    }
}
#[tokio::test]
async fn is_thread_narrows_results_to_thread_messages() {
    let app = app().await;
    let thread = app
        .db()
        .write(|tx| {
            campfire_db::ChannelThread::create(
                tx,
                campfire_db::NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    name: Some("Operator thread".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    app.db()
        .write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    thread_id: Some(thread.id),
                    markdown_source: Some("needlethread alpha".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    message(&app, DAVID, "needlethread alpha").await;
    assert_eq!(
        count(&find(&mut app.david(), "is:thread needlethread alpha").await),
        1
    );
}

async fn section_fixture(app: &TestApp) -> (i64, i64) {
    app.db().write(|tx|{
  let board=campfire_db::Room::create_for(tx,campfire_db::RoomType::Board,Some("Section Board"),DAVID,&[DAVID])?;
  campfire_db::ChannelThread::create(tx,campfire_db::NewChannelThread{room_id:board.id,creator_id:DAVID,name:Some("Sectionable launch plan".into()),work_status:Some("planned".into()),..Default::default()})?;
  campfire_db::ChannelThread::create(tx,campfire_db::NewChannelThread{room_id:ALL_TALK,creator_id:DAVID,name:Some("Sectionable launch work".into()),work_status:Some("planned".into()),..Default::default()})?;
  let hidden=campfire_db::Room::create_for(tx,campfire_db::RoomType::Closed,Some("Hidden Sections"),KEVIN,&[KEVIN])?;
  campfire_db::ChannelThread::create(tx,campfire_db::NewChannelThread{room_id:hidden.id,creator_id:KEVIN,name:Some("Sectionable launch hidden".into()),work_status:Some("planned".into()),..Default::default()})?;
  let event:i64=tx.conn().query_row("INSERT INTO events(room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES(?,?,?,?,?,?,?) RETURNING id",(DIRECT_DAVID_JASON,DAVID,"Sectionable launch gathering",tx.now(),"America/New_York",tx.now(),tx.now()),|r|r.get(0))?;
  Ok((board.id,event))
 }).await.unwrap()
}
#[tokio::test]
async fn boards_work_threads_and_events_render_as_sections_scoped_to_access() {
    let app = app().await;
    section_fixture(&app).await;
    let r = find(&mut app.david(), "sectionable launch").await;
    assert_eq!(r.status, StatusCode::OK);
    for text in [
        "Sectionable launch plan",
        "Sectionable launch work",
        "Sectionable launch gathering",
        "Board posts",
        "Work threads",
        "Events",
    ] {
        assert!(r.text().contains(text), "{text}: {}", r.text());
    }
    assert!(!r.text().contains("Sectionable launch hidden"));
}
#[tokio::test]
async fn sections_exclude_soft_deleted_rooms() {
    let app = app().await;
    let (board, _) = section_fixture(&app).await;
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE rooms SET deleted_at=? WHERE id IN (?,?)",
                (tx.now(), board, ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let r = find(&mut app.david(), "sectionable launch").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(!r.text().contains("Sectionable launch plan"));
    assert!(!r.text().contains("Sectionable launch work"));
    assert!(r.text().contains("Sectionable launch gathering"));
}
#[tokio::test]
async fn search_sections_label_direct_rooms_neutrally() {
    let app = app().await;
    section_fixture(&app).await;
    let r = find(&mut app.david(), "sectionable launch").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.text().contains("a direct message ·"));
}
#[tokio::test]
async fn search_sections_render_without_queries_regardless_of_section_size() {
    use askama::Template;
    let app = app().await;
    section_fixture(&app).await;
    let sections = app
        .db()
        .read(|c| {
            campfire_db::search_query::SearchQuery::parse("sectionable launch")
                .sections_for_user(c, DAVID)
        })
        .await
        .unwrap()
        .into_iter()
        .map(super::section_view)
        .collect::<Vec<_>>();
    // No database connection is available to the view; row count cannot add a query.
    for n in [1, 16] {
        let mut sections = sections.clone();
        for s in &mut sections {
            s.records = s.records.iter().cycle().take(n).cloned().collect();
        }
        let r = crate::controllers::presenters::page::render_detached_at(
            &app.booted.app,
            None,
            "http://campfire.test",
            |ctx| {
                campfire_views::searches::Sections {
                    ctx,
                    sections: &sections,
                }
                .render()
                .unwrap()
            },
        );
        assert_eq!(r.matches("class=\"search-sections__item\"").count(), 3 * n);
    }
}

#[tokio::test]
async fn search_sections_load_older_and_older_stream_match_pinned_rails_bytes() {
    use askama::Template;
    let app = app().await;
    let vector = oracle();
    let sections = vector["sections_data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let kind = match s["kind"].as_str().unwrap() {
                "board-posts" => "board-posts",
                "work-threads" => "work-threads",
                _ => "events",
            };
            super::section_view(campfire_db::search_query::SearchSection {
                kind,
                records: s["records"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| campfire_db::search_query::SearchSectionRecord {
                        id: r["id"].as_i64().unwrap(),
                        room_id: r["room_id"].as_i64().unwrap(),
                        room_type: r["room_type"].as_str().unwrap().into(),
                        room_name: r["room_name"].as_str().map(str::to_owned),
                        title: r["title"].as_str().unwrap().into(),
                        time: campfire_db::Timestamp::from_jiff(
                            r["time"].as_str().unwrap().parse().unwrap(),
                        ),
                        status: r["status"].as_str().map(str::to_owned),
                        cancelled: r["cancelled"].as_bool().unwrap(),
                    })
                    .collect(),
            })
        })
        .collect::<Vec<_>>();
    let actual = crate::controllers::presenters::page::render_detached_at(
        &app.booted.app,
        None,
        "http://campfire.test",
        |ctx| {
            campfire_views::searches::Sections {
                ctx,
                sections: &sections,
            }
            .render()
            .unwrap()
        },
    );
    assert_eq!(actual, vector["sections"].as_str().unwrap());
    assert_eq!(
        campfire_views::searches::LoadOlder {
            query: "from:@jz has:file launch",
            oldest_id: 123
        }
        .render()
        .unwrap(),
        vector["older"].as_str().unwrap()
    );
    let index = campfire_views::searches::IndexView {
        query: None,
        q: None,
        messages: vec![],
        recent_searches: vec![],
        return_to_room: None,
        has_more: false,
        oldest_id: None,
        chips: vec![],
        sections: vec![],
    };
    let actual = crate::controllers::presenters::page::render_detached_at(
        &app.booted.app,
        None,
        "http://campfire.test",
        |ctx| {
            campfire_views::searches::Older { ctx, index: &index }
                .render()
                .unwrap()
        },
    );
    assert_eq!(actual, vector["older_empty"].as_str().unwrap());
}
#[tokio::test]
async fn search_history_http_matches_pinned_rails_responses() {
    let app = app().await;
    let mut b = app.david();
    for s in oracle()["steps"].as_array().unwrap() {
        let method = if s["method"] == "post" {
            Method::POST
        } else {
            Method::DELETE
        };
        let path = s["path"].as_str().unwrap();
        let fields = s["input"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str().unwrap()))
            .collect::<Vec<_>>();
        let r = b
            .write(
                Req::new(method, path)
                    .header("accept", s["accept"].as_str().unwrap())
                    .form(&fields),
            )
            .await;
        assert_eq!(r.status.as_u16(), s["status"].as_u64().unwrap() as u16);
        assert_eq!(r.location(), s["location"].as_str());
        assert_eq!(r.text(), s["body"].as_str().unwrap());
        assert_eq!(
            r.headers["content-type"]
                .to_str()
                .unwrap()
                .split(';')
                .next()
                .unwrap(),
            s["content_type"].as_str().unwrap()
        );
    }
}

#[tokio::test]
async fn blank_search_does_not_resolve_an_invalid_older_cursor() {
    let app = app().await;
    let r = app
        .david()
        .send(
            Req::new(Method::GET, "/searches?before=garbage")
                .header("accept", "text/vnd.turbo-stream.html"),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        r.text()
            .contains("action=\"prepend\" target=\"search-results\"")
    );
}
#[tokio::test]
async fn search_supplies_the_room_icon_only_on_shared_fragment_misses() {
    let app = app().await;
    let m = message(&app, DAVID, "needleicon alpha").await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE rooms SET icon_name='github' WHERE id=?", [ALL_TALK])?;
            Ok(())
        })
        .await
        .unwrap();
    let r = find(&mut app.david(), "needleicon alpha").await;
    assert_eq!(count(&r), 1);
    assert!(r.text().contains("message__room--custom"), "{}", r.text());
    assert!(r.text().contains("icon-avatar--brand"));
    app.db()
        .write(move |tx| {
            tx.conn()
                .execute("UPDATE rooms SET icon_name=NULL WHERE id=?", [ALL_TALK])?;
            Ok(())
        })
        .await
        .unwrap();
    let r = find(&mut app.david(), "needleicon alpha").await;
    assert!(
        r.text().contains("icon-avatar--brand"),
        "shared hit: {}",
        r.text()
    );
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE messages SET updated_at=? WHERE id=?",
                (tx.now(), m.id),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let r = find(&mut app.david(), "needleicon alpha").await;
    assert!(!r.text().contains("message__room--custom"));
}
