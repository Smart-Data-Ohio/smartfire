//! Who may connect, and subscribe to what: every channel (and the room-scoped ones for a room and
//! one of its threads) against a member, a non-member, a banned user, a deactivated user, a bot, a
//! session still waiting on its second factor, and an expired administrator session. The whole
//! table is compared at once, so a regression reports every cell it flips.
use serde_json::json;

use super::support::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Principal {
    Member,
    NonMember,
    Banned,
    Deactivated,
    Bot,
    TwoFactorPending,
    ExpiredSession,
}

/// The subscriptions tried, in this order on one connection.
const CHANNELS: [&str; 12] = [
    "HeartbeatChannel",
    "RoomChannel",
    "PresenceChannel",
    "TypingNotificationsChannel",
    "TypingNotificationsChannel (thread)",
    "ActivityChannel",
    "HuddleNoticeChannel",
    "AgentsChannel",
    "UnreadRoomsChannel",
    "UnreadThreadsChannel",
    "ReadRoomsChannel",
    "WorkspacePresenceChannel",
];

/// `C` confirmed, `R` rejected, per [`CHANNELS`]; `None` when the connection itself is refused.
fn expected(principal: Principal) -> Option<&'static str> {
    match principal {
        Principal::Member => Some("CCCCCCCCCCCC"),
        Principal::NonMember => Some("CRRRRCCCCCCC"),
        Principal::Bot => Some("CCCCCRRRCCCC"),
        Principal::Banned | Principal::Deactivated | Principal::TwoFactorPending | Principal::ExpiredSession => None,
    }
}

/// The room is watercooler (Jason, David and Bender; not Kevin).
fn identifiers(_app: &TestApp, room: &campfire_db::Room, thread: i64, _user_id: i64) -> Vec<String> {
    let plain = |channel: &str| identifier(json!({ "channel": channel }));
    vec![
        plain("HeartbeatChannel"),
        room_identifier("RoomChannel", room.id),
        room_identifier("PresenceChannel", room.id),
        room_identifier("TypingNotificationsChannel", room.id),
        identifier(json!({ "channel": "TypingNotificationsChannel", "room_id": room.id, "thread_id": thread })),
        plain("ActivityChannel"),
        plain("HuddleNoticeChannel"),
        plain("AgentsChannel"),
        plain("UnreadRoomsChannel"),
        plain("UnreadThreadsChannel"),
        plain("ReadRoomsChannel"),
        plain("WorkspacePresenceChannel"),
    ]
}

/// The principal's user and a session cookie issued before whatever revokes them.
async fn prepare(app: &TestApp, principal: Principal) -> (&'static str, String) {
    let user = match principal {
        Principal::NonMember => "kevin",
        Principal::Bot => "bender",
        _ => "jason",
    };
    let verified = !matches!(principal, Principal::TwoFactorPending | Principal::Bot);
    let session = app.session_for(user, verified).await;
    let user_id = id(user);
    match principal {
        Principal::Banned => app.db.write(move |tx| campfire_db::User::find(tx.conn(), user_id)?.ban(tx)).await.unwrap(),
        Principal::Deactivated => app.db.write(move |tx| campfire_db::User::find(tx.conn(), user_id)?.deactivate(tx)).await.unwrap(),
        Principal::ExpiredSession => {
            // Jason is an administrator; the connection checks expiry on the process clock.
            let idle = campfire_db::Timestamp::from_jiff(jiff::Timestamp::now()).ago(jiff::SignedDuration::from_hours(8 * 24));
            let session_id = session.id;
            app.db
                .write(move |tx| {
                    tx.conn().execute("UPDATE sessions SET last_active_at = ? WHERE id = ?", rusqlite::params![idle, session_id])?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        _ => {}
    }
    (user, app.cookie_with_token(&session.token))
}

async fn outcomes(app: &TestApp, principal: Principal) -> Option<String> {
    let room = app.room("watercooler").await;
    let thread = app.create_thread("watercooler", "david", "Matrix").await;
    let (user, cookie) = prepare(app, principal).await;
    let mut client = app.connect_with_cookie(Some(&cookie)).await;
    let first = client.next_text().await;
    if first == r#"{"type":"disconnect","reason":"unauthorized","reconnect":false}"# {
        return None;
    }
    assert_eq!(first, r#"{"type":"welcome"}"#);
    let mut row = String::new();
    for subscription in identifiers(app, &room, thread, id(user)) {
        let reply = client.subscribe_reply(&subscription).await;
        row.push(if reply == confirmation(&subscription) {
            'C'
        } else if reply == rejection(&subscription) {
            'R'
        } else {
            panic!("{principal:?} subscribing to {subscription}: {reply}")
        });
    }
    Some(row)
}

#[tokio::test]
async fn authorization_matrix() {
    let mut mismatches = Vec::new();
    for principal in [
        Principal::Member,
        Principal::NonMember,
        Principal::Banned,
        Principal::Deactivated,
        Principal::Bot,
        Principal::TwoFactorPending,
        Principal::ExpiredSession,
    ] {
        let app = start().await;
        let actual = outcomes(&app, principal).await;
        match (expected(principal), actual.as_deref()) {
            (Some(expected), Some(actual)) => {
                for ((channel, want), got) in CHANNELS.iter().zip(expected.chars()).zip(actual.chars()) {
                    if want != got {
                        mismatches.push(format!("{principal:?} on {channel}: expected {want}, got {got}"));
                    }
                }
            }
            (None, None) => {}
            (None, Some(_)) => mismatches.push(format!("{principal:?}: expected the connection refused, but it connected")),
            (Some(_), None) => mismatches.push(format!("{principal:?}: expected a connection, but it was refused")),
        }
    }
    assert!(mismatches.is_empty(), "authorization mismatches:\n{}", mismatches.join("\n"));
}

/// The expired session is destroyed as it's refused.
#[tokio::test]
async fn an_expired_administrator_session_is_destroyed_when_refused() {
    let app = start().await;
    let (_, cookie) = prepare(&app, Principal::ExpiredSession).await;
    let mut client = app.connect_with_cookie(Some(&cookie)).await;
    client.until_closed().await;
    let sessions = app.db.read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM sessions WHERE user_id = ?", [id("jason")], |row| row.get::<_, i64>(0))?)).await.unwrap();
    assert_eq!(sessions, 0);
}
