//! `reference/app/services/messages/forwarder.rb`: forwards a message to up to five rooms or
//! threads at once. Each forward is a new message holding a snapshot of the source's rendered
//! body (so later edits or deletion of the source never change it, and its mentions never
//! re-resolve), a copy of its attachment and its Drive file ids, plus an optional note, whose
//! `@[Name]` tokens are the only mentions a forward notifies (`Message::forward_note_mentionees`).
//!
//! Every destination is written in the caller's transaction: an `Err` must abort the write (as
//! `Database::write` does), which rolls back the forwards already made, and the attachment
//! copies are discarded from storage through [`BlobCopier::discard`]. Analyzing the copied
//! attachments (`process_attachment`) is the caller's, after commit, as for any new message.

use rusqlite::Connection;

use crate::database::Tx;
use crate::error::{Error, Result};
use crate::models::{ChannelThread, Membership, Message, NewMessage, Room};
use crate::models::active_storage::Blob;

/// `Messages::Forwarder::MAX_DESTINATIONS`
pub const MAX_DESTINATIONS: usize = 5;

/// One `{ room_id:, thread_id: }` destination, as submitted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Destination {
    pub room_id: Option<i64>,
    pub thread_id: Option<i64>,
}

impl Destination {
    pub fn room(room_id: i64) -> Self {
        Self { room_id: Some(room_id), thread_id: None }
    }

    pub fn thread(room_id: i64, thread_id: i64) -> Self {
        Self { room_id: Some(room_id), thread_id: Some(thread_id) }
    }
}

/// `Messages::Forwarder::Result`
#[derive(Debug, Clone, PartialEq)]
pub struct Forwarded {
    pub message: Message,
    pub room: Room,
    pub thread: Option<ChannelThread>,
}

/// The forwarder's own exceptions, raised before anything is written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// `TooManyDestinations`
    #[error("{0}")]
    TooManyDestinations(String),
    /// `InvalidDestination`
    #[error("{0}")]
    InvalidDestination(String),
}

/// A normalized destination: the room, and the thread in it, if any.
type Target = (Room, Option<ChannelThread>);

fn invalid(message: &str) -> Refusal {
    Refusal::InvalidDestination(message.into())
}

/// The storage half of `copy_attachment_to`: `campfire_storage` uploads a copy of the blob's
/// file under a new key (`ActiveStorage::Blob.create_and_upload!`, `identify: false`, keeping the
/// filename, content type and metadata) and inserts its row in `tx`. `discard` removes the copies'
/// files when the forward fails (`purge_copied_blobs`); their rows roll back with the write.
pub trait BlobCopier {
    fn copy(&self, tx: &mut Tx<'_>, blob: &Blob) -> Result<Blob>;
    fn discard(&self, blobs: &[Blob]);
}

/// `Messages::Forwarder.call(source:, destinations:, note:, creator:)`. The outer `Result` is
/// the write's; the inner one refuses the destinations before anything is written.
pub fn forward(
    tx: &mut Tx<'_>,
    source: &Message,
    destinations: &[Destination],
    note: Option<&str>,
    creator_id: i64,
    copier: &dyn BlobCopier,
) -> Result<std::result::Result<Vec<Forwarded>, Refusal>> {
    let destinations = match normalize_destinations(tx.conn(), destinations, creator_id)? {
        Ok(destinations) => destinations,
        Err(refusal) => return Ok(Err(refusal)),
    };
    let note = note.filter(|note| !note.trim().is_empty());
    let mut copied = Vec::new();
    let mut results = Vec::new();
    for (room, thread) in destinations {
        match create_forward(tx, source, room, thread, note, creator_id, copier, &mut copied) {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(refusal)) => {
                copier.discard(&copied);
                return Err(Error::Other(refusal.to_string()));
            }
            Err(error) => {
                copier.discard(&copied);
                return Err(error);
            }
        }
    }
    Ok(Ok(results))
}

