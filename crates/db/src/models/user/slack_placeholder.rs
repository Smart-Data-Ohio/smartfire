//! Verified claims of Slack-imported accounts and their history.
use campfire_richtext::content::attachment_nodes;
use campfire_richtext::{Content, ruby::is_blank};
use rails_compat::global_id::unverified_attachable_user;
use rusqlite::params;

use crate::audit_log::{AuditLog, Context, NewAuditLog};
use crate::broadcasts::Broadcast;
use crate::sql::{exists, query_all};
use crate::{
    Connection, Error, Event, Involvement, Membership, Message, Result, Role, Room,
    RoomRemovalBroadcast, Status, Tx, User,
};

pub fn is_unclaimed_slack_placeholder(conn: &Connection, user_id: i64) -> Result<bool> {
    let Some(user) = User::find_by_id(conn, user_id)? else {
        return Ok(false);
    };
    if user.role != Role::Member
        || user.status != Status::Active
        || user
            .password_digest
            .as_deref()
            .is_some_and(|s| !is_blank(s))
    {
        return Ok(false);
    }
    exists(
        conn,
        "SELECT 1 FROM users WHERE id = ?1
         AND email_self_changed_at IS NULL AND github_login IS NULL
         AND EXISTS (SELECT 1 FROM slack_import_records
                     WHERE slack_kind = 'user' AND record_type = 'User'
                       AND record_id = ?1 AND created_record = 1)
         AND NOT EXISTS (SELECT 1 FROM sessions WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM google_identities WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM two_factor_credentials WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM two_factor_remembered_devices WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM user_email_addresses WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM slack_connections WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM slack_imports WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM google_accounts WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM github_connected_accounts WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM fizzy_connected_accounts WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM oauth_authorizations WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM agents WHERE user_id = ?1 OR owner_id = ?1 OR paused_by_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM agent_credentials WHERE created_by_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM webhooks WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM push_subscriptions WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM user_devices WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM workspace_presence_leases WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM bans WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM saved_items WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM scheduled_messages WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM searches WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM keyword_alerts WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM room_categories WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM events WHERE organizer_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM poll_votes WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM workspace_invites WHERE creator_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM workspace_icons WHERE creator_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM channel_webhooks WHERE creator_id = ?1 OR author_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM announcement_follows WHERE creator_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM announcement_publications WHERE publisher_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM soundboard_sounds WHERE uploader_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM stickers WHERE uploader_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM huddle_grants WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM streams WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM audit_logs WHERE actor_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM calendar_meeting_caches WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM calendar_push_channels WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM event_calendar_entries WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM fizzy_card_caches WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM github_repository_subscriptions WHERE created_by_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM slack_workspaces WHERE configured_by_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM email_address_deactivations WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM work_handoffs WHERE sender_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM bot_avatars WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM user_stars WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM user_blocks WHERE blocker_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM dnd_allowed_users WHERE user_id = ?1)
         AND NOT EXISTS (SELECT 1 FROM active_storage_attachments
                         WHERE record_type = 'User' AND record_id = ?1)",
        [user_id],
    )
}

pub fn has_active_slack_import(conn: &Connection, except_run_id: Option<i64>) -> Result<bool> {
    exists(
        conn,
        "SELECT 1 FROM slack_imports
         WHERE status IN ('queued', 'running', 'undoing') AND (?1 IS NULL OR id != ?1)",
        [except_run_id],
    )
}

