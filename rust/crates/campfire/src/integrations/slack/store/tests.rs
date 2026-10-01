use super::super::client::tests::fake;
use super::super::jobs::tests::{run, setup, start};
use super::super::{client::Client, jobs::perform_import, runner::Runner};
use super::*;
use crate::integrations::test_support::Route;
use std::time::Duration;

pub(crate) fn routes(personal: bool) -> Vec<Route> {
    let route = |path: &str, body: &str| {
        Route::new("GET", "slack.com", path, 200).body(body.as_bytes().to_vec())
    };
    let mut routes = vec![
        route(
            "/api/users.list?limit=200",
            include_str!("../fixtures/users.json"),
        ),
        if personal {
            route(
                "/api/conversations.list?types=im%2Cmpim%2Cprivate_channel&exclude_archived=false&limit=200",
                include_str!("../fixtures/conversations_personal.json"),
            )
        } else {
            route(
                "/api/conversations.list?types=public_channel%2Cprivate_channel&exclude_archived=false&limit=200",
                include_str!("../fixtures/conversations_workspace.json"),
            )
        },
    ];
    for (id, members, history) in [
        (
            "CARCH",
            include_str!("../fixtures/members_CARCH.json"),
            include_str!("../fixtures/history_CARCH.json"),
        ),
        (
            "CCHAN",
            include_str!("../fixtures/members_CCHAN.json"),
            include_str!("../fixtures/history_CCHAN_p1.json"),
        ),
        (
            "CPRIV",
            include_str!("../fixtures/members_CPRIV.json"),
            include_str!("../fixtures/history_CPRIV.json"),
        ),
        (
            "DIM",
            include_str!("../fixtures/members_DIM.json"),
            include_str!("../fixtures/history_DIM.json"),
        ),
        (
            "GMPIM",
            include_str!("../fixtures/members_GMPIM.json"),
            include_str!("../fixtures/history_GMPIM.json"),
        ),
    ] {
        routes.push(route(
            &format!("/api/conversations.members?channel={id}&limit=1000"),
            members,
        ));
        routes.push(route(
            &format!("/api/conversations.history?channel={id}&limit=200"),
            history,
        ));
    }
    routes.push(route(
        "/api/conversations.history?channel=CCHAN&cursor=cchan-page-2&limit=200",
        include_str!("../fixtures/history_CCHAN_p2.json"),
    ));
    routes.push(route(
        "/api/conversations.replies?channel=CCHAN&ts=1700000002.000002&limit=200",
        include_str!("../fixtures/replies_CCHAN_parent.json"),
    ));
    routes.push(route(
        "/api/conversations.replies?channel=GMPIM&ts=1700000050.000050&limit=200",
        include_str!("../fixtures/replies_GMPIM_parent.json"),
    ));
    routes
}
pub(crate) async fn import(
    db: &Database,
    crypto: std::sync::Arc<rails_compat::ar_encryption::ArEncryption>,
    id: i64,
    network: crate::integrations::net::Network,
) -> SlackImport {
    for _ in 0..100 {
        let store_db = db.clone();
        let network = network.clone();
        perform_import(
            db.clone(),
            crypto.clone(),
            id,
            move |run, lease, token| async move {
                Runner::new(
                    SqlStore {
                        db: store_db,
                        lease,
                        allowed_domains: ["example.com".into()].into(),
                    },
                    Client::with_network(token, None, false, network),
                    run,
                )
                .with_budget(Duration::ZERO)
                .step()
                .await
            },
        )
        .await
        .unwrap();
        let row = run(db, id).await;
        if row.status != "running" && row.status != "queued" {
            assert_eq!(row.status, "completed", "{}", row.error.unwrap_or_default());
            return row;
        }
    }
    panic!("fixture import did not finish");
}
#[tokio::test]
async fn slack_sql_store_full_workspace_import_persists_pages_and_finishes_all_rooms() {
    let (db, crypto, _dir) = setup().await;
    let id = start(&db).await;
    db.write(|tx| {
        tx.conn()
            .execute("UPDATE slack_connections SET slack_user_id='UADMIN'", [])?;
        Ok(())
    })
    .await
    .unwrap();
    let (server, network) = fake(routes(false)).await;
    let row = import(&db, crypto, id, network).await;
    assert_eq!(row.state["phase"], "done");
    assert!(row.state.get("step_lease_token").is_none());
    assert_eq!(row.stats["counts"]["rooms_created"], 3);
    assert!(
        array(&row.stats["conversations"])
            .iter()
            .all(|c| c["done"] == true)
    );
    assert_eq!(row.stats["api_calls"], server.received().len());
    db.read(|c|{
        let rooms=Room::all(c)?;assert_eq!(rooms.len(),3);
        for room in rooms{
            let last:Timestamp=c.query_row("SELECT MAX(created_at) FROM messages WHERE room_id=?",[room.id],|r|r.get(0))?;
            assert_eq!(room.updated_at,last);
            let unread:i64=c.query_row("SELECT count(*) FROM memberships WHERE room_id=? AND (unread_at IS NOT NULL OR last_read_message_id IS NULL)",[room.id],|r|r.get(0))?;assert_eq!(unread,0);
        }
        assert_eq!(c.query_row("SELECT count(*) FROM activity_items",[],|r|r.get::<_,i64>(0))?,0);
        assert_eq!(c.query_row("SELECT count(*) FROM channel_threads",[],|r|r.get::<_,i64>(0))?,1);
        Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn slack_sql_store_dry_run_collects_samples_without_domain_rows_or_reply_fetches() {
    let (db, crypto, _dir) = setup().await;
    let id = start(&db).await;
    db.write(move |tx| {
        tx.conn()
            .execute("UPDATE slack_imports SET mode='dry_run' WHERE id=?", [id])?;
        Ok(())
    })
    .await
    .unwrap();
    let (server, network) = fake(routes(false)).await;
    let row = import(&db, crypto, id, network).await;
    assert!(array(&row.stats["samples"]).len() <= 20);
    assert!(!array(&row.stats["samples"]).is_empty());
    assert!(
        !server
            .received()
            .iter()
            .any(|r| r.target.starts_with("/api/conversations.replies"))
    );
    db.read(|c| {
        for table in [
            "rooms",
            "messages",
            "slack_import_records",
            "channel_threads",
        ] {
            assert_eq!(
                c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))?,
                0
            );
        }
        assert_eq!(
            c.query_row("SELECT count(*) FROM users", [], |r| r.get::<_, i64>(0))?,
            1
        );
        Ok(())
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn slack_sql_store_rejects_cancelled_or_replaced_lease_before_domain_writes() {
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let lease = db
        .write(move |tx| {
            SlackImport::claim_running(tx, id)?;
            Ok(SlackImport::acquire_step_lease(
                tx,
                id,
                campfire_db::models::slack_import::StepStatus::Running,
            )?
            .unwrap())
        })
        .await
        .unwrap();
    let row = run(&db, id).await;
    let p = Progress::new(row);
    let wrong = SqlStore {
        db: db.clone(),
        lease: "wrong owner".into(),
        allowed_domains: HashSet::new(),
    };
    assert!(
        wrong
            .commit(
                p.clone(),
                Operation::Users(json!([{"id":"UWRITE","is_bot":true}]))
            )
            .await
            .unwrap()
            .is_none()
    );
    db.write(move |tx| SlackImport::cancel(tx, id))
        .await
        .unwrap();
    let store = SqlStore {
        db: db.clone(),
        lease,
        allowed_domains: HashSet::new(),
    };
    assert!(
        store
            .commit(p, Operation::Users(json!([{"id":"UWRITE","is_bot":true}])))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        db.read(|c| Ok(
            c.query_row("SELECT count(*) FROM slack_import_records", [], |r| r
                .get::<_, i64>(0))?
        ))
        .await
        .unwrap(),
        0
    );
}
