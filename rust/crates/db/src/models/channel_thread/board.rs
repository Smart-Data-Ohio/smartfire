//! Rails board listings and commit callbacks. Assignment automation is layered onto these
//! writes by the automation domain; work mutations use the merged WS11 agent APIs.
use super::*;
use crate::Involvement;
use crate::broadcasts::{TurboAction, TurboStream};

pub const BOARD_POSTS_PER_PAGE: i64 = 50;
pub const BOARD_POSTS_MAX_PAGE: i64 = 20;

/// Live owner labels/availability for one page. This is a read snapshot, not write authority.
pub struct WorkOwners {
    users: HashMap<i64, User>,
    available: HashMap<(i64, i64), bool>,
}
impl WorkOwners {
    pub fn owner(&self, post: &ChannelThread) -> Option<&User> {
        post.work_owner_id.and_then(|id| self.users.get(&id))
    }
    pub fn available(&self, post: &ChannelThread) -> bool {
        post.work_owner_id.is_some_and(|id| {
            self.available
                .get(&(post.room_id, id))
                .copied()
                .unwrap_or(false)
        })
    }
    fn load(conn: &Connection, pairs: &[(i64, i64)]) -> Result<Self> {
        if pairs.is_empty() {
            return Ok(Self {users: HashMap::new(), available: HashMap::new()});
        }
        let ids = pairs
            .iter()
            .map(|&(_, id)| id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let users = User::where_ids(conn, &ids)?
            .into_iter()
            .map(|user| (user.id, user))
            .collect::<HashMap<_, _>>();
        let room_ids = pairs
            .iter()
            .map(|&(room, _)| room)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let members = Membership::for_rooms(conn, &room_ids)?
            .into_iter()
            .map(|member| (member.room_id, member.user_id))
            .collect::<HashSet<_>>();
        let bot_ids = users
            .values()
            .filter(|user| user.is_active() && user.is_bot())
            .map(|user| user.id)
            .collect::<Vec<_>>();
        let agents = crate::Agent::for_users(conn, &bot_ids)?
            .into_iter()
            .map(|agent| (agent.user_id, agent.id))
            .collect::<HashMap<_, _>>();
        let requests = pairs
            .iter()
            .filter_map(|&(room, user)| agents.get(&user).map(|&agent| (agent, Some(room))))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let capabilities = crate::Agent::capabilities_for_rooms(conn, "post_messages", &requests)?;
        let available = pairs
            .iter()
            .map(|&(room, id)| {
                let active = users.get(&id).is_some_and(|user| {
                    user.is_active()
                        && members.contains(&(room, id))
                        && (!user.is_bot()
                            || agents.get(&id).is_some_and(|agent| {
                                capabilities
                                    .get(&(*agent, Some(room)))
                                    .copied()
                                    .unwrap_or(false)
                            }))
                });
                ((room, id), active)
            })
            .collect();
        Ok(Self { users, available })
    }
}

/// `[[params[:page].to_s.to_i, 1].max, BOARD_POSTS_MAX_PAGE].min`, including Ruby's
/// numeric prefix and inter-digit underscores. Saturation preserves the clamp for big ints.
pub fn board_page_number(value: &str) -> i64 {
    let mut text = value.trim_start_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']);
    let negative = text.starts_with('-');
    if text.starts_with(['-', '+']) {
        text = &text[1..];
    }
    let mut number = 0i64;
    let mut bytes = text.bytes().peekable();
    let mut digit_seen = false;
    while let Some(byte) = bytes.next() {
        if byte.is_ascii_digit() {
            number = number
                .saturating_mul(10)
                .saturating_add(i64::from(byte - b'0'));
            digit_seen = true;
        } else if byte != b'_' || !digit_seen || !bytes.peek().is_some_and(u8::is_ascii_digit) {
            break;
        }
    }
    if negative {
        1
    } else {
        number.clamp(1, BOARD_POSTS_MAX_PAGE)
    }
}

impl ChannelThread {
    /// `board_posts_for`: cumulative windows, one probe row, clock-independent status
    /// filtering, and owner/tag predicates on the full relation before applying the limit.
    pub fn board_posts_for(
        conn: &Connection,
        room_id: i64,
        status: &str,
        owner: &str,
        tag: &str,
        viewer_id: Option<i64>,
        page: i64,
    ) -> Result<Vec<Self>> {
        use rusqlite::types::Value;
        let mut sql = "SELECT channel_threads.* FROM channel_threads WHERE room_id=?".to_string();
        let mut values = vec![Value::Integer(room_id)];
        match status {
            "done" => sql.push_str(" AND work_status='done'"),
            "all" => (),
            _ => sql.push_str(" AND work_status != 'done'"),
        }
        if owner == "me" {
            match viewer_id {
                Some(id) => {
                    sql.push_str(" AND work_owner_id=?");
                    values.push(Value::Integer(id));
                }
                None => sql.push_str(" AND work_owner_id IS NULL"),
            }
        } else if owner == "agents" {
            // The Rails filter includes every Agent row, including unavailable owners.
            sql.push_str(" AND work_owner_id IN (SELECT user_id FROM agents)");
        } else if !owner.is_empty() && owner.bytes().all(|byte| byte.is_ascii_digit()) {
            sql.push_str(" AND work_owner_id=?");
            values.push(
                owner
                    .parse::<i64>()
                    .map(Value::Integer)
                    .unwrap_or(Value::Null),
            );
        }
        if !campfire_richtext::ruby::is_blank(tag) {
            sql.push_str(" AND id IN (SELECT channel_thread_id FROM thread_tags WHERE name=?)");
            values.push(Value::Text(rails_compat::unicode::downcase(
                campfire_richtext::ruby::strip(tag),
            )));
        }
        sql.push_str(" ORDER BY last_activity_at DESC, id DESC LIMIT ?");
        values.push(Value::Integer(
            page.clamp(1, BOARD_POSTS_MAX_PAGE) * BOARD_POSTS_PER_PAGE + 1,
        ));
        query_all(
            conn,
            &sql,
            rusqlite::params_from_iter(values),
            Self::from_row,
        )
    }

