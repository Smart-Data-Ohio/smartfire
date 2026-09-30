use super::*;
use crate::controllers::presenters::test_support::TestApp;
use crate::integrations::test_support::*;
use rusqlite::params;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
fn vectors() -> Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/ws15e_twitter_fetch.json"
    )))
    .unwrap()
}
async fn tls_server(route: Route) -> (FakeServer, Network, Arc<FakeResolver>, Arc<MappingDialer>) {
    let (server, roots) =
        FakeServer::start_named_tls_ws15e(vec![route], vec![API_HOST.into()]).await;
    let resolver = Arc::new(FakeResolver::new([(API_HOST, vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: HashSet::from(["93.184.216.34".parse().unwrap()]),
        to: server.addr,
        dialed: Mutex::new(vec![]),
    });
    let net = Network {
        resolver: resolver.clone(),
        dialer: dialer.clone(),
        tls: crate::integrations::net::tls_config(roots),
    };
    (server, net, resolver, dialer)
}
async fn make_post(app: &TestApp, case: &Value) -> Post {
    let case = case.clone();
    app.db()
        .write(move |tx| {
            let post = Post::for_reference(
                tx,
                case["post_id"].as_str().unwrap(),
                case["input_url"].as_str(),
            )?;
            if let Some(initial) = case.get("initial") {
                tx.conn().execute(
                    "UPDATE twitter_posts SET text=?,author_name=?,media=? WHERE id=?",
                    params![
                        initial["text"].as_str(),
                        initial["author_name"].as_str(),
                        initial["media"].to_string(),
                        post.id
                    ],
                )?;
            }
            Ok(post)
        })
        .await
        .unwrap()
}
async fn snapshot(app: &TestApp, id: i64) -> Value {
    app.db().read(move|conn| {
        conn.query_row("SELECT * FROM twitter_posts WHERE id=?",[id],|row| {
            let text=|field|row.get::<_,Option<String>>(field);
            let count=|field|row.get::<_,Option<i64>>(field);
            let posted:Option<Timestamp>=row.get("posted_at")?;
            let posted=posted.map(|t|format!("{}.{:06}",t.jiff().strftime("%Y-%m-%d %H:%M:%S"),t.subsec_microsecond()));
            let json_field=|field|text(field).map(|s|s.and_then(|s|serde_json::from_str::<Value>(&s).ok()));
            Ok(json!({"url":text("url")?,"author_handle":text("author_handle")?,"author_name":text("author_name")?,"author_avatar_url":text("author_avatar_url")?,"text":text("text")?,"posted_at":posted,"replies":count("replies")?,"reposts":count("reposts")?,"likes":count("likes")?,"media":json_field("media")?,"quote":json_field("quote")?,"fetch_error":text("fetch_error")?}))
        }).map_err(Into::into)
    }).await.unwrap()
}
#[tokio::test]
async fn ws15e_x_fetch_matches_pinned_persisted_values_over_verified_tls() {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    for case in vectors()["cases"].as_array().unwrap() {
        if case.get("transport_error").is_some() {
            continue;
        } // Executed by the separate real read-timeout test below.
        let post = make_post(&app, case).await;
        assert_eq!(
            request_path(&post),
            case["path"].as_str().unwrap(),
            "{}",
            case["name"]
        );
        let route = Route::new(
            "GET",
            API_HOST,
            case["path"].as_str().unwrap(),
            case["status"].as_u64().unwrap() as u16,
        )
        .body(case["body"].as_str().unwrap());
        let (server, net, resolver, dialer) = tls_server(route).await;
        fetch(&app.booted.app, &net, post.id).await.unwrap();
        assert_eq!(
            snapshot(&app, post.id).await,
            case["result"],
            "{}",
            case["name"]
        );
        let fetched = app
            .db()
            .read(move |conn| Post::find(conn, post.id))
            .await
            .unwrap();
        assert!(fetched.fetched_at.is_some());
        assert!(!fetched.needs_fetch(fetched.fetched_at.unwrap()));
        let received = server.received.lock().unwrap().clone();
        assert_eq!(received.len(), 1);
        assert_eq!(
            received[0].header("User-Agent"),
            Some("Smartfire-X-Post-Cards")
        );
        assert_eq!(received[0].header("Accept"), Some("application/json"));
        assert_eq!(received[0].header("Authorization"), None);
        assert_eq!(received[0].header("Cookie"), None);
        assert_eq!(resolver.lookups(), vec![API_HOST]);
        assert_eq!(
            dialer.dialed.lock().unwrap()[0].ip().to_string(),
            "93.184.216.34"
        );
        app.db()
            .write(move |tx| {
                tx.conn()
                    .execute("DELETE FROM twitter_posts WHERE id=?", [post.id])?;
                Ok(())
            })
            .await
            .unwrap();
    }
}
#[tokio::test]
async fn ws15e_x_fetch_caps_header_streamed_and_decoded_bodies() {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    for mode in ["header", "streamed", "lying", "gzip", "exact"] {
        let case = json!({"post_id":"cap","input_url":"https://x.com/jack/status/424242"});
        let post = make_post(&app, &case).await;
        let mut route = Route::new("GET", API_HOST, "/jack/status/cap", 200);
        match mode {
            "header" => route = route.header("Content-Length", &(MAX_BODY_BYTES + 1).to_string()),
            "exact" => {
                route.body = vec![b' '; MAX_BODY_BYTES];
                route.chunked = true;
            }
            "gzip" => {
                route.body = gzip_bomb(3);
                route.chunked = true;
                route = route.header("Content-Encoding", "gzip");
            }
            _ => {
                route.body = vec![b'x'; MAX_BODY_BYTES + 1];
                route.chunked = true;
                if mode == "lying" {
                    route = route.header("Content-Length", "10");
                }
            }
        }
        let (_server, net, _, _) = tls_server(route).await;
        fetch(&app.booted.app, &net, post.id).await.unwrap();
        let result = app
            .db()
            .read(move |conn| Post::find(conn, post.id))
            .await
            .unwrap();
        assert_eq!(
            result.fetch_error.as_deref(),
            Some(if mode == "exact" {
                "Could not load this post"
            } else {
                "Post response too large"
            }),
            "{mode}"
        );
        assert!(result.fetched_at.is_some());
        app.db()
            .write(move |tx| {
                tx.conn()
                    .execute("DELETE FROM twitter_posts WHERE id=?", [post.id])?;
                Ok(())
            })
            .await
            .unwrap();
    }
}
#[tokio::test]
async fn ws15e_x_fetch_read_timeout_records_error_and_replays_get_once() {
    let case = vectors()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "read timeout")
        .unwrap()
        .clone();
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let post = make_post(&app, &case).await;
    let mut route = Route::new("GET", API_HOST, "/jack/status/424242", 200);
    route.delay = Duration::from_secs(11);
    let (server, net, resolver, _) = tls_server(route).await;
    let started = std::time::Instant::now();
    fetch(&app.booted.app, &net, post.id).await.unwrap();
    assert!(started.elapsed() >= Duration::from_secs(20));
    assert!(started.elapsed() < Duration::from_secs(25));
    assert_eq!(snapshot(&app, post.id).await, case["result"]);
    assert_eq!(server.received.lock().unwrap().len(), 2);
    assert_eq!(resolver.lookups(), vec![API_HOST, API_HOST]);
    assert_eq!(TIMEOUTS.open, Duration::from_secs(5));
    assert_eq!(TIMEOUTS.write, Duration::from_secs(60));
}
#[tokio::test]
async fn ws15e_x_claims_are_transactional_and_durable_missing_jobs_discard() {
    let mut app = TestApp::boot().await.expect("pinned seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let post = app
        .db()
        .write(|tx| {
            Post::for_reference(
                tx,
                "9999999999999999999999999",
                Some("https://x.com/u/status/9999999999999999999999999"),
            )
        })
        .await
        .unwrap();
    let ids = (0..12)
        .map(|_| {
            let db = app.db().clone();
            tokio::spawn(async move {
                db.write(move |tx| {
                    let post = Post::find(tx.conn(), post.id)?;
                    assert_eq!(post.post_id, "9999999999999999999999999");
                    post.request_fetch(tx)
                })
                .await
                .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let mut wins = 0;
    for task in ids {
        wins += task.await.unwrap() as usize;
    }
    assert_eq!(wins, 1);
    app.db().write(move|tx|{tx.conn().execute("UPDATE twitter_posts SET fetch_requested_at=NULL WHERE id=?",[post.id])?;tx.conn().execute_batch("CREATE TEMP TRIGGER reject_x_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Twitter::FetchPostJob' BEGIN SELECT RAISE(ABORT,'reject X'); END;")?;Ok(())}).await.unwrap();
    assert!(
        app.db()
            .write(move |tx| Post::find(tx.conn(), post.id)?.request_fetch(tx))
            .await
            .is_err()
    );
    app.db()
        .read(move |conn| {
            assert!(
                conn.query_row(
                    "SELECT fetch_requested_at FROM twitter_posts WHERE id=?",
                    [post.id],
                    |r| r.get::<_, Option<Timestamp>>(0)
                )?
                .is_none()
            );
            Ok(())
        })
        .await
        .unwrap();
    app.db()
        .write(move |tx| {
            tx.conn().execute_batch("DROP TRIGGER reject_x_job;")?;
            tx.conn()
                .execute("DELETE FROM twitter_posts WHERE id=?", [post.id])?;
            tx.emit_after_commit(campfire_db::Event::job(&FetchJob { post_id: -1 }));
            Ok(())
        })
        .await
        .unwrap();
    let config =
        campfire_jobs::RunnerConfig::new(vec![campfire_jobs::QueueConfig::new("default", 1)]);
    let mut registry = campfire_jobs::Registry::new();
    registry.register(perform);
    let queue = campfire_jobs::JobQueue::new(&registry, &config).unwrap();
    assert_eq!(FetchJob::retry_policy().attempts, 1);
    assert_eq!(FetchJob::retry_policy().retry_delay(1, None, 0.0), None);
    let runner = campfire_jobs::start(
        app.db().clone(),
        queue,
        registry,
        app.booted.app.clone(),
        config,
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if app
                .db()
                .read(campfire_jobs::inspect::all)
                .await
                .unwrap()
                .is_empty()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    runner.shutdown(Duration::from_secs(1)).await;
}

#[tokio::test]
async fn ws15e_review_x_dns_and_tcp_share_one_open_budget() {
    use crate::integrations::net::{BoxFuture, Dialer, Resolver};
    struct SlowDns(Arc<dyn Resolver>);
    impl Resolver for SlowDns {
        fn lookup<'a>(&'a self, host: &'a str) -> BoxFuture<'a, std::io::Result<Vec<std::net::IpAddr>>> {
            Box::pin(async move { tokio::time::sleep(Duration::from_millis(120)).await; self.0.lookup(host).await })
        }
    }
    struct SlowTcp(Arc<dyn Dialer>);
    impl Dialer for SlowTcp {
        fn connect(&self, addr: std::net::SocketAddr) -> BoxFuture<'_, std::io::Result<tokio::net::TcpStream>> {
            Box::pin(async move { tokio::time::sleep(Duration::from_millis(120)).await; self.0.connect(addr).await })
        }
    }
    let (server,mut net,_,_) = tls_server(Route::new("GET",API_HOST,"/i/status/123",200).body("{}")).await;
    net.resolver = Arc::new(SlowDns(net.resolver.clone()));
    net.dialer = Arc::new(SlowTcp(net.dialer.clone()));
    let result = get(&net,"/i/status/123",&Timeouts {open:Duration::from_millis(200),..TIMEOUTS}).await;
    eprintln!("REVIEW X combined open budget: received requests={}",server.received().len());
    assert!(matches!(result,Err(Failure::Transport(HttpError::OpenTimeout))));
    assert!(server.received().is_empty());
}
