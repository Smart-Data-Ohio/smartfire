//! `Rooms::Fizzy::MessageCardsController`: viewer credentials and WS8 posting hooks.
use crate::{
    app::AppCtx,
    concerns::{self, Before, before_actions, cast_integer, require_current_user},
    controllers::{
        fizzy_connections, messages,
        presenters::page::{self, db_error},
    },
    integrations::{
        fizzy::{
            accounts::{Account, REJECTED_TOKEN_REASON},
            client::{self, Client, ErrorKind},
        },
        net::Network,
    },
};
use campfire_db::{ChannelThread, Message, NewMessage, Room, Timeline, User};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::Value;

pub(crate) fn blank(value: &Value) -> bool {
    crate::integrations::fizzy::blank(value)
}
pub struct Source {
    room_name: String,
    room: Room,
    thread: Option<ChannelThread>,
    message: Message,
    plain: String,
    creator: User,
    account: Option<Account>,
    token: Option<String>,
}
impl Source {
    fn path(&self) -> String {
        self.thread.as_ref().map_or_else(
            || campfire_routes::room(self.room.id),
            |t| format!("/rooms/{}/threads/{}", self.room.id, t.id),
        )
    }
    fn link(&self, c: &Ctx) -> String {
        let path = self.thread.as_ref().map_or_else(
            || campfire_routes::room_at_message(self.room.id, self.message.id),
            |t| {
                format!(
                    "/rooms/{}?thread={}&message_id={}",
                    self.room.id, t.id, self.message.id
                )
            },
        );
        c.url_for(&path)
    }
    fn client(&self) -> Client {
        Client::new(
            Network::system(),
            self.token.clone().expect("usable account"),
            &client::api_base_url(),
        )
    }
}
pub async fn source(c: &mut Ctx) -> Result<Source> {
    before_actions(c, Before::default()).await?;
    let viewer = require_current_user(c)?.id;
    let room = concerns::set_room(c).await?.1;
    let message = c
        .param_str("message_id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let thread = c
        .param_str("thread_id")
        .filter(|s| !s.is_empty())
        .map(|s| cast_integer(s).ok_or(Error::NotFound))
        .transpose()?;
    let crypto = ArEncryption::new(&c.app().secrets);
    let app = c.app().clone();
    c.app()
        .db
        .write(move |tx| {
            let thread = thread
                .map(|id| ChannelThread::find(tx.conn(), id))
                .transpose()?;
            if thread.as_ref().is_some_and(|t| t.room_id != room.id) {
                return Err(campfire_db::Error::RecordNotFound("ChannelThread"));
            }
            let message = Message::find_in(
                tx.conn(),
                thread
                    .as_ref()
                    .map_or(Timeline::Room(room.id), |t| Timeline::Thread(t.id)),
                message,
            )?;
            let plain = message.plain_text_body(tx.conn(), tx.rich_text())?;
            let creator = message.creator(tx.conn())?;
            let account = Account::for_user(tx.conn(), viewer)?;
            let token = account
                .as_ref()
                .map(|a| a.usable_token(tx, &crypto))
                .transpose()?
                .flatten();
            let user = User::find(tx.conn(), viewer)?;
            let room_name = crate::controllers::presenters::Presenter::new(tx.conn(), &app, None)
                .room_display_name(&room, Some(&user))?;
            Ok(Source {
                room_name,
                room,
                thread,
                message,
                plain,
                creator,
                account,
                token,
            })
        })
        .await
        .map_err(db_error)
}
fn redirect(c: &mut Ctx, source: &Source, notice: bool, message: &str) -> Result {
    c.flash()
        .set(if notice { "notice" } else { "alert" }, message);
    c.redirect_to(&c.url_for(&source.path()))
}
/// Expected Fizzy failures, independent of the classic redirect or JSON adapter.
#[derive(Debug)]
pub enum Failure {
    NotConnected,
    Locked,
    Invalid {
        boards: Value,
        board_missing: bool,
        title_missing: bool,
    },
    Unreachable,
    VerificationUnreachable,
    CreateUnreachable(String),
    Rejected,
    ReadOnly,
    Refused(String),
    ReplyFailed {
        number: String,
        url: String,
        details: String,
    },
}
impl Failure {
    pub fn reply_failed(number: String, url: String, error: Error) -> Self {
        tracing::error!(%error, %number, %url, "Fizzy card created, but its local reply failed");
        Self::ReplyFailed {
            number,
            url,
            details: "A local error prevented posting the reply".into(),
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::NotConnected => "Connect Fizzy on your profile first.".into(),
            Self::Locked => "This thread is locked".into(),
            Self::Invalid { .. } => "Choose a board and enter a title.".into(),
            Self::Unreachable => "Could not reach Fizzy. Try again.".into(),
            Self::VerificationUnreachable => {
                "Could not reach Fizzy to verify the token. Try again.".into()
            }
            Self::Rejected => "Fizzy rejected the linked token. Reconnect on your profile.".into(),
            Self::ReadOnly => {
                "That Fizzy token is read-only. Generate a Read + Write token to create cards."
                    .into()
            }
            Self::Refused(message) | Self::CreateUnreachable(message) => {
                format!("Fizzy refused the new card ({message}).")
            }
            Self::ReplyFailed {
                number, details, ..
            } => format!(
                "Fizzy card #{number} created, but the reply could not be posted ({details})."
            ),
        }
    }
}
pub type Outcome<T> = Result<std::result::Result<T, Failure>>;

impl Source {
    pub fn connected(&self) -> bool {
        self.token.is_some()
    }
}

pub async fn boards(c: &mut Ctx, source: &Source) -> Outcome<Value> {
    match source
        .client()
        .boards(&source.account.as_ref().unwrap().account_id)
        .await
    {
        Ok(value) => Ok(Ok(value)),
        Err(error) if error.kind == ErrorKind::Unauthorized => {
            mark_rejected(c, source).await?;
            Ok(Err(Failure::Rejected))
        }
        Err(_) => Ok(Err(Failure::Unreachable)),
    }
}
async fn mark_rejected(c: &mut Ctx, source: &Source) -> Result<()> {
    let account = source.account.clone().expect("usable account");
    c.app()
        .db
        .write(move |tx| account.mark_disconnected(tx, REJECTED_TOKEN_REASON))
        .await
        .map_err(db_error)
}
fn classic_failure(c: &mut Ctx, source: &Source, failure: &Failure) -> Result {
    let message = failure.message();
    if matches!(failure, Failure::NotConnected | Failure::Rejected) {
        fizzy_connections::redirect(c, false, &message)
    } else {
        redirect(c, source, false, &message)
    }
}
pub async fn new(c: &mut Ctx) -> Result {
    let source = source(c).await?;
    let boards = if source.token.is_some() {
        match boards(c, &source).await? {
            Ok(boards) => boards,
            Err(failure) => return classic_failure(c, &source, &failure),
        }
    } else {
        Value::Null
    };
    render(c, source, boards, StatusCode::OK).await
}
/// The same defaults and display fields both forms use. Board names are a flat list.
pub fn form_view(
    c: &Ctx,
    source: &Source,
    boards: Value,
) -> campfire_views::fizzy_message_cards::FormView {
    let title = c.param_str("title").map(str::to_owned).unwrap_or_else(|| {
        campfire_richtext::ruby::truncate(
            campfire_richtext::ruby::strip(source.plain.lines().next().unwrap_or("")),
            120,
            "...",
        )
    });
    let description = c
        .param_str("description")
        .map(str::to_owned)
        .unwrap_or_else(|| {
            let link = format!("Source: {}", source.link(c));
            if campfire_richtext::ruby::is_blank(&source.plain) {
                link
            } else {
                format!("{}\n\n{link}", source.plain)
            }
        });
    campfire_views::fizzy_message_cards::FormView {
        back_path: source.path(),
        action: format!(
            "{}/messages/{}/fizzy_cards",
            source.path(),
            source.message.id
        ),
        room_name: source.room_name.clone(),
        plain: source.plain.clone(),
        creator: source.creator.name.clone(),
        connected: source.token.is_some(),
        boards,
        board_id: c.param_str("board_id").unwrap_or("").into(),
        title,
        description,
        user_name: source
            .account
            .as_ref()
            .and_then(|a| a.fizzy_user_name.clone())
            .unwrap_or_default(),
        account_name: source
            .account
            .as_ref()
            .and_then(|a| a.account_name.clone())
            .unwrap_or_default(),
    }
}
async fn render(c: &mut Ctx, source: Source, boards: Value, status: StatusCode) -> Result {
    let view = form_view(c, &source, boards);
    page::framed_page!(c, status, |ctx| campfire_views::fizzy_message_cards::New {
        ctx,
        view: &view
    })
    .await
}
pub struct Created {
    pub number: String,
    pub url: String,
    pub reply: Message,
}

/// Retains the remote card even when posting or broadcasting its local reply fails.
pub struct CardCreation {
    number: String,
    url: String,
    reply: Outcome<Message>,
}

impl CardCreation {
    pub fn into_classic(self) -> Outcome<Created> {
        Ok(self.reply?.map(|reply| Created {
            number: self.number,
            url: self.url,
            reply,
        }))
    }

    pub fn into_spa(self) -> std::result::Result<Created, Failure> {
        let details = match self.reply {
            Ok(Ok(reply)) => {
                return Ok(Created {
                    number: self.number,
                    url: self.url,
                    reply,
                });
            }
            Ok(Err(Failure::ReplyFailed { details, .. })) => details,
            Ok(Err(failure)) => failure.message(),
            Err(error) => return Err(Failure::reply_failed(self.number, self.url, error)),
        };
        tracing::error!(%details, number = %self.number, url = %self.url, "Fizzy card created, but its local reply failed");
        Err(Failure::ReplyFailed {
            number: self.number,
            url: self.url,
            details,
        })
    }
}

/// Creates in Fizzy and posts through the classic room/thread write, webhook and broadcast path.
pub async fn create_card(
    c: &mut Ctx,
    source: &Source,
    board: String,
    title: String,
    description: String,
) -> Outcome<CardCreation> {
    if source.token.is_none() {
        return Ok(Err(Failure::NotConnected));
    }
    if source
        .thread
        .as_ref()
        .is_some_and(|t| t.locked_at.is_some())
    {
        return Ok(Err(Failure::Locked));
    }
    let title = campfire_richtext::ruby::strip(&title).to_owned();
    let board_missing = campfire_richtext::ruby::is_blank(&board);
    let title_missing = campfire_richtext::ruby::is_blank(&title);
    if board_missing || title_missing {
        let boards = match boards(c, source).await? {
            Ok(boards) => boards,
            Err(failure) => return Ok(Err(failure)),
        };
        return Ok(Err(Failure::Invalid {
            boards,
            board_missing,
            title_missing,
        }));
    }
    let client = source.client();
    let card = match client
        .create_card(
            &source.account.as_ref().unwrap().account_id,
            &board,
            Value::String(title),
            Some(Value::String(description)),
        )
        .await
    {
        Ok(card) => card,
        Err(error) if error.kind == ErrorKind::Unauthorized => {
            let failure = match client.identity().await {
                Ok(_) => Failure::ReadOnly,
                Err(probe) if probe.kind == ErrorKind::Unauthorized => {
                    mark_rejected(c, source).await?;
                    Failure::Rejected
                }
                Err(_) => Failure::VerificationUnreachable,
            };
            return Ok(Err(failure));
        }
        Err(error) if error.message.starts_with("Could not reach Fizzy (") => {
            return Ok(Err(Failure::CreateUnreachable(error.message)));
        }
        Err(error) => return Ok(Err(Failure::Refused(error.message))),
    };
    let number = card["number"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| {
            if card["number"].is_null() {
                String::new()
            } else {
                card["number"].to_string()
            }
        });
    let url = card["url"]
        .as_str()
        .filter(|s| !campfire_richtext::ruby::is_blank(s))
        .map(str::to_owned)
        .unwrap_or_else(|| {
            format!(
                "{}/{}/cards/{number}",
                client::api_base_url(),
                source.account.as_ref().unwrap().account_id
            )
        });
    let reply = post_reply(c, source, &number, &url).await;
    Ok(Ok(CardCreation { number, url, reply }))
}

async fn post_reply(c: &Ctx, source: &Source, number: &str, url: &str) -> Outcome<Message> {
    let markdown = format!("Created from {}:\n{url}", source.link(c));
    let creator_id = require_current_user(c)?.id;
    let room = source.room.clone();
    let reply_to = source.message.id;
    let mut thread = source.thread.clone();
    let posted = c
        .app()
        .db
        .write(move |tx| {
            let attrs = NewMessage {
                room_id: room.id,
                creator_id,
                reply_to_message_id: Some(reply_to),
                markdown_source: Some(markdown),
                ..Default::default()
            };
            let message = if let Some(thread) = thread.as_mut() {
                thread.post_message(tx, creator_id, attrs)?
            } else {
                Message::create(tx, attrs)?
            };
            if message.thread_id.is_none() {
                messages::deliver_webhooks_to_bots(tx, &room, &message)?;
            }
            Ok(message)
        })
        .await;
    let reply = match posted {
        Ok(reply) => reply,
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            return Ok(Err(Failure::ReplyFailed {
                number: number.into(),
                url: url.into(),
                details: errors.full_messages().join(", "),
            }));
        }
        Err(campfire_db::Error::Other(message)) if message == "This thread is locked" => {
            return Ok(Err(Failure::Locked));
        }
        Err(error) => return Err(db_error(error)),
    };
    messages::broadcast_create(c, &source.room, &reply).await?;
    if reply.thread_id.is_none() {
        messages::release_webhooks(c, &reply).await;
    }
    Ok(Ok(reply))
}