/// The caller verifies ownership and excludes other active Slack runs in this same write.
pub fn claim_slack_placeholder(tx: &mut Tx<'_>, target: &User, placeholder: &User) -> Result<()> {
    if !tx.in_transaction()
        || target.id == placeholder.id
        || !User::find_by_id(tx.conn(), target.id)?.is_some_and(|u| u.is_active() && !u.is_bot())
        || !is_unclaimed_slack_placeholder(tx.conn(), placeholder.id)?
    {
        return Err(Error::Other(
            "Slack placeholder is no longer claimable".into(),
        ));
    }
    let from = placeholder.id;
    let to = target.id;
    let conn = tx.conn();
    // Undo may remove imported history. It must treat the destination account as pre-existing.
    conn.execute(
        "UPDATE slack_import_records SET record_id = ?2, created_record = 0, updated_at = ?3
         WHERE slack_kind = 'user' AND record_type = 'User' AND record_id = ?1",
        params![from, to, tx.now()],
    )?;
    conn.execute("DELETE FROM activity_items WHERE user_id = ?", [from])?;
    for (table, column) in [
        ("messages", "creator_id"),
        ("channel_threads", "creator_id"),
        ("channel_threads", "work_owner_id"),
        ("channel_threads", "result_updated_by_id"),
        ("message_pins", "pinner_id"),
        ("rooms", "creator_id"),
        ("board_tag_assignments", "assignee_id"),
        ("board_tag_assignments", "created_by_id"),
        ("work_thread_events", "from_owner_id"),
        ("work_thread_events", "to_owner_id"),
        ("work_thread_events", "actor_id"),
        ("work_thread_links", "created_by_id"),
        ("agent_grants", "granted_by_id"),
        ("agent_approvals", "decided_by_id"),
        ("agent_events", "actor_id"),
        ("thread_access_memberships", "added_by_id"),
        ("board_sla_nudges", "recipient_id"),
    ] {
        conn.execute(
            &format!("UPDATE {table} SET {column} = ?2 WHERE {column} = ?1"),
            params![from, to],
        )?;
    }
    move_unique_references(tx, from, to)?;

    for membership in Membership::for_user(conn, from)? {
        let room = Room::find(conn, membership.room_id)?;
        if room.direct() && !room.deleted() {
            move_direct_membership(tx, &room, &membership, to)?;
        } else {
            let membership_id = move_membership(tx, &membership, to)?;
            if !room.deleted() {
                broadcast_membership(tx, membership_id);
            }
        }
    }
    rename_user_suffix(tx, "Membership", ":user-", from, to)?;
    retarget_mentions(tx, from, to)?;
    conn.execute("DELETE FROM users WHERE id = ?", [from])?;
    Ok(())
}

pub fn audit_claim(
    tx: &Tx<'_>,
    target: &User,
    placeholder_id: i64,
    context: &Context,
) -> Result<()> {
    AuditLog::record(
        tx,
        NewAuditLog {
            action: "user.slack_placeholder.claim".into(),
            actor: Some(target.into()),
            target: Some(target.into()),
            changes: Some(serde_json::json!({"placeholder_id": placeholder_id})),
            ..Default::default()
        },
        context,
    )?;
    Ok(())
}

