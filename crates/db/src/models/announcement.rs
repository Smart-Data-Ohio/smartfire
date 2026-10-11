//! Same-instance announcement publications and their reserved destinations.
use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::sql::{placeholders, query_all, query_one};
use crate::{
    CachedStatements, Errors, Event, Job, Message, MessageChanges, NewMessage, Room, RoomType,
    Snapshot, Timestamp, Tx,
};

#[derive(Debug, Clone)]
pub struct Follow {
    pub source_room_id: i64,
    pub target_room_id: i64,
    pub creator_id: i64,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone)]
pub struct Source {
    pub room_id: i64,
    pub message_id: i64,
    pub room_name: Option<String>,
}

#[derive(Debug, Default)]
pub struct ReplayAccess {
    pub rooms: BTreeSet<i64>,
    pub sources: BTreeMap<i64, Source>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJob {
    pub publication_id: i64,
}
impl Job for SyncJob {
    const CLASS: &'static str = "Announcement::SyncJob";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCopiesJob {
    pub publication_id: i64,
    pub message_ids: Vec<i64>,
}
impl Job for DeleteCopiesJob {
    const CLASS: &'static str = "Announcement::DeleteCopiesJob";
}

fn invalid(field: &'static str, text: &str) -> crate::Error {
    let mut errors = Errors::default();
    errors.add(field, text);
    crate::Error::RecordInvalid(errors)
}

fn alive(conn: &Connection, id: i64) -> crate::Result<Room> {
    let room = Room::find(conn, id)?;
    if room.deleted() {
        return Err(crate::Error::RecordNotFound("Room"));
    }
    Ok(room)
}

pub fn set_flag(tx: &mut Tx<'_>, room_id: i64, value: bool) -> crate::Result<Room> {
    let room = alive(tx.conn(), room_id)?;
    if room.direct() {
        return Err(invalid(
            "announcement",
            "is not supported for direct messages",
        ));
    }
    tx.conn().execute_cached(
        "UPDATE rooms SET announcement=?, updated_at=? WHERE id=? AND announcement!=?",
        params![value, tx.now(), room_id, value],
    )?;
    Room::find(tx.conn(), room_id)
}

pub fn follow(tx: &mut Tx<'_>, source: i64, target: i64, creator: i64) -> crate::Result<()> {
    let source_room = alive(tx.conn(), source)?;
    let target_room = alive(tx.conn(), target)?;
    if !source_room.announcement {
        return Err(invalid("sourceRoomId", "must be an announcement room"));
    }
    match target_room.room_type {
        RoomType::Open | RoomType::Closed | RoomType::Voice | RoomType::Stage => {}
        RoomType::Direct => return Err(invalid("targetRoomId", "cannot be a direct message")),
        RoomType::Board => {
            return Err(invalid(
                "targetRoomId",
                "must show plain timeline messages; boards are not supported",
            ));
        }
    }
    let cycle: bool = tx.conn().query_row_cached(
        "WITH RECURSIVE chain(id) AS (SELECT ? UNION SELECT f.target_room_id FROM announcement_follows f JOIN chain ON f.source_room_id=chain.id) SELECT EXISTS(SELECT 1 FROM chain WHERE id=?)",
        [target, source], |r| r.get(0))?;
    if cycle {
        return Err(invalid(
            "targetRoomId",
            "would create an announcement cycle",
        ));
    }
    tx.conn().execute_cached("INSERT INTO announcement_follows VALUES (?, ?, ?, ?) ON CONFLICT(source_room_id,target_room_id) DO NOTHING", params![source, target, creator, tx.now()])?;
    Ok(())
}

pub fn unfollow(tx: &mut Tx<'_>, source: i64, target: i64) -> crate::Result<()> {
    tx.conn().execute_cached(
        "DELETE FROM announcement_follows WHERE source_room_id=? AND target_room_id=?",
        [source, target],
    )?;
    // A pending delivery no longer has a destination. Delivered copies still receive edits.
    tx.conn().execute_cached("DELETE FROM announcement_deliveries WHERE target_room_id=? AND message_id IS NULL AND publication_id IN (SELECT id FROM announcement_publications WHERE source_message_id IN (SELECT id FROM messages WHERE room_id=?))", [target, source])?;
    Ok(())
}

fn follows(conn: &Connection, column: &str, room_id: i64) -> crate::Result<Vec<Follow>> {
    query_all(
        conn,
        &format!(
            "SELECT f.* FROM announcement_follows f JOIN rooms s ON s.id=f.source_room_id JOIN rooms t ON t.id=f.target_room_id WHERE f.{column}=? AND s.deleted_at IS NULL AND t.deleted_at IS NULL ORDER BY f.source_room_id,f.target_room_id"
        ),
        [room_id],
        |r| {
            Ok(Follow {
                source_room_id: r.get("source_room_id")?,
                target_room_id: r.get("target_room_id")?,
                creator_id: r.get("creator_id")?,
                created_at: r.get("created_at")?,
            })
        },
    )
}
pub fn followers(conn: &Connection, room_id: i64) -> crate::Result<Vec<Follow>> {
    follows(conn, "source_room_id", room_id)
}
pub fn following(conn: &Connection, room_id: i64) -> crate::Result<Vec<Follow>> {
    follows(conn, "target_room_id", room_id)
}

pub fn publication_id(conn: &Connection, message_id: i64) -> crate::Result<Option<i64>> {
    query_one(
        conn,
        "SELECT id FROM announcement_publications WHERE source_message_id=?",
        [message_id],
        |r| r.get(0),
    )
}

pub fn is_copy(conn: &Connection, message_id: i64) -> crate::Result<bool> {
    Ok(conn.query_row_cached(
        "SELECT EXISTS(SELECT 1 FROM announcement_deliveries WHERE message_id=?)",
        [message_id],
        |r| r.get(0),
    )?)
}

pub fn publish(tx: &mut Tx<'_>, message: &Message, publisher: i64) -> crate::Result<i64> {
    if let Some(id) = publication_id(tx.conn(), message.id)? {
        return Ok(id);
    }
    let mut message = Message::find(tx.conn(), message.id)?;
    if !alive(tx.conn(), message.room_id)?.announcement {
        return Err(invalid("message", "must be in an announcement room"));
    }
    if message.system_note || message.streaming || is_copy(tx.conn(), message.id)? {
        return Err(invalid(
            "message",
            "cannot publish a copy, system note or unfinished message",
        ));
    }
    let publication: i64 = tx.conn().query_row_cached("INSERT INTO announcement_publications(source_message_id,publisher_id,published_at) VALUES (?,?,?) RETURNING id", params![message.id, publisher, tx.now()], |r| r.get(0))?;
    // Reserve the follower set now, never on a retry. Later follows do not backfill.
    tx.conn().execute_cached("INSERT INTO announcement_deliveries(publication_id,target_room_id) SELECT ?, f.target_room_id FROM announcement_follows f JOIN rooms r ON r.id=f.target_room_id WHERE f.source_room_id=? AND r.deleted_at IS NULL AND r.type IN ('Rooms::Open','Rooms::Closed','Rooms::Voice','Rooms::Stage')", [publication, message.room_id])?;
    message.touch(tx)?;
    enqueue_update(tx, Some(publication))?;
    tx.emit_after_commit(Event::broadcast(
        &crate::broadcasts::Broadcast::MessageUpdated {
            message_id: message.id,
        },
    ));
    Ok(publication)
}

/// Read publication state with the association a mutation already needs, including when the
/// association is absent. Ordinary message writes need no additional read statement.
pub(crate) fn record_with_publication<T>(
    conn: &Connection,
    message_id: i64,
    table: &str,
    name: &str,
    from_row: impl FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> crate::Result<(Option<T>, Option<i64>)> {
    Ok(conn.query_row_cached(
        &format!(
            "SELECT record.*, publication.id AS publication_id FROM (SELECT ? AS id) message \
             LEFT JOIN {table} record ON record.record_type='Message' AND record.record_id=message.id AND record.name=? \
             LEFT JOIN announcement_publications publication ON publication.source_message_id=message.id LIMIT 1"
        ),
        params![message_id, name],
        |row| {
            let record = row.get::<_, Option<i64>>("id")?
                .map(|_| from_row(row)).transpose()?;
            Ok((record, row.get("publication_id")?))
        },
    )?)
}

pub(crate) fn enqueue_update(tx: &mut Tx<'_>, publication_id: Option<i64>) -> crate::Result<()> {
    if let Some(publication_id) = publication_id {
        // Delivery uses the final source state and must succeed before its write commits.
        tx.before_commit_record_latest("announcement", publication_id, move |tx| {
            sync(tx, publication_id)
        })?;
        tx.emit_after_commit(Event::job(&SyncJob { publication_id }));
    }
    Ok(())
}

pub(crate) fn enqueue_delete(tx: &mut Tx<'_>, publication: Option<i64>) -> crate::Result<()> {
    if let Some(publication) = publication {
        let message_ids = copies(tx.conn(), publication)?
            .into_iter()
            .map(|m| m.id)
            .collect();
        // Keep the publication until its copies are removed, so copies cannot be republished
        // between source deletion and the background cleanup.
        tx.emit_after_commit(Event::job(&DeleteCopiesJob {
            publication_id: publication,
            message_ids,
        }));
    }
    Ok(())
}

pub fn copies(conn: &Connection, publication: i64) -> crate::Result<Vec<Message>> {
    query_all(
        conn,
        "SELECT m.* FROM announcement_deliveries d JOIN messages m ON m.id=d.message_id WHERE d.publication_id=? ORDER BY d.target_room_id",
        [publication],
        Message::from_row,
    )
}

pub fn source_for_viewer(
    conn: &Connection,
    message_id: i64,
    viewer: i64,
) -> crate::Result<Option<Source>> {
    query_one(
        conn,
        "SELECT r.id, m.id, r.name FROM announcement_deliveries d JOIN announcement_publications p ON p.id=d.publication_id JOIN messages m ON m.id=p.source_message_id JOIN rooms r ON r.id=m.room_id JOIN memberships member ON member.room_id=r.id AND member.user_id=? WHERE d.message_id=? AND r.deleted_at IS NULL",
        [viewer, message_id],
        |r| {
            Ok(Source {
                room_id: r.get(0)?,
                message_id: r.get(1)?,
                room_name: r.get(2)?,
            })
        },
    )
}

pub fn replay_access(
    conn: &Snapshot<'_>,
    viewer: i64,
    room_ids: BTreeSet<i64>,
    message_ids: BTreeSet<i64>,
) -> crate::Result<ReplayAccess> {
    // Detect copies with the room grants so ordinary batches need no provenance read.
    let grants = query_all(
        conn,
        &format!(
            "SELECT r.id,d.message_id FROM rooms r JOIN memberships member ON member.room_id=r.id AND member.user_id=? LEFT JOIN announcement_deliveries d ON d.target_room_id=r.id AND d.message_id IN ({}) WHERE r.id IN ({}) AND r.deleted_at IS NULL",
            placeholders(message_ids.len()),
            placeholders(room_ids.len())
        ),
        rusqlite::params_from_iter(std::iter::once(viewer).chain(message_ids).chain(room_ids)),
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
    )?;
    let rooms = grants.iter().map(|(room, _)| *room).collect();
    let copies: BTreeSet<_> = grants
        .into_iter()
        .filter_map(|(_, message)| message)
        .collect();
    let sources = if copies.is_empty() {
        BTreeMap::new()
    } else {
        query_all(conn, &format!(
            "SELECT d.message_id,r.id,m.id,r.name FROM announcement_deliveries d JOIN announcement_publications p ON p.id=d.publication_id JOIN messages m ON m.id=p.source_message_id JOIN rooms r ON r.id=m.room_id JOIN memberships member ON member.room_id=r.id AND member.user_id=? WHERE d.message_id IN ({}) AND r.deleted_at IS NULL",
            placeholders(copies.len())
        ), rusqlite::params_from_iter(std::iter::once(viewer).chain(copies)), |row| Ok((row.get::<_, i64>(0)?, Source {
            room_id: row.get(1)?, message_id: row.get(2)?, room_name: row.get(3)?,
        })))?.into_iter().collect()
    };
    Ok(ReplayAccess { rooms, sources })
}

pub fn sync(tx: &mut Tx<'_>, publication: i64) -> crate::Result<()> {
    for (message, created) in deliver(tx, publication)? {
        if created {
            super::agent_posting::broadcast_create(tx, &message)?;
        } else {
            tx.emit_after_commit(Event::broadcast(
                &crate::broadcasts::Broadcast::MessageUpdated {
                    message_id: message.id,
                },
            ));
        }
    }
    Ok(())
}

/// Reserved deliveries and their normal message callbacks commit with the source write.
pub fn deliver(tx: &mut Tx<'_>, publication: i64) -> crate::Result<Vec<(Message, bool)>> {
    let Some(source_id) = query_one(
        tx.conn(),
        "SELECT source_message_id FROM announcement_publications WHERE id=?",
        [publication],
        |r| r.get::<_, Option<i64>>(0),
    )?
    .flatten() else {
        return Ok(Vec::new());
    };
    let source = Message::find(tx.conn(), source_id)?;
    if source.room(tx.conn())?.deleted() {
        return Ok(Vec::new());
    }
    let destinations = query_all(
        tx.conn(),
        "SELECT d.target_room_id,d.message_id FROM announcement_deliveries d JOIN rooms r ON r.id=d.target_room_id WHERE d.publication_id=? AND r.deleted_at IS NULL AND r.type IN ('Rooms::Open','Rooms::Closed','Rooms::Voice','Rooms::Stage') ORDER BY d.target_room_id",
        [publication],
        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?)),
    )?;
    let body = source.body_html(tx.conn())?;
    let legacy_attachment_snapshot = body
        .as_deref()
        .map(campfire_richtext::legacy_markdown::non_mention_attachments)
        .transpose()
        .map_err(|error| crate::Error::Other(error.to_string()))?;
    let blobs = source
        .attachments(tx.conn())?
        .into_iter()
        .map(|(a, _)| (a.name, a.blob_id))
        .collect::<Vec<_>>();
    let drive_ids = query_all(
        tx.conn(),
        "SELECT file_id FROM drive_attachments WHERE message_id=? ORDER BY id",
        [source.id],
        |r| r.get::<_, String>(0),
    )?;
    let mut changed = Vec::new();
    for (room_id, message_id) in destinations {
        if let Some(id) = message_id {
            let mut copy = Message::find(tx.conn(), id)?;
            let attachments = copy.attachments(tx.conn())?;
            let current = attachments
                .iter()
                .map(|(a, _)| (a.name.clone(), a.blob_id))
                .collect::<Vec<_>>();
            if current != blobs {
                for (attachment, _) in attachments {
                    attachment.delete(tx)?;
                    tx.emit_after_commit(Event::PurgeBlob {
                        blob_id: attachment.blob_id,
                    });
                }
                for (name, blob_id) in &blobs {
                    crate::Attachment::create(tx, "Message", copy.id, name, *blob_id)?;
                }
                copy.touch(tx)?;
            }
            copy.edit(
                tx,
                MessageChanges {
                    markdown_source: source.markdown_source.clone(),
                    clear_markdown_source: source.markdown_source.is_none(),
                    body: if source.markdown_source.is_some() {
                        None
                    } else {
                        body.clone()
                    },
                    legacy_attachment_snapshot: legacy_attachment_snapshot.clone(),
                    embeds_suppressed: Some(source.embeds_suppressed),
                    drive_file_ids: Some(drive_ids.clone()),
                    forward_note: Some(source.forward_note.clone()),
                    ..Default::default()
                },
            )?;
            changed.push((copy, false));
        } else {
            let mut copy = Message::create(
                tx,
                NewMessage {
                    room_id,
                    creator_id: source.creator_id,
                    body: body.clone(),
                    markdown_source: source.markdown_source.clone(),
                    action: source.action,
                    forward_note: source.forward_note.clone(),
                    forwarded_at: source.forwarded_at,
                    forwarded_from_message_id: source.forwarded_from_message_id,
                    forwarded_markdown: source.forwarded_markdown,
                    embeds_suppressed: source.embeds_suppressed,
                    attachment_blob_id: blobs
                        .iter()
                        .find(|(name, _)| name == "attachment")
                        .map(|(_, id)| *id),
                    attachment_blob_ids: blobs
                        .iter()
                        .filter(|(name, _)| name == "attachments")
                        .map(|(_, id)| *id)
                        .collect(),
                    drive_file_ids: drive_ids.clone(),
                    webhook: super::channel_webhook::ChannelWebhook::message_snapshot(
                        tx.conn(), source.id,
                    )?,
                    ..Default::default()
                },
            )?;
            if source.markdown_source.is_some()
                && let Some(snapshot) = legacy_attachment_snapshot
                    .as_deref()
                    .filter(|snapshot| !snapshot.is_empty())
            {
                let rendered = copy.body_html(tx.conn())?.unwrap_or_default();
                copy.update_body(tx, &format!("{rendered}\n{snapshot}"))?;
            }
            tx.conn().execute_cached("UPDATE announcement_deliveries SET message_id=? WHERE publication_id=? AND target_room_id=?", [copy.id,publication,room_id])?;
            changed.push((copy, true));
        }
    }
    Ok(changed)
}

pub fn delete_copies(
    tx: &mut Tx<'_>,
    publication: i64,
    message_ids: &[i64],
) -> crate::Result<Vec<(Message, Option<i64>)>> {
    let mut removed = Vec::new();
    for &id in message_ids {
        if let Some(message) = Message::find_by_id(tx.conn(), id)? {
            let parent_thread = crate::ChannelThread::find_by_parent_message(tx.conn(), id)?
                .map(|thread| thread.id);
            let replies = query_all(
                tx.conn(),
                "SELECT id FROM messages WHERE reply_to_message_id=?",
                [id],
                |r| r.get::<_, i64>(0),
            )?;
            message.destroy(tx)?;
            for message_id in replies {
                tx.emit_after_commit(Event::broadcast(
                    &crate::broadcasts::Broadcast::MessageUpdated { message_id },
                ));
            }
            removed.push((message, parent_thread));
        }
    }
    tx.conn().execute_cached(
        "DELETE FROM announcement_publications WHERE id=?",
        [publication],
    )?;
    Ok(removed)
}
