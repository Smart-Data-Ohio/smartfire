//! The huddle and stage JSON (`campfire_api_types::huddle` and `::stage`): built here once, for
//! both the `/api/v1` endpoints and the sync twins of `huddle_effects::*`, the join notices and the
//! invitation frames.
use campfire_api_types::{
    HuddleNotice, HuddleParticipant, HuddlePresence, HuddleRing, HuddleRingEvent, HuddleRingState,
    HuddleRoleChanged, StageMember, StageRole, StageState, StageStream, StageStreamStopped,
    StreamQuality, SyncPayload,
};
use campfire_cable::sync::{Audience, SyncPublication};
use campfire_db::models::huddle_grant::IN_CALL_WINDOW;
use campfire_db::models::stream::Stream;
use campfire_db::{Connection, Membership, Room, Timestamp};
use jiff::SignedDuration;
use rails_compat::unicode;

use super::Cable;
use super::sync::room_topic;

/// A timestamp as the wire carries it: RFC 3339 in UTC with milliseconds.
pub fn stage_role(role: campfire_db::StageRole) -> StageRole {
    match role {
        campfire_db::StageRole::Listener => StageRole::Listener,
        campfire_db::StageRole::Speaker => StageRole::Speaker,
        campfire_db::StageRole::Host => StageRole::Host,
    }
}

pub fn stream_quality(quality: &str) -> Option<StreamQuality> {
    match quality {
        "720p15" => Some(StreamQuality::P720Fps15),
        "1080p15" => Some(StreamQuality::P1080Fps15),
        "1080p30" => Some(StreamQuality::P1080Fps30),
        "1080p60" => Some(StreamQuality::P1080Fps60),
        _ => None,
    }
}

pub fn stream_quality_name(quality: StreamQuality) -> &'static str {
    match quality {
        StreamQuality::P720Fps15 => "720p15",
        StreamQuality::P1080Fps15 => "1080p15",
        StreamQuality::P1080Fps30 => "1080p30",
        StreamQuality::P1080Fps60 => "1080p60",
    }
}

