//! S3 polls and cards on `/api/v1` (`campfire_api_types::cards` documents each endpoint): a
//! message's poll and cards as `MessageDTO` carries them, posting and voting on polls
//! (`rooms/polls#create/show/vote`), event responses (`rooms/events/attendances#show/update`),
//! and the per-viewer previews the classic lazy frames serve (GitHub pull requests, Fizzy cards,
//! quoted messages). The `poll.updated`, `poll.ballot` and `message.cards` twins are published by
//! `campfire_app::cable::sync` and built here, through the [`crate::sync::Renderer`].

use std::collections::{BTreeSet, HashMap, HashSet};

use campfire_api_types as api;
use campfire_app::app::{App, AppCtx};
use campfire_app::integrations::fizzy::{
    accounts::Account as FizzyAccount,
    cards::{Cache as FizzyCache, Card as FizzyCard, NOT_FOUND as FIZZY_NOT_FOUND},
};
use campfire_app::integrations::github::pull_requests::{self, PullRequest};
use campfire_app::integrations::link_embed::Reference as LinkReference;
use campfire_app::integrations::twitter::post::Post;
use campfire_db::models::calendar_event::attendance::RESPONSES;
use campfire_db::models::poll::{LABEL_LIMIT, MAX_OPTIONS, MIN_OPTIONS};
use campfire_db::{
    CalendarEvent, ChannelThread, Connection, Message, NewMessage, NewPoll, Poll, Room, Timestamp,
    User, message_quote,
};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use campfire_messages::controllers::message_features as features;
use campfire_messages::controllers::messages as posting;
use campfire_runtime::concerns::{self, cast_integer};
use campfire_runtime::presenters::{self, Presenter};
use campfire_runtime::context::db_error;
use rails_compat::ar_encryption::ArEncryption;
use serde_json::Value;

use crate::dto::{self, ids_query};
use crate::endpoints::{before_actions, body, now, set_room};
use crate::error::{fail, not_found, validation};