    pub fn board_tag_counts(conn: &Connection, room_id: i64) -> Result<Vec<(String, i64)>> {
        query_all(
            conn,
            "SELECT name, COUNT(*) FROM thread_tags WHERE channel_thread_id IN (SELECT id FROM channel_threads WHERE room_id=?) GROUP BY name ORDER BY name",
            [room_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    }

    pub fn board_has_posts(conn: &Connection, room_id: i64) -> Result<bool> {
        Ok(conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM channel_threads WHERE room_id=?)",
            [room_id],
            |row| row.get(0),
        )?)
    }

    pub fn board_reply_counts(conn: &Connection, ids: &[i64]) -> Result<HashMap<i64, i64>> {
        board_counts(conn, ids, "messages", "thread_id")
    }

    pub fn board_link_counts(conn: &Connection, ids: &[i64]) -> Result<HashMap<i64, i64>> {
        board_counts(conn, ids, "work_thread_links", "channel_thread_id")
    }

    /// Owner availability uses live memberships and WS11's public agent permission API.
    /// It does not restrict the `agents` filter, which selects by associated Agent existence.
    pub fn board_owner_active_map<'a>(
        conn: &Connection,
        room_id: i64,
        posts: impl IntoIterator<Item = &'a Self>,
    ) -> Result<HashMap<i64, bool>> {
        let pairs = posts
            .into_iter()
            .filter_map(|post| post.work_owner_id)
            .map(|id| (room_id, id))
            .collect::<Vec<_>>();
        Ok(WorkOwners::load(conn, &pairs)?
            .available
            .into_iter()
            .map(|((_, id), active)| (id, active))
            .collect())
    }

    /// WorkThreadsController/board includes, across every room and owner on the page.
    pub fn work_owners(conn: &Connection, posts: &[Self]) -> Result<WorkOwners> {
        WorkOwners::load(
            conn,
            &posts
                .iter()
                .filter_map(|post| post.work_owner_id.map(|owner| (post.room_id, owner)))
                .collect::<Vec<_>>(),
        )
    }
    pub fn work_status_label(&self) -> String {
        match self.work_status.as_deref() {
            Some("planned") => "Planned".into(),
            Some("in_progress") => "In progress".into(),
            Some("blocked") => "Blocked".into(),
            Some("done") => "Done".into(),
            Some(value) => {
                let label = value.replace('_', " ");
                let mut chars = label.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            }
            None => String::new(),
        }
    }

    pub fn broadcast_board_row_replace(&self, tx: &mut Tx<'_>) -> Result<()> {
        let room = self.room(tx.conn())?;
        for column in [false, true] {
            tx.emit_after_commit(Event::broadcast(&Broadcast::replace(
                room_messages(&room),
                self.board_row_id(column),
                Partial::BoardRow {
                    thread_id: self.id,
                    column,
                },
            )));
        }
        Ok(())
    }

    fn board_row_id(&self, column: bool) -> String {
        dom_id(
            "channel_thread",
            self.id,
            Some(if column {
                "board_column_row"
            } else {
                "board_row"
            }),
        )
    }