fn move_unique_references(tx: &Tx<'_>, from: i64, to: i64) -> Result<()> {
    let conn = tx.conn();
    // A member can already have starred, blocked, or allowed their Slack identity.
    for (table, owner, other) in [
        ("user_stars", "user_id", "starred_user_id"),
        ("user_blocks", "blocker_id", "blocked_id"),
        ("dnd_allowed_users", "user_id", "allowed_user_id"),
    ] {
        conn.execute(
            &format!("DELETE FROM {table} WHERE {owner} = ?2 AND {other} = ?1"),
            params![from, to],
        )?;
    }
    for (table, user, same_row, record_type) in [
        (
            "boosts",
            "booster_id",
            "dst.message_id = src.message_id AND dst.content IS src.content",
            Some("Boost"),
        ),
        (
            "thread_memberships",
            "user_id",
            "dst.thread_id = src.thread_id",
            Some("ThreadMembership"),
        ),
        (
            "thread_access_memberships",
            "user_id",
            "dst.thread_id = src.thread_id",
            None,
        ),
        (
            "event_attendances",
            "user_id",
            "dst.event_id = src.event_id",
            None,
        ),
        (
            "user_stars",
            "starred_user_id",
            "dst.user_id = src.user_id",
            None,
        ),
        (
            "user_blocks",
            "blocked_id",
            "dst.blocker_id = src.blocker_id",
            None,
        ),
        (
            "dnd_allowed_users",
            "allowed_user_id",
            "dst.user_id = src.user_id",
            None,
        ),
    ] {
        let conflicts = format!(
            "src.{user} = ?1 AND EXISTS (
                SELECT 1 FROM {table} dst WHERE dst.{user} = ?2 AND {same_row})"
        );
        if let Some(record_type) = record_type {
            conn.execute(
                &format!(
                    "DELETE FROM slack_import_records WHERE record_type = ?3
                     AND record_id IN (SELECT src.id FROM {table} src WHERE {conflicts})"
                ),
                params![from, to, record_type],
            )?;
        }
        conn.execute(
            &format!("DELETE FROM {table} AS src WHERE {conflicts}"),
            params![from, to],
        )?;
        conn.execute(
            &format!("UPDATE {table} SET {user} = ?2 WHERE {user} = ?1"),
            params![from, to],
        )?;
    }
    rename_user_suffix(tx, "ThreadMembership", ":follow:", from, to)?;
    Ok(())
}

fn move_membership(tx: &mut Tx<'_>, source: &Membership, to: i64) -> Result<i64> {
    let conn = tx.conn();
    if let Some(mut existing) = Membership::find_by_room_and_user(conn, source.room_id, to)? {
        if existing.involvement == Some(Involvement::Invisible)
            && let Some(involvement) = source.involvement.filter(|i| *i != Involvement::Invisible)
        {
            existing.update_involvement(tx, involvement)?;
        }
        delete_membership_mapping(conn, source.id)?;
        conn.execute("DELETE FROM memberships WHERE id = ?", [source.id])?;
        return Ok(existing.id);
    }
    conn.execute(
        "UPDATE memberships SET user_id = ?, updated_at = ? WHERE id = ?",
        params![to, tx.revision_after(source.updated_at), source.id],
    )?;
    Ok(source.id)
}

fn delete_membership_mapping(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM slack_import_records WHERE slack_kind = 'membership'
         AND record_type = 'Membership' AND record_id = ?",
        [id],
    )?;
    Ok(())
}

fn rename_user_suffix(
    tx: &Tx<'_>,
    record_type: &str,
    delimiter: &str,
    from: i64,
    to: i64,
) -> Result<()> {
    let previous = format!("{delimiter}{from}");
    let next = format!("{delimiter}{to}");
    let (table, kind) = match record_type {
        "Membership" => ("memberships", "membership"),
        _ => ("thread_memberships", "thread_membership"),
    };
    tx.conn().execute(
        &format!(
            "UPDATE slack_import_records AS src
             SET slack_key = substr(src.slack_key, 1, length(src.slack_key) - length(?1)) || ?2,
                 updated_at = ?3
             WHERE src.slack_kind = ?4 AND src.record_type = ?5
               AND src.record_id IN (SELECT id FROM {table} WHERE user_id = ?6)
               AND substr(src.slack_key, -length(?1)) = ?1
               AND NOT EXISTS (
                 SELECT 1 FROM slack_import_records dst
                 WHERE dst.slack_workspace_id = src.slack_workspace_id AND dst.slack_kind = src.slack_kind
                   AND dst.slack_key = substr(src.slack_key, 1, length(src.slack_key) - length(?1)) || ?2)"
        ),
        params![previous, next, tx.now(), kind, record_type, to],
    )?;
    Ok(())
}