/// Who's in the room's call (`HuddleGrant.participants_for` and `participant_identities_for`):
/// one entry per person with an unrevoked grant seen in the last 20 s, by name.
pub fn presence(
    conn: &Connection,
    room: &Room,
    now: Timestamp,
) -> campfire_db::Result<HuddlePresence> {
    let since = now.ago(SignedDuration::from_secs(IN_CALL_WINDOW));
    let mut statement = conn.prepare_cached(
        "SELECT g.user_id, u.name, g.membership_id, g.identity, m.server_muted_at IS NOT NULL \
         FROM huddle_grants g JOIN users u ON u.id = g.user_id \
         LEFT JOIN memberships m ON m.id = g.membership_id \
         WHERE g.room_id = ? AND g.revoked_at IS NULL AND g.last_seen_at > ? ORDER BY g.id",
    )?;
    let rows = statement
        .query_map(rusqlite::params![room.id, since], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut people: Vec<(String, HuddleParticipant)> = Vec::new();
    for (user_id, name, membership_id, identity, server_muted) in rows {
        match people.iter_mut().find(|(_, p)| p.user_id == user_id) {
            Some((_, participant)) => participant.identities.push(identity),
            None => people.push((
                unicode::downcase(&name),
                HuddleParticipant {
                    user_id,
                    membership_id,
                    identities: vec![identity],
                    server_muted,
                },
            )),
        }
    }
    people.sort_by(|(a, _), (b, _)| a.cmp(b));
    let live = room.stage() && Stream::live_for_room(conn, room.id)?.is_some();
    Ok(HuddlePresence {
        room_id: room.id,
        participants: people.into_iter().map(|(_, p)| p).collect(),
        live,
    })
}

/// The stage's roster and stream (`presenters::calls::stage_model`, without the viewer).
pub fn stage(conn: &Connection, room_id: i64) -> campfire_db::Result<StageState> {
    let members = Membership::for_room(conn, room_id)?
        .into_iter()
        .filter_map(|m| {
            Some(StageMember {
                membership_id: m.id,
                user_id: m.user_id,
                role: stage_role(m.stage_role?),
                hand_raised_at: m.hand_raised_at.map(|at| at.to_wire()),
                server_muted: m.server_muted_at.is_some(),
            })
        })
        .collect();
    let live = match Stream::live_for_room(conn, room_id)? {
        Some(stream) => self::stream(conn, &stream)?,
        None => None,
    };
    Ok(StageState {
        room_id,
        members,
        live,
    })
}

/// A stream as the stage shows it, with the presenter's newest live LiveKit identity; `None`
/// for a quality the wire doesn't know.
pub fn stream(conn: &Connection, stream: &Stream) -> campfire_db::Result<Option<StageStream>> {
    use campfire_db::CachedStatements;
    use rusqlite::OptionalExtension;
    let identity: Option<String> = conn
        .query_row_cached(
            "SELECT identity FROM huddle_grants WHERE room_id=? AND membership_id=? AND revoked_at IS NULL ORDER BY last_issued_at DESC LIMIT 1",
            rusqlite::params![stream.room_id, stream.membership_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(stream_quality(&stream.quality).map(|quality| StageStream {
        id: stream.id,
        membership_id: stream.membership_id,
        user_id: stream.user_id,
        identity,
        quality,
        started_at: stream.started_at.to_wire(),
    }))
}

fn send(server: &Cable, audience: Audience, payload: &SyncPayload, coalesce: Option<String>) {
    let payload = serde_json::to_string(payload).expect("sync payloads serialize");
    server.sync_publish(SyncPublication {
        coalesce,
        ..SyncPublication::new(audience, payload)
    });
}

/// `huddle.presence` to each member, latest per room in a batch: the twin of
/// `HuddleGrant#broadcast_voice_presence` and of the stage's sidebar live dot.
pub fn publish_presence(server: &Cable, conn: &Connection, room: &Room, now: Timestamp) {
    if !server.sync_wanted() {
        return;
    }
    let (presence, members) = match presence(conn, room, now)
        .and_then(|presence| Ok((presence, Membership::for_room(conn, room.id)?)))
    {
        Ok(read) => read,
        Err(error) => {
            return tracing::warn!(%error, room_id = room.id, "sync: huddle presence not read");
        }
    };
    let payload = SyncPayload::HuddlePresence(presence);
    for member in members {
        send(
            server,
            Audience::User(member.user_id),
            &payload,
            Some(format!("huddle.presence:{}", room.id)),
        );
    }
}

/// `stage.updated` on the room's topic, latest per room in a batch: the twin of the roster, the
/// panel and the stream's panel replace.
pub fn publish_stage(server: &Cable, conn: &Connection, room_id: i64) {
    if !server.sync_wanted() {
        return;
    }
    match stage(conn, room_id) {
        Ok(stage) => send(
            server,
            Audience::Topic(room_topic(room_id)),
            &SyncPayload::StageUpdated(stage),
            Some(format!("stage.updated:{room_id}")),
        ),
        Err(error) => tracing::warn!(%error, room_id, "sync: stage not read"),
    }
}

/// `huddle.role` to the member: the twin of `Stage#broadcast_role_event_to_member`.
pub fn publish_role(server: &Cable, member: &Membership) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::HuddleRole(HuddleRoleChanged {
        room_id: member.room_id,
        stage_role: member.stage_role.map(stage_role),
        server_muted: member.server_muted_at.is_some(),
    });
    send(server, Audience::User(member.user_id), &payload, None);
}

/// `stage.stream.stopped` to the presenter: the twin of `Stream#broadcast_stream_stopped_event`.
pub fn publish_stream_stopped(server: &Cable, room_id: i64, user_id: i64) {
    if !server.sync_wanted() {
        return;
    }
    let payload = SyncPayload::StageStreamStopped(StageStreamStopped { room_id });
    send(server, Audience::User(user_id), &payload, None);
}

/// The JSON twin of a raw cable frame on `user_<id>_huddle_notices` (`huddle.notice`) or of a
/// huddle invitation on `user_<id>_activity` (`huddle.ring`). Returns false for other frames.
pub fn cable_stream(server: &Cable, stream: &str, payload: &serde_json::Value) -> bool {
    let user_id = |suffix: &str| {
        stream
            .strip_prefix("user_")?
            .strip_suffix(suffix)?
            .parse::<i64>()
            .ok()
    };
    let event = if let Some(user_id) = user_id("_huddle_notices") {
        notice(&payload["huddleJoinNotice"])
            .map(|notice| (user_id, SyncPayload::HuddleNotice(notice)))
    } else if let Some(user_id) = user_id("_activity")
        && payload.get("huddleInvitation").is_some()
    {
        ring(&payload["huddleInvitation"]).map(|ring| (user_id, SyncPayload::HuddleRing(ring)))
    } else {
        return false;
    };
    match event {
        Some((user_id, payload)) => send(server, Audience::User(user_id), &payload, None),
        None => tracing::warn!(stream, %payload, "sync: huddle frame not understood"),
    }
    true
}

/// `{eventType, roomId, roomName, joinerId, joinerName, inCall, rejoin}` (`Huddle::JoinNotifier`).
pub fn notice(frame: &serde_json::Value) -> Option<HuddleNotice> {
    let room_id = frame["roomId"].as_i64()?;
    let text = |key: &str| frame[key].as_str().map(str::to_string);
    Some(match frame["eventType"].as_str()? {
        "huddle_joined" => HuddleNotice::Joined {
            room_id,
            room_name: text("roomName")?,
            user_id: frame["joinerId"].as_i64()?,
            user_name: text("joinerName")?,
            in_call: frame["inCall"].as_bool().unwrap_or(false),
            rejoin: frame["rejoin"].as_bool().unwrap_or(false),
        },
        "huddle_left" => HuddleNotice::Left {
            room_id,
            room_name: text("roomName")?,
            user_id: frame["joinerId"].as_i64()?,
            user_name: text("joinerName")?,
        },
        "huddle_ended" => HuddleNotice::Ended { room_id },
        _ => return None,
    })
}

/// `{activityItemId, eventType, state, roomId, roomName, callerName, silent}` (`HuddleGrant`'s
/// invitation frames; `activityItemId` 0 when there's no item).
pub fn ring(frame: &serde_json::Value) -> Option<HuddleRing> {
    Some(HuddleRing {
        activity_item_id: frame["activityItemId"].as_i64().filter(|id| *id != 0),
        event: match frame["eventType"].as_str()? {
            "huddle_started" => HuddleRingEvent::Started,
            "huddle_missed" => HuddleRingEvent::Missed,
            "huddle_ended" => HuddleRingEvent::Ended,
            _ => return None,
        },
        state: match frame["state"].as_str()? {
            "unread" => HuddleRingState::Unread,
            "read" => HuddleRingState::Read,
            "handled" => HuddleRingState::Handled,
            _ => return None,
        },
        room_id: frame["roomId"].as_i64()?,
        room_name: frame["roomName"].as_str()?.to_string(),
        caller_name: frame["callerName"].as_str()?.to_string(),
        silent: frame["silent"].as_bool().unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn notices_read_the_classic_frames() {
        assert_eq!(
            notice(
                &json!({"eventType":"huddle_joined","roomId":12,"roomName":"general","roomPath":"/rooms/12","joinerId":8,"joinerName":"Grace","inCall":true,"rejoin":false})
            ),
            Some(HuddleNotice::Joined {
                room_id: 12,
                room_name: "general".into(),
                user_id: 8,
                user_name: "Grace".into(),
                in_call: true,
                rejoin: false
            })
        );
        assert_eq!(
            notice(
                &json!({"eventType":"huddle_left","roomId":12,"roomName":"general","joinerId":8,"joinerName":"Grace"})
            ),
            Some(HuddleNotice::Left {
                room_id: 12,
                room_name: "general".into(),
                user_id: 8,
                user_name: "Grace".into()
            })
        );
        assert_eq!(
            notice(&json!({"eventType":"huddle_ended","roomId":12})),
            Some(HuddleNotice::Ended { room_id: 12 })
        );
        assert_eq!(
            notice(&json!({"eventType":"huddle_paused","roomId":12})),
            None
        );
    }

    #[test]
    fn rings_read_the_classic_frames() {
        let started = json!({"activityItemId":0,"eventType":"huddle_started","state":"unread","roomId":3,"roomName":"Ada","roomPath":"/rooms/3","callerName":"Ada","readPath":"","handledPath":"","silent":true});
        assert_eq!(
            ring(&started),
            Some(HuddleRing {
                activity_item_id: None,
                event: HuddleRingEvent::Started,
                state: HuddleRingState::Unread,
                room_id: 3,
                room_name: "Ada".into(),
                caller_name: "Ada".into(),
                silent: true
            })
        );
        let ended = json!({"activityItemId":41,"eventType":"huddle_ended","state":"read","roomId":3,"roomName":"Ada","roomPath":"/rooms/3","callerName":"Ada","readPath":"/activity/41/read?state=read","handledPath":"/activity/41/handled?state=handled"});
        let ended = ring(&ended).unwrap();
        assert_eq!(
            (
                ended.activity_item_id,
                ended.event,
                ended.state,
                ended.silent
            ),
            (
                Some(41),
                HuddleRingEvent::Ended,
                HuddleRingState::Read,
                false
            )
        );
        let missed = json!({"activityItemId":41,"eventType":"huddle_missed","state":"unread","roomId":3,"roomName":"Ada","callerName":"Ada","silent":false});
        assert_eq!(ring(&missed).unwrap().event, HuddleRingEvent::Missed);
    }

    #[test]
    fn every_stored_quality_round_trips_through_the_wire() {
        for name in campfire_db::models::stream::QUALITIES {
            let quality =
                stream_quality(name).unwrap_or_else(|| panic!("{name} has no wire value"));
            assert_eq!(stream_quality_name(quality), name);
            assert_eq!(serde_json::to_value(quality).unwrap(), json!(name));
        }
        assert_eq!(stream_quality("1080p60"), Some(StreamQuality::P1080Fps60));
        assert_eq!(stream_quality("4k60"), None);
    }
}
