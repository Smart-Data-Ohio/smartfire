//! The welcome card and the server guide (U1, U4): a welcome message, up to five suggested rooms
//! and up to ten read-only resources, kept in `accounts.settings` under `welcome`.
//!
//! Administrators read the configuration at `/api/v1/admin/welcome` and `PATCH` one section at a
//! time (the welcome or the guide), read and written in one transaction so two saves don't undo
//! each other. Everyone reads their own copy at `/api/v1/welcome`. Both copies are filtered for
//! the person asking: a room they neither belong to nor may join (a closed room, a direct message,
//! a deleted room) never reaches them by name or id. Viewers don't get it at all; administrators
//! get its row with an opaque `keep` token in place of the room, which keeps it when sent back.
//! Saving publishes `workspace.updated`, the event a branding change sends, and every client
//! reads its own copy again.

use std::collections::{BTreeSet, HashMap};

use axum::Router;
use axum::routing::get;
use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::{Account, Connection, Room, RoomType};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode, action, unparsed_action};
use campfire_richtext::MentionUser;
use campfire_richtext::markdown;
use campfire_runtime::context::db_error;
use campfire_runtime::presenters;
use rails_compat::message_encryptor::Serializer;
use rails_compat::{MessageEncryptor, Secrets};

use crate::admin::{administrator, body, viewer};
use crate::error::{fail, validation};

/// The `accounts.settings` key.
const SETTINGS_KEY: &str = "welcome";

pub const MESSAGE_MAX_CHARS: usize = 1_000;
pub const ROOMS_MAX: usize = 5;
pub const ROOM_DESCRIPTION_MAX_CHARS: usize = 100;
pub const GUIDE_MAX: usize = 10;
pub const TITLE_MAX_CHARS: usize = 80;
pub const BODY_MAX_CHARS: usize = 1_000;
/// Long enough for a flag or a family emoji (several code points), too short for prose.
pub const EMOJI_MAX_CHARS: usize = 16;

pub fn routes() -> Router<Kit> {
    Router::new()
        .route("/api/v1/welcome", get(action(welcome)))
        .route(
            "/api/v1/admin/welcome",
            get(action(admin_welcome)).patch(unparsed_action(update_welcome)),
        )
}

endpoint!(
    /// `GET /api/v1/welcome`
    welcome => show_welcome
);
endpoint!(
    /// `GET /api/v1/admin/welcome`
    admin_welcome => show_admin_welcome
);
endpoint!(
    /// `PATCH /api/v1/admin/welcome`
    update_welcome => save_welcome
);

/// A refused configuration: the wire field and what's wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid {
    pub field: &'static str,
    pub message: String,
}

fn invalid(field: &'static str, message: impl Into<String>) -> Invalid {
    Invalid {
        field,
        message: message.into(),
    }
}

/// The saved configuration; nothing (or something unreadable) is the empty one.
pub fn stored(account: &Account) -> api::WelcomeSettings {
    account
        .settings()
        .welcome()
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default()
}

/// Multi-line text: no control characters but line breaks and tabs.
fn multiline(value: &str) -> bool {
    !value
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
}

/// One line of text: no control characters at all.
fn single_line(value: &str) -> bool {
    !value.chars().any(char::is_control)
}

/// A blank emoji is none. Anything else is a short run of non-ASCII symbols: no words, markup or
/// spaces.
fn emoji(value: Option<String>) -> std::result::Result<Option<String>, &'static str> {
    let Some(value) = value.map(|value| value.trim().to_string()) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > EMOJI_MAX_CHARS
        || value
            .chars()
            .any(|c| c.is_ascii() || c.is_whitespace() || c.is_control())
    {
        return Err("emoji must be a single emoji");
    }
    Ok(Some(value))
}

