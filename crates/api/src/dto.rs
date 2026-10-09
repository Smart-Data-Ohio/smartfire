//! The domain rows as `campfire_api_types` puts them on the wire. Everything here reads; nothing
//! writes.

use std::collections::{BTreeSet, HashMap};

use campfire_api_types as api;
use campfire_app::app::AppState;
use campfire_db::models::workspace_presence_lease::Presence as LeasePresence;
use campfire_db::{
    CachedStatements, Connection, Involvement, Membership, Message, MessagePin, Result, Role, Room,
    RoomType, Snapshot, StageRole, Status, Timestamp, User, UserStatusSettings,
    WorkspacePresenceLease,
};
use campfire_web::controllers::presenters::{self, Presenter, accounts, room_shell};
use rails_compat::Secrets;

/// A [`Timestamp`] as the wire carries it: RFC 3339 in UTC with milliseconds.
pub fn time(time: Timestamp) -> String {
    time.to_wire()
}

/// A row version in UTC with exactly six fractional digits and `Z`, so string order is time order.
pub fn row_version(time: Timestamp) -> String {
    time.jiff().strftime("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

fn present(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

/// The per-person facts a [`User`] row doesn't hold, read for a batch at once: who uploaded a
/// picture, each agent's badge, and each bot's resolved icon.
#[derive(Default)]
pub struct UserExtras {
    avatars: BTreeSet<i64>,
    agents: HashMap<i64, api::AgentBadge>,
    icons: HashMap<i64, api::Icon>,
}

impl UserExtras {
    /// The extras for `users`.
    pub fn load<'a>(conn: &Connection, users: impl IntoIterator<Item = &'a User>) -> Result<Self> {
        let users: Vec<&User> = users.into_iter().collect();
        let ids: Vec<i64> = users.iter().map(|user| user.id).collect();
        let avatars = uploaded_avatars(conn, &ids)?;
        let bots: Vec<i64> = users
            .iter()
            .filter(|user| user.is_bot())
            .map(|user| user.id)
            .collect();
        let agents = agent_badges(conn, &bots)?;
        let icon_users: Vec<_> = users
            .iter()
            .filter(|user| {
                user.is_bot() && !avatars.contains(&user.id) && user.icon_name.is_some()
            })
            .collect();
        let custom_titles: HashMap<String, String> = if icon_users.is_empty() {
            HashMap::new()
        } else {
            conn.prepare("SELECT name,title FROM workspace_icons")?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?
        };
        let mut icons = HashMap::new();
        for user in icon_users {
            let name = user.icon_name.as_deref().unwrap();
            if let Some(icon) = avatar_icon(name, custom_titles.get(name).map(String::as_str)) {
                icons.insert(user.id, icon);
            }
        }
        Ok(Self {
            avatars,
            agents,
            icons,
        })
    }

    pub fn has_avatar(&self, user_id: i64) -> bool {
        self.avatars.contains(&user_id)
    }
}

/// The badge of each agent among the bot users `user_ids`, by user id.
pub fn agent_badges(conn: &Connection, user_ids: &[i64]) -> Result<HashMap<i64, api::AgentBadge>> {
    Ok(ids_query(
        conn,
        r#"SELECT "agents"."user_id", "agents"."id", "agents"."kind", "agents"."status", "agents"."suspended_at" IS NOT NULL FROM "agents" WHERE "agents"."user_id" IN ({})"#,
        user_ids,
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                api::AgentBadge {
                    agent_id: row.get(1)?,
                    kind: agent_kind(&row.get::<_, String>(2)?),
                    status: agent_status(&row.get::<_, String>(3)?),
                    suspended: row.get(4)?,
                },
            ))
        },
    )?
    .into_iter()
    .collect())
}

/// `agents.kind` on the wire (`AgentKind`: anything but `workspace` is personal).
pub fn agent_kind(kind: &str) -> api::AgentKind {
    match kind {
        "workspace" => api::AgentKind::Workspace,
        _ => api::AgentKind::Personal,
    }
}

/// `agents.status` on the wire. The model only stores `Agent::STATUSES`; anything else reads
/// as idle, as an unset status does.
pub fn agent_status(status: &str) -> api::AgentStatus {
    match status {
        "working" => api::AgentStatus::Working,
        "waiting" => api::AgentStatus::Waiting,
        "failed" => api::AgentStatus::Failed,
        _ => api::AgentStatus::Idle,
    }
}

/// `users.icon_name` resolved as the classic avatar does (`resolve_avatar_icon`): a brand logo,
/// else a workspace icon, else the built-in icon or emoji of that name.
fn avatar_icon(name: &str, custom_title: Option<&str>) -> Option<api::Icon> {
    use campfire_views::helpers::AvatarIcon;
    let builtin = campfire_views::messages::reactions::static_icon(name);
    let resolved = if matches!(builtin, Some(AvatarIcon::Image { brand: true, .. })) {
        builtin
    } else {
        custom_title
            .map(|title| AvatarIcon::Image {
                title: title.to_string(),
                url: format!("/icons/{name}"),
                brand: false,
            })
            .or(builtin)
    };
    Some(match resolved? {
        AvatarIcon::Emoji { title, character } => api::Icon {
            name: name.to_string(),
            title,
            kind: api::IconKind::Emoji,
            character: Some(character),
            image_url: None,
        },
        AvatarIcon::Image { title, url, brand } => api::Icon {
            name: name.to_string(),
            title,
            kind: if brand {
                api::IconKind::Brand
            } else {
                api::IconKind::Custom
            },
            character: None,
            image_url: Some(url),
        },
    })
}

/// A person as the wire carries them, with their [`UserExtras`].
pub fn user(
    settings: &UserStatusSettings,
    secrets: &Secrets,
    now: Timestamp,
    extras: &UserExtras,
) -> api::User {
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
        has_avatar: extras.has_avatar(user.id),
        custom_status,
        avatar_icon: extras.icons.get(&user.id).cloned(),
        agent: extras.agents.get(&user.id).cloned(),
        created_at: time(user.created_at),
        updated_at: row_version(user.updated_at),
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
    let found: Vec<&UserStatusSettings> = ids.iter().filter_map(|id| settings.get(id)).collect();
    let extras = UserExtras::load(conn, found.iter().map(|settings| &settings.user))?;
    Ok(found
        .into_iter()
        .map(|settings| user(settings, secrets, now, &extras))
        .collect())
}

/// Who among `ids` uploaded a picture (`has_one_attached :avatar`).
pub fn uploaded_avatars(conn: &Connection, ids: &[i64]) -> Result<BTreeSet<i64>> {
    Ok(ids_query(
        conn,
        r#"SELECT "active_storage_attachments"."record_id" FROM "active_storage_attachments" WHERE "active_storage_attachments"."record_type" = 'User' AND "active_storage_attachments"."name" = 'avatar' AND "active_storage_attachments"."record_id" IN ({})"#,
        ids,
        |row| row.get(0),
    )?
    .into_iter()
    .collect())
}

pub fn room_kind(kind: RoomType) -> api::RoomKind {
    match kind {
        RoomType::Open => api::RoomKind::Open,
        RoomType::Closed => api::RoomKind::Closed,
        RoomType::Direct => api::RoomKind::Direct,
        RoomType::Voice => api::RoomKind::Voice,
        RoomType::Stage => api::RoomKind::Stage,
        RoomType::Board => api::RoomKind::Board,
    }
}