fn move_direct_membership(
    tx: &mut Tx<'_>,
    room: &Room,
    source: &Membership,
    to: i64,
) -> Result<()> {
    let mut members = room.user_ids(tx.conn())?;
    members.retain(|id| *id != source.user_id);
    members.push(to);
    members.sort_unstable();
    members.dedup();
    // A room-specific key identifies a group that shares a member set with another DM.
    let group_suffix = room.direct_member_key.as_deref().and_then(|key| {
        key.find('#')
            .or_else(|| key.find(":group"))
            .map(|start| &key[start..])
    });
    let named_small_group =
        members.len() <= 2 && room.name.as_deref().is_some_and(|n| !n.trim().is_empty());
    if group_suffix.is_none()
        && !named_small_group
        && let Some(existing) =
            Room::find_direct_for(tx.conn(), &members)?.filter(|r| r.id != room.id)
    {
        // Legacy data can have a stale key. Actual membership controls access to the history.
        let mut existing_members = existing.user_ids(tx.conn())?;
        existing_members.sort_unstable();
        if existing_members == members {
            return merge_direct_rooms(tx, room, &existing, source.user_id, to, members.len() == 1);
        }
    }

    move_membership(tx, source, to)?;
    if let Some(suffix) = group_suffix {
        let mut key = format!("{}{suffix}", Room::direct_member_key_for(&members));
        if exists(
            tx.conn(),
            "SELECT 1 FROM rooms WHERE direct_member_key = ? AND id != ? AND deleted_at IS NULL",
            params![key, room.id],
        )? {
            key = format!("{key}#{}", room.id);
        }
        tx.conn().execute(
            "UPDATE rooms SET direct_member_key = ? WHERE id = ?",
            params![key, room.id],
        )?;
    } else {
        room.refresh_direct_member_key(tx)?;
    }
    if Room::is_note_to_self_of(tx.conn(), room.id, to)? {
        scope_notes_mappings(tx, room.id, source.user_id, to)?;
    }
    broadcast_room_memberships(tx, room)
}

fn merge_direct_rooms(
    tx: &mut Tx<'_>,
    room: &Room,
    existing: &Room,
    from: i64,
    to: i64,
    notes: bool,
) -> Result<()> {
    let conn = tx.conn();
    for membership in room.memberships(conn)? {
        delete_membership_mapping(conn, membership.id)?;
        if membership.user_id != from {
            tx.emit_after_commit(Event::broadcast(&RoomRemovalBroadcast {
                user_id: membership.user_id,
                room_id: room.id,
                room_class: room.room_type.class_name().into(),
            }));
        }
    }
    for table in [
        "messages",
        "channel_threads",
        "events",
        "scheduled_messages",
        "board_sla_nudges",
    ] {
        conn.execute(
            &format!("UPDATE {table} SET room_id = ?2 WHERE room_id = ?1"),
            params![room.id, existing.id],
        )?;
    }
    // Pins are already unique by message ID across all rooms.
    let moved_pins = conn.execute(
        "UPDATE message_pins SET room_id = ?2 WHERE room_id = ?1",
        params![room.id, existing.id],
    )?;
    if moved_pins > 0 {
        let revision = existing
            .pins_changed_at
            .map_or_else(|| tx.now(), |at| tx.revision_after(at));
        conn.execute(
            "UPDATE rooms SET pins_changed_at = ? WHERE id = ?",
            params![revision, existing.id],
        )?;
    }
    conn.execute(
        "UPDATE events SET venue_room_id = ?2 WHERE venue_room_id = ?1",
        params![room.id, existing.id],
    )?;
    if notes {
        scope_notes_mappings(tx, room.id, from, to)?;
    }
    conn.execute(
        "UPDATE slack_import_records SET record_id = ?2, created_record = 0, updated_at = ?3
         WHERE slack_kind = 'conversation' AND record_type = 'Room' AND record_id = ?1",
        params![room.id, existing.id, tx.now()],
    )?;
    room.destroy_imported(tx)?;
    broadcast_room_memberships(tx, existing)
}