/// The configuration trimmed, with blank emoji and blank notes made `null`, or the first rule it
/// breaks. Rooms are checked separately, against the database ([`check_rooms`]).
pub fn normalize(
    settings: api::WelcomeSettings,
) -> std::result::Result<api::WelcomeSettings, Invalid> {
    let message = settings.message.trim().to_string();
    if message.chars().count() > MESSAGE_MAX_CHARS {
        return Err(invalid(
            "message",
            format!("must be {MESSAGE_MAX_CHARS} characters or fewer"),
        ));
    }
    if !multiline(&message) {
        return Err(invalid("message", "can't contain control characters"));
    }
    if settings.rooms.len() > ROOMS_MAX {
        return Err(invalid(
            "rooms",
            format!("can suggest at most {ROOMS_MAX} rooms"),
        ));
    }
    let mut seen = BTreeSet::new();
    let mut rooms = Vec::with_capacity(settings.rooms.len());
    for (index, room) in settings.rooms.into_iter().enumerate() {
        let at = index + 1;
        if !seen.insert(room.room_id) {
            return Err(invalid(
                "rooms",
                format!("#{at}: suggests the same room twice"),
            ));
        }
        let description = room.description.trim().to_string();
        if description.chars().count() > ROOM_DESCRIPTION_MAX_CHARS {
            return Err(invalid(
                "rooms",
                format!(
                    "#{at}: description must be {ROOM_DESCRIPTION_MAX_CHARS} characters or fewer"
                ),
            ));
        }
        if !single_line(&description) {
            return Err(invalid(
                "rooms",
                format!("#{at}: description must be one line of text"),
            ));
        }
        let emoji =
            emoji(room.emoji).map_err(|message| invalid("rooms", format!("#{at}: {message}")))?;
        rooms.push(api::WelcomeRoom {
            room_id: room.room_id,
            emoji,
            description,
        });
    }
    if settings.guide.len() > GUIDE_MAX {
        return Err(invalid(
            "guide",
            format!("can hold at most {GUIDE_MAX} resources"),
        ));
    }
    let mut guide = Vec::with_capacity(settings.guide.len());
    for (index, resource) in settings.guide.into_iter().enumerate() {
        let at = index + 1;
        let title = resource.title.trim().to_string();
        if title.is_empty() {
            return Err(invalid("guide", format!("#{at}: title can't be blank")));
        }
        if title.chars().count() > TITLE_MAX_CHARS || !single_line(&title) {
            return Err(invalid(
                "guide",
                format!("#{at}: title must be one line of {TITLE_MAX_CHARS} characters or fewer"),
            ));
        }
        let emoji = emoji(resource.emoji)
            .map_err(|message| invalid("guide", format!("#{at}: {message}")))?;
        let body = resource
            .body
            .map(|body| body.trim().to_string())
            .filter(|body| !body.is_empty());
        match (resource.room_id, &body) {
            (Some(_), Some(_)) => {
                return Err(invalid(
                    "guide",
                    format!("#{at}: links a room or holds a note, not both"),
                ));
            }
            (None, None) => {
                return Err(invalid("guide", format!("#{at}: needs a room or a note")));
            }
            _ => {}
        }
        if let Some(body) = &body {
            if body.chars().count() > BODY_MAX_CHARS {
                return Err(invalid(
                    "guide",
                    format!("#{at}: note must be {BODY_MAX_CHARS} characters or fewer"),
                ));
            }
            if !multiline(body) {
                return Err(invalid(
                    "guide",
                    format!("#{at}: note can't contain control characters"),
                ));
            }
        }
        guide.push(api::GuideResource {
            title,
            emoji,
            room_id: resource.room_id,
            body,
        });
    }
    Ok(api::WelcomeSettings {
        message,
        rooms,
        guide,
    })
}

/// How one person may reach a room: they belong to it, or it's an open room they may join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    Joined,
    Joinable,
}

/// A room someone can reach, with its name.
#[derive(Debug, Clone)]
pub struct Reachable {
    pub name: String,
    pub reach: Reach,
}

/// Which of `ids` `user_id` can reach. A deleted room, a direct message and a room that is
/// neither theirs nor open are missing from the answer, as are ids that name no room.
pub fn reachable(
    conn: &Connection,
    user_id: i64,
    ids: &[i64],
) -> campfire_db::Result<HashMap<i64, Reachable>> {
    let mine: BTreeSet<i64> = Room::for_user_without_directs(conn, user_id)?
        .into_iter()
        .map(|room| room.id)
        .collect();
    Ok(Room::for_ids(conn, ids)?
        .into_iter()
        .filter(|room| room.deleted_at.is_none() && room.room_type != RoomType::Direct)
        .filter_map(|room| {
            let reach = if mine.contains(&room.id) {
                Reach::Joined
            } else if room.room_type == RoomType::Open {
                Reach::Joinable
            } else {
                return None;
            };
            Some((
                room.id,
                Reachable {
                    name: room.name.unwrap_or_default(),
                    reach,
                },
            ))
        })
        .collect())
}

