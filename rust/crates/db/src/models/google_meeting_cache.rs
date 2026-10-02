//! WS14 fetch-side cache writes; WS17 owns cache readers, status dispatch and broadcast claims.
use super::google_calendar::MeetingRefreshJob;
use crate::{Connection, Event, Result, Timestamp, Tx, User};
use jiff::SignedDuration;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
#[derive(Clone, Debug)]
pub struct Cache {
    pub id: i64,
    pub busy: Value,
    pub ooo: Value,
    pub fetched_at: Option<Timestamp>,
    pub fetch_error: Option<String>,
    pub refresh_pending_at: Option<Timestamp>,
}
pub fn find(conn: &Connection, user_id: i64) -> Result<Option<Cache>> {
    Ok(conn.query_row("SELECT id,busy_intervals,ooo_intervals,fetched_at,fetch_error,refresh_pending_at FROM calendar_meeting_caches WHERE user_id=?",[user_id],|r|{let busy:String=r.get(1)?;let ooo:String=r.get(2)?;Ok(Cache{id:r.get(0)?,busy:serde_json::from_str(&busy).unwrap_or(Value::Null),ooo:serde_json::from_str(&ooo).unwrap_or(Value::Null),fetched_at:r.get(3)?,fetch_error:r.get(4)?,refresh_pending_at:r.get(5)?})}).optional()?)
}
fn validate(tx: &Tx<'_>, user_id: i64) -> Result<()> {
    let mut errors = crate::Errors::default();
    if User::find_by_id(tx.conn(), user_id)?.is_none() {
        errors.add("user", "must exist");
    }
    errors.into_result()
}
/// Validated creation needed by the settings model; refresh itself uses Rails' upsert semantics.
pub fn create(tx: &mut Tx<'_>, user_id: i64) -> Result<Cache> {
    validate(tx, user_id)?;
    if find(tx.conn(), user_id)?.is_some() {
        let mut errors = crate::Errors::default();
        errors.add("user_id", "has already been taken");
        return Err(crate::Error::RecordInvalid(errors));
    }
    tx.conn().execute(
        "INSERT INTO calendar_meeting_caches(user_id,created_at,updated_at) VALUES(?,?,?)",
        params![user_id, tx.now(), tx.now()],
    )?;
    Ok(find(tx.conn(), user_id)?.expect("created cache"))
}
pub fn complete(
    tx: &mut Tx<'_>,
    user_id: i64,
    busy: Option<Value>,
    ooo: Option<Value>,
    error: Option<String>,
    fetched_at: Timestamp,
) -> Result<()> {
    validate(tx, user_id)?;
    let old = find(tx.conn(), user_id)?;
    let busy = busy.unwrap_or_else(|| old.as_ref().map(|c| c.busy.clone()).unwrap_or(json!([])));
    let ooo = ooo.unwrap_or_else(|| old.as_ref().map(|c| c.ooo.clone()).unwrap_or(json!([])));
    if old.as_ref().is_some_and(|c| {
        c.busy == busy
            && c.ooo == ooo
            && c.fetch_error == error
            && c.fetched_at == Some(fetched_at)
            && c.refresh_pending_at.is_none()
    }) {
        return Ok(());
    }
    tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,fetch_error,fetched_at,refresh_pending_at,created_at,updated_at) VALUES(?,?,?,?,?,NULL,STRFTIME('%Y-%m-%d %H:%M:%f','NOW'),STRFTIME('%Y-%m-%d %H:%M:%f','NOW')) ON CONFLICT(user_id) DO UPDATE SET busy_intervals=excluded.busy_intervals,ooo_intervals=excluded.ooo_intervals,fetch_error=excluded.fetch_error,fetched_at=excluded.fetched_at,refresh_pending_at=NULL,updated_at=excluded.updated_at",params![user_id,busy.to_string(),ooo.to_string(),error,fetched_at])?;
    Ok(())
}
pub fn follow_up(tx: &mut Tx<'_>, user_id: i64, now: Timestamp) -> Result<bool> {
    let won=tx.conn().execute("UPDATE calendar_meeting_caches SET refresh_pending_at=?,updated_at=? WHERE user_id=? AND (refresh_pending_at IS NULL OR refresh_pending_at<=?)",params![now,tx.now(),user_id,now.ago(SignedDuration::from_secs(60))])?==1;
    if won {
        tx.emit_after_commit(Event::job_in(
            std::time::Duration::from_secs(60),
            &MeetingRefreshJob { user_id },
        ));
    }
    Ok(won)
}
fn truthy(v: &Value) -> bool {
    !matches!(v, Value::Null | Value::Bool(false))
}
fn parse_time(value: &Value) -> Option<jiff::Timestamp> {
    let text = value.as_str()?;
    text.parse()
        .ok()
        .or_else(|| crate::Timestamp::parse_db(text).map(crate::Timestamp::jiff))
}
pub fn intervals(items: &Value, zone_name: &str) -> (Value, Value) {
    let zone = crate::slash_commands::time_parser::zone(zone_name);
    let mut busy = vec![];
    let mut ooo = vec![];
    for item in items.as_array().into_iter().flatten() {
        if !item.is_object() || item["status"] == "cancelled" {
            continue;
        }
        let all_day = item["start"]["dateTime"]
            .as_str()
            .is_none_or(campfire_richtext::ruby::is_blank);
        let (start, end) = if all_day {
            let day = |v: &Value| -> Option<jiff::Timestamp> {
                let date = v.as_str()?.parse::<jiff::civil::Date>().ok()?;
                Some(date.at(0, 0, 0, 0).to_zoned(zone.clone()).ok()?.timestamp())
            };
            (day(&item["start"]["date"]), day(&item["end"]["date"]))
        } else {
            (
                parse_time(&item["start"]["dateTime"]),
                parse_time(&item["end"]["dateTime"]),
            )
        };
        let (Some(start), Some(end)) = (start, end) else {
            continue;
        };
        if end <= start {
            continue;
        }
        if item["eventType"] == "outOfOffice" {
            let format = |t: jiff::Timestamp| {
                if all_day && !matches!(zone.iana_name(), Some("UTC" | "Etc/UTC")) {
                    t.to_zoned(zone.clone())
                        .strftime("%Y-%m-%dT%H:%M:%S%:z")
                        .to_string()
                } else {
                    t.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
                }
            };
            ooo.push((start, [format(start), format(end)]));
        } else if !all_day
            && item["eventType"] != "focusTime"
            && item["transparency"] != "transparent"
            && !item["attendees"].as_array().is_some_and(|a| {
                a.iter().any(|p| {
                    p.is_object() && truthy(&p["self"]) && p["responseStatus"] == "declined"
                })
            })
        {
            busy.push((start, end));
        }
    }
    let pairs = |mut v: Vec<(jiff::Timestamp, jiff::Timestamp)>| {
        v.sort_by_key(|p| p.0);
        json!(
            v.into_iter()
                .map(|(s, e)| [
                    s.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    e.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
                ])
                .collect::<Vec<_>>()
        )
    };
    ooo.sort_by_key(|p| p.0);
    (
        pairs(busy),
        json!(ooo.into_iter().map(|p| p.1).collect::<Vec<_>>()),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NewUser, tests::TestDb};
    #[test]
    fn google_meeting_cache_creation_validates_user_and_uniqueness() {
        let db = TestDb::new();
        db.write(|tx| {
            let user = User::create(
                tx,
                NewUser {
                    name: "Cache member".into(),
                    email_address: Some("cache@external.test".into()),
                    ..Default::default()
                },
            )?;
            assert!(matches!(
                create(tx, -1),
                Err(crate::Error::RecordInvalid(_))
            ));
            create(tx, user.id)?;
            assert!(matches!(
                create(tx, user.id),
                Err(crate::Error::RecordInvalid(_))
            ));
            Ok(())
        });
    }
    #[test]
    fn google_meeting_intervals_match_rails() {
        let v: Value = serde_json::from_str(include_str!(
            "../../../../vectors/google_meeting_intervals.json"
        ))
        .unwrap();
        for case in v["cases"].as_array().unwrap() {
            let (busy, ooo) = intervals(&case["items"], case["zone"].as_str().unwrap());
            assert_eq!(busy, case["busy"], "{}", case["name"]);
            assert_eq!(ooo, case["ooo"], "{}", case["name"]);
        }
    }
}
