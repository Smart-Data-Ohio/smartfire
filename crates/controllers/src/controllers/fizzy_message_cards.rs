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
struct Source {
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
async fn source(c: &mut Ctx) -> Result<Source> {
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
async fn boards(c: &mut Ctx, source: &Source) -> campfire_kit::Result<Value> {
    match source
        .client()
        .boards(&source.account.as_ref().unwrap().account_id)
        .await
    {
        Ok(value) => Ok(value),
        Err(error) => {
            let response = if error.kind == ErrorKind::Unauthorized {
                rejected(c, source).await
            } else {
                redirect(c, source, false, "Could not reach Fizzy. Try again.")
            };
            // Preserve adapter errors rather than disguising them as successful redirects.
            campfire_kit::halt(response?)
        }
    }
}
async fn rejected(c: &mut Ctx, source: &Source) -> Result {
    let account = source.account.clone().expect("usable account");
    c.app()
        .db
        .write(move |tx| account.mark_disconnected(tx, REJECTED_TOKEN_REASON))
        .await
        .map_err(db_error)?;
    fizzy_connections::redirect(
        c,
        false,
        "Fizzy rejected the linked token. Reconnect on your profile.",
    )
}
pub async fn new(c: &mut Ctx) -> Result {
    let source = source(c).await?;
    let boards = if source.token.is_some() {
        boards(c, &source).await?
    } else {
        Value::Null
    };
    render(c, source, boards, StatusCode::OK).await
}
async fn render(c: &mut Ctx, source: Source, boards: Value, status: StatusCode) -> Result {
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
    let view = campfire_views::fizzy_message_cards::FormView {
        back_path: source.path(),
        action: format!(
            "{}/messages/{}/fizzy_cards",
            source.path(),
            source.message.id
        ),
        room_name: source.room_name,
        plain: source.plain,
        creator: source.creator.name,
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
    };
    page::framed_page!(c, status, |ctx| campfire_views::fizzy_message_cards::New {
        ctx,
        view: &view
    })
    .await
}
pub async fn create(c: &mut Ctx) -> Result {
    let source = source(c).await?;
    if source.token.is_none() {
        return fizzy_connections::redirect(c, false, "Connect Fizzy on your profile first.");
    }
    if source
        .thread
        .as_ref()
        .is_some_and(|t| t.locked_at.is_some())
    {
        return redirect(c, &source, false, "This thread is locked");
    }
    let board = c.param_str("board_id").unwrap_or("").to_owned();
    let title = campfire_richtext::ruby::strip(c.param_str("title").unwrap_or("")).to_owned();
    let description = c.param_str("description").unwrap_or("").to_owned();
    if campfire_richtext::ruby::is_blank(&board) || campfire_richtext::ruby::is_blank(&title) {
        let boards = boards(c, &source).await?;
        c.flash().now("alert", "Choose a board and enter a title.");
        return render(c, source, boards, StatusCode::UNPROCESSABLE_ENTITY).await;
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
            return match client.identity().await {
                Ok(_) => redirect(
                    c,
                    &source,
                    false,
                    "That Fizzy token is read-only. Generate a Read + Write token to create cards.",
                ),
                Err(probe) if probe.kind == ErrorKind::Unauthorized => rejected(c, &source).await,
                Err(_) => redirect(
                    c,
                    &source,
                    false,
                    "Could not reach Fizzy to verify the token. Try again.",
                ),
            };
        }
        Err(error) => {
            return redirect(
                c,
                &source,
                false,
                &format!("Fizzy refused the new card ({}).", error.message),
            );
        }
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
            return redirect(
                c,
                &source,
                false,
                &format!(
                    "Fizzy card #{number} created, but the reply could not be posted ({}).",
                    errors.full_messages().join(", ")
                ),
            );
        }
        Err(campfire_db::Error::Other(message)) if message == "This thread is locked" => {
            return redirect(c, &source, false, &message);
        }
        Err(error) => return Err(db_error(error)),
    };
    messages::broadcast_create(c, &source.room, &reply).await?;
    if reply.thread_id.is_none() {
        messages::release_webhooks(c, &reply).await;
    }
    redirect(c, &source, true, &format!("Fizzy card #{number} created."))
}