/// Every room id the configuration names, suggested or linked.
fn room_ids(settings: &api::WelcomeSettings) -> Vec<i64> {
    let ids: BTreeSet<i64> = settings
        .rooms
        .iter()
        .map(|room| room.room_id)
        .chain(
            settings
                .guide
                .iter()
                .filter_map(|resource| resource.room_id),
        )
        .collect();
    ids.into_iter().collect()
}

/// What a `keep` token is encrypted for.
const KEEP_PURPOSE: &str = "welcome_room";

/// Seals and opens `keep` tokens: a saved room's id, encrypted (with a fresh IV each time) so an
/// administrator who can't see the room can hand it back without learning which room it is.
fn keeper(secrets: &Secrets) -> MessageEncryptor {
    MessageEncryptor::new(
        &secrets.key_generator.generate_key(KEEP_PURPOSE, 32),
        Serializer::Json,
    )
}

fn keep_token(keeper: &MessageEncryptor, room_id: i64) -> String {
    keeper.encrypt_and_sign(&serde_json::Value::from(room_id), Some(KEEP_PURPOSE), None)
}

fn kept_room(keeper: &MessageEncryptor, token: &str) -> Option<i64> {
    keeper
        .decrypt_and_verify(token, Some(KEEP_PURPOSE), jiff::Timestamp::now())
        .ok()?
        .as_i64()
}

/// The configuration as administrator `user_id` may see it: a saved room they can't reach comes
/// without its id, as a `keep` token.
pub fn for_admin(
    conn: &Connection,
    user_id: i64,
    settings: &api::WelcomeSettings,
    keeper: &MessageEncryptor,
) -> campfire_db::Result<api::WelcomeAdmin> {
    let reach = reachable(conn, user_id, &room_ids(settings))?;
    let mask = |id: i64| {
        if reach.contains_key(&id) {
            (Some(id), None)
        } else {
            (None, Some(keep_token(keeper, id)))
        }
    };
    let rooms = settings
        .rooms
        .iter()
        .map(|room| {
            let (room_id, keep) = mask(room.room_id);
            api::WelcomeRoomEdit {
                room_id,
                keep,
                emoji: room.emoji.clone(),
                description: room.description.clone(),
            }
        })
        .collect();
    let guide = settings
        .guide
        .iter()
        .map(|resource| {
            let (room_id, keep) = resource.room_id.map_or((None, None), mask);
            api::GuideResourceEdit {
                title: resource.title.clone(),
                emoji: resource.emoji.clone(),
                room_id,
                keep,
                body: resource.body.clone(),
            }
        })
        .collect();
    Ok(api::WelcomeAdmin {
        message: settings.message.clone(),
        rooms,
        guide,
        choices: choices(conn, user_id)?,
    })
}

