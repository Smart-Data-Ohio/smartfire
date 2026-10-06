use crate::controllers::presenters::test_support::{BENDER, JASON, KEVIN, TestApp};
use axum::http::StatusCode;
use campfire_db::{Session, Timestamp, WorkspacePresenceLease};
use serde_json::json;

#[test]
fn ws17_presence_ids_match_ruby_integer_coercion() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../db/src/tests/ws17_vectors.json"))
            .unwrap();
    for row in vectors["integer_coercions"].as_array().unwrap() {
        let expected = if row["output"].is_null() {
            None
        } else {
            Some(row["output"].as_i64())
        };
        assert_eq!(super::integer_id(&row["input"]), expected, "{row}");
    }
}

#[tokio::test]
async fn ws17_presence_http_bodies_match_rails_vectors() {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the Rails parity seed");
    let session_id = app
        .db()
        .write(|tx| Ok(Session::start(tx, JASON, Some("WS17"), None)?.id))
        .await
        .unwrap();
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../../vectors/ws17_presence.json"))
            .unwrap();
    for row in vectors["cases"].as_array().unwrap() {
        let setup = row.clone();
        app.db().write(move |tx| {
            tx.conn().execute_batch("DELETE FROM workspace_presence_leases; DELETE FROM calendar_meeting_caches")?;
            tx.conn().execute("UPDATE users SET presence_setting='auto',custom_status_emoji=NULL,custom_status_text=NULL,custom_status_expires_at=NULL,meeting_status_enabled=0,meeting_dnd_enabled=0,dnd_enabled=0,quiet_hours_enabled=0,ooo_until=NULL,ooo_note=NULL,ooo_calendar_enabled=0 WHERE id=?",[JASON])?;
            for (key,value) in setup["attrs"].as_object().unwrap() {
                let value = match value {
                    serde_json::Value::Bool(b) => rusqlite::types::Value::Integer(i64::from(*b)),
                    serde_json::Value::String(s) if key.ends_with("_until") || key.ends_with("_expires_at") => rusqlite::types::Value::Text(Timestamp::from_jiff(s.parse().unwrap()).to_db()),
                    serde_json::Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                    _ => panic!("unexpected oracle attribute"),
                };
                tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"),rusqlite::params![value,JASON])?;
            }
            if let Some(kind)=setup["lease"].as_str() {
                let lease=WorkspacePresenceLease::establish(tx,JASON,session_id)?.unwrap();
                if kind=="expired" { tx.conn().execute("UPDATE workspace_presence_leases SET expires_at=? WHERE id=?",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_mins(1)),lease.id])?; }
                if kind=="idle" { tx.conn().execute("UPDATE workspace_presence_leases SET last_active_at=? WHERE id=?",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_mins(11)),lease.id])?; }
            }
            if setup["busy"]==true {
                let busy=json!([[tx.now().ago(jiff::SignedDuration::from_mins(5)).jiff().to_string(),tx.now().since(jiff::SignedDuration::from_mins(55)).jiff().to_string()]]).to_string();
                tx.conn().execute("INSERT INTO calendar_meeting_caches (user_id,busy_intervals,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![JASON,busy,tx.now(),tx.now()])?;
            }
            Ok(())
        }).await.unwrap();
        let reply = app.david().get(row["path"].as_str().unwrap()).await;
        assert_eq!(reply.status, StatusCode::OK, "{row}");
        assert_eq!(reply.header("content-type"), row["content_type"].as_str());
        assert_eq!(
            reply.text(),
            row["body"].as_str().unwrap(),
            "{}",
            row["name"]
        );
        assert_eq!(
            app.db()
                .read(|c| Ok(c.query_row(
                    "SELECT COUNT(*) FROM workspace_presence_leases",
                    [],
                    |r| r.get::<_, i64>(0)
                )?))
                .await
                .unwrap(),
            row["retained_leases"].as_i64().unwrap()
        );
    }
}

#[tokio::test]
async fn ws17_presence_endpoint_returns_live_humans_and_expired_status_is_hidden() {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the Rails parity seed");
    app.booted.app.db.write(|tx| {
        let session = Session::start(tx, JASON, Some("WS17"), None)?;
        WorkspacePresenceLease::establish(tx,JASON,session.id)?;
        tx.conn().execute("UPDATE users SET custom_status_text='Old',custom_status_expires_at=? WHERE id=?",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_hours(1)),JASON])?;
        Ok(())
    }).await.unwrap();
    let reply = app
        .david()
        .get(&format!(
            "/users/presence?ids[]={JASON}&ids[]={KEVIN}&ids[]={BENDER}&ids[]=nope"
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&reply.body).unwrap(),
        json!({"presences":[
            {"id":JASON,"presence":"online","status":null},{"id":KEVIN,"presence":"offline","status":null}
        ]})
    );
}

#[tokio::test]
async fn ws17_presence_endpoint_never_prunes_stale_leases() {
    let app = TestApp::boot()
        .await
        .expect("WS17 requires the Rails parity seed");
    let id = app.booted.app.db.write(|tx| {
        let session = Session::start(tx,JASON,Some("WS17"),None)?;
        let lease = WorkspacePresenceLease::establish(tx,JASON,session.id)?.unwrap();
        tx.conn().execute("UPDATE workspace_presence_leases SET expires_at=? WHERE id=?",rusqlite::params![Timestamp::from_second(0),lease.id])?;
        tx.conn().execute_batch("CREATE TRIGGER no_presence_read_prune BEFORE DELETE ON workspace_presence_leases BEGIN SELECT RAISE(ABORT,'read pruned a lease'); END")?;
        Ok(lease.id)
    }).await.unwrap();
    let reply = app
        .david()
        .get(&format!("/users/presence?ids[]={JASON}"))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&reply.body).unwrap(),
        json!({"presences":[{"id":JASON,"presence":"offline","status":null}]})
    );
    assert!(
        app.booted
            .app
            .db
            .read(move |c| WorkspacePresenceLease::find_by_id(c, id))
            .await
            .unwrap()
            .is_some()
    );
}
