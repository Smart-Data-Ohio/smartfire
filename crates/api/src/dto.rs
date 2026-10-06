//! The domain rows as `campfire_api_types` puts them on the wire. Everything here reads; nothing
//! writes.

use std::collections::{BTreeSet, HashMap};

use campfire_api_types as api;
use campfire_app::app::AppState;
use campfire_db::models::workspace_presence_lease::Presence as LeasePresence;
use campfire_db::{
    CachedStatements, Connection, Involvement, Membership, Message, MessagePin, Result, Role, Room,
    RoomType, StageRole, Status, Timestamp, User, UserStatusSettings, WorkspacePresenceLease,
};
use campfire_web::controllers::presenters::{self, Presenter, accounts, room_shell};
use rails_compat::Secrets;

/// A [`Timestamp`] as the wire carries it: RFC 3339 in UTC with milliseconds.
pub fn time(time: Timestamp) -> String {
    time.jiff().strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn present(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

pub fn user(settings: &UserStatusSettings, secrets: &Secrets, now: Timestamp) -> api::User {
    let user = &settings.user;
    let expired = settings
        .custom_status_expires_at
        .is_some_and(|until| until <= now);
    let (emoji, text) = (
        present(settings.custom_status_emoji.as_deref()),
        present(settings.custom_status_text.as_deref()),
    );
    let custom_status =
        (!expired && (emoji.is_some() || text.is_some())).then(|| api::CustomStatus {
            emoji,
            text,
            expires_at: settings.custom_status_expires_at.map(time),
        });
    api::User {
        id: user.id,
        name: user.name.clone(),
        role: match user.role {
            Role::Member => api::UserRole::Member,
            Role::Administrator => api::UserRole::Administrator,
            Role::Bot => api::UserRole::Bot,
        },
        status: match user.status {
            Status::Active => api::UserStatus::Active,
            Status::Deactivated => api::UserStatus::Deactivated,
            Status::Banned => api::UserStatus::Banned,
        },
        bio: user.bio.clone(),
        avatar_url: presenters::avatar_path(secrets, user),
        custom_status,
        created_at: time(user.created_at),
    }
}

/// The directory entries for `ids` that exist, in id order, once each.
pub fn users(
    conn: &Connection,
    secrets: &Secrets,
    ids: impl IntoIterator<Item = i64>,
    now: Timestamp,
) -> Result<Vec<api::User>> {
    let ids: Vec<i64> = ids
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let settings = UserStatusSettings::for_ids(conn, &ids)?;
    Ok(ids
        .iter()
        .filter_map(|id| settings.get(id))
        .map(|settings| user(settings, secrets, now))
        .collect())
}

pub fn room(room: &Room) -> api::Room {
    api::Room {
        id: room.id,
        kind: match room.room_type {
            RoomType::Open => api::RoomKind::Open,
            RoomType::Closed => api::RoomKind::Closed,
            RoomType::Direct => api::RoomKind::Direct,
            RoomType::Voice => api::RoomKind::Voice,
            RoomType::Stage => api::RoomKind::Stage,
            RoomType::Board => api::RoomKind::Board,
        },
        name: if room.direct() {
            None
        } else {
            room.name.clone()
        },
        icon_name: room.icon_name.clone(),
        creator_id: room.creator_id,
        created_at: time(room.created_at),
        updated_at: time(room.updated_at),
    }
}

pub fn membership(membership: &Membership) -> api::Membership {
    api::Membership {
        id: membership.id,
        room_id: membership.room_id,
        user_id: membership.user_id,
        involvement: match membership.involvement.unwrap_or(Involvement::Mentions) {
            Involvement::Invisible => api::Involvement::Invisible,
            Involvement::Nothing => api::Involvement::Nothing,
            Involvement::Muted => api::Involvement::Muted,
            Involvement::Mentions => api::Involvement::Mentions,
            Involvement::Everything => api::Involvement::Everything,
        },
        unread_at: membership.unread_at.map(time),
        last_read_message_id: membership.last_read_message_id,
        room_category_id: membership.room_category_id,
        favorite_position: membership.favorite_position,
        stage_role: membership.stage_role.map(|role| match role {
            StageRole::Listener => api::StageRole::Listener,
            StageRole::Speaker => api::StageRole::Speaker,
            StageRole::Host => api::StageRole::Host,
        }),
    }
}

/// Messages with their bodies rendered by one presenter.
pub fn messages(
    conn: &Connection,
    app: &AppState,
    messages: &[Message],
) -> Result<Vec<api::MessageDTO>> {
    let presenter = Presenter::new(conn, app, None);
    messages
        .iter()
        .map(|message| {
            Ok(api::MessageDTO {
                id: message.id,
                room_id: message.room_id,
                thread_id: message.thread_id,
                creator_id: message.creator_id,
                client_message_id: message.client_message_id.clone(),
                body_html: presenter.rendered_body_html(message)?,
                markdown_source: message.markdown_source.clone(),
                system_note: message.system_note,
                action: message.action,
                streaming: message.streaming,
                embeds_suppressed: message.embeds_suppressed,
                reply_to_message_id: message.reply_to_message_id,
                forwarded_from_message_id: message.forwarded_from_message_id,
                edited_at: message.edited_at.map(time),
                created_at: time(message.created_at),
                updated_at: time(message.updated_at),
            })
        })
        .collect()
}

pub fn message(conn: &Connection, app: &AppState, message: &Message) -> Result<api::MessageDTO> {
    Ok(messages(conn, app, std::slice::from_ref(message))?.remove(0))
}

/// The room's members as `(user id, name)`, oldest membership first (people whose row is gone
/// are skipped).
fn members(conn: &Connection, room_id: i64) -> Result<Vec<(i64, String)>> {
    let mut statement = conn.prepare_cached(
        r#"SELECT "users"."id", "users"."name" FROM "memberships" INNER JOIN "users" ON "users"."id" = "memberships"."user_id" WHERE "memberships"."room_id" = ? ORDER BY "memberships"."id""#,
    )?;
    let rows = statement.query_map([room_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// A direct room's other members, or just the viewer for a note-to-self.
fn direct_members(
    members: &[(i64, String)],
    viewer_id: i64,
    viewer_name: &str,
) -> Vec<(i64, String)> {
    let others: Vec<(i64, String)> = members
        .iter()
        .filter(|(id, _)| *id != viewer_id)
        .cloned()
        .collect();
    if others.is_empty() {
        vec![(viewer_id, viewer_name.to_string())]
    } else {
        others
    }
}

/// The viewer's unread `mention` activity items per room.
fn mention_counts(
    conn: &Connection,
    user_id: i64,
    room_id: Option<i64>,
) -> Result<HashMap<i64, i64>> {
    let sql = format!(
        r#"SELECT "messages"."room_id", COUNT(*) FROM "activity_items" INNER JOIN "messages" ON "messages"."id" = "activity_items"."source_id" WHERE "activity_items"."user_id" = ? AND "activity_items"."source_type" = 'Message' AND "activity_items"."event_type" = 'mention' AND "activity_items"."read_at" IS NULL{} GROUP BY "messages"."room_id""#,
        if room_id.is_some() {
            r#" AND "messages"."room_id" = ?"#
        } else {
            ""
        }
    );
    let mut statement = conn.prepare_cached(&sql)?;
    let values: Vec<i64> = std::iter::once(user_id).chain(room_id).collect();
    let rows = statement.query_map(rusqlite::params_from_iter(values), |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Whether the membership has a sidebar row (`memberships.visible`, of an alive room).
fn visible(room: &Room, membership: &Membership) -> bool {
    !room.deleted()
        && membership
            .involvement
            .is_some_and(|involvement| involvement != Involvement::Invisible)
}

fn sidebar_row_with(
    conn: &Connection,
    room: &Room,
    membership: &Membership,
    viewer_name: &str,
    members: Option<&[(i64, String)]>,
    mention_count: i64,
) -> Result<api::SidebarRow> {
    let (display_name, direct_member_ids) = match members {
        Some(members) => {
            let direct = direct_members(members, membership.user_id, viewer_name);
            let names: Vec<&str> = direct.iter().map(|(_, name)| name.as_str()).collect();
            (
                accounts::sidebar_direct_label_for_names(room.name.as_deref(), &names),
                direct.iter().map(|(id, _)| *id).collect(),
            )
        }
        None => (room.name.clone().unwrap_or_default(), Vec::new()),
    };
    let unread_count = room_shell::first_unread(conn, membership)?.map_or(0, |(_, count)| count);
    Ok(api::SidebarRow {
        room: self::room(room),
        membership: self::membership(membership),
        display_name,
        direct_member_ids,
        unread_count,
        mention_count,
    })
}

/// The membership's sidebar row, or `None` when the room isn't in the person's sidebar.
pub fn sidebar_row(
    conn: &Connection,
    room: &Room,
    membership: &Membership,
) -> Result<Option<api::SidebarRow>> {
    if !visible(room, membership) {
        return Ok(None);
    }
    let viewer = User::find(conn, membership.user_id)?;
    let members = if room.direct() {
        Some(members(conn, room.id)?)
    } else {
        None
    };
    let mentions = mention_counts(conn, membership.user_id, Some(room.id))?
        .get(&room.id)
        .copied()
        .unwrap_or(0);
    sidebar_row_with(
        conn,
        room,
        membership,
        &viewer.name,
        members.as_deref(),
        mentions,
    )
    .map(Some)
}

pub fn sidebar(
    conn: &Connection,
    secrets: &Secrets,
    viewer: &User,
    can_create_rooms: bool,
    now: Timestamp,
) -> Result<api::Sidebar> {
    let all = Membership::visible_with_ordered_room(conn, viewer.id)?;
    let mentions = mention_counts(conn, viewer.id, None)?;
    let mut user_ids = BTreeSet::new();
    let mut rows = Vec::with_capacity(all.len());
    for (membership, room) in &all {
        let members = if room.direct() {
            Some(members(conn, room.id)?)
        } else {
            None
        };
        let row = sidebar_row_with(
            conn,
            room,
            membership,
            &viewer.name,
            members.as_deref(),
            mentions.get(&room.id).copied().unwrap_or(0),
        )?;
        user_ids.extend(row.direct_member_ids.iter().copied());
        rows.push(row);
    }
    let placeholders: Vec<i64> = accounts::direct_placeholder_user_rows(conn, viewer)?
        .iter()
        .map(|user| user.id)
        .collect();
    user_ids.extend(placeholders.iter().copied());
    let categories = campfire_db::RoomCategory::ordered_for_user(conn, viewer.id)?
        .into_iter()
        .map(|category| api::RoomCategory {
            id: category.id,
            name: category.name,
            collapsed: category.collapsed,
            position: category.position,
        })
        .collect();
    Ok(api::Sidebar {
        rows,
        categories,
        users: users(conn, secrets, user_ids, now)?,
        direct_placeholder_user_ids: placeholders,
        can_create_rooms,
    })
}

pub fn room_detail(
    conn: &Connection,
    secrets: &Secrets,
    viewer: &User,
    room: &Room,
    membership: &Membership,
    now: Timestamp,
) -> Result<api::RoomDetail> {
    let members = members(conn, room.id)?;
    let member_count: i64 = conn.query_row_cached(
        r#"SELECT COUNT(*) FROM "memberships" WHERE "memberships"."room_id" = ?"#,
        [room.id],
        |row| row.get(0),
    )?;
    let direct_member_ids: Vec<i64> = if room.direct() {
        direct_members(&members, viewer.id, &viewer.name)
            .into_iter()
            .map(|(id, _)| id)
            .collect()
    } else {
        Vec::new()
    };
    let member_preview_ids: Vec<i64> = members.iter().take(5).map(|(id, _)| *id).collect();
    let unread =
        room_shell::first_unread(conn, membership)?.map(|(first_unread_message_id, count)| {
            api::UnreadDivider {
                first_unread_message_id,
                count,
            }
        });
    Ok(api::RoomDetail {
        room: self::room(room),
        membership: self::membership(membership),
        display_name: accounts::room_display_name(conn, room, viewer)?,
        member_count,
        pins_count: MessagePin::count_for_room(conn, room.id)?,
        users: users(
            conn,
            secrets,
            direct_member_ids.iter().chain(&member_preview_ids).copied(),
            now,
        )?,
        direct_member_ids,
        member_preview_ids,
        unread,
    })
}

pub fn me(
    conn: &Connection,
    secrets: &Secrets,
    viewer: &User,
    last_room_id: Option<i64>,
    now: Timestamp,
) -> Result<api::Me> {
    let settings = UserStatusSettings::for_ids(conn, &[viewer.id])?
        .remove(&viewer.id)
        .ok_or_else(|| campfire_db::Error::RecordNotFound("User".into()))?;
    let (tour_completed, voice_mode, push_to_talk_key): (bool, Option<String>, Option<String>) = conn.query_row_cached(
        r#"SELECT COALESCE("tour_completed_at", '') != '', "voice_mode", "push_to_talk_key" FROM "users" WHERE "users"."id" = ?"#,
        [viewer.id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let dnd = settings.manual_dnd_active(now);
    let quiet_hours = match (
        settings.quiet_hours_enabled,
        settings.quiet_hours_start_minute,
        settings.quiet_hours_end_minute,
    ) {
        (true, Some(start), Some(end)) => Some(api::QuietHours {
            start_minute: minute(start),
            end_minute: minute(end),
        }),
        _ => None,
    };
    Ok(api::Me {
        user: user(&settings, secrets, now),
        email_address: viewer.email_address.clone(),
        preferences: api::Preferences {
            theme: match settings.theme.as_str() {
                "light" => api::Theme::Light,
                "dark" => api::Theme::Dark,
                _ => api::Theme::System,
            },
            text_size: match settings.text_size.as_str() {
                "smaller" => api::TextSize::Smaller,
                "small" => api::TextSize::Small,
                "large" => api::TextSize::Large,
                "larger" => api::TextSize::Larger,
                _ => api::TextSize::Default,
            },
            time_zone: present(settings.time_zone.as_deref()),
            time_zone_explicit: settings.time_zone_explicit,
            tour_completed,
            voice_mode: match voice_mode.as_deref() {
                Some("push_to_talk") => api::VoiceMode::PushToTalk,
                _ => api::VoiceMode::VoiceActivity,
            },
            push_to_talk_key: present(push_to_talk_key.as_deref())
                .unwrap_or_else(|| "`".to_string()),
        },
        presence_setting: match settings.presence_setting.as_str() {
            "dnd" => api::PresenceSetting::Dnd,
            "invisible" => api::PresenceSetting::Invisible,
            _ => api::PresenceSetting::Auto,
        },
        do_not_disturb: api::DoNotDisturb {
            enabled: dnd,
            until: if dnd {
                settings.dnd_until.map(time)
            } else {
                None
            },
        },
        quiet_hours,
        out_of_office: settings
            .ooo_until_effective(now)
            .map(|until| api::OutOfOffice {
                until: time(until),
                note: present(settings.ooo_note.as_deref()),
                keep_notifications: settings.ooo_notify_enabled,
            }),
        last_room_id,
    })
}

fn minute(value: i64) -> u16 {
    u16::try_from(value.clamp(0, 24 * 60 - 1)).unwrap_or_default()
}

/// Presence for the active humans among `ids`, in id order (`users/presences#show`).
pub fn presences(conn: &Connection, ids: &[i64], now: Timestamp) -> Result<Vec<api::UserPresence>> {
    let mut people: Vec<UserStatusSettings> = UserStatusSettings::for_ids(conn, ids)?
        .into_values()
        .filter(UserStatusSettings::active_human)
        .collect();
    people.sort_by_key(|settings| settings.user.id);
    let ids: Vec<i64> = people.iter().map(|settings| settings.user.id).collect();
    let leases = WorkspacePresenceLease::presence_by_user_id(conn, &ids, now)?;
    Ok(people
        .iter()
        .map(|settings| api::UserPresence {
            user_id: settings.user.id,
            presence: match settings.effective_presence(
                leases
                    .get(&settings.user.id)
                    .copied()
                    .unwrap_or(LeasePresence::Offline),
            ) {
                LeasePresence::Online => api::Presence::Online,
                LeasePresence::Idle => api::Presence::Idle,
                LeasePresence::Dnd => api::Presence::Dnd,
                LeasePresence::Offline => api::Presence::Offline,
            },
            status_text: settings.status_text_display(now),
        })
        .collect())
}