    fn prepend_board_column(&self, tx: &mut Tx<'_>, room: &Room) {
        tx.emit_after_commit(Event::broadcast(&board_prepend(
            room,
            &format!(
                "board_column_{}",
                self.work_status.as_deref().unwrap_or_default()
            ),
            self.id,
            true,
        )));
    }

    pub(super) fn register_board_creation(tx: &mut Tx<'_>, id: i64, room: &Room) {
        if !room.board() {
            return;
        }
        tx.after_commit_record_latest("board_post_creation", id, move |tx| {
            let Some(thread) = Self::find_by_id(tx.conn(), id)? else {
                return Ok(());
            };
            let room = thread.room(tx.conn())?;
            tx.emit_after_commit(Event::broadcast(&board_prepend(
                &room,
                "board_posts",
                id,
                false,
            )));
            thread.prepend_board_column(tx, &room);
            let recipients = Membership::for_room(tx.conn(), room.id)?
                .into_iter()
                .filter(|membership| {
                    membership.user_id != thread.creator_id
                        && membership.involvement.is_some_and(|level| {
                            !matches!(level, Involvement::Invisible | Involvement::Muted)
                        })
                        && membership
                            .connected_at
                            .is_none_or(|at| at < Membership::connection_cutoff(tx.now()))
                })
                .collect::<Vec<_>>();
            if !recipients.is_empty() {
                let ids = recipients
                    .iter()
                    .map(|membership| membership.id)
                    .collect::<Vec<_>>();
                let mut values = vec![
                    rusqlite::types::Value::Text(tx.now().to_db()),
                    rusqlite::types::Value::Text(tx.now().to_db()),
                ];
                values.extend(ids.into_iter().map(rusqlite::types::Value::Integer));
                tx.conn().execute(
                    &format!(
                        "UPDATE memberships SET unread_at=?, updated_at=? WHERE id IN ({})",
                        placeholders(recipients.len())
                    ),
                    rusqlite::params_from_iter(values),
                )?;
            }
            for membership in recipients {
                tx.emit_after_commit(Event::broadcast(&Broadcast::Cable {
                    stream: format!("user_{}_unread_rooms", membership.user_id),
                    payload: serde_json::json!({"roomId": room.id}),
                }));
            }
            Ok(())
        });
    }

    pub(super) fn register_board_update(
        &self,
        tx: &mut Tx<'_>,
        room: &Room,
        row_changed: bool,
        status_changed: bool,
    ) -> Result<()> {
        if !room.board() || tx.has_commit_record("board_post_creation", self.id) {
            return Ok(());
        }
        let id = self.id;
        tx.after_commit_record_latest("board_post_update", id, move |tx| {
            if !row_changed {
                return Ok(());
            }
            let Some(thread) = Self::find_by_id(tx.conn(), id)? else {
                return Ok(());
            };
            if status_changed {
                let room = thread.room(tx.conn())?;
                tx.emit_after_commit(Event::broadcast(&Broadcast::replace(
                    room_messages(&room),
                    thread.board_row_id(false),
                    Partial::BoardRow {
                        thread_id: id,
                        column: false,
                    },
                )));
                tx.emit_after_commit(Event::broadcast(&Broadcast::remove(
                    room_messages(&room),
                    thread.board_row_id(true),
                )));
                thread.prepend_board_column(tx, &room);
            } else {
                thread.broadcast_board_row_replace(tx)?;
            }
            Ok(())
        });
        Ok(())
    }

    pub(super) fn register_board_destruction(&self, tx: &mut Tx<'_>) -> Result<()> {
        let room = self.room(tx.conn())?;
        if !room.board() {
            return Ok(());
        }
        let id = self.id;
        tx.after_commit_record("channel_threads", id, move |tx| {
            for prefix in ["board_row", "board_column_row"] {
                tx.emit_after_commit(Event::broadcast(&Broadcast::remove(
                    room_messages(&room),
                    dom_id("channel_thread", id, Some(prefix)),
                )));
            }
            Ok(())
        });
        Ok(())
    }
}

fn board_prepend(room: &Room, target: &str, thread_id: i64, column: bool) -> Broadcast {
    Broadcast::Turbo(TurboStream {
        streamables: room_messages(room),
        action: TurboAction::Prepend,
        target: target.into(),
        partial: Some(Partial::BoardRow { thread_id, column }),
        maintain_scroll: false,
    })
}

fn board_counts(
    conn: &Connection,
    ids: &[i64],
    table: &str,
    key: &str,
) -> Result<HashMap<i64, i64>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(query_all(
        conn,
        &format!(
            "SELECT {key}, COUNT(*) FROM {table} WHERE {key} IN ({}) GROUP BY {key}",
            placeholders(ids.len())
        ),
        rusqlite::params_from_iter(ids),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?
    .into_iter()
    .collect())
}
