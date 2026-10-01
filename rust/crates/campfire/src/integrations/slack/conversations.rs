//! Slack channel/DM targets (`app/models/slack/conversation_mapper.rb`).
use super::runner::{conversation_type, present, string, truthy};
use super::users::{self, SLACKBOT_ID};
use campfire_db::models::slack::SlackConnection;
use campfire_db::models::slack_import::{IssueLevel, SlackImport};
use campfire_db::{Result, Room, RoomType, Tx, User};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub struct Target {
    pub action: &'static str,
    pub room: Option<Room>,
    pub reason: Option<&'static str>,
}
impl Target {
    fn skip(reason: &'static str) -> Self {
        Self {
            action: "skip",
            room: None,
            reason: Some(reason),
        }
    }
    fn room(room: Option<Room>, created: bool) -> Self {
        Self {
            action: if created { "create" } else { "merge" },
            room,
            reason: None,
        }
    }
}
#[derive(PartialEq)]
enum Choice {
    Auto,
    New,
    Skip,
    Room(i64),
}
fn choice(run: &SlackImport, id: &str) -> Choice {
    let value = &run.options["room_targets"][id];
    if value == "skip" {
        Choice::Skip
    } else if value == "new" {
        Choice::New
    } else if run.kind == "personal" {
        Choice::Auto
    } else {
        let text = if let Some(id) = value.as_i64() {
            id.to_string()
        } else {
            string(value)
        };
        if !text.is_empty()
            && text.bytes().all(|c| c.is_ascii_digit())
            && let Ok(id) = text.parse()
        {
            Choice::Room(id)
        } else {
            Choice::Auto
        }
    }
}
pub fn describe(conversation: &Value, count: usize) -> String {
    match conversation_type(conversation) {
        "im" => "Direct message".into(),
        "mpim" => {
            if count > 10 {
                present(&conversation["name"]).unwrap_or_else(|| "Group DM".into())
            } else {
                "Group DM".into()
            }
        }
        _ => channel_name(conversation),
    }
}
fn channel_name(c: &Value) -> String {
    let name = present(&c["name"]).unwrap_or_else(|| "slack-channel".into());
    if truthy(&c["is_archived"]) {
        format!("{name} (archived)")
    } else {
        name
    }
}
fn room_type(c: &Value) -> RoomType {
    if truthy(&c["is_private"]) {
        RoomType::Closed
    } else {
        RoomType::Open
    }
}
fn alive_channel(tx: &Tx<'_>, id: i64) -> Result<Option<Room>> {
    Ok(Room::find_by_id(tx.conn(), id)?.filter(|r| !r.deleted() && (r.open() || r.closed())))
}
fn merge_room(tx: &Tx<'_>, run: &SlackImport, c: &Value, target: &Choice) -> Result<Option<Room>> {
    if *target == Choice::New || run.kind == "personal" || truthy(&c["is_private"]) {
        return Ok(None);
    }
    let name = channel_name(c).to_lowercase();
    Ok(Room::of_type(tx.conn(), room_type(c))?
        .into_iter()
        .filter(|r| !r.deleted() && r.name.as_ref().is_some_and(|n| n.to_lowercase() == name))
        .min_by_key(|r| r.id))
}
fn mapped_members(member_ids: &[Value], users: &HashMap<String, User>) -> Vec<i64> {
    let mut result = Vec::new();
    for key in member_ids {
        if let Some(user) = users.get(&string(key))
            && !result.contains(&user.id)
        {
            result.push(user.id);
        }
    }
    result
}
fn record_memberships(
    tx: &Tx<'_>,
    run: &SlackImport,
    c: &Value,
    room: &Room,
    users: &HashMap<String, User>,
    member_ids: &[Value],
    channel: bool,
) -> Result<()> {
    let members: HashSet<_> = member_ids.iter().map(string).collect();
    let mut keys: Vec<_> = users.keys().collect();
    keys.sort();
    let slack_by_user: HashMap<_, _> = keys.into_iter().map(|key| (users[key].id, key)).collect();
    // Match Rails pluck(:id, :user_id): its covering room/user index orders the
    // mapping rows, including inactive Slack members appended after the active grant.
    let mut q = tx
        .conn()
        .prepare("SELECT id,user_id FROM memberships WHERE room_id=?")?;
    let memberships = q
        .query_map([room.id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(q);
    for (membership_id, user_id) in memberships {
        let key = if channel {
            slack_by_user
                .get(&user_id)
                .filter(|key| members.contains(**key))
                .map_or_else(
                    || format!("{}:user-{}", string(&c["id"]), user_id),
                    |key| format!("{}:{key}", string(&c["id"])),
                )
        } else {
            format!("{}:user-{}", string(&c["id"]), user_id)
        };
        // Rails insert_all ignores an existing unique mapping, unlike record_conversation.
        if users::mapped_id(tx.conn(), run.slack_workspace_id, "membership", &key)?.is_none() {
            users::record(
                tx,
                run,
                "membership",
                &key,
                "Membership",
                membership_id,
                true,
            )?;
        }
    }
    Ok(())
}
fn issue(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    c: &Value,
    level: IssueLevel,
    message: &str,
) -> Result<()> {
    SlackImport::record_issue(
        tx,
        run.id,
        level,
        Some(&format!("channel:{}", string(&c["id"]))),
        message,
    )?;
    Ok(())
}
fn target_room(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    c: &Value,
    id: i64,
    dry: bool,
) -> Result<Target> {
    let Some(room) = alive_channel(tx, id)? else {
        if !dry {
            issue(
                tx,
                run,
                c,
                IssueLevel::Error,
                &format!(
                    "Room target {id} for #{} is not an alive Open or Closed room; skipped",
                    string(&c["name"])
                ),
            )?;
        }
        return Ok(Target::skip("invalid room target"));
    };
    if !dry {
        users::record(
            tx,
            run,
            "conversation",
            &string(&c["id"]),
            "Room",
            room.id,
            false,
        )?;
    }
    Ok(Target::room(Some(room), false))
}
pub fn resolve(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    c: &Value,
    member_ids: &[Value],
    users: &HashMap<String, User>,
    dry: bool,
) -> Result<Target> {
    let target = choice(run, &string(&c["id"]));
    if target == Choice::Skip {
        return Ok(Target::skip(if dry {
            "skipped"
        } else {
            "skipped by room target"
        }));
    }
    match conversation_type(c) {
        "im" => {
            let owner = run
                .slack_connection_id
                .map(|id| SlackConnection::find(tx.conn(), id))
                .transpose()?
                .flatten();
            let owner_key = owner.map(|c| c.slack_user_id).unwrap_or_default();
            let mut peers: Vec<_> = member_ids
                .iter()
                .map(string)
                .filter(|id| *id != owner_key)
                .collect();
            if peers.is_empty()
                && let Some(peer) = present(&c["user"])
                && peer != owner_key
            {
                peers.push(peer);
            }
            if matches!(target, Choice::Room(_)) && !dry {
                issue(
                    tx,
                    run,
                    c,
                    IssueLevel::Error,
                    "Room targets only apply to channels; this DM keeps its own Direct room",
                )?;
            }
            if peers.is_empty() {
                return Ok(Target::skip("self DM"));
            }
            // Import checks mapping before Slackbot; preview checks Slackbot first.
            let peer_users: Vec<_> = peers.iter().filter_map(|id| users.get(id)).collect();
            if dry && peers.iter().any(|id| id == SLACKBOT_ID) {
                return Ok(Target::skip("Slackbot DM"));
            }
            if peer_users.is_empty() {
                return Ok(Target::skip(if dry {
                    "DM peer not found"
                } else {
                    "DM peer is not mapped"
                }));
            }
            if peers.iter().any(|id| id == SLACKBOT_ID) {
                return Ok(Target::skip("Slackbot DM"));
            }
            let mut members = vec![run.user_id];
            for user in peer_users {
                if !members.contains(&user.id) {
                    members.push(user.id);
                }
            }
            if members.len() == 1 {
                return Ok(Target::skip("self DM"));
            }
            direct(tx, run, c, &members, users, member_ids, dry)
        }
        "mpim" => {
            let members = mapped_members(member_ids, users);
            if members.len() < 2 {
                if !dry {
                    issue(
                        tx,
                        run,
                        c,
                        IssueLevel::Warning,
                        &format!(
                            "Group DM {} has fewer than 2 mapped members; skipped",
                            c["name"]
                                .as_str()
                                .unwrap_or_else(|| c["id"].as_str().unwrap_or(""))
                        ),
                    )?;
                }
                return Ok(Target::skip("too few members"));
            }
            if members.len() <= 10 {
                if matches!(target, Choice::Room(_)) && !dry {
                    issue(
                        tx,
                        run,
                        c,
                        IssueLevel::Error,
                        "Room targets only apply to channels; this group DM keeps its own room",
                    )?;
                }
                direct(tx, run, c, &members, users, member_ids, dry)
            } else if let Choice::Room(id) = target {
                target_room(tx, run, c, id, dry)
            } else {
                if dry {
                    return Ok(Target::room(None, true));
                }
                let mut names: Vec<_> = users
                    .values()
                    .map(|u| u.name.clone())
                    .filter(|n| !n.trim().is_empty())
                    .collect();
                names.sort_by_key(|n| n.to_lowercase());
                let name = if names.is_empty() {
                    present(&c["name"]).unwrap_or_else(|| "Group DM".into())
                } else if names.len() <= 4 {
                    names.join(", ")
                } else {
                    format!("{} +{}", names[..3].join(", "), names.len() - 3)
                };
                let room = Room::create(tx, RoomType::Closed, Some(&name), run.user_id)?;
                room.grant_to(tx, &members)?;
                record_memberships(tx, run, c, &room, users, member_ids, false)?;
                users::record(
                    tx,
                    run,
                    "conversation",
                    &string(&c["id"]),
                    "Room",
                    room.id,
                    true,
                )?;
                Ok(Target::room(Some(room), true))
            }
        }
        _ => {
            if let Choice::Room(id) = target {
                return target_room(tx, run, c, id, dry);
            }
            if let Some(room) = merge_room(tx, run, c, &target)? {
                if !dry {
                    users::record(
                        tx,
                        run,
                        "conversation",
                        &string(&c["id"]),
                        "Room",
                        room.id,
                        false,
                    )?;
                }
                return Ok(Target::room(Some(room), false));
            }
            if dry {
                return Ok(Target::room(None, true));
            }
            let room = Room::create(tx, room_type(c), Some(&channel_name(c)), run.user_id)?;
            let members = mapped_members(member_ids, users);
            let mut grants = if room.open() {
                User::active(tx.conn())?
                    .into_iter()
                    .map(|u| u.id)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            for member in &members {
                if !grants.contains(member) {
                    grants.push(*member);
                }
            }
            room.grant_to(tx, &grants)?;
            for membership in room.memberships(tx.conn())? {
                if truthy(&c["is_archived"])
                    || (room.open() && !members.contains(&membership.user_id))
                {
                    tx.conn().execute(
                        "UPDATE memberships SET involvement='invisible' WHERE id=?",
                        [membership.id],
                    )?;
                }
            }
            users::record(
                tx,
                run,
                "conversation",
                &string(&c["id"]),
                "Room",
                room.id,
                true,
            )?;
            record_memberships(tx, run, c, &room, users, member_ids, true)?;
            Ok(Target::room(Some(room), true))
        }
    }
}
fn direct(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    c: &Value,
    members: &[i64],
    users: &HashMap<String, User>,
    member_ids: &[Value],
    dry: bool,
) -> Result<Target> {
    let room = Room::find_direct_for(tx.conn(), members)?;
    if let Some(room) = room {
        if !dry {
            users::record(
                tx,
                run,
                "conversation",
                &string(&c["id"]),
                "Room",
                room.id,
                false,
            )?;
        }
        return Ok(Target::room(Some(room), false));
    }
    if dry {
        return Ok(Target::room(None, true));
    }
    let room = Room::find_or_create_direct_for(tx, members, run.user_id)?;
    users::record(
        tx,
        run,
        "conversation",
        &string(&c["id"]),
        "Room",
        room.id,
        true,
    )?;
    record_memberships(tx, run, c, &room, users, member_ids, false)?;
    Ok(Target::room(Some(room), true))
}
pub fn dry_users(
    tx: &Tx<'_>,
    run: &SlackImport,
    member_ids: &[Value],
) -> Result<HashMap<String, User>> {
    let owner = User::find(tx.conn(), run.user_id)?;
    Ok(member_ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let mut user = owner.clone();
            user.id = -(i as i64 + 1);
            user.name.clear();
            (string(id), user)
        })
        .collect())
}

#[cfg(test)]
mod tests;