/// The names a cross-room list's rows refer to, one per distinct `(room, thread)` pair, as the
/// classic pages name them: `Room::display_names_for` for the room and the thread's name. A
/// room or thread that no longer exists is left out.
pub fn conversation_names(
    conn: &Connection,
    viewer: &User,
    pairs: impl IntoIterator<Item = (i64, Option<i64>)>,
) -> Result<Vec<api::ConversationName>> {
    let pairs = pairs.into_iter().collect::<BTreeSet<_>>();
    let mut rooms = Vec::new();
    for room_id in pairs
        .iter()
        .map(|(room_id, _)| *room_id)
        .collect::<BTreeSet<_>>()
    {
        rooms.extend(Room::find_by_id(conn, room_id)?);
    }
    let names = Room::display_names_for(conn, &rooms, Some(viewer))?;
    let rooms = rooms
        .into_iter()
        .map(|room| (room.id, room))
        .collect::<HashMap<_, _>>();
    let mut out = Vec::with_capacity(pairs.len());
    for (room_id, thread_id) in pairs {
        let Some(room) = rooms.get(&room_id) else {
            continue;
        };
        let thread_name = match thread_id {
            Some(id) => match campfire_db::ChannelThread::find_by_id(conn, id)? {
                Some(thread) if thread.room_id == room_id => Some(thread.name),
                _ => continue,
            },
            None => None,
        };
        out.push(api::ConversationName {
            room_id,
            thread_id,
            room_kind: room_kind(room.room_type),
            room_name: names.get(&room_id).cloned().unwrap_or_default(),
            room_icon_name: room.icon_name.clone(),
            thread_name,
        });
    }
    Ok(out)
}

pub fn room(room: &Room) -> api::Room {
    api::Room {
        id: room.id,
        kind: room_kind(room.room_type),
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

/// Messages with their bodies rendered by one presenter, and what the timeline shows with them:
/// the attachment (`Presenter#attachment` and its blob row), reactions and boosts
/// (`message.boosts.ordered`, grouped as `MessageView#reaction_groups` does), the pin, the
/// forward, the thread indicator, the poll and the cards.
pub fn messages(
    conn: &Connection,
    app: &AppState,
    messages: &[Message],
) -> Result<Vec<api::MessageDTO>> {
    Ok(messages_and_fetches(conn, app, messages)?.0)
}

/// [`messages`], with the card fetches the read asks for: a page of messages requests them, as
/// the classic timeline does.
pub(crate) fn messages_and_fetches(
    conn: &Connection,
    app: &AppState,
    messages: &[Message],
) -> Result<(Vec<api::MessageDTO>, crate::cards::Fetches)> {
    // When the read began: orders the polls and cards against `poll.updated` and
    // `message.cards` (`Poll::as_of`, `MessageDTO::cards_as_of`).
    let now = app.db.env().now();
    let as_of = time(now);
    let presenter = Presenter::new(conn, app, None);
    let ids: Vec<i64> = messages.iter().map(|message| message.id).collect();
    let mut polls = crate::cards::polls(conn, &ids, now)?;
    let mut fetches = crate::cards::Fetches::default();
    let mut cards = crate::cards::cards(&presenter, conn, messages, now, &mut fetches)?;
    let pinned: BTreeSet<i64> = ids_query(
        conn,
        r#"SELECT "message_pins"."message_id" FROM "message_pins" WHERE "message_pins"."message_id" IN ({})"#,
        &ids,
        |row| row.get(0),
    )?
    .into_iter()
    .collect();
    let blobs: HashMap<i64, (Option<String>, i64)> = ids_query(
        conn,
        r#"SELECT "active_storage_attachments"."record_id", "active_storage_blobs"."content_type", "active_storage_blobs"."byte_size" FROM "active_storage_attachments" INNER JOIN "active_storage_blobs" ON "active_storage_blobs"."id" = "active_storage_attachments"."blob_id" WHERE "active_storage_attachments"."record_type" = 'Message' AND "active_storage_attachments"."name" = 'attachment' AND "active_storage_attachments"."record_id" IN ({})"#,
        &ids,
        |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))),
    )?
    .into_iter()
    .collect();
    let threads = thread_indicators(conn, messages)?;
    let mut steps = message_steps(conn, &ids)?;
    messages
        .iter()
        .map(|message| {
            let attachment = match blobs.get(&message.id) {
                Some((content_type, byte_size)) => presenter
                    .attachment(message)?
                    .map(|view| attachment(view, content_type.as_deref(), *byte_size)),
                None => None,
            };
            let (reactions, boosts) = reactions_and_boosts(conn, &presenter, message.id)?;
            Ok(api::MessageDTO {
                id: message.id,
                room_id: message.room_id,
                thread_id: message.thread_id,
                creator_id: message.creator_id,
                client_message_id: message.client_message_id.clone(),
                body_html: inline_mentions(&presenter.rendered_body_html(message)?),
                markdown_source: message.markdown_source.clone(),
                system_note: message.system_note,
                action: message.action,
                streaming: message.streaming,
                embeds_suppressed: message.embeds_suppressed,
                reply_to_message_id: message.reply_to_message_id,
                forwarded_from_message_id: message.forwarded_from_message_id,
                forwarded_at: message.forwarded_at.map(time),
                forward_note: present(message.forward_note.as_deref()),
                edited_at: message.edited_at.map(time),
                attachment,
                reactions,
                boosts,
                pinned: pinned.contains(&message.id),
                thread: threads.get(&message.id).cloned(),
                poll: polls.remove(&message.id),
                cards: cards.remove(&message.id).unwrap_or_default(),
                cards_as_of: as_of.clone(),
                steps: steps.remove(&message.id).unwrap_or_default(),
                created_at: time(message.created_at),
                updated_at: time(message.updated_at),
            })
        })
        .collect::<Result<Vec<_>>>()
        .map(|dtos| (dtos, fetches))
}

/// `agent_steps`' columns as [`agent_step`] reads them.
const STEP_COLUMNS: &str = r#""agent_steps"."id", "agent_steps"."message_id", "agent_steps"."channel_thread_id", "agent_steps"."name", "agent_steps"."status", "agent_steps"."input_summary", "agent_steps"."output_summary", "agent_steps"."duration_ms", "agent_steps"."position", "agent_steps"."created_at", "agent_steps"."updated_at""#;

/// One `agent_steps` row (selected as [`STEP_COLUMNS`]) as the wire carries it (`AgentStep#payload`).
fn agent_step(row: &rusqlite::Row<'_>) -> rusqlite::Result<api::AgentStep> {
    Ok(api::AgentStep {
        id: row.get(0)?,
        message_id: row.get(1)?,
        thread_id: row.get(2)?,
        name: row.get(3)?,
        status: match row.get::<_, String>(4)?.as_str() {
            "running" => api::AgentStepStatus::Running,
            "done" => api::AgentStepStatus::Done,
            "failed" => api::AgentStepStatus::Failed,
            _ => api::AgentStepStatus::Pending,
        },
        input_summary: row.get(5)?,
        output_summary: row.get(6)?,
        duration_ms: row.get(7)?,
        position: row.get(8)?,
        created_at: time(row.get(9)?),
        updated_at: row_version(row.get(10)?),
    })
}