/// `normalize_destinations`: 1 to 5 destinations, each a room of the creator's that isn't a
/// board, optionally one of its threads that isn't locked (and never in a direct room), with no
/// destination twice.
fn normalize_destinations(
    conn: &Connection,
    destinations: &[Destination],
    creator_id: i64,
) -> Result<std::result::Result<Vec<Target>, Refusal>> {
    if destinations.is_empty() || destinations.len() > MAX_DESTINATIONS {
        return Ok(Err(Refusal::TooManyDestinations(format!("Choose between 1 and {MAX_DESTINATIONS} destinations"))));
    }
    let mut normalized: Vec<Target> = Vec::new();
    for destination in destinations {
        let Some(room_id) = destination.room_id else { return Ok(Err(invalid("A destination room is required"))) };
        let Some(room) = Room::find_for_user(conn, creator_id, room_id)? else {
            return Ok(Err(invalid("You cannot forward to that room")));
        };
        if room.board() {
            return Ok(Err(invalid("You cannot forward to a board")));
        }
        let thread = match destination.thread_id {
            Some(thread_id) => {
                let Some(thread) = ChannelThread::find_by_id(conn, thread_id)?.filter(|thread| thread.room_id == room.id) else {
                    return Ok(Err(invalid("That thread is unavailable")));
                };
                if room.direct() {
                    return Ok(Err(invalid("Direct rooms cannot contain threads")));
                }
                if thread.locked_at.is_some() {
                    return Ok(Err(invalid("That thread is locked")));
                }
                Some(thread)
            }
            None => None,
        };
        normalized.push((room, thread));
    }
    let mut keys: Vec<(i64, Option<i64>)> = normalized.iter().map(|(room, thread)| (room.id, thread.as_ref().map(|t| t.id))).collect();
    keys.sort();
    keys.dedup();
    if keys.len() != normalized.len() {
        return Ok(Err(invalid("Destinations must be unique")));
    }
    Ok(Ok(normalized))
}

/// `create_forward!`: the creator must still be in the room (`lock_current_parent_membership!`);
/// a thread forward joins the thread and reopens it, as any thread post does.
#[allow(clippy::too_many_arguments)]
fn create_forward(
    tx: &mut Tx<'_>,
    source: &Message,
    room: Room,
    thread: Option<ChannelThread>,
    note: Option<&str>,
    creator_id: i64,
    copier: &dyn BlobCopier,
    copied: &mut Vec<Blob>,
) -> Result<std::result::Result<Forwarded, Refusal>> {
    if Membership::find_by_room_and_user(tx.conn(), room.id, creator_id)?.is_none() {
        return Ok(Err(invalid("You cannot forward to that room")));
    }
    let attributes = build_forward(tx, source, &room, note, creator_id, copier, copied)?;
    match thread {
        Some(mut thread) => {
            thread.reload(tx.conn())?;
            if thread.locked_at.is_some() {
                return Ok(Err(invalid("That thread is locked")));
            }
            // `ThreadMembership.join!`, then `update!(closed_at: nil, last_activity_at:)`.
            let message = thread.post_message(tx, creator_id, attributes)?;
            thread.reload(tx.conn())?;
            Ok(Ok(Forwarded { message, room, thread: Some(thread) }))
        }
        None => {
            let message = Message::create(tx, attributes)?;
            Ok(Ok(Forwarded { message, room, thread: None }))
        }
    }
}

/// `build_forward`: a new client id, the body snapshot, the forward markers (a Markdown source,
/// or a forward of one, makes a `forwarded_markdown` forward, which is still not a Markdown
/// record), the note, and copies of the attachment and Drive ids.
fn build_forward(
    tx: &mut Tx<'_>,
    source: &Message,
    room: &Room,
    note: Option<&str>,
    creator_id: i64,
    copier: &dyn BlobCopier,
    copied: &mut Vec<Blob>,
) -> Result<NewMessage> {
    let body = snapshot_body(tx.conn(), source)?;
    let body = tx.rich_text().try_canonicalize_html(tx.conn(), &body).map_err(Error::Other)?;
    let attachment_blob_id = match source.attachment(tx.conn())? {
        Some((_, blob)) => {
            let copy = copier.copy(tx, &blob)?;
            let id = copy.id;
            copied.push(copy);
            Some(id)
        }
        None => None,
    };
    Ok(NewMessage {
        room_id: room.id,
        creator_id,
        client_message_id: None,
        body: Some(body),
        attachment_blob_id,
        forwarded_from_message_id: Some(source.id),
        forwarded_at: Some(tx.now()),
        forwarded_markdown: source.markdown() || source.forwarded_markdown,
        forward_note: note.map(Into::into),
        drive_file_ids: source.drive_file_ids(tx.conn())?,
        ..Default::default()
    })
}

/// `snapshot_body`: the source's rendered HTML, after its own forward note (escaped, newlines as
/// `<br>`) when it's a forward with one.
fn snapshot_body(conn: &Connection, source: &Message) -> Result<String> {
    let html = source.body_html(conn)?.unwrap_or_default();
    Ok(match source.forward_note.as_deref().filter(|note| !note.trim().is_empty()) {
        Some(note) => format!("<p>{}</p>{html}", html_escape(note).replace('\n', "<br>")),
        None => html,
    })
}

/// `ERB::Util.html_escape`
fn html_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            c => escaped.push(c),
        }
    }
    escaped
}