/// `previous` with the sections `update` names replaced, rooms resolved: a `roomId` must be a room
/// administrator `user_id` can reach, and a `keep` token must open to a room already saved. A
/// room that doesn't exist and one they can't see are refused in the same words. The result
/// still needs [`normalize`].
pub fn resolve(
    conn: &Connection,
    user_id: i64,
    update: api::WelcomeUpdate,
    previous: &api::WelcomeSettings,
    keeper: &MessageEncryptor,
) -> campfire_db::Result<std::result::Result<api::WelcomeSettings, Invalid>> {
    if update
        .rooms
        .as_ref()
        .is_some_and(|rooms| rooms.len() > ROOMS_MAX)
    {
        return Ok(Err(invalid(
            "rooms",
            format!("can suggest at most {ROOMS_MAX} rooms"),
        )));
    }
    if update
        .guide
        .as_ref()
        .is_some_and(|guide| guide.len() > GUIDE_MAX)
    {
        return Ok(Err(invalid(
            "guide",
            format!("can hold at most {GUIDE_MAX} resources"),
        )));
    }
    let named: Vec<i64> = update
        .rooms
        .iter()
        .flatten()
        .filter_map(|room| room.room_id)
        .chain(
            update
                .guide
                .iter()
                .flatten()
                .filter_map(|resource| resource.room_id),
        )
        .collect();
    let reach = reachable(conn, user_id, &named)?;
    let saved: BTreeSet<i64> = room_ids(previous).into_iter().collect();
    let pick = |field: &'static str,
                at: usize,
                room_id: Option<i64>,
                keep: Option<&str>,
                unreachable: &str|
     -> std::result::Result<Option<i64>, Invalid> {
        match (room_id, keep) {
            (Some(_), Some(_)) => Err(invalid(
                field,
                format!("#{at}: names a room and keeps one; send one or the other"),
            )),
            (Some(id), None) if reach.contains_key(&id) => Ok(Some(id)),
            (Some(_), None) => Err(invalid(field, format!("#{at}: {unreachable}"))),
            (None, Some(token)) => kept_room(keeper, token)
                .filter(|id| saved.contains(id))
                .map(Some)
                .ok_or_else(|| {
                    invalid(
                        field,
                        format!("#{at}: that room is no longer saved; choose another"),
                    )
                }),
            (None, None) => Ok(None),
        }
    };
    let mut next = previous.clone();
    if let Some(message) = update.message {
        next.message = message;
    }
    if let Some(rooms) = update.rooms {
        let mut resolved = Vec::with_capacity(rooms.len());
        for (index, room) in rooms.into_iter().enumerate() {
            let at = index + 1;
            let picked = pick(
                "rooms",
                at,
                room.room_id,
                room.keep.as_deref(),
                "isn't a room you can suggest",
            );
            let room_id = match picked {
                Ok(Some(id)) => id,
                Ok(None) => return Ok(Err(invalid("rooms", format!("#{at}: choose a room")))),
                Err(refused) => return Ok(Err(refused)),
            };
            resolved.push(api::WelcomeRoom {
                room_id,
                emoji: room.emoji,
                description: room.description,
            });
        }
        next.rooms = resolved;
    }
    if let Some(guide) = update.guide {
        let mut resolved = Vec::with_capacity(guide.len());
        for (index, resource) in guide.into_iter().enumerate() {
            let picked = pick(
                "guide",
                index + 1,
                resource.room_id,
                resource.keep.as_deref(),
                "links to a room you can't choose",
            );
            let room_id = match picked {
                Ok(room_id) => room_id,
                Err(refused) => return Ok(Err(refused)),
            };
            resolved.push(api::GuideResource {
                title: resource.title,
                emoji: resource.emoji,
                room_id,
                body: resource.body,
            });
        }
        next.guide = resolved;
    }
    Ok(Ok(next))
}

/// `source` rendered as a message's markdown is (no mentions resolve) and sanitized for display.
fn render_markdown(conn: &Connection, source: &str) -> campfire_db::Result<String> {
    let icons = campfire_app::icons::icons(conn).map_err(campfire_db::Error::Other)?;
    let no_mentions = |_: &str| -> Option<MentionUser> { None };
    let html = markdown::render(source, &no_mentions, &icons)
        .and_then(|html| markdown::sanitize_presentation(&html, &icons, None))
        .map_err(|error| campfire_db::Error::Other(error.to_string()))?;
    Ok(format!("<div class=\"markdown-body\">{html}</div>"))
}

/// What `viewer` sees of `settings`: rooms they can't reach are left out.
pub fn for_viewer(
    conn: &Connection,
    viewer: i64,
    settings: &api::WelcomeSettings,
) -> campfire_db::Result<(
    Option<String>,
    Vec<api::WelcomeRoomCard>,
    Vec<api::GuideEntry>,
)> {
    let reach = reachable(conn, viewer, &room_ids(settings))?;
    let message_html = if settings.message.is_empty() {
        None
    } else {
        Some(render_markdown(conn, &settings.message)?)
    };
    let rooms = settings
        .rooms
        .iter()
        .filter_map(|room| {
            let found = reach.get(&room.room_id)?;
            Some(api::WelcomeRoomCard {
                room_id: room.room_id,
                name: found.name.clone(),
                emoji: room.emoji.clone(),
                description: room.description.clone(),
                joined: found.reach == Reach::Joined,
            })
        })
        .collect();
    let mut guide = Vec::with_capacity(settings.guide.len());
    for resource in &settings.guide {
        let room = match resource.room_id {
            Some(id) => match reach.get(&id) {
                Some(found) => Some(api::GuideRoom {
                    id,
                    name: found.name.clone(),
                    joined: found.reach == Reach::Joined,
                }),
                None => continue,
            },
            None => None,
        };
        let body_html = match &resource.body {
            Some(body) if room.is_none() => Some(render_markdown(conn, body)?),
            _ => None,
        };
        guide.push(api::GuideEntry {
            title: resource.title.clone(),
            emoji: resource.emoji.clone(),
            body_html,
            room,
        });
    }
    Ok((message_html, rooms, guide))
}

