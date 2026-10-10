//! Named request ports of test/controllers/searches_controller_test.rb.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::Search;


async fn app() -> TestApp {
    TestApp::boot().await.expect("WS8bm2 requires default seed")
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
async fn clear_redirects_after_deleting_only_the_viewers_recents() {
    let app = app().await;
    record(&app, "hello").await;
    let r = app
        .david()
        .write(
            Req::new(Method::DELETE, "/searches/clear")
                .header("accept", "text/vnd.turbo-stream.html"),
        )
        .await;
    assert_eq!(r.status, StatusCode::FOUND);
    assert_eq!(r.location(), Some("http://campfire.test/searches"));
    assert!(r.text().is_empty());
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
            b.get("/app/")
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

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../vectors/messaging/search.json")).unwrap()
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
        if s["accept"] == "text/vnd.turbo-stream.html" {
            assert_eq!(r.status, StatusCode::FOUND);
            assert_eq!(r.location(), Some("http://campfire.test/searches"));
            assert!(r.text().is_empty());
        } else {
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
}
