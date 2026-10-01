use super::{
    card_tests::Fresh,
    test_support::{request, sudo},
};
use serde_json::{Value, json};
#[tokio::test]
async fn github_discuss_security_requires_membership_root_parent_and_exact_reference() {
    for input in [
        json!({"member":false}),
        json!({"reference":false}),
        json!({"deleted":true}),
        json!({"reply":true}),
    ] {
        let fresh = Fresh::new(&input).await;
        if input["reply"] == true {
            fresh
                .app
                .db
                .write(|tx| {
                    tx.conn()
                        .execute("UPDATE messages SET thread_id=817 WHERE id=818", [])?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        assert_eq!(
            request(
                &fresh,
                "POST",
                "/rooms/815/github/pull_request_threads",
                json!({"pull_request_id":816,"message_id":818}),
                sudo()
            )
            .await
            .0,
            404,
            "{input}"
        );
        assert!(fresh.server.received().is_empty());
    }
}
#[tokio::test]
async fn github_discuss_http_redirect_persistence_membership_and_fetch_match_rails() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/github_discussions_http.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let mut input = case.clone();
        input["mapping"] = case.get("mapping").cloned().unwrap_or(json!(false));
        let fresh = Fresh::new(&input).await;
        let input = case.clone();
        fresh
            .app
            .db
            .write(move |tx| {
                if input["mapping"] != true && input["reply"] != true {
                    tx.conn()
                        .execute("DELETE FROM channel_threads WHERE id=817", [])?;
                }
                if input["reply"] == true {
                    tx.conn()
                        .execute("UPDATE messages SET thread_id=817 WHERE id=818", [])?;
                }
                tx.conn().execute(
                    "UPDATE messages SET markdown_source='Discussion' WHERE id=818",
                    [],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let (status, headers, body) = request(
            &fresh,
            "POST",
            "/rooms/815/github/pull_request_threads",
            case["request_body"].clone(),
            sudo(),
        )
        .await;
        assert_eq!(status, case["status"], "{} {body}", case["name"]);
        assert_eq!(
            headers.get("location").and_then(|h| h.to_str().ok()),
            case["location"].as_str(),
            "{}",
            case["name"]
        );
        let result=fresh.app.db.read(|conn|{
   let count=|sql:&str|conn.query_row(sql,[],|r|r.get::<_,i64>(0));
   let mapping=crate::integrations::github::threads::PullRequestThread::for_room_pr(conn,815,816)?;
   let joined=if let Some(m)=&mapping{Some(campfire_db::ThreadMembership::find_by_thread_and_user(conn,m.channel_thread_id,811)?.is_some())}else{None};
   Ok(json!({"threads":count("SELECT COUNT(*) FROM channel_threads WHERE room_id=815")?,"mappings":count("SELECT COUNT(*) FROM github_pull_request_threads")?,"mapping_thread":mapping.map(|m|m.channel_thread_id),"joined":joined,"fetches":count("SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'")?}))
  }).await.unwrap();
        for key in ["threads", "mappings", "mapping_thread", "joined", "fetches"] {
            assert_eq!(result[key], case[key], "{} {key}", case["name"]);
        }
        assert!(fresh.server.received().is_empty());
    }
}
#[tokio::test]
async fn github_discuss_concurrent_duplicates_create_one_joined_thread_and_job_and_enqueue_failure_rolls_back()
 {
    let fresh = Fresh::new(&json!({"mapping":false})).await;
    fresh
        .app
        .db
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM channel_threads WHERE id=817", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut requests = Vec::new();
    for _ in 0..12 {
        requests.push(request(
            &fresh,
            "POST",
            "/rooms/815/github/pull_request_threads",
            json!({"pull_request_id":816,"message_id":818}),
            sudo(),
        ));
    }
    let results = futures_util::future::join_all(requests).await;
    assert!(results.iter().all(|r| r.0 == 303));
    assert!(
        results
            .windows(2)
            .all(|r| r[0].1["location"] == r[1].1["location"])
    );
    fresh.app.db.read(|conn|{for sql in ["SELECT COUNT(*) FROM github_pull_request_threads WHERE room_id=815","SELECT COUNT(*) FROM channel_threads WHERE room_id=815","SELECT COUNT(*) FROM thread_memberships WHERE user_id=811 AND thread_id IN (SELECT id FROM channel_threads WHERE room_id=815)","SELECT COUNT(*) FROM background_jobs WHERE job_class='Github::FetchPullRequestJob'"]{assert_eq!(conn.query_row(sql,[],|r|r.get::<_,i64>(0))?,1);}Ok(())}).await.unwrap();
    let fresh = Fresh::new(&json!({"mapping":false})).await;
    fresh.app.db.write(|tx|{tx.conn().execute_batch("DELETE FROM channel_threads WHERE id=817; CREATE TRIGGER reject_discuss_fetch BEFORE INSERT ON background_jobs WHEN NEW.job_class='Github::FetchPullRequestJob' BEGIN SELECT RAISE(ABORT,'queue rejected'); END;")?;Ok(())}).await.unwrap();
    assert_eq!(
        request(
            &fresh,
            "POST",
            "/rooms/815/github/pull_request_threads",
            json!({"pull_request_id":816,"message_id":818}),
            sudo()
        )
        .await
        .0,
        500
    );
    fresh
        .app
        .db
        .read(|conn| {
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM channel_threads WHERE room_id=815",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM github_pull_request_threads WHERE room_id=815",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}