async fn account(c: &Ctx) -> Result<Account> {
    c.app()
        .db
        .read(Account::first)
        .await
        .map_err(Error::internal)?
        .ok_or_else(|| Error::internal(std::io::Error::other("no account")))
}

async fn show_welcome(c: &mut Ctx) -> Result {
    let user = viewer(c).await?;
    let account = account(c).await?;
    let settings = stored(&account);
    let viewer_id = user.id;
    let (message_html, rooms, guide) = c
        .app()
        .db
        .read(move |conn| for_viewer(conn, viewer_id, &settings))
        .await
        .map_err(db_error)?;
    let welcome = api::WorkspaceWelcome {
        name: account.name.clone(),
        description: account.settings().description().to_string(),
        message_html,
        rooms,
        guide,
        can_administer: user.can_administer(None, false),
    };
    c.json(StatusCode::OK, &welcome)
}

/// The rooms an administrator may choose: theirs and the open ones, by name.
fn choices(conn: &Connection, user_id: i64) -> campfire_db::Result<Vec<api::WelcomeRoomChoice>> {
    let mut rooms: HashMap<i64, String> = HashMap::new();
    for room in Room::for_user_without_directs(conn, user_id)?
        .into_iter()
        .chain(Room::of_type(conn, RoomType::Open)?)
    {
        rooms.insert(room.id, room.name.unwrap_or_default());
    }
    let mut choices: Vec<_> = rooms
        .into_iter()
        .map(|(id, name)| api::WelcomeRoomChoice { id, name })
        .collect();
    choices.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    Ok(choices)
}

async fn reply_admin(c: &mut Ctx, user_id: i64) -> Result {
    let settings = stored(&account(c).await?);
    let secrets = c.app().secrets.clone();
    let admin = c
        .app()
        .db
        .read(move |conn| for_admin(conn, user_id, &settings, &keeper(&secrets)))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &admin)
}

async fn show_admin_welcome(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    reply_admin(c, user.id).await
}