/// Each message's agent steps, in `(position, id)` order (`agent_steps/_steps.html`).
pub fn message_steps(conn: &Connection, ids: &[i64]) -> Result<HashMap<i64, Vec<api::AgentStep>>> {
    let mut steps: HashMap<i64, Vec<api::AgentStep>> = HashMap::new();
    for step in ids_query(
        conn,
        &format!(
            r#"SELECT {STEP_COLUMNS} FROM "agent_steps" WHERE "agent_steps"."message_id" IN ({{}}) ORDER BY "agent_steps"."position", "agent_steps"."id""#
        ),
        ids,
        agent_step,
    )? {
        if let Some(message_id) = step.message_id {
            steps.entry(message_id).or_default().push(step);
        }
    }
    Ok(steps)
}

/// A work thread's agent steps, in `(position, id)` order (`agent_steps_channel_thread_<id>`).
pub fn thread_steps(conn: &Connection, thread_id: i64) -> Result<Vec<api::AgentStep>> {
    ids_query(
        conn,
        &format!(
            r#"SELECT {STEP_COLUMNS} FROM "agent_steps" WHERE "agent_steps"."channel_thread_id" IN ({{}}) ORDER BY "agent_steps"."position", "agent_steps"."id""#
        ),
        &[thread_id],
        agent_step,
    )
}

