//! ActivityItem model checks on the real pinned seed, with an injectable FrozenClock.
use super::presenters::test_support::{TestApp, DAVID, JASON, KEVIN, BENDER, SEED_NOW};
use campfire_db::{ActivityItem, Database, Env, RecordingSink, Timestamp};
use campfire_db::models::activity_item::ActivityQuery;
use std::sync::Arc;

fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../vectors/activity_domain.json")).unwrap()
}

struct FrozenDbClock(Arc<campfire_kit::FrozenClock>);
impl campfire_db::Clock for FrozenDbClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_jiff(campfire_kit::Clock::now(&*self.0))
    }
}

struct ModelDb {
    _app: TestApp,
    db: Database,
    sink: RecordingSink,
    clock: Arc<campfire_kit::FrozenClock>,
}
impl ModelDb {
    async fn boot() -> Self {
        let app = TestApp::boot_frozen().await.expect("WS12 requires default seed");
        let clock = Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
        let sink = RecordingSink::new();
        let env = Env {
            clock: Arc::new(FrozenDbClock(clock.clone())),
            sink: Arc::new(sink.clone()),
            ..Env::default()
        };
        let db = Database::open(campfire_db::Config::new(app.db().path()),env).unwrap();
        Self {_app:app,db,sink,clock}
    }
    async fn item(&self) -> ActivityItem {
        self.db.write(|tx| {
            let source: i64 = tx.conn().query_row("SELECT id FROM messages WHERE room_id=486777696 ORDER BY id LIMIT 1",[],|row| row.get(0))?;
            ActivityItem::refresh_unread(tx,DAVID,"Message",source,"mention")
        }).await.unwrap()
    }
}

#[tokio::test]
async fn ws12_activity_same_instant_handling_is_a_noop_after_the_first_commit() {
    let t = ModelDb::boot().await;
    let item = t.item().await;
    t.sink.take();
    let one = item.clone();
    t.db.write(move |tx| one.mark_handled(tx)).await.unwrap();
    assert_eq!(t.sink.take().len(),1);
    // Read the fresh row, as the Rails controller does. At a frozen instant nothing changes.
    let current = t.db.read(move |conn| ActivityItem::find(conn,item.id)).await.unwrap();
    t.db.write(move |tx| current.mark_handled(tx)).await.unwrap();
    assert!(t.sink.take().is_empty(),"Rails after_update_commit broadcasts only saved state changes");
    t.clock.advance(jiff::SignedDuration::from_secs(1));
}

async fn assert_snapshot(t: &ModelDb, snapshot: &serde_json::Value) {
    for viewer in snapshot["viewers"].as_array().unwrap() {
        let viewer = viewer.clone();
        t.db.read(move |conn| {
            let user = campfire_db::User::find(conn,viewer["user_id"].as_i64().unwrap())?;
            let ids = |items: Vec<ActivityItem>| serde_json::to_value(items.iter().map(|i|i.id).collect::<Vec<_>>()).unwrap();
            assert_eq!(ids(ActivityItem::accessible_to(conn,&user)?),viewer["ids"]);
            assert_eq!(ActivityItem::unread_count(conn,&user)?,viewer["unread_count"].as_i64().unwrap());
            for (state,expected) in viewer["filters"].as_object().unwrap() {
                assert_eq!(ids(ActivityItem::query_accessible(conn,&user,ActivityQuery {state:Some(state),..Default::default()})?),*expected);
            }
            for (kind,expected) in viewer["types"].as_object().unwrap() {
                assert_eq!(ids(ActivityItem::query_accessible(conn,&user,ActivityQuery {type_filter:Some(kind),..Default::default()})?),*expected);
            }
            Ok(())
        }).await.unwrap();
    }
}

#[tokio::test]
async fn ws12_activity_membership_active_human_and_foreign_security_access_match_rails() {
    let t = ModelDb::boot().await;
    let snapshots = vectors()["snapshots"].as_array().unwrap().clone();
    assert_snapshot(&t,&snapshots[0]).await;
    t.db.write(|tx| {tx.conn().execute("DELETE FROM memberships WHERE user_id=?",[DAVID])?;Ok(())}).await.unwrap();
    assert_snapshot(&t,&snapshots[1]).await;
    t.db.write(|tx| {tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[DAVID])?;Ok(())}).await.unwrap();
    assert_snapshot(&t,&snapshots[2]).await;
    t.db.write(|tx| {
        tx.conn().execute("UPDATE users SET status=0 WHERE id=?",[DAVID])?;
        tx.conn().execute("DELETE FROM activity_items WHERE user_id=?",[DAVID])?;
        let session: i64 = tx.conn().query_row("SELECT id FROM sessions WHERE user_id=? ORDER BY id LIMIT 1",[JASON],|row|row.get(0))?;
        ActivityItem::refresh_unread(tx,DAVID,"Session",session,"new_sign_in")?;
        Ok(())
    }).await.unwrap();
    assert_snapshot(&t,&snapshots[3]).await;
}

fn insert_row(tx: &campfire_db::Tx<'_>, table: &str, row: &serde_json::Value) -> campfire_db::Result<()> {
    let columns = row.as_object().unwrap();
    let names = columns.keys().map(|key|format!("\"{key}\"")).collect::<Vec<_>>().join(",");
    let values = columns.values().map(|value| match value {
        serde_json::Value::Null=>rusqlite::types::Value::Null,
        serde_json::Value::Number(n)=>rusqlite::types::Value::Integer(n.as_i64().unwrap()),
        serde_json::Value::String(s)=>rusqlite::types::Value::Text(s.clone()),
        _=>panic!("unsupported fixture SQLite value"),
    });
    tx.conn().execute(&format!("INSERT INTO {table} ({names}) VALUES ({})",std::iter::repeat_n("?",columns.len()).collect::<Vec<_>>().join(",")),rusqlite::params_from_iter(values))?;
    Ok(())
}