async fn save_welcome(c: &mut Ctx) -> Result {
    let user = administrator(c).await?;
    let update: api::WelcomeUpdate = body(c).await?;
    let (user_id, secrets) = (user.id, c.app().secrets.clone());
    // Read, merged and written in one transaction: a save of the guide can't put back a welcome
    // another save just replaced.
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            let mut account =
                Account::first(tx.conn())?.ok_or(campfire_db::Error::RecordNotFound("Account"))?;
            let previous = stored(&account);
            let next = match resolve(tx.conn(), user_id, update, &previous, &keeper(&secrets))?
                .and_then(normalize)
            {
                Ok(next) => next,
                Err(refused) => return Ok(Err(refused)),
            };
            let json = serde_json::to_string(&next)
                .map_err(|error| campfire_db::Error::Other(error.to_string()))?;
            account.update(tx, None, None, Some(&[(SETTINGS_KEY, json.as_str())]))?;
            Ok(Ok(()))
        })
        .await
        .map_err(db_error)?;
    if let Err(refused) = outcome {
        return Err(fail(c, validation(refused.field, &refused.message)));
    }
    presenters::workspace_branding::publish(c.app()).await;
    reply_admin(c, user_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(room_id: i64, description: &str) -> api::WelcomeRoom {
        api::WelcomeRoom {
            room_id,
            emoji: None,
            description: description.into(),
        }
    }

    fn note(title: &str, body: &str) -> api::GuideResource {
        api::GuideResource {
            title: title.into(),
            emoji: None,
            room_id: None,
            body: Some(body.into()),
        }
    }

    fn settings(
        rooms: Vec<api::WelcomeRoom>,
        guide: Vec<api::GuideResource>,
    ) -> api::WelcomeSettings {
        api::WelcomeSettings {
            message: "Welcome".into(),
            rooms,
            guide,
        }
    }

    fn refused(settings: api::WelcomeSettings) -> Invalid {
        normalize(settings).expect_err("refused")
    }

    #[test]
    fn trims_and_blanks_optional_parts() {
        let normalized = normalize(api::WelcomeSettings {
            message: "  Hello **there**  ".into(),
            rooms: vec![api::WelcomeRoom {
                room_id: 1,
                emoji: Some("  ".into()),
                description: "  Say hi  ".into(),
            }],
            guide: vec![api::GuideResource {
                title: " Start ".into(),
                emoji: Some(" 📌 ".into()),
                room_id: Some(1),
                body: Some("   ".into()),
            }],
        })
        .unwrap();
        assert_eq!(normalized.message, "Hello **there**");
        assert_eq!(normalized.rooms[0].emoji, None);
        assert_eq!(normalized.rooms[0].description, "Say hi");
        assert_eq!(normalized.guide[0].title, "Start");
        assert_eq!(normalized.guide[0].emoji.as_deref(), Some("📌"));
        assert_eq!(normalized.guide[0].body, None);
    }

    #[test]
    fn bounds_lengths_and_counts() {
        let long = |n: usize| "x".repeat(n);
        let mut big = settings(vec![], vec![]);
        big.message = long(MESSAGE_MAX_CHARS + 1);
        assert_eq!(refused(big).field, "message");
        let mut exact = settings(vec![room(1, &long(ROOM_DESCRIPTION_MAX_CHARS))], vec![]);
        exact.message = long(MESSAGE_MAX_CHARS);
        assert!(normalize(exact).is_ok());
        assert_eq!(
            refused(settings(
                vec![room(1, &long(ROOM_DESCRIPTION_MAX_CHARS + 1))],
                vec![]
            )),
            invalid("rooms", "#1: description must be 100 characters or fewer")
        );
        let six = (1..=6).map(|id| room(id, "")).collect();
        assert_eq!(refused(settings(six, vec![])).field, "rooms");
        let eleven = (0..=GUIDE_MAX).map(|_| note("Tip", "Body")).collect();
        assert_eq!(refused(settings(vec![], eleven)).field, "guide");
        assert_eq!(
            refused(settings(
                vec![],
                vec![note(&long(TITLE_MAX_CHARS + 1), "Body")]
            ))
            .field,
            "guide"
        );
        assert_eq!(
            refused(settings(
                vec![],
                vec![note("Tip", &long(BODY_MAX_CHARS + 1))]
            ))
            .field,
            "guide"
        );
    }

    #[test]
    fn refuses_malformed_entries() {
        assert_eq!(
            refused(settings(vec![room(1, ""), room(1, "")], vec![])),
            invalid("rooms", "#2: suggests the same room twice")
        );
        assert_eq!(
            refused(settings(vec![room(1, "two\nlines")], vec![])).field,
            "rooms"
        );
        let mut word = room(1, "");
        word.emoji = Some("wave".into());
        assert_eq!(
            refused(settings(vec![word], vec![])),
            invalid("rooms", "#1: emoji must be a single emoji")
        );
        let both = api::GuideResource {
            room_id: Some(1),
            ..note("Tip", "Body")
        };
        assert_eq!(
            refused(settings(vec![], vec![both])),
            invalid("guide", "#1: links a room or holds a note, not both")
        );
        let neither = api::GuideResource {
            body: None,
            ..note("Tip", "")
        };
        assert_eq!(
            refused(settings(vec![], vec![neither])),
            invalid("guide", "#1: needs a room or a note")
        );
        assert_eq!(
            refused(settings(vec![], vec![note("  ", "Body")])),
            invalid("guide", "#1: title can't be blank")
        );
        let mut control = settings(vec![], vec![]);
        control.message = "bell\u{7}".into();
        assert_eq!(refused(control).field, "message");
    }
}