pub async fn create(c: &mut Ctx) -> Result {
    let source = source(c).await?;
    let board = c.param_str("board_id").unwrap_or("").to_owned();
    let title = c.param_str("title").unwrap_or("").to_owned();
    let description = c.param_str("description").unwrap_or("").to_owned();
    let outcome = match create_card(c, &source, board, title, description).await? {
        Ok(creation) => creation.into_classic()?,
        Err(failure) => Err(failure),
    };
    match outcome {
        Ok(created) => redirect(
            c,
            &source,
            true,
            &format!("Fizzy card #{} created.", created.number),
        ),
        Err(Failure::Invalid { boards, .. }) => {
            c.flash().now("alert", "Choose a board and enter a title.");
            render(c, source, boards, StatusCode::UNPROCESSABLE_ENTITY).await
        }
        Err(failure) => classic_failure(c, &source, &failure),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fizzy_locked_reply_keeps_classic_failure_and_spa_card() {
        let creation = || CardCreation {
            number: "580".into(),
            url: "https://fizzy.test/cards/580".into(),
            reply: Ok(Err(Failure::Locked)),
        };
        assert!(matches!(
            creation().into_classic(),
            Ok(Err(Failure::Locked))
        ));
        let Failure::ReplyFailed {
            number,
            url,
            details,
        } = creation().into_spa().err().unwrap()
        else {
            panic!("a created card must be terminal");
        };
        assert_eq!(number, "580");
        assert_eq!(url, "https://fizzy.test/cards/580");
        assert_eq!(details, "This thread is locked");
    }

    #[test]
    fn fizzy_internal_reply_error_keeps_classic_error_and_spa_card() {
        let creation = || CardCreation {
            number: "580".into(),
            url: "https://fizzy.test/cards/580".into(),
            reply: Err(Error::internal(std::io::Error::other("queue rejected"))),
        };
        assert!(matches!(creation().into_classic(), Err(Error::Internal(_))));
        let Failure::ReplyFailed {
            number,
            url,
            details,
        } = creation().into_spa().err().unwrap()
        else {
            panic!("a created card must be terminal");
        };
        assert_eq!(number, "580");
        assert_eq!(url, "https://fizzy.test/cards/580");
        assert_eq!(details, "A local error prevented posting the reply");
    }
}