/// Runs `sql`, whose one `IN ({})` takes `ids`; nothing for no ids.
pub(crate) fn ids_query<T>(
    conn: &Connection,
    sql: &str,
    ids: &[i64],
    row: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = sql.replace("{}", &vec!["?"; ids.len()].join(", "));
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(ids), row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// `AttachmentView` with its blob's type and size.
fn attachment(
    view: campfire_views::messages::AttachmentView,
    content_type: Option<&str>,
    byte_size: i64,
) -> api::Attachment {
    use campfire_views::messages::AttachmentPreview;
    use campfire_views::messages::support::RubyNumber;
    let pixels = |number: Option<RubyNumber>| {
        number.map(|number| match number {
            RubyNumber::Int(value) => value,
            RubyNumber::Float(value) => value.round() as i64,
        })
    };
    let (preview, thumbnail_url) = match view.preview {
        AttachmentPreview::Image { thumb_url } => (api::AttachmentPreview::Image, Some(thumb_url)),
        AttachmentPreview::Video { poster_url } => (api::AttachmentPreview::Video, poster_url),
        AttachmentPreview::File => (api::AttachmentPreview::File, None),
    };
    api::Attachment {
        filename: view.filename,
        content_type: present(content_type)
            .unwrap_or_else(|| "application/octet-stream".to_string()),
        byte_size,
        width: pixels(view.width),
        height: pixels(view.height),
        preview,
        url: view.blob_path,
        download_url: view.download_path,
        thumbnail_url,
    }
}

/// `message.boosts.ordered`: the reactions grouped by content, in order of first reaction, with
/// each group's distinct reactors (`MessageView#reaction_groups`), and the free-text boosts.
fn reactions_and_boosts(
    conn: &Connection,
    presenter: &Presenter<'_>,
    message_id: i64,
) -> Result<(Vec<api::Reaction>, Vec<api::Boost>)> {
    use campfire_views::helpers::AvatarIcon;
    let (mut reactions, mut boosts) = (Vec::<api::Reaction>::new(), Vec::new());
    for boost in campfire_db::Boost::for_message_ordered(conn, message_id)? {
        let Some(reaction) = campfire_views::messages::reactions::resolve(&boost.content, presenter)
        else {
            boosts.push(api::Boost {
                id: boost.id,
                booster_id: boost.booster_id,
                content: boost.content,
                created_at: time(boost.created_at),
            });
            continue;
        };
        match reactions.iter_mut().find(|group| group.content == boost.content) {
            Some(group) => {
                if !group.reactor_ids.contains(&boost.booster_id) {
                    group.reactor_ids.push(boost.booster_id);
                }
            }
            None => reactions.push(api::Reaction {
                content: boost.content,
                title: sentence_case(&reaction.title),
                image_url: match reaction.icon {
                    Some(AvatarIcon::Image { url, .. }) => Some(url),
                    _ => None,
                },
                reactor_ids: vec![boost.booster_id],
            }),
        }
    }
    Ok((reactions, boosts))
}

/// A reaction's tooltip name read alike for every kind: a quick reaction's label ("Thumbs up"),
/// an emoji's stored name ("thumbs_up") or an icon's title all start with a capital, with
/// spaces for underscores.
fn sentence_case(title: &str) -> String {
    let spaced = title.replace('_', " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => spaced,
    }
}

/// The body with each mention's `<div class="mention …">` wrapper as a `<span>`: inside a `<p>`,
/// an HTML parser closes the paragraph at a `<div>`, splitting the sentence. Only the JSON copy
/// changes; the classic pages render the body as before.
pub(crate) fn inline_mentions(html: &str) -> String {
    const OPEN: &str = r#"<div class="mention"#;
    if !html.contains(OPEN) {
        return html.to_owned();
    }
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find(OPEN) {
        out.push_str(&rest[..start]);
        let wrapper = &rest[start..];
        match matching_close(wrapper) {
            Some(close) => {
                // `<div` → `<span`, and its own `</div>` → `</span>`; anything nested stays.
                out.push_str("<span");
                out.push_str(&wrapper["<div".len()..close]);
                out.push_str("</span>");
                rest = &wrapper[close + "</div>".len()..];
            }
            None => {
                // Unbalanced: leave the rest as it is.
                out.push_str(wrapper);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// The offset of the `</div>` that closes the `<div` at the start of `html`, counting the
/// `<div>`s nested inside it.
fn matching_close(html: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut at = 0;
    while at < html.len() {
        let tail = &html[at..];
        if tail.starts_with("</div>") {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(at);
            }
            at += "</div>".len();
        } else if tail.starts_with("<div") && tail[4..].starts_with(|c: char| c == '>' || c.is_whitespace()) {
            depth += 1;
            at += "<div".len();
        } else {
            at += tail.chars().next().map_or(1, char::len_utf8);
        }
    }
    None
}

/// The message's reactions and boosts after a change (`MessageReactions`).
pub fn message_reactions(
    conn: &Connection,
    app: &AppState,
    message: &Message,
) -> Result<api::MessageReactions> {
    let presenter = Presenter::new(conn, app, None);
    let (reactions, boosts) = reactions_and_boosts(conn, &presenter, message.id)?;
    Ok(api::MessageReactions {
        message_id: message.id,
        room_id: message.room_id,
        thread_id: message.thread_id,
        reactions,
        boosts,
        updated_at: time(message.updated_at),
    })
}

/// The pin state after a pin or unpin (`PinState`).
pub fn pin_state(conn: &Connection, message: &Message) -> Result<api::PinState> {
    Ok(api::PinState {
        message_id: message.id,
        room_id: message.room_id,
        pinned: MessagePin::pinned(conn, message.id)?,
        pin_count: MessagePin::count_for_room(conn, message.room_id)?,
    })
}

/// `GET /api/v1/rooms/:id/pins`: newest first, at most `MessagePin::MAX_PER_ROOM`, with the
/// messages and the people.
pub fn pin_list(
    conn: &Connection,
    app: &AppState,
    room_id: i64,
    now: Timestamp,
) -> Result<api::PinList> {
    let mut statement = conn.prepare_cached(
        r#"SELECT "message_pins"."message_id", "message_pins"."pinner_id", "message_pins"."created_at" FROM "message_pins" WHERE "message_pins"."room_id" = ? ORDER BY "message_pins"."created_at" DESC, "message_pins"."id" DESC LIMIT ?"#,
    )?;
    let pins: Vec<(i64, i64, Timestamp)> = statement
        .query_map(
            rusqlite::params![room_id, campfire_db::message_pin::MAX_PER_ROOM],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?
        .collect::<rusqlite::Result<_>>()?;
    let mut pinned = Vec::with_capacity(pins.len());
    for (message_id, _, _) in &pins {
        if let Some(message) = Message::find_by_id(conn, *message_id)? {
            pinned.push(message);
        }
    }
    let messages = messages(conn, app, &pinned)?;
    // A pin whose message is gone (or doesn't render) is left out, so every pin has its message.
    let listed: BTreeSet<i64> = messages.iter().map(|message| message.id).collect();
    let pins: Vec<_> = pins
        .into_iter()
        .filter(|(message_id, _, _)| listed.contains(message_id))
        .collect();
    let people: Vec<i64> = pins
        .iter()
        .map(|(_, pinner, _)| *pinner)
        .chain(pinned.iter().map(|message| message.creator_id))
        .collect();
    Ok(api::PinList {
        pins: pins
            .into_iter()
            .map(|(message_id, pinner_id, pinned_at)| api::Pin {
                message_id,
                pinner_id,
                pinned_at: time(pinned_at),
            })
            .collect(),
        messages,
        users: users(conn, &app.secrets, people, now)?,
    })
}

/// One of the viewer's `saved_items`.
pub fn saved_item(item: &campfire_db::SavedItem) -> api::SavedItem {
    api::SavedItem {
        id: item.id,
        message_id: item.message_id,
        status: if item.status == "done" {
            api::SavedStatus::Done
        } else {
            api::SavedStatus::InProgress
        },
        remind_at: item.remind_at.map(time),
        reminded_at: item.reminded_at.map(time),
        created_at: time(item.created_at),
    }
}

/// The reply indicators of the root messages that started a thread: `channel_threads`'
/// `messages_count` and `last_activity_at`, and the latest three distinct repliers (replies as
/// `messages_count` counts them: no system notes, nothing still streaming).
fn thread_indicators(
    conn: &Connection,
    messages: &[Message],
) -> Result<HashMap<i64, api::ThreadIndicator>> {
    let roots: Vec<i64> = messages
        .iter()
        .filter(|message| message.thread_id.is_none())
        .map(|message| message.id)
        .collect();
    let threads: Vec<(i64, i64, i64, Timestamp)> = ids_query(
        conn,
        r#"SELECT "channel_threads"."parent_message_id", "channel_threads"."id", "channel_threads"."messages_count", "channel_threads"."last_activity_at" FROM "channel_threads" WHERE "channel_threads"."parent_message_id" IN ({})"#,
        &roots,
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let mut repliers = conn.prepare_cached(
        r#"SELECT "messages"."creator_id" FROM "messages" WHERE "messages"."thread_id" = ? AND "messages"."system_note" = 0 AND "messages"."streaming" = 0 GROUP BY "messages"."creator_id" ORDER BY MAX("messages"."created_at") DESC, MAX("messages"."id") DESC LIMIT 3"#,
    )?;
    let mut indicators = HashMap::new();
    for (parent_id, thread_id, reply_count, last_activity_at) in threads {
        let replier_ids = repliers
            .query_map([thread_id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        indicators.insert(
            parent_id,
            api::ThreadIndicator {
                thread_id,
                reply_count,
                last_reply_at: time(last_activity_at),
                replier_ids,
            },
        );
    }
    Ok(indicators)
}

/// The viewer's `saved_items` among `messages`, in page order.
pub fn saved(conn: &Connection, viewer_id: i64, messages: &[Message]) -> Result<Vec<api::SavedMark>> {
    if messages.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        r#"SELECT "saved_items"."message_id", "saved_items"."id" FROM "saved_items" WHERE "saved_items"."user_id" = ? AND "saved_items"."message_id" IN ({})"#,
        vec!["?"; messages.len()].join(", ")
    );
    let values: Vec<i64> = std::iter::once(viewer_id).chain(messages.iter().map(|message| message.id)).collect();
    let mut statement = conn.prepare(&sql)?;
    let mut marks: HashMap<i64, i64> = statement
        .query_map(rusqlite::params_from_iter(&values), |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(messages
        .iter()
        .filter_map(|message| {
            marks.remove(&message.id).map(|saved_item_id| api::SavedMark { message_id: message.id, saved_item_id })
        })
        .collect())
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

/// A room's counts for the viewer, from [`notification_counts`].
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct RoomCounts {
    /// Root messages in the room's unread range (`unreadCount`), as `room_shell::first_unread`
    /// counts them.
    pub(crate) unread: i64,
    /// Unread `mention` inbox items about messages in the room (`mentionCount`).
    pub(crate) mentions: i64,
    /// The red pill (`notificationCount`).
    pub(crate) notifications: i64,
    /// The part of `notifications` in threads (`threadNotificationCount`).
    pub(crate) thread_notifications: i64,
}

/// Inside the room's unread range, as `room_shell::first_unread` draws it, for a message `m`
/// against its membership's bounds `b`: the room is unread, and the message follows the last read
/// root message (or the moment it went unread, when there's no read position to go by). The
/// leading `created_at` bound follows from each case and lets SQLite seek
/// `index_messages_on_room_thread_created` instead of walking the room's whole timeline.
const IN_UNREAD_RANGE: &str = r#""b"."unread_at" IS NOT NULL AND "m"."created_at" >= COALESCE("b"."read_created_at", "b"."unread_at") AND CASE WHEN "b"."read_id" IS NULL THEN "m"."created_at" >= "b"."unread_at" WHEN "b"."read_created_at" IS NOT NULL THEN ("m"."created_at", "m"."id") > ("b"."read_created_at", "b"."read_id") ELSE ("m"."created_at", "m"."id") > ("b"."unread_at", "b"."read_id") END"#;

/// Each room's [`RoomCounts`] for the viewer, in one statement (so one read snapshot) over all
/// their rooms, or only `room_id`. Rooms with nothing to count are left out.
///
/// `unread` is every root message in the room's unread range, as the timeline's divider counts.
///
/// The red number is what classic would have pushed to the viewer and is still unread:
/// `Notifications::Policy#push` (`crates/db/src/models/notification_policy.rs`), leaving out
/// quiet hours and do-not-disturb, which only hold delivery back.
///
/// - root messages count while they're in the room's unread range, once per message, so reading
///   the room clears them: in an `everything` room every one but system notes and the viewer's
///   own; in a `mentions` room those with an unread mention or reply inbox item; in a `muted`
///   room those with an unread mention;
/// - in threads the unit is the unread inbox item, while the thread is unread for the viewer
///   (`thread_memberships.unread_at`) and posted after they last read it
///   (`thread_memberships.last_read_message_id`, so a reply that makes the thread unread again
///   doesn't bring back pings already read): classic reads a thread apart from its room, so
///   reading the room leaves them and reading the thread clears them. Keyword alerts and `nothing`
///   rooms aside, the recorder writes a thread's items exactly where the policy pushes (a
///   mention when the viewer follows the thread, every reply when they follow it with
///   `everything` in an `everything` or `mentions` room). Thread activity is grouped, one item per followed thread that moves to the
///   newest reply (`ActivityItem` recorder), so any number of ordinary replies in a thread
///   count 1; a mention or reply to you there is its own item and counts on its own. In a
///   `muted` room only mentions push;
/// - keyword alerts alone never push, so their items don't count; `nothing` and `invisible`
///   rooms never push, so they count none.
///
/// Reading the inbox item clears a ping too, since a ping counts only while it's unread both in
/// the inbox and where it was posted.
pub(crate) fn notification_counts(
    conn: &Connection,
    user_id: i64,
    room_id: Option<i64>,
) -> Result<HashMap<i64, RoomCounts>> {
    let mut statement = conn.prepare_cached(&notification_counts_sql(room_id.is_some()))?;
    let map = |row: &rusqlite::Row<'_>| {
        Ok((
            row.get::<_, i64>(0)?,
            RoomCounts {
                unread: row.get(1)?,
                mentions: row.get(2)?,
                notifications: row.get(3)?,
                thread_notifications: row.get(4)?,
            },
        ))
    };
    let rows = match room_id {
        Some(room_id) => statement.query_map(rusqlite::params![user_id, room_id], map)?,
        None => statement.query_map(rusqlite::params![user_id], map)?,
    };
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The counts statement: `?1` is the viewer, `?2` the one room when `one_room`. `roots` and
/// `pinged` are materialized, each read once through its own index (the room's unread range on
/// `index_messages_on_room_thread_created`, the viewer's unread pings on
/// `index_activity_items_on_unread_message_pings`); inlined, SQLite would walk the room's whole
/// history by `room_id` to find them.
fn notification_counts_sql(one_room: bool) -> String {
    let only = |column: &str| match one_room {
        true => format!(r#" AND {column} = ?2"#),
        false => String::new(),
    };
    format!(
        r#"WITH "bounds" AS (SELECT "ms"."room_id", "ms"."involvement", "ms"."unread_at", "ms"."last_read_message_id" AS "read_id", "read"."created_at" AS "read_created_at" FROM "memberships" "ms" LEFT JOIN "messages" "read" ON "read"."id" = "ms"."last_read_message_id" AND "read"."room_id" = "ms"."room_id" AND "read"."thread_id" IS NULL WHERE "ms"."user_id" = ?1 AND "ms"."involvement" IN ('everything', 'mentions', 'muted', 'nothing') AND "ms"."unread_at" IS NOT NULL{bounds_only}), "roots" AS MATERIALIZED (SELECT "m"."room_id", "m"."id", "b"."involvement" = 'everything' AND NOT "m"."system_note" AND "m"."creator_id" != ?1 AS "notifies" FROM "bounds" "b" INNER JOIN "messages" "m" ON "m"."room_id" = "b"."room_id" AND "m"."thread_id" IS NULL WHERE {IN_UNREAD_RANGE}), "pinged" AS MATERIALIZED (SELECT "m"."room_id", "m"."id", "m"."thread_id", "activity_items"."event_type" = 'mention' AS "mention" FROM "activity_items" INNER JOIN "messages" "m" ON "m"."id" = "activity_items"."source_id" WHERE "activity_items"."user_id" = ?1 AND "activity_items"."source_type" = 'Message' AND "activity_items"."read_at" IS NULL AND "activity_items"."event_type" IN ('mention', 'reply', 'thread_activity'){pinged_only}), "notifying" AS (SELECT "room_id", "id", 0 AS "thread" FROM "roots" WHERE "notifies" UNION SELECT "p"."room_id", "p"."id", "p"."thread_id" IS NOT NULL FROM "pinged" "p" INNER JOIN "memberships" "ms" ON "ms"."room_id" = "p"."room_id" AND "ms"."user_id" = ?1 WHERE ("ms"."involvement" IN ('everything', 'mentions') OR ("ms"."involvement" = 'muted' AND "p"."mention")) AND CASE WHEN "p"."thread_id" IS NULL THEN "p"."id" IN (SELECT "id" FROM "roots") ELSE EXISTS (SELECT 1 FROM "thread_memberships" "tm" WHERE "tm"."thread_id" = "p"."thread_id" AND "tm"."user_id" = ?1 AND "tm"."unread_at" IS NOT NULL AND ("tm"."last_read_message_id" IS NULL OR "p"."id" > "tm"."last_read_message_id")) END) SELECT "room_id", SUM("unread"), SUM("mentions"), SUM("notifications"), SUM("threads") FROM (SELECT "room_id", 1 AS "unread", 0 AS "mentions", 0 AS "notifications", 0 AS "threads" FROM "roots" UNION ALL SELECT "room_id", 0, "mention", 0, 0 FROM "pinged" UNION ALL SELECT "room_id", 0, 0, 1, "thread" FROM "notifying") GROUP BY "room_id""#,
        bounds_only = only(r#""ms"."room_id""#),
        pinged_only = only(r#""m"."room_id""#),
    )
}

/// Each direct room's newest root message that isn't a system note, by room: one statement,
/// whose per-room subquery walks `index_messages_on_room_thread_created` back from the newest
/// (SQLite's lateral join). The excerpt is the search index's body (the plain text
/// `create_in_index` stores) with whitespace collapsed.
fn last_direct_messages(
    conn: &Connection,
    room_ids: &[i64],
) -> Result<HashMap<i64, api::SidebarLastMessage>> {
    let rows = ids_query(
        conn,
        r#"SELECT "messages"."room_id", "messages"."creator_id", COALESCE("message_search_index"."body", ''), "messages"."created_at" FROM "messages" LEFT JOIN "message_search_index" ON "message_search_index"."rowid" = "messages"."id" WHERE "messages"."id" IN (SELECT (SELECT "newest"."id" FROM "messages" AS "newest" WHERE "newest"."room_id" = "rooms"."id" AND "newest"."thread_id" IS NULL AND NOT "newest"."system_note" ORDER BY "newest"."created_at" DESC, "newest"."id" DESC LIMIT 1) FROM "rooms" WHERE "rooms"."id" IN ({}))"#,
        room_ids,
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Timestamp>(3)?,
            ))
        },
    )?;
    Ok(rows
        .into_iter()
        .map(|(room_id, creator_id, body, created_at)| {
            let text = body.split_whitespace().collect::<Vec<_>>().join(" ");
            let last = api::SidebarLastMessage {
                creator_id,
                excerpt: campfire_views::helpers::truncate(&text, 140, "…"),
                created_at: time(created_at),
            };
            (room_id, last)
        })
        .collect())
}

/// Whether the membership has a sidebar row (`memberships.visible`, of an alive room).
fn visible(room: &Room, membership: &Membership) -> bool {
    !room.deleted()
        && membership
            .involvement
            .is_some_and(|involvement| involvement != Involvement::Invisible)
}

fn sidebar_row_with(
    room: &Room,
    membership: &Membership,
    viewer_name: &str,
    members: Option<&[(i64, String)]>,
    counts: RoomCounts,
    last_message: Option<api::SidebarLastMessage>,
) -> api::SidebarRow {
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
    api::SidebarRow {
        room: self::room(room),
        membership: self::membership(membership),
        display_name,
        direct_member_ids,
        unread_count: counts.unread,
        mention_count: counts.mentions,
        notification_count: counts.notifications,
        thread_notification_count: counts.thread_notifications,
        last_message,
        refresh_room: None,
    }
}

/// The membership's sidebar row, or `None` when the room isn't in the person's sidebar. Read
/// from one snapshot, as the membership must be: its direct preview is only what it could see.
pub fn sidebar_row(
    conn: &Snapshot<'_>,
    room: &Room,
    membership: &Membership,
) -> Result<Option<api::SidebarRow>> {
    if !visible(room, membership) {
        return Ok(None);
    }
    membership_row(conn, room, membership).map(Some)
}

/// The membership's row as the sidebar would show it, even when it's hidden (`invisible`): the
/// answer to an organising call on a hidden room. From one snapshot, as [`sidebar_row`] is.
pub fn membership_row(
    conn: &Snapshot<'_>,
    room: &Room,
    membership: &Membership,
) -> Result<api::SidebarRow> {
    let viewer = User::find(conn, membership.user_id)?;
    let members = if room.direct() {
        Some(members(conn, room.id)?)
    } else {
        None
    };
    let counts = notification_counts(conn, membership.user_id, Some(room.id))?
        .remove(&room.id)
        .unwrap_or_default();
    let last_message = if room.direct() {
        last_direct_messages(conn, &[room.id])?.remove(&room.id)
    } else {
        None
    };
    Ok(sidebar_row_with(
        room,
        membership,
        &viewer.name,
        members.as_deref(),
        counts,
        last_message,
    ))
}

/// The person's whole sidebar, from one snapshot: the memberships it lists, their counts (one
/// statement for every room) and the direct previews it shows are read together, so neither a
/// count nor a preview is newer than the membership.
pub fn sidebar(
    conn: &Snapshot<'_>,
    secrets: &Secrets,
    viewer: &User,
    can_create_rooms: bool,
    now: Timestamp,
) -> Result<api::Sidebar> {
    let all = Membership::visible_with_ordered_room(conn, viewer.id)?;
    #[cfg(feature = "test-support")]
    crate::test_hooks::after_sidebar_memberships(conn, viewer.id);
    let counts = notification_counts(conn, viewer.id, None)?;
    let direct_ids: Vec<i64> = all
        .iter()
        .filter(|(_, room)| room.direct())
        .map(|(_, room)| room.id)
        .collect();
    let mut last_messages = last_direct_messages(conn, &direct_ids)?;
    let mut user_ids = BTreeSet::new();
    let mut rows = Vec::with_capacity(all.len());
    for (membership, room) in &all {
        let members = if room.direct() {
            Some(members(conn, room.id)?)
        } else {
            None
        };
        let row = sidebar_row_with(
            room,
            membership,
            &viewer.name,
            members.as_deref(),
            counts.get(&room.id).copied().unwrap_or_default(),
            last_messages.remove(&room.id),
        );
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
        .map(room_category)
        .collect();
    Ok(api::Sidebar {
        rows,
        categories,
        users: users(conn, secrets, user_ids, now)?,
        direct_placeholder_user_ids: placeholders,
        can_create_rooms,
    })
}

pub fn room_category(category: campfire_db::RoomCategory) -> api::RoomCategory {
    api::RoomCategory {
        id: category.id,
        name: category.name,
        collapsed: category.collapsed,
        position: category.position,
    }
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
        .ok_or_else(|| campfire_db::Error::RecordNotFound("User"))?;
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
        user: user(
            &settings,
            secrets,
            now,
            &UserExtras::load(conn, [&settings.user])?,
        ),
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

/// A thread's status as the wire names it.
pub fn thread_status(status: campfire_db::models::channel_thread::ThreadStatus) -> api::ThreadStatus {
    use campfire_db::models::channel_thread::ThreadStatus as Db;
    match status {
        Db::Active => api::ThreadStatus::Active,
        Db::Closed => api::ThreadStatus::Closed,
        Db::Locked => api::ThreadStatus::Locked,
    }
}

/// `message_forwards#destinations`: the viewer's rooms except boards, sorted by lowercased name,
/// with each non-direct room's unlocked threads. A direct room is named by its other members.
pub fn forward_destinations(
    conn: &Connection,
    viewer: &User,
    now: Timestamp,
) -> Result<api::ForwardDestinationList> {
    let mut rooms = Room::for_user(conn, viewer.id)?;
    rooms.sort_by_cached_key(|room| room.name.as_ref().map(|name| name.to_ascii_lowercase()));
    let ids: Vec<i64> = rooms.iter().map(|room| room.id).collect();
    let threads = campfire_db::ChannelThread::for_rooms(conn, &ids)?;
    let mut direct_names = HashMap::<i64, Vec<String>>::new();
    let mut statement = conn.prepare_cached(
        "SELECT memberships.room_id, users.id, users.name FROM memberships \
         INNER JOIN users ON users.id = memberships.user_id \
         INNER JOIN rooms ON rooms.id = memberships.room_id \
         WHERE rooms.type = 'Rooms::Direct' \
         AND rooms.id IN (SELECT room_id FROM memberships WHERE user_id = ?) \
         ORDER BY memberships.id",
    )?;
    let rows = statement.query_map([viewer.id], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
    })?;
    for row in rows {
        let (room, user, name) = row?;
        if user != viewer.id {
            direct_names.entry(room).or_default().push(name);
        }
    }
    let destinations = rooms
        .iter()
        .filter(|room| !room.board())
        .map(|room| {
            let name = if room.direct() {
                let names = direct_names.get(&room.id).map_or(&[][..], Vec::as_slice);
                let name = campfire_views::helpers::to_sentence(names, " and ");
                if name.is_empty() { viewer.name.clone() } else { name }
            } else {
                room.name.clone().unwrap_or_default()
            };
            let threads = if room.direct() {
                Vec::new()
            } else {
                threads
                    .iter()
                    .filter(|thread| thread.room_id == room.id && thread.locked_at.is_none())
                    .map(|thread| api::ForwardThread {
                        id: thread.id,
                        name: thread.name.clone(),
                        status: thread_status(thread.status_in_room(room, now)),
                    })
                    .collect()
            };
            api::ForwardDestination { room_id: room.id, name, direct: room.direct(), threads }
        })
        .collect();
    Ok(api::ForwardDestinationList { destinations })
}

/// A thread as every member of its room sees it, with its work facts ([`crate::work::facts`])
/// when it's tracked.
pub fn thread(
    thread: &campfire_db::ChannelThread,
    room: &Room,
    now: Timestamp,
    work: Option<api::WorkFacts>,
) -> api::Thread {
    api::Thread {
        id: thread.id,
        room_id: thread.room_id,
        parent_message_id: thread.parent_message_id,
        creator_id: thread.creator_id,
        name: thread.name.clone(),
        status: thread_status(thread.status_in_room(room, now)),
        reply_count: thread.messages_count,
        last_activity_at: time(thread.last_activity_at),
        auto_archive_after_minutes: thread.auto_archive_after_minutes,
        created_at: time(thread.created_at),
        work,
    }
}

/// A person's `thread_memberships` row.
pub fn thread_membership(membership: &campfire_db::ThreadMembership) -> api::ThreadMembership {
    use campfire_db::ThreadInvolvement as Db;
    api::ThreadMembership {
        thread_id: membership.thread_id,
        involvement: match membership.involvement {
            Db::Nothing => api::ThreadInvolvement::Nothing,
            Db::Mentions => api::ThreadInvolvement::Mentions,
            Db::Everything => api::ThreadInvolvement::Everything,
        },
        unread_at: membership.unread_at.map(time),
        joined_at: time(membership.joined_at),
    }
}

/// The reply indicator of the thread started from `parent`, if one was.
pub fn thread_indicator(conn: &Connection, parent: &Message) -> Result<Option<api::ThreadIndicator>> {
    Ok(thread_indicators(conn, std::slice::from_ref(parent))?.remove(&parent.id))
}

/// What `viewer` may do to `thread`, as `channel_threads#update` and `#destroy` decide it.
pub fn thread_permissions(
    thread: &campfire_db::ChannelThread,
    room: &Room,
    viewer: &User,
    member: bool,
    now: Timestamp,
) -> api::ThreadPermissions {
    use campfire_db::models::channel_thread::ThreadStatus as Db;
    let settings = thread.settings_manageable_in_room(room, viewer);
    let moderator = thread.manageable_in_room(room, viewer);
    let status = thread.status_in_room(room, now);
    let locked = thread.locked_at.is_some();
    // The viewer reads the thread from a room they belong to; a board post's name is work
    // metadata, which its owner may change too (`channel_threads#update`).
    let rename = if room.board() {
        thread.work_manageable_in_room(room, viewer, true)
    } else {
        settings
    };
    let [
        can_convert_work,
        can_manage_work,
        can_update_work_status,
        can_assign_work,
        can_remove_work,
    ] = crate::work::permissions(thread, room, viewer);
    api::ThreadPermissions {
        can_rename: rename,
        can_close: status == Db::Active && if room.board() { moderator } else { settings },
        can_reopen: status == Db::Closed && member,
        can_lock: moderator && !locked,
        can_unlock: moderator && locked,
        can_delete: moderator,
        can_convert_work,
        can_manage_work,
        can_update_work_status,
        can_assign_work,
        can_remove_work,
    }
}

/// `GET /api/v1/threads/:id`.
pub(crate) fn thread_detail(
    conn: &Connection,
    app: &AppState,
    viewer: &User,
    thread: &campfire_db::ChannelThread,
    room: &Room,
    now: Timestamp,
) -> Result<(api::ThreadDetail, crate::cards::Fetches)> {
    let membership = thread.membership_for(conn, viewer.id)?;
    let parent = match thread.parent_message_id {
        Some(id) => Message::find_by_id(conn, id)?,
        None => None,
    };
    let (mut parents, mut fetches) = messages_and_fetches(conn, app, parent.as_slice())?;
    let parent_message = parents.pop();
    fetches.thread_header(conn, thread, now)?;
    let facts = crate::work::facts(conn, &app.secrets, std::slice::from_ref(thread), now)?
        .remove(&thread.id);
    let permissions = thread_permissions(thread, room, viewer, membership.is_some(), now);
    let (work, work_people) = match facts {
        Some(_) => {
            let (work, people) = crate::work::detail(conn, app, thread, room, &permissions)?;
            (Some(work), people)
        }
        None => (None, Vec::new()),
    };
    let people = std::iter::once(thread.creator_id)
        .chain(parent.as_ref().map(|parent| parent.creator_id))
        .chain(work_people);
    let detail = api::ThreadDetail {
        thread: self::thread(thread, room, now, facts),
        permissions,
        membership: membership.as_ref().map(thread_membership),
        parent_message,
        work,
        users: users(conn, &app.secrets, people, now)?,
    };
    Ok((detail, fetches))
}

/// `GET /api/v1/rooms/:id/threads`: `channel_threads#index`'s filters, most recently active
/// first, with the viewer's memberships and the creators.
pub fn thread_list(
    conn: &Connection,
    secrets: &Secrets,
    viewer_id: i64,
    room: &Room,
    filter: api::ThreadFilter,
    now: Timestamp,
) -> Result<api::ThreadList> {
    use campfire_db::ChannelThread;
    let threads: Vec<ChannelThread> = match filter {
        api::ThreadFilter::Closed => ChannelThread::effectively_closed_for_room(conn, room.id, now)?,
        api::ThreadFilter::All => ChannelThread::for_room(conn, room.id)?,
        api::ThreadFilter::Locked => ChannelThread::for_room(conn, room.id)?
            .into_iter()
            .filter(|thread| thread.locked_at.is_some())
            .collect(),
        api::ThreadFilter::Active => ChannelThread::for_room(conn, room.id)?
            .into_iter()
            .filter(|thread| {
                thread.closed_at.is_none()
                    && thread.locked_at.is_none()
                    && (room.board() || thread.auto_archive_at() > now)
            })
            .collect(),
    };
    Ok(api::ThreadList {
        users: users(conn, secrets, threads.iter().map(|thread| thread.creator_id), now)?,
        threads: thread_summaries(conn, secrets, viewer_id, room, &threads, now)?,
    })
}

/// Thread rows for a room page, with work facts and viewer memberships loaded in batches.
pub(crate) fn thread_summaries(
    conn: &Connection,
    secrets: &Secrets,
    viewer_id: i64,
    room: &Room,
    threads: &[campfire_db::ChannelThread],
    now: Timestamp,
) -> Result<Vec<api::ThreadSummary>> {
    let mut facts = crate::work::facts(conn, secrets, threads, now)?;
    let ids: Vec<i64> = threads.iter().map(|thread| thread.id).collect();
    let memberships: HashMap<i64, campfire_db::ThreadMembership> =
        campfire_db::ThreadMembership::for_user_threads(conn, viewer_id, &ids)?
            .into_iter()
            .map(|membership| (membership.thread_id, membership))
            .collect();
    Ok(threads
        .iter()
        .map(|thread| api::ThreadSummary {
            thread: self::thread(thread, room, now, facts.remove(&thread.id)),
            membership: memberships.get(&thread.id).map(thread_membership),
        })
        .collect())
}

/// Files a page of `GET /api/v1/rooms/:id/files` holds.
const FILES_PAGE: i64 = 30;
/// The last page `rooms/files#index` serves.
const FILES_LAST_PAGE: i64 = 20;

/// `rooms/files#index`'s uploads, the `page`th 30 (from 1, at most 20), with their messages'
/// creators.
pub fn room_files(
    conn: &Connection,
    app: &AppState,
    room_id: i64,
    file_type: &str,
    filename: &str,
    page: i64,
    now: Timestamp,
) -> Result<api::FileList> {
    use campfire_db::models::room_files;
    let presenter = Presenter::new(conn, app, None);
    let size = page * FILES_PAGE;
    let rows = room_files::uploads(conn, room_id, file_type, filename, size)?;
    let start = (((page - 1) * FILES_PAGE) as usize).min(rows.len());
    let end = (size as usize).min(rows.len());
    let files = room_file_rows(conn, &presenter, &rows[start..end])?;
    // Another page only if a row past this one would show: rows whose message or file is
    // gone are left out, so look on until one shows or none are left.
    let mut more = false;
    if page < FILES_LAST_PAGE && rows.len() > end {
        let (mut seen, mut window) = (end, size);
        loop {
            window += FILES_PAGE;
            let rows = room_files::uploads(conn, room_id, file_type, filename, window)?;
            let upto = rows.len().min(window as usize);
            if seen < upto && !room_file_rows(conn, &presenter, &rows[seen..upto])?.is_empty() {
                more = true;
                break;
            }
            if rows.len() <= window as usize {
                break;
            }
            seen = upto;
        }
    }
    let creators: Vec<i64> = files.iter().map(|file| file.creator_id).collect();
    Ok(api::FileList {
        users: users(conn, &app.secrets, creators, now)?,
        files,
        next_page: more.then_some(page + 1),
    })
}

/// The rows that still show: a message and a file that both exist.
fn room_file_rows(
    conn: &Connection,
    presenter: &Presenter<'_>,
    rows: &[campfire_db::models::room_files::Upload],
) -> Result<Vec<api::RoomFile>> {
    let messages: HashMap<i64, Message> = Message::for_ids(
        conn,
        &rows.iter().map(|row| row.message_id).collect::<Vec<_>>(),
    )?
    .into_iter()
    .map(|message| (message.id, message))
    .collect();
    let blobs = campfire_storage::Blob::find_many(
        conn,
        &rows.iter().map(|row| row.blob_id).collect::<Vec<_>>(),
    )
    .map_err(|error| campfire_db::Error::Other(error.to_string()))?;
    let mut files = Vec::with_capacity(rows.len());
    for row in rows {
        let (Some(message), Some(blob)) = (messages.get(&row.message_id), blobs.get(&row.blob_id))
        else {
            continue;
        };
        let Some(view) = presenter.attachment(message)? else {
            continue;
        };
        files.push(api::RoomFile {
            message_id: message.id,
            thread_id: row.thread_id,
            creator_id: message.creator_id,
            attachment: attachment(view, blob.content_type.as_deref(), blob.byte_size),
            created_at: time(row.created_at),
        });
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::{Timestamp, api, avatar_icon, inline_mentions, notification_counts_sql, row_version, time};

    /// The counts read the unread range and the viewer's unread pings through their indexes,
    /// for every room and for one: never a room's whole history by `room_id`, nor every inbox
    /// item the viewer ever had (SQLite's plan with no statistics, as production runs).
    #[test]
    fn the_counts_never_walk_a_rooms_history_or_the_whole_inbox() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        campfire_db::schema::prepare(&mut conn, "production", &campfire_db::SystemClock).unwrap();
        for one_room in [false, true] {
            let sql = format!("EXPLAIN QUERY PLAN {}", notification_counts_sql(one_room));
            let mut statement = conn.prepare(&sql).unwrap();
            let params: &[i64] = if one_room { &[1, 2] } else { &[1] };
            let plan: Vec<String> = statement
                .query_map(rusqlite::params_from_iter(params), |row| row.get::<_, String>(3))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            let text = plan.join("\n");
            assert!(!text.contains("index_messages_on_room_id "), "one_room={one_room}:\n{text}");
            assert!(!text.contains("SCAN m"), "one_room={one_room}:\n{text}");
            assert!(!text.contains("SCAN activity_items"), "one_room={one_room}:\n{text}");
            assert!(
                text.contains("SEARCH activity_items USING INDEX index_activity_items_on_unread_message_pings"),
                "one_room={one_room}:\n{text}"
            );
            assert!(
                text.contains("SEARCH m USING INDEX index_messages_on_room_thread_created")
                    || text.contains("SEARCH m USING COVERING INDEX index_messages_on_room_thread_created"),
                "one_room={one_room}:\n{text}"
            );
        }
    }

    #[test]
    fn preloaded_avatar_icons_preserve_brand_custom_and_emoji_precedence() {
        assert_eq!(
            avatar_icon("github", Some("Custom GitHub")),
            Some(api::Icon {
                name: "github".into(),
                title: "GitHub".into(),
                kind: api::IconKind::Brand,
                character: None,
                image_url: Some("icons/brands/github.svg".into()),
            }),
        );
        assert_eq!(
            avatar_icon("robot", Some("Workspace robot")),
            Some(api::Icon {
                name: "robot".into(),
                title: "Workspace robot".into(),
                kind: api::IconKind::Custom,
                character: None,
                image_url: Some("/icons/robot".into()),
            }),
        );
        assert_eq!(
            avatar_icon("robot", None),
            Some(api::Icon {
                name: "robot".into(),
                title: "Robot".into(),
                kind: api::IconKind::Emoji,
                character: Some("🤖".into()),
                image_url: None,
            }),
        );
        assert_eq!(avatar_icon("missing-icon", None), None);
    }

    #[test]
    fn row_versions_pad_seconds_and_preserve_microseconds() {
        for (input, version, milliseconds) in [
            (
                "2026-10-07T10:15:00Z",
                "2026-10-07T10:15:00.000000Z",
                "2026-10-07T10:15:00.000Z",
            ),
            (
                "2026-10-07T10:15:00.123Z",
                "2026-10-07T10:15:00.123000Z",
                "2026-10-07T10:15:00.123Z",
            ),
            (
                "2026-10-07T10:15:00.123456Z",
                "2026-10-07T10:15:00.123456Z",
                "2026-10-07T10:15:00.123Z",
            ),
        ] {
            let stamp = Timestamp::from_jiff(input.parse().unwrap());
            assert_eq!(row_version(stamp), version);
            assert_eq!(time(stamp), milliseconds);
        }
    }

    #[test]
    fn agent_step_revisions_pad_seconds_and_preserve_microseconds() {
        let conn = campfire_db::Connection::open_in_memory().unwrap();
        for (input, expected) in [
            ("2026-10-07 10:15:00", "2026-10-07T10:15:00.000000Z"),
            ("2026-10-07 10:15:00.123456", "2026-10-07T10:15:00.123456Z"),
        ] {
            let step = conn
                .query_row(
                    "SELECT 1,NULL,NULL,'Check','done',NULL,NULL,NULL,0,?1,?1",
                    [input],
                    super::agent_step,
                )
                .unwrap();
            assert_eq!(step.updated_at, expected);
        }
    }

    #[test]
    fn a_mention_wrapper_becomes_a_span_with_its_own_close() {
        let html = concat!(
            r#"<p>Hi <div class="mention mention--user-1" data-user-id="1">"#,
            r#"<a href="/users/1">D</a><div class="inner"><b>x</b></div>"#,
            r#"<button>David</button></div>, and <div class="mention">"#,
            r#"<span>Kevin</span></div>!</p><div class="after"></div>"#,
        );
        assert_eq!(
            inline_mentions(html),
            concat!(
                r#"<p>Hi <span class="mention mention--user-1" data-user-id="1">"#,
                r#"<a href="/users/1">D</a><div class="inner"><b>x</b></div>"#,
                r#"<button>David</button></span>, and <span class="mention">"#,
                r#"<span>Kevin</span></span>!</p><div class="after"></div>"#,
            )
        );
    }

    #[test]
    fn an_unbalanced_mention_is_left_alone() {
        let html = r#"<p><div class="mention"><div>open</p>"#;
        assert_eq!(inline_mentions(html), html);
        assert_eq!(inline_mentions("<p>plain</p>"), "<p>plain</p>");
    }
}