fn scope_notes_mappings(tx: &Tx<'_>, room_id: i64, from: i64, to: i64) -> Result<()> {
    let conn = tx.conn();
    let mappings = query_all(
        conn,
        "SELECT slack_workspace_id, slack_key FROM slack_import_records
         WHERE slack_kind = 'conversation' AND record_type = 'Room' AND record_id = ?",
        [room_id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    )?;
    let from_prefix = format!("notes:{from}:");
    let to_prefix = format!("notes:{to}:");
    for (workspace, old_key) in mappings {
        if old_key.starts_with(&to_prefix) {
            continue;
        }
        let conversation = old_key.strip_prefix(&from_prefix).unwrap_or(&old_key);
        let new_key = format!("{to_prefix}{conversation}");
        // Both notes rooms can contain the same Slack item. Keep the earlier mapping
        // canonical and give the moved record a distinct key so undo still owns it.
        conn.execute(
            "UPDATE slack_import_records AS src
             SET slack_key = CASE WHEN EXISTS (
                 SELECT 1 FROM slack_import_records dst
                 WHERE dst.slack_workspace_id = src.slack_workspace_id AND dst.slack_kind = src.slack_kind
                   AND dst.slack_key = ?1 || substr(src.slack_key, length(?2) + 1))
               THEN ?1 || ':claimed:' || src.id
               ELSE ?1 || substr(src.slack_key, length(?2) + 1) END,
               updated_at = ?3
             WHERE src.slack_workspace_id = ?4
               AND ((src.slack_kind = 'conversation' AND src.slack_key = ?2 AND src.record_id = ?5)
                 OR (src.slack_kind IN ('message', 'thread', 'pin', 'reaction', 'membership', 'thread_membership')
                   AND src.slack_key >= ?2 || ':' AND src.slack_key < ?2 || ';'))",
            params![new_key, old_key, tx.now(), workspace, room_id],
        )?;
    }
    Ok(())
}

fn broadcast_room_memberships(tx: &mut Tx<'_>, room: &Room) -> Result<()> {
    for membership in room.memberships(tx.conn())? {
        broadcast_membership(tx, membership.id);
    }
    Ok(())
}

fn broadcast_membership(tx: &mut Tx<'_>, membership_id: i64) {
    tx.emit_after_commit(Event::broadcast(&Broadcast::MembershipChanged {
        membership_id,
    }));
}

fn retarget_mentions(tx: &Tx<'_>, from: i64, to: i64) -> Result<()> {
    // Slack stores source Markdown by name, then renders it into Action Text user attachments.
    // Resolve the saved IDs rather than changing every occurrence of the Slack display name.
    let conn = tx.conn();
    let replacement = tx.rich_text().user_attachable_sgid(to);
    let mut cursor = 0;
    loop {
        let rows = query_all(
            conn,
            "SELECT id, record_id, body FROM action_text_rich_texts
             WHERE id > ? AND record_type = 'Message' AND instr(body, 'sgid=') > 0
             ORDER BY id LIMIT 250",
            [cursor],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        if rows.is_empty() {
            break;
        }
        for (id, message_id, html) in rows {
            cursor = id;
            let mut content =
                Content::wrap(&html).map_err(|error| Error::Other(error.to_string()))?;
            let mut changed = false;
            for node in attachment_nodes(&content.dom, content.root) {
                if unverified_attachable_user(content.dom.attr(node, "sgid"))
                    .ok()
                    .flatten()
                    .is_some_and(|gid| gid.id.parse::<i64>() == Ok(from))
                {
                    content.dom.set_attr(node, "sgid", &replacement);
                    changed = true;
                }
            }
            if changed {
                conn.execute(
                    "UPDATE action_text_rich_texts SET body = ? WHERE id = ?",
                    params![content.to_html(), id],
                )?;
                if let Some(message) = Message::find_by_id(conn, message_id)? {
                    message.update_in_index(tx)?;
                }
            }
        }
    }
    Ok(())
}
