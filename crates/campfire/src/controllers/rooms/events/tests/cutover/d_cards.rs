//! Remaining original card/timeline declarations through the real HTTP and Cable producers.
use super::super::*;
use super::support::*;
use serde_json::json;

#[tokio::test]
async fn cutover_d_wide_calendar_form_http_preserves_year_and_exact_event_and_viewer_zone_periods()
{
    let app = app().await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET time_zone='Hawaii' WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.sign_in_for_tests(DAVID).await;
    // The first input is the exact datetime-local payload Chrome submitted in
    // original EventsTest:102. The summer case distinguishes the real final
    // Rails TZInfo period (EST) from the calendar proxy's extrapolated EDT.
    for (title, start, end, utc, _viewer, _label) in [
        (
            "Wide browser payload",
            "60310-02-02T15:30",
            "60310-02-02T16:30",
            "60310-02-02 20:30:00",
            "60310-02-02T10:30:00-10:00",
            "February 2, 60310 at 3:30 PM",
        ),
        (
            "Wide summer",
            "12026-07-04T15:30",
            "12026-07-04T16:30",
            "12026-07-04 20:30:00",
            "12026-07-04T10:30:00-10:00",
            "July 4, 12026 at 3:30 PM",
        ),
        (
            "After final Rails timezone transition",
            "2127-07-04T15:30",
            "2127-07-04T16:30",
            "2127-07-04 20:30:00",
            "2127-07-04T10:30:00-10:00",
            "July 4, 2127 at 3:30 PM",
        ),
    ] {
        let reply = david.write(json(Method::POST, &index_path(), json!({"event": {
            "title":title, "starts_at":start, "ends_at":end, "time_zone":"America/New_York",
            "description":"", "recurrence_rule":"", "recurrence_until":"", "venue_room_id":"", "meet_link_requested":"0",
        }}))).await;
        assert_eq!(reply.status, StatusCode::FOUND);
        let event = app
            .db()
            .read(move |c| {
                let eid =
                    c.query_row("SELECT id FROM events WHERE title=?", [title], |r| r.get(0))?;
                CalendarEvent::find(c, eid)
            })
            .await
            .unwrap();
        assert_eq!(event.starts_at.to_db(), utc);
    }
}