#[tokio::test]
async fn ws12_activity_all_eleven_sources_and_live_agent_access_match_rails() {
    let t = ModelDb::boot().await;
    let matrix = vectors()["matrix"].clone();
    let setup = matrix.clone();
    t.db.write(move |tx| {
        tx.conn().execute("DELETE FROM memberships",[])?;
        tx.conn().execute("DELETE FROM activity_items",[])?;
        for row in setup["memberships"].as_array().unwrap() {insert_row(tx,"memberships",row)?;}
        for row in setup["setup"].as_array().unwrap() {insert_row(tx,row["table"].as_str().unwrap(),&row["row"])?;}
        for row in setup["items"].as_array().unwrap() {insert_row(tx,"activity_items",row)?;}
        Ok(())
    }).await.unwrap();
    let snapshots = matrix["snapshots"].as_array().unwrap();
    assert_snapshot(&t,&snapshots[0]).await;
    t.db.write(|tx| {tx.conn().execute("DELETE FROM memberships WHERE user_id=?",[DAVID])?;Ok(())}).await.unwrap();
    assert_snapshot(&t,&snapshots[1]).await;
    let agent = matrix["agent_id"].as_i64().unwrap();
    t.db.write(move |tx| {tx.conn().execute("UPDATE agents SET owner_id=? WHERE id=?",rusqlite::params![KEVIN,agent])?;Ok(())}).await.unwrap();
    assert_snapshot(&t,&snapshots[2]).await;
    let agent_user = matrix["agent_user_id"].as_i64().unwrap();
    t.db.write(move |tx| {tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[agent_user])?;Ok(())}).await.unwrap();
    assert_snapshot(&t,&snapshots[3]).await;
}

#[tokio::test]
async fn ws12_activity_state_rows_and_dirty_broadcasts_match_frozen_rails_steps() {
    let t = ModelDb::boot().await;
    let mut item = t.item().await;
    t.sink.take();
    for step in vectors()["states"].as_array().unwrap() {
        t.clock.set(jiff::Timestamp::from_second(step["now"].as_i64().unwrap()).unwrap());
        let before = item.clone();
        let operation = step["operation"].as_str().unwrap().to_string();
        item = t.db.write(move |tx| {
            item = match operation.as_str() {
                "read"=>item.mark_read(tx)?, "handled"=>item.mark_handled(tx)?,
                "unhandled"=>item.mark_unhandled(tx)?, "unread"=>item.mark_unread(tx)?, "initial"=>item,
                _=>panic!("unexpected operation"),
            };
            Ok(item)
        }).await.unwrap();
        assert_eq!(item.state(),step["state"].as_str().unwrap());
        assert_eq!(item.read_at.map(|time|time.as_second()),step["read_at"].as_i64());
        assert_eq!(item.handled_at.map(|time|time.as_second()),step["handled_at"].as_i64());
        assert_eq!(item.updated_at.as_second(),step["updated_at"].as_i64().unwrap());
        let changed = before.read_at!=item.read_at || before.handled_at!=item.handled_at;
        assert_eq!(t.sink.take().len(),usize::from(changed));
        let item_id = item.id;
        assert_eq!(t.db.read(move |conn| ActivityItem::find(conn,item_id)).await.unwrap(),item);
    }
}

#[tokio::test]
async fn ws12_activity_rollback_and_inactive_or_bot_recipients_do_not_broadcast() {
    let t = ModelDb::boot().await;
    let item = t.item().await;
    let id = item.id;
    t.sink.take();
    let result = t.db.write(move |tx| {
        item.mark_read(tx)?;
        Err::<(),_>(campfire_db::Error::Other("rollback fixture".into()))
    }).await;
    assert!(result.is_err());
    assert!(t.sink.take().is_empty());
    assert!(t.db.read(move |conn| ActivityItem::find(conn,id)).await.unwrap().unread());
    for user in [DAVID,BENDER] {
        t.db.write(move |tx| {
            if user==DAVID {tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[user])?;}
            ActivityItem::refresh_unread(tx,user,"Message",-1,"mention")?;
            Ok(())
        }).await.unwrap();
        assert!(t.sink.take().is_empty());
    }
}

#[tokio::test]
async fn ws12_activity_cursor_follows_updated_at_and_id_and_ignores_inaccessible_ids() {
    let t = ModelDb::boot().await;
    let item = t.item().await;
    let id = item.id;
    t.clock.advance(jiff::SignedDuration::from_secs(1));
    let refreshed = t.db.write(move |tx| ActivityItem::find(tx.conn(),id)?.mark_read(tx)).await.unwrap();
    t.db.read(move |conn| {
        let user=campfire_db::User::find(conn,DAVID)?;
        let ordered=ActivityItem::accessible_to(conn,&user)?;
        assert_eq!(ordered[0].id,refreshed.id);
        let cursor=refreshed.id.to_string();
        let page=ActivityItem::query_accessible(conn,&user,ActivityQuery {before:Some(&cursor),..Default::default()})?;
        assert_eq!(page,ordered[1..]);
        for invalid in ["-1","missing","999999999999"," 123 ","1.0"] {
            assert_eq!(ActivityItem::query_accessible(conn,&user,ActivityQuery {before:Some(invalid),..Default::default()})?,ordered);
        }
        assert_eq!(ActivityItem::query_accessible(conn,&user,ActivityQuery {limit:Some(1),..Default::default()})?,ordered[..1]);
        assert!(ActivityItem::find_accessible(conn,&campfire_db::User::find(conn,JASON)?,id)?.is_none());
        Ok(())
    }).await.unwrap();
}
