//! `app/models/rooms/direct.rb`. HTTP selection/access and huddle-specific rendering are WS8b/WS13.
use crate::broadcasts::{Broadcast, Partial, Streamable, TurboAction, room_dom_id, room_messages};
use crate::sql::exists;
use crate::{Connection, Errors, Event, Membership, Message, NewMessage, Result, Room, Tx, User};
use campfire_richtext::ruby::{is_blank, strip};
use rusqlite::params;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaveOutcome {
    Left,
    Destroyed,
}
pub const MAX_MEMBERS: usize = 10;
pub fn display_name(
    name: Option<&str>,
    for_user: Option<&User>,
    members: &[User],
) -> Option<String> {
    if let Some(name) = name.filter(|n| !is_blank(n)) {
        return Some(name.into());
    }
    let mut list: Vec<_> = members
        .iter()
        .filter(|u| for_user.is_none_or(|f| f.id != u.id))
        .collect();
    list.sort_by_key(|u| u.name.to_lowercase());
    match list.len() {
        0 => for_user.map(|u| u.name.clone()),
        1 => Some(list[0].name.clone()),
        n => {
            let names = list
                .iter()
                .take(3)
                .map(|u| {
                    u.name
                        .split([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
                        .find(|part| !part.is_empty())
                        .unwrap_or("")
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(if n > 3 {
                format!("{names} +{}", n - 3)
            } else {
                names
            })
        }
    }
}
fn sentence(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        2 => format!("{} and {}", names[0], names[1]),
        n => format!("{}, and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}
impl Room {
    pub fn direct_group_capable(&self, conn: &Connection) -> Result<bool> {
        let current = Room::find(conn, self.id)?;
        Ok(current.direct()
            && (current.user_ids(conn)?.len() > 2
                || current.name.as_deref().is_some_and(|s| !is_blank(s))))
    }
    pub fn direct_display_name(
        &self,
        conn: &Connection,
        for_user: Option<&User>,
        members: Option<&[User]>,
    ) -> Result<Option<String>> {
        match members {
            Some(members) => Ok(display_name(self.name.as_deref(), for_user, members)),
            None => Ok(display_name(
                self.name.as_deref(),
                for_user,
                &self.users(conn)?,
            )),
        }
    }
    pub fn rename_direct(&mut self, tx: &mut Tx<'_>, name: &str, actor: i64) -> Result<()> {
        self.reload(tx.conn())?;
        if !self.direct_group_capable(tx.conn())? {
            return Err(crate::Error::Other("NotAGroup".into()));
        }
        let clean = strip(name);
        self.update(tx, Some(Some(clean).filter(|s| !is_blank(s))), None)?;
        if !exists(
            tx.conn(),
            "SELECT 1 FROM messages WHERE room_id=? AND system_note=1 AND created_at>=?",
            params![self.id, tx.now().ago(jiff::SignedDuration::from_secs(60))],
        )? {
            let text = if is_blank(clean) {
                "cleared the group name".into()
            } else {
                format!("renamed the group to {clean}")
            };
            self.direct_note(tx, &text, actor)?;
        }
        self.direct_directory_updates(tx, &[])?;
        Ok(())
    }
    pub fn add_direct_members(
        &self,
        tx: &mut Tx<'_>,
        users: &[i64],
        actor: i64,
    ) -> Result<Vec<i64>> {
        if !self.direct_group_capable(tx.conn())? {
            return Err(crate::Error::Other("NotAGroup".into()));
        }
        let current = self.user_ids(tx.conn())?;
        let fresh: Vec<_> = users
            .iter()
            .copied()
            .filter(|id| !current.contains(id))
            .collect();
        if current.len() + fresh.len() > MAX_MEMBERS {
            return Err(crate::Error::Other("OverCapacity".into()));
        }
        if fresh.is_empty() {
            return Ok(fresh);
        }
        let names: Vec<_> = fresh
            .iter()
            .map(|id| User::find(tx.conn(), *id).map(|u| u.name))
            .collect::<Result<_>>()?;
        self.grant_to(tx, &fresh)?;
        self.direct_note(
            tx,
            &format!("added {} to the group", sentence(&names)),
            actor,
        )?;
        self.direct_directory_updates(tx, &fresh)?;
        Ok(fresh)
    }
    pub fn leave_direct(&self, tx: &mut Tx<'_>, user: i64) -> Result<LeaveOutcome> {
        let member = Membership::find_by_room_and_user(tx.conn(), self.id, user)?
            .ok_or_else(|| crate::Error::RecordNotFound("Membership"))?;
        member.destroy(tx)?;
        if self.user_ids(tx.conn())?.is_empty() {
            self.begin_destroy(tx)?;
            Ok(LeaveOutcome::Destroyed)
        } else {
            self.direct_note(tx, "left the group", user)?;
            self.direct_directory_updates(tx, &[])?;
            Ok(LeaveOutcome::Left)
        }
    }
    fn direct_note(&self, tx: &mut Tx<'_>, text: &str, actor: i64) -> Result<()> {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;");
        let message = Message::create(
            tx,
            NewMessage {
                room_id: self.id,
                creator_id: actor,
                body: Some(escaped),
                system_note: true,
                ..Default::default()
            },
        )?;
        tx.emit_after_commit(Event::Broadcast(Broadcast::append(
            room_messages(self),
            room_dom_id(self, Some("messages")),
            Partial::Message {
                message_id: message.id,
            },
        )));
        Ok(())
    }
    fn direct_directory_updates(&self, tx: &mut Tx<'_>, newcomers: &[i64]) -> Result<()> {
        let all = self.user_ids(tx.conn())?;
        for membership in self.memberships(tx.conn())? {
            let stream = vec![
                Streamable::User(membership.user_id),
                Streamable::Name("rooms"),
            ];
            let new = newcomers.contains(&membership.user_id);
            let mut event = Broadcast::replace(
                stream.clone(),
                if new {
                    "direct_rooms".into()
                } else {
                    room_dom_id(self, Some("list"))
                },
                Partial::DirectSidebar {
                    membership_id: membership.id,
                    member_ids: all
                        .iter()
                        .copied()
                        .filter(|id| *id != membership.user_id)
                        .collect(),
                },
            );
            if new && let Broadcast::Turbo(ref mut frame) = event {
                frame.action = TurboAction::Prepend;
            }
            tx.emit_after_commit(Event::Broadcast(event));
            tx.emit_after_commit(Event::Broadcast(Broadcast::replace(
                stream,
                room_dom_id(self, Some("header")),
                Partial::RoomHeader {
                    room_id: self.id,
                    for_user_id: membership.user_id,
                },
            )));
        }
        Ok(())
    }
}
pub(crate) fn validate_name(name: Option<&str>) -> Result<()> {
    let mut errors = Errors::default();
    if name.is_some_and(|name| name.chars().count() > 100) {
        errors.add("name", "is too long (maximum is 100 characters)");
    }
    errors.into_result()
}