endpoint!(
    /// `POST /api/v1/rooms/:room_id/polls`
    create_poll => post_poll
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/polls/:poll_id`
    poll => show_poll
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/polls/:poll_id/vote`
    vote => post_vote
);
endpoint!(
    /// `POST /api/v1/rooms/:room_id/polls/:poll_id/end`
    end_poll => post_end_poll
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/events/:event_id/attendance`
    attendance => show_attendance
);
endpoint!(
    /// `PUT /api/v1/rooms/:room_id/events/:event_id/attendance`
    respond => put_attendance
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/github/pull_requests/:id/card`
    github_card => show_github_card
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/fizzy/cards/:id/card`
    fizzy_card => show_fizzy_card
);
endpoint!(
    /// `GET /api/v1/rooms/:room_id/message_links/:reference_id/card`
    quote_card => show_quote_card
);

// Reading

/// The fetches a read of cards asks for, as the classic render does
/// (`Presenter#request_link_fetch`, `#request_twitter_fetch`, the stale pull requests): claimed
/// and enqueued on the writer after the read ([`Fetches::request`]).
#[derive(Default)]
pub(crate) struct Fetches {
    pub(crate) render_refreshes: campfire_runtime::presenters::RenderRefreshes,
    links: BTreeSet<i64>,
    posts: BTreeSet<i64>,
    pull_requests: BTreeSet<i64>,
}

impl Fetches {
    /// Classic also renders a discussion header when the starter has been deleted.
    pub(crate) fn thread_header(
        &mut self,
        conn: &Connection,
        thread: &campfire_db::ChannelThread,
        now: Timestamp,
    ) -> campfire_db::Result<()> {
        use rusqlite::OptionalExtension;
        let id = conn.query_row(
            "SELECT github_pull_request_id FROM github_pull_request_threads WHERE channel_thread_id=? AND room_id=?",
            rusqlite::params![thread.id, thread.room_id],
            |row| row.get::<_, i64>(0),
        ).optional()?;
        if let Some(id) = id
            && PullRequest::find(conn, id)?.stale(now)
        {
            self.pull_requests.insert(id);
        }
        Ok(())
    }

    /// `enqueue_render_fetches` and `refresh_after_render`. A failure is logged: the page was
    /// read, and the next read asks again.
    pub(crate) async fn request(self, app: &App) {
        let Fetches {
            render_refreshes,
            links,
            posts,
            pull_requests,
        } = self;
        if let Err(error) = presenters::link_embeds::enqueue_render_fetches(
            app,
            links.into_iter().collect(),
            posts.into_iter().collect(),
        )
        .await
        {
            tracing::warn!(%error, "card fetches not requested");
        }
        campfire_runtime::presenters::refresh_after_render(&app.db, render_refreshes).await;
        pull_requests::refresh_after_render(&app.db, pull_requests.into_iter().collect()).await;
    }
}

/// The polls on `message_ids`, by message (`Poll#results_payload` without the viewer): options in
/// `(position, id)` order, each with its voters' ids (none for an anonymous poll).
pub(crate) fn polls(
    conn: &Connection,
    message_ids: &[i64],
    now: Timestamp,
    verifier: &dyn campfire_storage::Verifier,
) -> campfire_db::Result<HashMap<i64, api::Poll>> {
    let rows: Vec<Poll> = ids_query(
        conn,
        r#"SELECT "polls".* FROM "polls" WHERE "polls"."message_id" IN ({})"#,
        message_ids,
        |row| {
            Ok(Poll {
                id: row.get("id")?,
                message_id: row.get("message_id")?,
                multiple: row.get("multiple")?,
                anonymous: row.get("anonymous")?,
                closes_at: row.get("closes_at")?,
                closed_at: row.get("closed_at")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    )?;
    let poll_ids: Vec<i64> = rows.iter().map(|poll| poll.id).collect();
    let options: Vec<campfire_db::PollOption> = ids_query(
        conn,
        r#"SELECT * FROM "poll_options" WHERE "poll_id" IN ({}) ORDER BY "poll_id", "position", "id""#,
        &poll_ids,
        |row| Ok(campfire_db::PollOption {
            id: row.get("id")?, poll_id: row.get("poll_id")?, label: row.get("label")?,
            position: row.get("position")?, created_at: row.get("created_at")?, updated_at: row.get("updated_at")?,
        }),
    )?;
    let votes: Vec<(i64, i64, i64)> = ids_query(
        conn,
        r#"SELECT "poll_id", "poll_option_id", "user_id" FROM "poll_votes" WHERE "poll_id" IN ({}) ORDER BY "id""#,
        &poll_ids,
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let as_of = dto::time(now);
    rows
        .into_iter()
        .map(|poll| {
            let poll_votes: Vec<&(i64, i64, i64)> =
                votes.iter().filter(|vote| vote.0 == poll.id).collect();
            let options = options
                .iter()
                .filter(|option| option.poll_id == poll.id)
                .map(|option| {
                    let mut voter_ids: Vec<i64> = poll_votes
                        .iter()
                        .filter(|vote| vote.1 == option.id)
                        .map(|vote| vote.2)
                        .collect();
                    let votes = voter_ids.len() as i64;
                    if poll.anonymous {
                        voter_ids.clear();
                    } else {
                        voter_ids.sort_unstable();
                    }
                    Ok(api::PollOption {
                        id: option.id,
                        label: option.label.clone(),
                        votes,
                        voter_ids,
                        media: option.media(conn, verifier)?.map(serde_json::from_value).transpose().map_err(|e| campfire_db::Error::Other(e.to_string()))?,
                    })
                })
                .collect::<campfire_db::Result<_>>()?;
            let dto = api::Poll {
                id: poll.id,
                message_id: poll.message_id,
                as_of: as_of.clone(),
                multiple: poll.multiple,
                anonymous: poll.anonymous,
                closes_at: poll.closes_at.map(dto::time),
                closed_at: poll.closed_at.map(dto::time),
                closed: poll.closed(now),
                total_votes: poll_votes.len() as i64,
                options,
            };
            Ok((poll.message_id, dto))
        })
        .collect::<campfire_db::Result<_>>()
}

/// The options `user_id` chose on the poll, in option order.
fn my_option_ids(conn: &Connection, poll_id: i64, user_id: i64) -> campfire_db::Result<Vec<i64>> {
    let mut statement = conn.prepare_cached(
        r#"SELECT "poll_votes"."poll_option_id" FROM "poll_votes" INNER JOIN "poll_options" ON "poll_options"."id" = "poll_votes"."poll_option_id" WHERE "poll_votes"."poll_id" = ? AND "poll_votes"."user_id" = ? ORDER BY "poll_options"."position", "poll_options"."id""#,
    )?;
    let rows = statement.query_map([poll_id, user_id], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn poll_results(
    conn: &Connection,
    poll: &Poll,
    viewer_id: i64,
    now: Timestamp,
    verifier: &dyn campfire_storage::Verifier,
) -> campfire_db::Result<api::PollResults> {
    let poll_dto = polls(conn, &[poll.message_id], now, verifier)?
        .remove(&poll.message_id)
        .ok_or(campfire_db::Error::RecordNotFound("Poll"))?;
    Ok(api::PollResults {
        poll: poll_dto,
        my_option_ids: my_option_ids(conn, poll.id, viewer_id)?,
    })
}

/// `poll.updated` for the poll, and `voter_id`'s `poll.ballot` when given; `None` when the poll
/// or its message is gone.
pub(crate) fn poll_changed(
    conn: &Connection,
    poll_id: i64,
    voter_id: Option<i64>,
    now: Timestamp,
    verifier: &dyn campfire_storage::Verifier,
) -> campfire_db::Result<Option<(api::PollUpdated, Option<api::PollBallot>)>> {
    let poll = match Poll::find(conn, poll_id) {
        Ok(poll) => poll,
        Err(campfire_db::Error::RecordNotFound(_)) => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(message) = Message::find_by_id(conn, poll.message_id)? else {
        return Ok(None);
    };
    let Some(dto) = polls(conn, &[message.id], now, verifier)?.remove(&message.id) else {
        return Ok(None);
    };
    let ballot = match voter_id {
        Some(voter_id) => Some(api::PollBallot {
            poll_id: poll.id,
            message_id: message.id,
            room_id: message.room_id,
            thread_id: message.thread_id,
            my_option_ids: my_option_ids(conn, poll.id, voter_id)?,
            as_of: dto.as_of.clone(),
        }),
        None => None,
    };
    let updated = api::PollUpdated {
        room_id: message.room_id,
        thread_id: message.thread_id,
        poll: dto,
    };
    Ok(Some((updated, ballot)))
}

fn present(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| !campfire_richtext::ruby::is_blank(value))
        .map(str::to_string)
}

/// Each message's cards, in the classic slot order (Drive attachments, GitHub pull requests,
/// posts on X, events, Fizzy cards, quoted messages, LinkedIn posts, other pages); within a kind,
/// in the order the classic slot lists them. Records the fetches the classic render asks for.
pub(crate) fn cards(
    presenter: &Presenter<'_>,
    conn: &Connection,
    messages: &[Message],
    now: Timestamp,
    fetches: &mut Fetches,
) -> campfire_db::Result<HashMap<i64, Vec<api::MessageCard>>> {
    let ids: Vec<i64> = messages.iter().map(|message| message.id).collect();
    let mut cards: HashMap<i64, Vec<api::MessageCard>> = HashMap::new();
    let mut push = |message_id: i64, card: api::MessageCard| {
        cards.entry(message_id).or_default().push(card);
    };

    // Google Drive attachments (`messages/_drive_attachments.html`).
    let drive: Vec<(i64, String)> = ids_query(
        conn,
        r#"SELECT "message_id", "file_id" FROM "drive_attachments" WHERE "message_id" IN ({}) ORDER BY "id""#,
        &ids,
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut drive_by_message: HashMap<i64, Vec<String>> = HashMap::new();
    for (message_id, file_id) in drive {
        drive_by_message
            .entry(message_id)
            .or_default()
            .push(file_id);
    }

    let mut pull_requests = PullRequest::for_messages(conn, &ids)?;
    let mut posts = Post::for_messages(conn, &ids)?;
    let mut events = CalendarEvent::for_message_ids(conn, &ids)?;
    let mut fizzy = FizzyCard::for_messages(conn, &ids)?;
    let shown: Vec<i64> = messages
        .iter()
        .filter(|message| !message.embeds_suppressed)
        .map(|message| message.id)
        .collect();
    let mut links = LinkReference::for_messages(conn, &shown)?;

    // Events: their organizers' and venues' rows.
    let event_rows: Vec<&CalendarEvent> = events.values().flatten().collect();
    let venue_ids: Vec<i64> = event_rows
        .iter()
        .filter_map(|event| event.venue_room_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let venues: HashMap<i64, Room> = Room::for_ids(conn, &venue_ids)?
        .into_iter()
        .map(|room| (room.id, room))
        .collect();

    // Quotes: the sources, and for same-room ones their authors and rooms.
    let references: Vec<(i64, i64, i64)> = ids_query(
        conn,
        r#"SELECT "message_id", "id", "referenced_message_id" FROM "message_references" WHERE "message_id" IN ({}) ORDER BY "id""#,
        &ids,
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let source_ids: Vec<i64> = references
        .iter()
        .map(|reference| reference.2)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let sources: HashMap<i64, Message> = Message::for_ids(conn, &source_ids)?
        .into_iter()
        .map(|message| (message.id, message))
        .collect();
    let author_ids: Vec<i64> = sources
        .values()
        .map(|source| source.creator_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let authors: HashMap<i64, String> = User::where_ids(conn, &author_ids)?
        .into_iter()
        .map(|user| (user.id, user.name))
        .collect();
    let source_room_ids: Vec<i64> = sources
        .values()
        .map(|source| source.room_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let source_rooms: HashMap<i64, Room> = Room::for_ids(conn, &source_room_ids)?
        .into_iter()
        .map(|room| (room.id, room))
        .collect();
    let mut quotes: HashMap<i64, Vec<(i64, &Message)>> = HashMap::new();
    for (message_id, reference_id, source_id) in &references {
        // A reference whose source was deleted renders nothing, as in the classic slot.
        if let Some(source) = sources.get(source_id) {
            quotes
                .entry(*message_id)
                .or_default()
                .push((*reference_id, source));
        }
    }

    for message in messages {
        for file_id in drive_by_message.remove(&message.id).unwrap_or_default() {
            push(
                message.id,
                api::MessageCard::Drive(api::DriveFileCard {
                    url: format!("https://drive.google.com/open?id={file_id}"),
                    file_id,
                }),
            );
        }
        for pr in pull_requests.remove(&message.id).unwrap_or_default() {
            if pr.stale(now) {
                fetches.pull_requests.insert(pr.id);
            }
            push(
                message.id,
                api::MessageCard::Github(api::GithubCardRef {
                    pull_request_id: pr.id,
                    url: format!(
                        "https://github.com/{}/{}/pull/{}",
                        pr.owner, pr.repo, pr.number
                    ),
                    owner: pr.owner,
                    repo: pr.repo,
                    number: pr.number,
                }),
            );
        }
        let mut message_posts = posts.remove(&message.id).unwrap_or_default();
        Post::order_cards(&mut message_posts);
        for post in message_posts {
            if post.fetch_pending() {
                fetches.posts.insert(post.id);
            }
            push(message.id, api::MessageCard::X(x_post(post)));
        }
        for event in events.remove(&message.id).unwrap_or_default() {
            if event.room_id != message.room_id {
                continue;
            }
            let venue = event.venue_room_id.and_then(|id| venues.get(&id));
            push(
                message.id,
                api::MessageCard::Event(api::EventCard {
                    event_id: event.id,
                    room_id: event.room_id,
                    organizer_id: event.organizer_id,
                    starts_at: dto::time(event.starts_at),
                    ends_at: event.ends_at.map(dto::time),
                    recurring: event.series(),
                    cancelled: event.cancelled(),
                    venue_room_id: venue.map(|room| room.id),
                    venue_name: venue.and_then(|room| room.name.clone()),
                    meet_link: event
                        .meet_link
                        .as_deref()
                        .and_then(rails_compat::safe_https),
                    title: event.title,
                    time_zone: event.time_zone,
                }),
            );
        }
        for card in fizzy.remove(&message.id).unwrap_or_default() {
            push(
                message.id,
                api::MessageCard::Fizzy(api::FizzyCardRef {
                    fizzy_card_id: card.id,
                    url: card.web_url(),
                    account_id: card.account_id,
                    number: card.number,
                }),
            );
        }
        for (reference_id, source) in quotes.remove(&message.id).unwrap_or_default() {
            // Anyone who sees this message sees a source in its own room; another room's
            // source is fetched per viewer.
            let preview = if source.room_id == message.room_id {
                let room = source_rooms
                    .get(&source.room_id)
                    .ok_or(campfire_db::Error::RecordNotFound("Room"))?;
                Some(quote_preview(
                    presenter,
                    source,
                    room,
                    authors
                        .get(&source.creator_id)
                        .ok_or(campfire_db::Error::RecordNotFound("User"))?,
                )?)
            } else {
                None
            };
            push(
                message.id,
                api::MessageCard::Quote(api::QuoteCard {
                    reference_id,
                    preview,
                }),
            );
        }
        let message_links = links.remove(&message.id).unwrap_or_default();
        for reference in &message_links {
            if reference.embed.needs_fetch(now) {
                fetches.links.insert(reference.embed.id);
            }
        }
        // LinkedIn posts take their own slot, above the other pages.
        for reference in message_links.iter().filter(|r| r.embed.linkedin()) {
            let embed = &reference.embed;
            push(
                message.id,
                api::MessageCard::Linkedin(api::LinkedinCard {
                    url: reference.display_url().to_string(),
                    title: embed.title.clone(),
                    description: embed.description.clone(),
                    image_url: embed.image_url.clone(),
                    embed_url: campfire_app::integrations::linkedin::embed_url_for(
                        reference.display_url(),
                    ),
                }),
            );
        }
        for reference in message_links
            .iter()
            .filter(|r| !r.embed.linkedin() && r.embed.usable())
        {
            let embed = &reference.embed;
            push(
                message.id,
                api::MessageCard::Link(api::LinkCard {
                    url: reference.display_url().to_string(),
                    site_name: embed.site_name.clone(),
                    title: embed.title.clone(),
                    description: embed.description.clone(),
                    image_url: embed.image_url.clone(),
                }),
            );
        }
    }
    Ok(cards)
}

/// `campfire_presentation::message_links::Card`'s facts.
fn quote_preview(
    presenter: &Presenter<'_>,
    source: &Message,
    room: &Room,
    author_name: &str,
) -> campfire_db::Result<api::QuotePreview> {
    Ok(api::QuotePreview {
        message_id: source.id,
        room_id: source.room_id,
        thread_id: source.thread_id,
        creator_id: source.creator_id,
        author_name: author_name.to_string(),
        room_label: if room.direct() {
            "a direct message".into()
        } else {
            room.name.clone().unwrap_or_default()
        },
        excerpt: campfire_presentation::helpers::truncate(&presenter.plain_text_body(source)?, 200, "..."),
        created_at: dto::time(source.created_at),
    })
}

/// The X card (`presenters/twitter_cards.rs`, `campfire_presentation::twitter::Card`): a failed fetch
/// wins over a stored one, as the classic card shows the error.
fn x_post(post: Post) -> api::XPostCard {
    let fetch = if present(post.fetch_error.as_deref()).is_some() {
        api::CardFetch::Failed
    } else if post.fetched_at.is_some() {
        api::CardFetch::Loaded
    } else {
        api::CardFetch::Loading
    };
    let url = post.view_url();
    let integer = |value: &Value| match value {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|value| value as i64)),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    };
    let string = |value: &Value| present(value.as_str());
    // The classic card shows the first four.
    let media = post
        .media
        .iter()
        .take(4)
        .filter_map(|item| {
            let kind = match item["type"].as_str()? {
                "photo" => api::XMediaKind::Photo,
                "video" => api::XMediaKind::Video,
                "gif" => api::XMediaKind::Gif,
                _ => return None,
            };
            Some(api::XMedia {
                kind,
                url: string(&item["url"]).unwrap_or_else(|| url.clone()),
                thumbnail_url: string(&item["thumbnail_url"]),
                width: integer(&item["width"]),
                height: integer(&item["height"]),
                alt: string(&item["alt"]),
            })
        })
        .collect();
    let quote = post
        .quote
        .as_ref()
        .and_then(Value::as_object)
        .filter(|quote| !quote.is_empty())
        .map(|quote| {
            let field = |name: &str| quote.get(name).and_then(Value::as_str);
            api::XQuote {
                // The classic card links only stored http(s) addresses.
                url: present(field("url"))
                    .filter(|url| url.starts_with("http://") || url.starts_with("https://")),
                author_name: present(field("author_name")),
                author_handle: present(field("author_handle")),
                text: present(field("text")),
            }
        });
    api::XPostCard {
        fetch,
        author_name: post.display_name(),
        author_handle: post.display_handle(),
        author_avatar_url: present(post.author_avatar_url.as_deref()),
        text: present(post.text.as_deref()),
        posted_at: post.posted_at.map(dto::time),
        replies: post.replies,
        reposts: post.reposts,
        likes: post.likes,
        media,
        quote,
        post_id: post.post_id,
        url,
    }
}

/// `message.cards` for each of `messages`, read now.
pub(crate) fn message_cards(
    conn: &Connection,
    app: &App,
    messages: &[Message],
) -> campfire_db::Result<Vec<api::MessageCards>> {
    let now = app.db.env().now();
    let presenter = Presenter::new(conn, app, None);
    // A twin asks for no fetches: the classic replaces it shadows ask for none either.
    let mut cards = cards(&presenter, conn, messages, now, &mut Fetches::default())?;
    let as_of = dto::time(now);
    Ok(messages
        .iter()
        .map(|message| api::MessageCards {
            message_id: message.id,
            room_id: message.room_id,
            thread_id: message.thread_id,
            cards: cards.remove(&message.id).unwrap_or_default(),
            as_of: as_of.clone(),
        })
        .collect())
}

// Polls

fn path_id(c: &Ctx, name: &str) -> Result<i64> {
    c.param_str(name)
        .and_then(cast_integer)
        .ok_or(Error::NotFound)
}

enum PreparedPollMedia {
    None,
    Emoji(String),
    Image(Box<campfire_storage::branding::Prepared>),
}

fn valid_poll_emoji(conn: &Connection, emoji: &str) -> campfire_db::Result<bool> {
    if emoji.is_ascii() {
        let name = emoji.strip_prefix(':').and_then(|s| s.strip_suffix(':')).unwrap_or(emoji);
        return Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM workspace_icons WHERE name=?)", [name], |r| r.get(0))?);
    }
    // One Unicode emoji sequence, including joined families, flags, skin tones and keycaps.
    static EMOJI: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"\A(?:(?:\p{Regional_Indicator}{2})|[0-9#*]\x{FE0F}?\x{20E3}|(?:\p{Emoji_Presentation}|\p{Extended_Pictographic})\x{FE0F}?\p{Emoji_Modifier}?(?:[\x{E0020}-\x{E007E}]+\x{E007F})?(?:\x{200D}(?:\p{Emoji_Presentation}|\p{Extended_Pictographic})\x{FE0F}?\p{Emoji_Modifier}?)*)\z").unwrap());
    Ok(EMOJI.is_match(emoji))
}

fn save_poll_media(tx: &mut campfire_db::Tx<'_>, storage: &campfire_storage::Storage, option: &campfire_db::PollOption, media: PreparedPollMedia) -> campfire_db::Result<()> {
    use campfire_runtime::presenters::attachments::storage_error;
    match media {
        PreparedPollMedia::None => Ok(()),
        PreparedPollMedia::Emoji(content) => option.attach_emoji(tx, &content),
        PreparedPollMedia::Image(prepared) => {
            let campfire_storage::branding::Prepared { mut blob, still } = *prepared;
            blob.metadata.set("poll_media", campfire_storage::Json::Bool(true));
            tx.conn().execute("UPDATE active_storage_blobs SET metadata=?, content_type=? WHERE id=?", rusqlite::params![blob.metadata.encode(), blob.content_type, blob.id])?;
            if let Some((variation, still)) = still {
                blob.metadata.set("poll_still", campfire_storage::Json::Bool(true));
                blob.update_metadata(tx.conn(), blob.metadata.clone()).map_err(storage_error)?;
                if storage.record_variant(tx.conn(), &blob, &variation, &still, tx.now().jiff()).map_err(storage_error)?.is_some() {
                    campfire_runtime::active_storage::keep_after_commit(tx, still);
                }
            }
            option.attach_media(tx, blob.id)
        }
    }
}

async fn poll_image(c: &mut Ctx, signed: &str) -> Result<campfire_storage::branding::Prepared> {
    use campfire_runtime::presenters::attachments::Assignment;
    use campfire_storage::branding::{self, Kind};
    let limit = campfire_runtime::active_storage::upload_limit_bytes(c.app()).await? as u64;
    let assignment = Assignment::Signed(signed.to_owned()).stage_with_limit(c.app(), limit).await
        .map_err(|_| fail(c, validation("optionMedia", "isn't a valid upload or exceeds the workspace upload limit")))?;
    let Assignment::Existing(blob) = assignment else { return Err(fail(c, validation("optionMedia", "isn't an uploaded image"))); };
    if !matches!(blob.content_type(), "image/png" | "image/jpeg" | "image/gif" | "image/webp") {
        return Err(fail(c, validation("optionMedia", "must be a PNG, JPEG, GIF or WebP image")));
    }
    let storage = c.app().storage.clone();
    let prepared = campfire_runtime::active_storage::process_branding_with_deadline(
        branding::processing_timeout(&blob),
        move |cancel| Ok(branding::prepare_with_limit(&storage, blob, Kind::Logo, &cancel, limit)),
    ).await.map_err(|_| fail(c, validation("optionMedia", "couldn't be read as an image")))?;
    prepared.map_err(|invalid| {
        let message = match invalid {
            branding::Invalid::Size => "exceeds the workspace upload limit",
            other => other.message(Kind::Logo),
        };
        fail(c, validation("optionMedia", message))
    })
}

async fn post_poll(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    features::active_human(c)?;
    let input: api::CreatePoll = body(c).await?;
    let client_message_id = input.client_message_id.trim().to_string();
    if client_message_id.is_empty() {
        return Err(fail(c, validation("clientMessageId", "can't be blank")));
    }
    let (room_id, creator_id) = (room.id, concerns::require_current_user(c)?.id);
    // `Message.find_duplicate` first: a retry gets the question the first attempt posted, even
    // once its `closesAt` has passed. The write checks again, for retries racing each other.
    let lookup = client_message_id.clone();
    let duplicate = c
        .app()
        .db
        .read(move |conn| Message::find_duplicate(conn, room_id, creator_id, &lookup))
        .await
        .map_err(db_error)?;
    if let Some(message) = duplicate {
        if message.thread_id != input.thread_id {
            return Err(fail(
                c,
                validation("clientMessageId", "is already used in another conversation"),
            ));
        }
        return render_message(c, message, StatusCode::OK).await;
    }
    #[cfg(feature = "test-support")]
    crate::test_hooks::after_duplicate_check(&client_message_id).await;
    let question = input.question;
    if question.trim().is_empty() {
        return Err(fail(c, validation("question", "can't be blank")));
    }
    if question.chars().count() > campfire_db::message::SOURCE_LIMIT {
        return Err(fail(
            c,
            validation(
                "question",
                &format!(
                    "is too long (maximum is {} characters)",
                    campfire_db::message::SOURCE_LIMIT
                ),
            ),
        ));
    }
    let labels = Poll::normalize_labels(&input.options);
    if !(MIN_OPTIONS..=MAX_OPTIONS).contains(&labels.len()) {
        return Err(fail(
            c,
            validation(
                "options",
                &format!("need between {MIN_OPTIONS} and {MAX_OPTIONS} non-blank options"),
            ),
        ));
    }
    if labels
        .iter()
        .any(|label| label.chars().count() > LABEL_LIMIT)
    {
        return Err(fail(
            c,
            validation(
                "options",
                &format!("can't be longer than {LABEL_LIMIT} characters each"),
            ),
        ));
    }
    let media_inputs = input.option_media.unwrap_or_default();
    if !media_inputs.is_empty() && media_inputs.len() != input.options.len() {
        return Err(fail(c, validation("optionMedia", "must match the option labels")));
    }
    let mut media = Vec::new();
    for (index, label) in input.options.iter().enumerate() {
        let choice = media_inputs.get(index).and_then(Option::as_ref);
        if campfire_richtext::ruby::is_blank(label) {
            if choice.is_some() {
                return Err(fail(c, validation("optionMedia", "needs an option label")));
            }
            continue;
        }
        let prepared = match choice {
            None => PreparedPollMedia::None,
            Some(choice) => match (&choice.signed_id, &choice.emoji) {
                (Some(signed), None) => {
                    let mut prepared = poll_image(c, signed).await?;
                    if prepared.blob.metadata.get("uploader_id").and_then(campfire_storage::Json::as_i64).is_some_and(|id| id != creator_id) {
                        return Err(fail(c, validation("optionMedia", "must be your uploaded image")));
                    }
                    // PNG can contain APNG animation even though its MIME type says PNG.
                    if prepared.still.is_none() {
                        let storage = c.app().storage.clone();
                        let blob = prepared.blob.clone();
                        let variation = campfire_storage::branding::Kind::Logo.still_variation();
                        let transform = variation.clone();
                        let still = campfire_runtime::active_storage::process_media(move || storage.transform_variant(&blob, &transform)).await.map_err(|_| fail(c, validation("optionMedia", "couldn't be read as an image")))?;
                        prepared.still = Some((variation, still));
                    }
                    PreparedPollMedia::Image(Box::new(prepared))
                }
                (None, Some(emoji)) => {
                    let mut emoji = emoji.trim().to_string();
                    let value = emoji.clone();
                    let valid = c.app().db.read(move |conn| valid_poll_emoji(conn, &value)).await.map_err(db_error)?;
                    if !valid { return Err(fail(c, validation("optionMedia", "must be a Unicode emoji or an existing workspace emoji"))); }
                    if emoji.is_ascii() && !emoji.starts_with(':') { emoji = format!(":{emoji}:"); }
                    PreparedPollMedia::Emoji(emoji)
                }
                _ => return Err(fail(c, validation("optionMedia", "choose one image or emoji"))),
            },
        };
        media.push(prepared);
    }
    let closes_at = match input.closes_at.as_deref() {
        None => None,
        Some(raw) => match raw.parse::<jiff::Timestamp>() {
            Ok(time) => Some(Timestamp::from_jiff(time)),
            Err(_) => return Err(fail(c, validation("closesAt", "is invalid"))),
        },
    };
    if closes_at.is_some_and(|closes_at| closes_at <= now(c)) {
        return Err(fail(c, validation("closesAt", "must be in the future")));
    }
    let poll = NewPoll {
        labels,
        multiple: input.multiple,
        anonymous: input.anonymous,
        closes_at,
    };
    let posted_room = room.clone();
    let storage = c.app().storage.clone();
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            if let Some(message) =
                Message::find_duplicate(tx.conn(), room_id, creator_id, &client_message_id)?
            {
                return Ok(if message.thread_id == input.thread_id {
                    Ok((message, false))
                } else {
                    Err(validation(
                        "clientMessageId",
                        "is already used in another conversation",
                    ))
                });
            }
            // Claim only fresh uploads. Reusing a room's attachment would share its signed URL.
            for image in &media {
                if let PreparedPollMedia::Image(prepared) = image {
                    let attached: bool = tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM active_storage_attachments WHERE blob_id=?)", [prepared.blob.id], |r| r.get(0))?;
                    if attached { return Ok(Err(validation("optionMedia", "must be a new uploaded image"))); }
                }
            }
            let thread = match input.thread_id {
                Some(id) => {
                    let Some(thread) = ChannelThread::find_by_id(tx.conn(), id)? else {
                        return Ok(Err(not_found()));
                    };
                    if thread.room_id != room_id {
                        return Ok(Err(not_found()));
                    }
                    if thread.status_in_room(&posted_room, tx.now())
                        != campfire_db::channel_thread::ThreadStatus::Active
                    {
                        return Ok(Err(api::ApiError::Forbidden {
                            message: "This thread is closed or locked".into(),
                        }));
                    }
                    Some(thread)
                }
                None => None,
            };
            // `rooms/polls#create`: the question, its poll and the bots' webhooks in one write.
            let attributes = NewMessage {
                room_id,
                creator_id,
                client_message_id: Some(client_message_id),
                markdown_source: Some(question),
                ..Default::default()
            };
            let message = match thread {
                Some(mut thread) => thread.post_message(tx, creator_id, attributes)?,
                None => Message::create(tx, attributes)?,
            };
            let poll = Poll::create_for_message(tx, &message, poll)?;
            for (option, media) in poll.options(tx.conn())?.into_iter().zip(media) {
                save_poll_media(tx, &storage, &option, media)?;
            }
            posting::deliver_webhooks_to_bots(tx, &posted_room, &message)?;
            Ok(Ok((message, true)))
        })
        .await
        .map_err(db_error)?;
    let (message, created) = outcome.map_err(|error| fail(c, error))?;
    if !created {
        return render_message(c, message, StatusCode::OK).await;
    }
    posting::broadcast_create(c, &room, &message).await?;
    posting::release_webhooks(c, &message).await;
    render_message(c, message, StatusCode::CREATED).await
}

async fn render_message(c: &mut Ctx, message: Message, status: StatusCode) -> Result {
    let app = c.app().clone();
    let dto = c
        .app()
        .db
        .read(move |conn| dto::message(conn, &app, &message))
        .await
        .map_err(db_error)?;
    c.json(status, &dto)
}

/// `rooms/polls`' `prepare` and `set_poll`: an active human member of the alive room, and its
/// poll (404).
async fn set_poll(c: &mut Ctx) -> Result<(Room, i64, i64)> {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    features::active_human(c)?;
    let poll_id = path_id(c, "poll_id")?;
    let viewer_id = concerns::require_current_user(c)?.id;
    Ok((room, poll_id, viewer_id))
}

async fn show_poll(c: &mut Ctx) -> Result {
    let (room, poll_id, viewer_id) = set_poll(c).await?;
    let app = c.app().clone();
    let results = c
        .app()
        .db
        .read(move |conn| {
            let poll = Poll::find_in_room(conn, room.id, poll_id)?;
            poll_results(conn, &poll, viewer_id, app.db.env().now(), &*app.storage.verifier)
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &results)
}

async fn post_vote(c: &mut Ctx) -> Result {
    let (room, poll_id, viewer_id) = set_poll(c).await?;
    let api::VotePoll { option_ids } = body(c).await?;
    let room_id = room.id;
    #[cfg(feature = "test-support")]
    crate::test_hooks::before_poll_vote_write(poll_id).await;
    let verifier = c.app().storage.verifier.clone();
    let outcome = c
        .app()
        .db
        .write(
            move |tx| {
                // Votes on a poll take turns in the writer: each reads the poll it changes.
                let mut poll = Poll::find_in_room(tx.conn(), room_id, poll_id)?;
                if poll.closed(tx.now()) {
                    return Ok(Err(validation("poll", "is closed")));
                }
                let mut ids: Vec<i64> = Vec::new();
                for id in option_ids {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
                let options: Vec<i64> = poll
                    .options(tx.conn())?
                    .into_iter()
                    .map(|option| option.id)
                    .collect();
                if ids.iter().any(|id| !options.contains(id)) {
                    return Ok(Err(validation(
                        "optionIds",
                        "aren't all options of this poll",
                    )));
                }
                if poll.single() && ids.len() > 1 {
                    return Ok(Err(validation("optionIds", "can name only one option")));
                }
                poll.cast_vote(tx, viewer_id, &ids)?;
                poll_results(tx.conn(), &poll, viewer_id, tx.now(), &*verifier).map(Ok)
            },
        )
        .await
        .map_err(db_error)?;
    match outcome {
        Ok(results) => c.json(StatusCode::OK, &results),
        Err(error) => Err(fail(c, error)),
    }
}

async fn post_end_poll(c: &mut Ctx) -> Result {
    let (room, poll_id, viewer_id) = set_poll(c).await?;
    let verifier = c.app().storage.verifier.clone();
    let outcome = c
        .app()
        .db
        .write(
            move |tx| {
                let mut poll = Poll::find_in_room(tx.conn(), room.id, poll_id)?;
                let message = Message::find(tx.conn(), poll.message_id)?;
                let viewer = User::find(tx.conn(), viewer_id)?;
                if message.creator_id != viewer_id && !viewer.is_administrator() {
                    return Ok(Err(api::ApiError::Forbidden {
                        message: "Only the poll author or an administrator can end this poll"
                            .into(),
                    }));
                }
                poll.close(tx, tx.now())?;
                poll_results(tx.conn(), &poll, viewer_id, tx.now(), &*verifier).map(Ok)
            },
        )
        .await
        .map_err(db_error)?;
    match outcome {
        Ok(results) => c.json(StatusCode::OK, &results),
        Err(error) => Err(fail(c, error)),
    }
}

// Events

/// `rooms/events/attendances`' `set_event`: an active human member of the alive room, and an
/// event in it (404).
async fn set_event(c: &mut Ctx) -> Result<(CalendarEvent, User)> {
    let (_, event, viewer) = crate::events::scoped_event(c).await?;
    Ok((event, viewer))
}

fn attendance_of(
    conn: &Connection,
    event: &CalendarEvent,
    viewer: &User,
) -> campfire_db::Result<api::EventAttendance> {
    let counts = event.attendance_counts(conn)?;
    let count = |response: &str| counts.get(response).copied().unwrap_or(0);
    Ok(api::EventAttendance {
        event_id: event.id,
        response: event
            .response_for(conn, Some(viewer.id))?
            .as_deref()
            .and_then(response_of),
        going_count: count("going"),
        maybe_count: count("maybe"),
        declined_count: count("declined"),
        respondable: event.respondable_by(conn, Some(viewer))?,
        can_apply_to_future: event.series_head()
            || (event.series() && event.next_occurrence(conn)?.is_some()),
    })
}

pub(crate) fn response_of(stored: &str) -> Option<api::AttendanceResponse> {
    match stored {
        "going" => Some(api::AttendanceResponse::Going),
        "maybe" => Some(api::AttendanceResponse::Maybe),
        "declined" => Some(api::AttendanceResponse::Declined),
        _ => None,
    }
}

async fn show_attendance(c: &mut Ctx) -> Result {
    let (event, viewer) = set_event(c).await?;
    let attendance = c
        .app()
        .db
        .read(move |conn| attendance_of(conn, &event, &viewer))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &attendance)
}

async fn put_attendance(c: &mut Ctx) -> Result {
    let (event, viewer) = set_event(c).await?;
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Input {
        #[serde(default)]
        response: String,
        #[serde(default)]
        apply_to_future: bool,
    }
    let input: Input = body(c).await?;
    let response = input.response;
    if !RESPONSES.contains(&response.as_str()) {
        return Err(fail(
            c,
            attendance_error("Choose going, maybe, or declined."),
        ));
    };
    let (event_id, room_id) = (event.id, event.room_id);
    #[cfg(feature = "test-support")]
    crate::test_hooks::before_attendance_write(event_id).await;
    let outcome = c
        .app()
        .db
        .write(move |tx| {
            // Responses to an event take turns in the writer: each reads the event it answers,
            // so one racing a cancellation can't land on a cancelled event.
            let event = CalendarEvent::find_visible(tx.conn(), room_id, event_id, viewer.id)?;
            if !event.respondable_by(tx.conn(), Some(&viewer))? {
                return Ok(Err(attendance_error(
                    "This event is no longer open for responses.",
                )));
            }
            CalendarEvent::respond(tx, event.id, viewer.id, &response, input.apply_to_future)?;
            attendance_of(tx.conn(), &event, &viewer).map(Ok)
        })
        .await
        .map_err(|error| match error {
            campfire_db::Error::RecordInvalid(errors) => {
                fail(c, crate::error::record_invalid(&errors, &[]))
            }
            other => db_error(other),
        })?;
    match outcome {
        Ok(attendance) => c.json(StatusCode::OK, &attendance),
        Err(error) => Err(fail(c, error)),
    }
}

fn attendance_error(message: &str) -> api::ApiError {
    api::ApiError::Validation {
        message: message.into(),
        fields: std::collections::BTreeMap::from([("response".into(), vec![message.into()])]),
    }
}

// Per-viewer previews

/// A query parameter's id; `None` when it's absent or blank, a 404 when it isn't an integer.
fn query_id(c: &Ctx, name: &str) -> Result<Option<i64>> {
    match c
        .param_str(name)
        .filter(|value| !campfire_richtext::ruby::is_blank(value))
    {
        None => Ok(None),
        Some(value) => cast_integer(value).map(Some).ok_or(Error::NotFound),
    }
}

async fn show_github_card(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let id = path_id(c, "id")?;
    // `messageId` wins when both are given, as in the classic frame.
    let (message_id, thread_id) = match query_id(c, "messageId")? {
        Some(message_id) => (Some(message_id), None),
        None => (None, query_id(c, "threadId")?),
    };
    if message_id.is_none() && thread_id.is_none() {
        return Err(fail(c, not_found()));
    }
    let room_id = room.id;
    let context = c
        .app()
        .db
        .read(move |conn| {
            pull_requests::viewer_card_context(conn, room_id, id, message_id, thread_id)
        })
        .await
        .map_err(db_error)?;
    let viewer_id = concerns::require_current_user(c)?.id;
    let visible = context
        .pull_request
        .visible_to(&c.app().db, &c.app().github_accounts, Some(viewer_id))
        .await
        .map_err(db_error)?;
    if !visible {
        return c.json(StatusCode::OK, &api::GithubPullRequestCard::Hidden);
    }
    let thread = context.thread.is_some();
    let pull_request = context.pull_request;
    let card = c
        .app()
        .db
        .read(move |conn| {
            let card = if thread {
                presenters::github::card_with_files(conn, &pull_request, room_id)?
            } else {
                presenters::github::card(conn, &pull_request, room_id)?
            };
            Ok(github_card_of(card, thread))
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &card)
}

/// `campfire_presentation::github::card`'s states: a failed fetch, a fetched title, or still loading.
fn github_card_of(card: campfire_presentation::github::Card, thread: bool) -> api::GithubPullRequestCard {
    if let Some(message) = present(card.fetch_error.as_deref()) {
        return api::GithubPullRequestCard::Failed { message };
    }
    let Some(title) = present(card.title.as_deref()) else {
        return api::GithubPullRequestCard::Loading;
    };
    let status = match card.state.as_deref() {
        Some("merged") => api::GithubPullRequestStatus::Merged,
        Some("closed") => api::GithubPullRequestStatus::Closed,
        Some("draft") => api::GithubPullRequestStatus::Draft,
        _ => api::GithubPullRequestStatus::Open,
    };
    let review = match card.review_decision.as_deref() {
        Some("approved") => Some(api::GithubReview::Approved),
        Some("changes_requested") => Some(api::GithubReview::ChangesRequested),
        Some("review_required") => Some(api::GithubReview::ReviewRequired),
        _ => None,
    };
    let checks = match card.check_status.as_deref() {
        Some("passing") => Some(api::GithubChecks::Passing),
        Some("pending") => Some(api::GithubChecks::Pending),
        Some("failing") => Some(api::GithubChecks::Failing),
        _ => None,
    };
    let url = present(card.html_url.as_deref()).unwrap_or_else(|| {
        format!(
            "https://github.com/{}/{}/pull/{}",
            card.owner, card.repo, card.number
        )
    });
    // The thread header's list, once GitHub reported it; "Loading files…" until then.
    let files = (thread && card.files_loaded).then(|| api::GithubChangedFiles {
        files: card
            .files
            .iter()
            .map(|file| api::GithubChangedFile {
                filename: file.filename.clone(),
                status: file.status.clone(),
                additions: file.additions,
                deletions: file.deletions,
            })
            .collect(),
        total_count: card.files_total,
    });
    api::GithubPullRequestCard::Loaded(Box::new(api::GithubPullRequest {
        title,
        url,
        status,
        author_login: present(card.author_login.as_deref()),
        author_avatar_url: present(card.author_avatar_url.as_deref()),
        base_branch: present(card.base_branch.as_deref()),
        head_branch: present(card.head_branch.as_deref()),
        review,
        checks,
        github_updated_at: card
            .github_updated_at
            .map(|at| dto::time(Timestamp::from_jiff(at))),
        discussion_thread_id: card.discussion_thread,
        files,
        owner: card.owner,
        repo: card.repo,
        number: card.number,
    }))
}

async fn show_fizzy_card(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let viewer_id = concerns::require_current_user(c)?.id;
    let (_, room) = set_room(c).await?;
    let card_id = path_id(c, "id")?;
    let Some(message_id) = query_id(c, "messageId")? else {
        return Err(fail(c, not_found()));
    };
    let app = c.app().clone();
    let crypto = ArEncryption::new(&app.secrets);
    let room_id = room.id;
    // `rooms/fizzy/cards#show`: the reference checks, then the viewer's cache and a refresh of
    // it, in one write.
    let (card, cache, zone) = app
        .db
        .write(move |tx| {
            let card = FizzyCard::find(tx.conn(), card_id)?;
            let message = Message::find(tx.conn(), message_id)?;
            let referenced: bool = tx.conn().query_row(
                "SELECT EXISTS(SELECT 1 FROM fizzy_card_references WHERE message_id=?1 AND fizzy_card_id=?2)",
                rusqlite::params![message.id, card.id],
                |row| row.get(0),
            )?;
            if message.room_id != room_id || !referenced {
                return Err(campfire_db::Error::RecordNotFound("Fizzy::CardReference"));
            }
            let usable = match FizzyAccount::for_user(tx.conn(), viewer_id)? {
                Some(account) => account.usable_token(tx, &crypto)?.is_some(),
                None => false,
            };
            let cache = if usable {
                let cache = FizzyCache::for_viewer(tx, &card, viewer_id)?;
                cache.request_fetch(tx)?;
                Some(cache)
            } else {
                None
            };
            let zone: Option<String> = tx.conn().query_row(
                "SELECT time_zone FROM users WHERE id=?",
                [viewer_id],
                |row| row.get(0),
            )?;
            Ok((card, cache, zone))
        })
        .await
        .map_err(db_error)?;
    let zone = campfire_presentation::time::Zone::for_user(zone.as_deref());
    let preview = fizzy_preview(&card, cache.as_ref(), &zone);
    c.json(StatusCode::OK, &preview)
}

fn json_blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(text) => campfire_richtext::ruby::is_blank(text),
        Value::Array(items) => items.is_empty(),
        Value::Object(fields) => fields.is_empty(),
        _ => false,
    }
}

fn json_truthy(value: &Value) -> bool {
    !matches!(value, Value::Null | Value::Bool(false))
}

fn json_text(value: &Value) -> String {
    campfire_richtext::ruby::json_value_to_s(value)
}

fn json_array(value: &Value) -> Vec<&Value> {
    match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items.iter().collect(),
        _ => vec![value],
    }
}

/// An absolute HTTPS address with a host, as the classic Fizzy card accepts.
fn https(url: &str) -> bool {
    campfire_richtext::uri::parse(url).ok().is_some_and(|uri| {
        uri.scheme
            .as_deref()
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https"))
            && uri
                .host
                .is_some_and(|host| !campfire_richtext::ruby::is_blank(&host))
    })
}

/// `last_active_at` as `Time.zone.parse` reads it: an offset timestamp, or a civil date or time
/// in the viewer's zone.
fn fizzy_time(zone: &campfire_presentation::time::Zone, value: &Value) -> Option<Timestamp> {
    let text = json_text(value);
    let text = text.trim();
    if let Ok(at) = text.parse::<jiff::Timestamp>() {
        return Some(Timestamp::from_jiff(at));
    }
    let local = text.parse::<jiff::civil::DateTime>().ok().or_else(|| {
        text.parse::<jiff::civil::Date>()
            .ok()
            .map(|date| date.at(0, 0, 0, 0))
    });
    local?
        .to_zoned(zone.tz().clone())
        .ok()
        .map(|time| Timestamp::from_jiff(time.timestamp()))
}

/// `campfire_presentation::fizzy_cards::Frame`'s states, in its order: not connected, a payload, Fizzy's
/// "not found", another error, else loading. A payload without a title is still loading, as the
/// classic card shows "Loading card…".
fn fizzy_preview(
    card: &FizzyCard,
    cache: Option<&FizzyCache>,
    zone: &campfire_presentation::time::Zone,
) -> api::FizzyCardPreview {
    let Some(cache) = cache else {
        return api::FizzyCardPreview::NotConnected;
    };
    if let Some(payload) = cache
        .payload
        .as_ref()
        .filter(|payload| !json_blank(payload))
    {
        if json_blank(&payload["title"]) {
            return api::FizzyCardPreview::Loading;
        }
        let column = &payload["column"]["name"];
        let status = if json_truthy(&payload["closed"]) {
            api::FizzyCardStatus::Closed
        } else if json_truthy(&payload["postponed"]) {
            api::FizzyCardStatus::Postponed
        } else if !json_blank(column) {
            api::FizzyCardStatus::Column
        } else {
            api::FizzyCardStatus::Triage
        };
        let steps = json_array(&payload["steps"]);
        let url = json_text(&payload["url"]);
        return api::FizzyCardPreview::Loaded(api::FizzyCard {
            title: json_text(&payload["title"]),
            url: if https(&url) { url } else { card.web_url() },
            board_name: Some(json_text(&payload["board"]["name"]))
                .filter(|name| !campfire_richtext::ruby::is_blank(name)),
            column_name: (status == api::FizzyCardStatus::Column).then(|| json_text(column)),
            status,
            assignees: json_array(&payload["assignees"])
                .into_iter()
                .map(|assignee| {
                    let avatar = json_text(&assignee["avatar_url"]);
                    api::FizzyAssignee {
                        name: json_text(&assignee["name"]),
                        avatar_url: https(&avatar).then_some(avatar),
                    }
                })
                .collect(),
            has_more_assignees: json_truthy(&payload["has_more_assignees"]),
            tags: json_array(&payload["tags"])
                .into_iter()
                .map(json_text)
                .collect(),
            steps_total: steps.len() as i64,
            steps_completed: steps
                .iter()
                .filter(|step| json_truthy(&step["completed"]))
                .count() as i64,
            last_active_at: fizzy_time(zone, &payload["last_active_at"]).map(dto::time),
        });
    }
    match cache.fetch_error.as_deref() {
        Some(FIZZY_NOT_FOUND) => api::FizzyCardPreview::NotFound,
        Some(error) if !campfire_richtext::ruby::is_blank(error) => api::FizzyCardPreview::Failed {
            message: error.to_string(),
        },
        _ => api::FizzyCardPreview::Loading,
    }
}

async fn show_quote_card(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let (_, room) = set_room(c).await?;
    let viewer_id = concerns::require_current_user(c)?.id;
    let reference_id = path_id(c, "reference_id")?;
    let app = c.app().clone();
    let result = c
        .app()
        .db
        .read(move |conn| {
            let source = message_quote::source(conn, room.id, reference_id)?;
            if !message_quote::visible(conn, &source, viewer_id)? {
                return Ok(api::QuotePreviewResult::Hidden);
            }
            let source_room = Room::find(conn, source.room_id)?;
            let author = User::find(conn, source.creator_id)?.name;
            let presenter = Presenter::new(conn, &app, None);
            Ok(api::QuotePreviewResult::Loaded(quote_preview(
                &presenter,
                &source,
                &source_room,
                &author,
            )?))
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &result)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn fizzy(payload: Option<Value>, fetch_error: Option<&str>) -> api::FizzyCardPreview {
        let card = FizzyCard {
            id: 1,
            account_id: "parity".into(),
            number: 42,
        };
        let cache = FizzyCache {
            id: 1,
            card_id: 1,
            user_id: 1,
            payload,
            fetched_at: None,
            fetch_error: fetch_error.map(str::to_owned),
        };
        fizzy_preview(&card, Some(&cache), &campfire_presentation::time::Zone::utc())
    }

    #[test]
    fn fizzy_previews_follow_the_cache() {
        let card = FizzyCard {
            id: 1,
            account_id: "parity".into(),
            number: 42,
        };
        assert_eq!(
            fizzy_preview(&card, None, &campfire_presentation::time::Zone::utc()),
            api::FizzyCardPreview::NotConnected
        );
        assert_eq!(fizzy(None, None), api::FizzyCardPreview::Loading);
        assert_eq!(
            fizzy(Some(json!({})), Some("")),
            api::FizzyCardPreview::Loading
        );
        assert_eq!(
            fizzy(None, Some(FIZZY_NOT_FOUND)),
            api::FizzyCardPreview::NotFound
        );
        assert_eq!(
            fizzy(None, Some("Fizzy is down")),
            api::FizzyCardPreview::Failed {
                message: "Fizzy is down".into()
            }
        );
        // A payload without a title is still loading, whatever the last error was.
        assert_eq!(
            fizzy(Some(json!({"title": " "})), Some("Fizzy is down")),
            api::FizzyCardPreview::Loading
        );
        let api::FizzyCardPreview::Loaded(loaded) = fizzy(
            Some(json!({
                "title": "Ship it",
                "url": "javascript:alert(1)",
                "board": {"name": " "},
                "column": {"name": "Doing"},
                "assignees": [
                    {"name": "Ann", "avatar_url": "https://fizzy.example/a.png"},
                    {"name": "Bo", "avatar_url": "http://fizzy.example/b.png"},
                ],
                "has_more_assignees": true,
                "tags": ["launch"],
                "steps": [{"completed": true}, {"completed": false}],
                "last_active_at": "2026-03-02T16:00:00Z",
            })),
            Some("old error"),
        ) else {
            panic!("not loaded")
        };
        assert_eq!(loaded.title, "Ship it");
        assert_eq!(loaded.url, card.web_url());
        assert_eq!(loaded.board_name, None);
        assert_eq!(
            (loaded.status, loaded.column_name.as_deref()),
            (api::FizzyCardStatus::Column, Some("Doing"))
        );
        assert_eq!(
            loaded
                .assignees
                .iter()
                .map(|a| (a.name.as_str(), a.avatar_url.as_deref()))
                .collect::<Vec<_>>(),
            [("Ann", Some("https://fizzy.example/a.png")), ("Bo", None)]
        );
        assert!(loaded.has_more_assignees);
        assert_eq!(loaded.tags, ["launch"]);
        assert_eq!((loaded.steps_total, loaded.steps_completed), (2, 1));
        assert!(loaded.last_active_at.is_some());
        for (flags, status) in [
            (
                json!({"closed": true, "postponed": true}),
                api::FizzyCardStatus::Closed,
            ),
            (json!({"postponed": true}), api::FizzyCardStatus::Postponed),
            (json!({}), api::FizzyCardStatus::Triage),
        ] {
            let mut payload = flags;
            payload["title"] = json!("Ship it");
            let api::FizzyCardPreview::Loaded(loaded) = fizzy(Some(payload), None) else {
                panic!("not loaded")
            };
            assert_eq!((loaded.status, loaded.column_name), (status, None));
        }
    }

    fn github(card: campfire_presentation::github::Card, thread: bool) -> api::GithubPullRequestCard {
        github_card_of(
            campfire_presentation::github::Card {
                owner: "smart-data-ohio".into(),
                repo: "smartfire".into(),
                number: 42,
                ..card
            },
            thread,
        )
    }

    #[test]
    fn github_cards_follow_the_fetch() {
        use campfire_presentation::github::{Card, File};
        assert_eq!(
            github(Card::default(), false),
            api::GithubPullRequestCard::Loading
        );
        assert_eq!(
            github(
                Card {
                    title: Some("Port it".into()),
                    fetch_error: Some("Not Found".into()),
                    ..Card::default()
                },
                false
            ),
            api::GithubPullRequestCard::Failed {
                message: "Not Found".into()
            }
        );
        let loaded = |card: Card, thread: bool| match github(card, thread) {
            api::GithubPullRequestCard::Loaded(pr) => pr,
            other => panic!("{other:?}"),
        };
        let pr = loaded(
            Card {
                title: Some("Port it".into()),
                ..Card::default()
            },
            true,
        );
        assert_eq!(
            (pr.status, pr.review, pr.checks, pr.url.as_str()),
            (
                api::GithubPullRequestStatus::Open,
                None,
                None,
                "https://github.com/smart-data-ohio/smartfire/pull/42"
            )
        );
        // A thread header's files show once GitHub reported them; a message card has none.
        assert_eq!(pr.files, None);
        let card = Card {
            title: Some("Port it".into()),
            state: Some("merged".into()),
            review_decision: Some("changes_requested".into()),
            check_status: Some("failing".into()),
            files_loaded: true,
            files: vec![File {
                filename: "src/main.rs".into(),
                status: Some("modified".into()),
                additions: 3,
                deletions: 1,
            }],
            files_total: 7,
            ..Card::default()
        };
        let pr = loaded(card.clone(), true);
        assert_eq!(
            (pr.status, pr.review, pr.checks),
            (
                api::GithubPullRequestStatus::Merged,
                Some(api::GithubReview::ChangesRequested),
                Some(api::GithubChecks::Failing)
            )
        );
        let files = pr.files.expect("the thread's files");
        assert_eq!(files.total_count, 7);
        assert_eq!(files.files[0].filename, "src/main.rs");
        assert_eq!(loaded(card, false).files, None);
    }
}
