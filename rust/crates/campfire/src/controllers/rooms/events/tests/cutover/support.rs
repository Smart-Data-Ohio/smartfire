use super::super::*;
use campfire_db::models::calendar_event::changes::EventChanges;
use campfire_db::models::stream::Stream;
use campfire_db::{ActivityItem, Membership, Room, RoomType, fixtures};
use std::sync::Arc;
pub(super) fn id(label: &str) -> i64 {
    fixtures::identify(label)
}
pub(super) fn path(eid: i64) -> String {
    format!("/rooms/{}/events/{eid}", id("designers"))
}
pub(super) fn index_path() -> String {
    format!("/rooms/{}/events", id("designers"))
}
pub(super) async fn app() -> TestApp {
    let app = TestApp::boot_with_test_clock(Arc::new(campfire_kit::FrozenClock::new(
        "2026-09-22T12:00:00Z".parse().unwrap(),
    )))
    .await
    .expect("pinned default seed")
    .without_job_runner()
    .await;
    app.db()
        .write(|tx| {
            // The parity seed adds rows in non-fixture tables. Clear application
            // tables as Rails' test database does before loading its exact fixtures.
            tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
            let tables=tx.conn().prepare("SELECT name FROM pragma_table_list WHERE schema='main' AND type='table' AND name NOT LIKE 'sqlite_%' AND name NOT IN ('schema_migrations','ar_internal_metadata')")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            for table in tables {tx.conn().execute(&format!("DELETE FROM \"{}\"",table.replace('"',"\"\"")),[])?;}
            fixtures::load(
                tx.conn(),
                &fixtures::reference_dir(),
                &fixtures::Options {
                    now: tx.now(),
                    bcrypt_cost: 4,
                },
            )
        })
        .await
        .unwrap();
    app
}
pub(super) async fn find(app: &TestApp, eid: i64) -> CalendarEvent {
    app.db()
        .read(move |c| CalendarEvent::find(c, eid))
        .await
        .unwrap()
}
pub(super) async fn rows(app: &TestApp, eid: i64) -> Vec<CalendarEvent> {
    app.db()
        .read(move |c| CalendarEvent::find(c, eid)?.series_events(c))
        .await
        .unwrap()
}
pub(super) async fn count(app: &TestApp) -> i64 {
    app.db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?))
        .await
        .unwrap()
}
pub(super) async fn item(app: &TestApp, eid: i64, user: i64) -> ActivityItem {
    app.db()
        .read(move |c| ActivityItem::find_by_user_and_source(c, user, "Event", eid))
        .await
        .unwrap()
        .unwrap()
}
pub(super) async fn series(app: &TestApp, title: &str, organizer: i64) -> CalendarEvent {
    series_at(
        app,
        title,
        organizer,
        "2026-09-24 15:30:00",
        "2026-09-24 16:30:00",
        "2026-10-08",
    )
    .await
}
pub(super) async fn series_at(
    app: &TestApp,
    title: &str,
    organizer: i64,
    start: &str,
    end: &str,
    until: &str,
) -> CalendarEvent {
    let title = title.to_owned();
    let start = Timestamp::parse_db(start);
    let end = Timestamp::parse_db(end);
    let until = until.parse().unwrap();
    app.db()
        .write(move |tx| {
            CalendarEvent::create(
                tx,
                NewCalendarEvent {
                    room_id: id("designers"),
                    organizer_id: organizer,
                    title,
                    starts_at: start,
                    ends_at: end,
                    time_zone: "UTC".into(),
                    recurrence_rule: Some("weekly".into()),
                    recurrence_until: Some(until),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap()
}
pub(super) async fn venue(
    app: &TestApp,
    kind: RoomType,
    name: &str,
    creator: i64,
    users: &[i64],
) -> i64 {
    let name = name.to_owned();
    let users = users.to_vec();
    app.db()
        .write(move |tx| Room::create_for(tx, kind, Some(&name), creator, &users))
        .await
        .unwrap()
        .id
}
pub(super) async fn set_venue(app: &TestApp, eid: i64, vid: i64) {
    app.db()
        .write(move |tx| {
            CalendarEvent::update(
                tx,
                eid,
                EventChanges {
                    venue_room_id: Some(Some(vid)),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
}
pub(super) async fn respond(app: &TestApp, eid: i64, user: i64, response: &str) {
    let response = response.to_owned();
    app.db()
        .write(move |tx| CalendarEvent::respond(tx, eid, user, &response, false))
        .await
        .unwrap();
}
pub(super) async fn item_rows(
    app: &TestApp,
    head: i64,
    user: i64,
    kind: Option<&str>,
) -> Vec<(i64, String)> {
    let kind = kind.map(str::to_owned);
    app.db().read(move |c|{let mut q=c.prepare("SELECT source_id,event_type FROM activity_items WHERE source_type='Event' AND user_id=? AND source_id IN (SELECT id FROM events WHERE series_id=?) AND (? IS NULL OR event_type=?) ORDER BY source_id")?;Ok(q.query_map(rusqlite::params![user,head,kind,kind],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?)}).await.unwrap()
}
pub(super) fn json(method: Method, path: &str, value: serde_json::Value) -> Req {
    Req::new(method, path)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&value).unwrap())
}
pub(super) fn redirected(reply: &Reply, eid: i64) {
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test{}", path(eid)).as_str())
    );
}
pub(super) async fn query_count(app: &TestApp, client: &mut Browser<'_>, url: &str) -> usize {
    let capture = app.db().capture_queries();
    let reply = client.get(url).await;
    assert_eq!(reply.status, StatusCode::OK);
    app.db().stop_capturing_queries();
    let count = capture.lock().unwrap().len();
    count
}
pub(super) async fn live(app: &TestApp, vid: i64) {
    app.db()
        .write(move |tx| {
            let m = Membership::find_by_room_and_user(tx.conn(), vid, DAVID)?.unwrap();
            Stream::create(tx, vid, m.id, DAVID, "1080p15", None)
        })
        .await
        .unwrap();
}
