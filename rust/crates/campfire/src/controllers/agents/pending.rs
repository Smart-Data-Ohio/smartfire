//! HTTP policy/validation around explicitly missing WS11 service operations.
//! REST and MCP enter the same operation after their own callback/throttle order.
use super::mcp::blank;
use super::{id, render_result, ruby_i64, text};
use crate::app::AppCtx;
use crate::concerns::{self, Before, agent_api};
use crate::controllers::presenters::page::db_error;
use campfire_db::models::{agent_access as access, agent_service::ServiceResult};
use campfire_db::{Agent, Membership, Message, Room, Tx};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use serde_json::{Value, json};

fn present(args: &Value, key: &str) -> bool {
    args.get(key).is_some_and(|v| !blank(v))
}
fn number(args: &Value, key: &str) -> i64 {
    args.get(key).map_or(0, ruby_i64)
}
fn fail(error: &str, status: u16) -> ServiceResult {
    ServiceResult::fail(error, status)
}
fn forbidden(cap: &str) -> ServiceResult {
    ServiceResult::fail(format!("Forbidden: agent lacks {cap} capability"), 403)
}
fn member_room(tx: &Tx<'_>, agent: &Agent, id: i64) -> campfire_db::Result<Option<Room>> {
    Room::find_for_user(tx.conn(), agent.user_id, id)
}
fn allowed(tx: &Tx<'_>, agent: &Agent, cap: &str, room: Option<i64>) -> campfire_db::Result<bool> {
    access::capability_for_agent(tx.conn(), agent.id, cap, room)
}
fn message(tx: &Tx<'_>, agent: &Agent, id: i64) -> campfire_db::Result<Option<Message>> {
    let Some(message) = Message::find_by_id(tx.conn(), id)? else {
        return Ok(None);
    };
    Ok(
        Membership::find_by_room_and_user(tx.conn(), message.room_id, agent.user_id)?
            .map(|_| message),
    )
}

pub async fn operation(
    c: &Ctx,
    agent_id: i64,
    operation: &str,
    args: Value,
) -> Result<ServiceResult> {
    dispatch(c, agent_id, operation, args, false).await
}
async fn dispatch(
    c: &Ctx,
    agent_id: i64,
    operation: &str,
    args: Value,
    rest: bool,
) -> Result<ServiceResult> {
    match operation {
        "start_stream" | "append_stream" | "finalize_stream" => {
            return super::conversations::stream(c, agent_id, operation, args, rest).await;
        }
        "get_context" => return super::conversations::context(c, agent_id, args).await,
        "post_message" => return super::conversations::post(c, agent_id, args, rest).await,
        "open_dm" => return super::conversations::dm(c, agent_id, args, rest).await,
        _ => {}
    }
    let reader = matches!(
        operation,
        "list_rooms" | "read_messages" | "list_work" | "get_work" | "list_board_posts"
    );
    let op = operation.to_owned();
    let fields = args.clone();
    let result = c
        .app()
        .db
        .write(move |tx| preflight(tx, agent_id, &op, fields))
        .await
        .map_err(db_error)?;
    if reader && result.is_ok() {
        super::reads::operation(c, agent_id, operation, args).await
    } else {
        Ok(result)
    }
}
fn preflight(
    tx: &mut Tx<'_>,
    agent_id: i64,
    op: &str,
    args: Value,
) -> campfire_db::Result<ServiceResult> {
    let agent =
        Agent::find(tx.conn(), agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
    match op {
        "get_context" | "read_messages" => {
            let context = op == "get_context";
            let first = if context { "message_id" } else { "room_id" };
            if !present(&args, first) && !present(&args, "thread_id") {
                return Ok(fail(
                    if context {
                        "message_id or thread_id is required"
                    } else {
                        "room_id or thread_id is required"
                    },
                    422,
                ));
            }
            if !context && present(&args, first) && present(&args, "thread_id") {
                return Ok(fail("Pass only one of room_id, thread_id", 422));
            }
            let (room_id, missing, thread_id) = if present(&args, first) {
                if context {
                    let Some(message) = Message::find_by_id(tx.conn(), number(&args, first))?
                    else {
                        return Ok(fail("Message not found", 404));
                    };
                    (message.room_id, "Message not found", message.thread_id)
                } else {
                    (number(&args, first), "Room not found", None)
                }
            } else {
                let record = tx
                    .conn()
                    .query_row(
                        "SELECT room_id FROM channel_threads WHERE id=?",
                        [number(&args, "thread_id")],
                        |r| r.get::<_, i64>(0),
                    )
                    .optional()?;
                let Some(room) = record else {
                    return Ok(fail("Thread not found", 404));
                };
                (room, "Thread not found", Some(number(&args, "thread_id")))
            };
            if member_room(tx, &agent, room_id)?.is_none() {
                return Ok(fail(missing, 404));
            }
            if !allowed(tx, &agent, "read_messages", Some(room_id))? {
                return Ok(forbidden("read_messages"));
            }
            if context
                && present(&args, "message_id")
                && present(&args, "thread_id")
                && number(&args, "thread_id") != thread_id.unwrap_or(0)
            {
                return Ok(fail("Message is not in the given thread", 422));
            }
            if !context {
                for cursor in ["before", "after"] {
                    if present(&args, cursor) {
                        let anchor = Message::find_by_id(tx.conn(), number(&args, cursor))?
                            .filter(|m| m.room_id == room_id && m.thread_id == thread_id);
                        if anchor.is_none() {
                            return Ok(fail("Message not found", 404));
                        }
                    }
                }
            }
        }
        "open_dm" => {
            let target = campfire_db::User::find_by_id(tx.conn(), number(&args, "user_id"))?;
            let Some(target) = target else {
                return Ok(fail("User not found", 404));
            };
            if target.is_bot() {
                return Ok(fail("Cannot open a DM with a bot", 422));
            }
            if !target.is_active() {
                return Ok(fail("Cannot open a DM with an inactive account", 422));
            }
            if !access::has_capability_anywhere(tx.conn(), agent_id, "post_messages")? {
                return Ok(forbidden("post_messages"));
            }
            if agent.owner_id != Some(target.id) && !allowed(tx, &agent, "dm_anyone", None)? {
                let prior:bool=tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM agent_events WHERE agent_id=? AND actor_id=? AND event_type IN ('mention','reply','direct_message'))",rusqlite::params![agent_id,target.id],|r|r.get(0))?;
                if !prior {
                    return Ok(fail(
                        "Forbidden: agent may only DM its owner or humans who messaged it without the dm_anyone capability",
                        403,
                    ));
                }
            }
        }
        "pin_message" | "unpin_message" | "react" | "append_stream" | "finalize_stream" => {
            let Some(message) = message(tx, &agent, number(&args, "message_id"))? else {
                return Ok(fail("Message not found", 404));
            };
            let stream = op == "append_stream" || op == "finalize_stream";
            if stream && message.creator_id != agent.user_id {
                return Ok(fail("Message not found", 404));
            }
            let cap = if op == "react" {
                "react"
            } else {
                "post_messages"
            };
            if !allowed(tx, &agent, cap, Some(message.room_id))? {
                return Ok(forbidden(cap));
            }
            if op == "append_stream" {
                if !message.streaming {
                    return Ok(fail("Message is not streaming", 422));
                }
                if args.get("append").is_none_or(Value::is_null)
                    && args.get("markdown_source").is_none_or(Value::is_null)
                {
                    return Ok(fail("append or markdown_source is required", 422));
                }
            }
        }
        "post_message" | "start_stream" | "create_poll" | "get_poll" | "list_board_posts"
        | "create_board_post" => {
            let missing = if op == "get_poll" {
                "Poll not found"
            } else {
                "Room not found"
            };
            let Some(room) = member_room(tx, &agent, number(&args, "room_id"))? else {
                return Ok(fail(missing, 404));
            };
            if op == "get_poll" {
                let exists:bool=tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM polls p JOIN messages m ON m.id=p.message_id WHERE p.id=? AND m.room_id=?)",rusqlite::params![number(&args,"poll_id"),room.id],|r|r.get(0))?;
                if !exists {
                    return Ok(fail("Poll not found", 404));
                }
            }
            let cap = if op == "list_board_posts" {
                "read_messages"
            } else {
                "post_messages"
            };
            if !allowed(tx, &agent, cap, Some(room.id))? {
                return Ok(forbidden(cap));
            }
            if op == "create_board_post" && !allowed(tx, &agent, "manage_threads", Some(room.id))? {
                return Ok(forbidden("manage_threads"));
            }
            if matches!(op, "list_board_posts" | "create_board_post") && !room.board() {
                return Ok(fail("Room is not a board", 422));
            }
            if matches!(op, "post_message" | "start_stream") && present(&args, "thread_id") {
                let locked=tx.conn().query_row("SELECT locked_at IS NOT NULL FROM channel_threads WHERE id=? AND room_id=?",rusqlite::params![number(&args,"thread_id"),room.id],|r|r.get::<_,bool>(0)).optional()?;
                let Some(locked) = locked else {
                    return Ok(fail("Thread not found", 404));
                };
                if locked {
                    return Ok(fail("This thread is locked", 422));
                }
            }
            if matches!(
                op,
                "post_message" | "start_stream" | "create_poll" | "create_board_post"
            ) {
                let cap = if op == "create_board_post" {
                    campfire_db::models::agent_posting::Cap::BoardPosts
                } else {
                    campfire_db::models::agent_posting::Cap::Messages
                };
                if let Some(denial) =
                    campfire_db::models::agent_posting::check_budget(tx, agent_id, cap)?
                {
                    return Ok(ServiceResult::budget(denial));
                }
            }
            if op == "list_board_posts" {
                let status = text(args.get("status")).unwrap_or_default();
                let status = status.trim();
                if !status.is_empty()
                    && !matches!(
                        status,
                        "planned" | "in_progress" | "blocked" | "done" | "open" | "all"
                    )
                {
                    return Ok(fail(
                        "Status must be one of planned, in_progress, blocked, done, open, all",
                        422,
                    ));
                }
                let owner = text(args.get("owner")).unwrap_or_default();
                let owner = owner.trim();
                if !owner.is_empty()
                    && !matches!(owner, "me" | "agents")
                    && !owner.bytes().all(|b| b.is_ascii_digit())
                {
                    return Ok(fail("Owner must be a user id, me, or agents", 422));
                }
            }
            if op == "create_poll" {
                let labels: Vec<_> = args
                    .get("options")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| text(Some(v)))
                            .map(|s| s.trim().to_owned())
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();
                if !(2..=10).contains(&labels.len()) {
                    return Ok(fail("Poll needs between 2 and 10 options", 422));
                }
                if labels.iter().any(|s| s.chars().count() > 200) {
                    return Ok(fail("Options are limited to 200 characters", 422));
                }
                if text(args.get("question")).is_none_or(|s| s.trim().is_empty()) {
                    return Ok(fail("Question can't be blank", 422));
                }
            }
        }
        "get_work" | "update_work" | "update_board_post" | "set_result" | "handoff_work" => {
            let owned=tx.conn().query_row("SELECT t.room_id FROM channel_threads t JOIN memberships m ON m.room_id=t.room_id WHERE t.id=? AND t.work_status IS NOT NULL AND t.work_owner_id=? AND m.user_id=?",rusqlite::params![number(&args,"work_id"),agent.user_id,agent.user_id],|r|r.get::<_,i64>(0)).optional()?;
            let Some(room) = owned else {
                return Ok(fail("Work not found", 404));
            };
            if !allowed(tx, &agent, "read_messages", Some(room))? {
                return Ok(fail("Work not found", 404));
            }
            if op != "get_work" && !allowed(tx, &agent, "manage_threads", Some(room))? {
                return Ok(forbidden("manage_threads"));
            }
            if op == "set_result" && !args.as_object().is_some_and(|a| a.contains_key("markdown")) {
                return Ok(fail("Markdown can't be blank", 422));
            }
        }
        "list_work" | "list_rooms" => {}
        _ => {
            return Err(campfire_db::Error::Other(format!(
                "Unknown agent operation: {op}"
            )));
        }
    }
    if matches!(
        op,
        "list_rooms" | "read_messages" | "list_work" | "get_work" | "list_board_posts"
    ) {
        return Ok(ServiceResult::ok(Value::Null, 200));
    }
    if matches!(op, "pin_message" | "unpin_message") {
        return super::pins::operation(tx, agent_id, op, args);
    }
    campfire_db::models::agent_api_pending::execute(tx, agent_id, op, args)
}
use rusqlite::OptionalExtension;

struct Action {
    controller: &'static str,
    action: &'static str,
    op: &'static str,
    room_first: bool,
    rescue: bool,
    no_store: bool,
    limit: u64,
}
async fn rest(c: &mut Ctx, settings: Action) -> Result {
    let Action {
        controller,
        action,
        op,
        room_first,
        rescue,
        no_store,
        limit,
    } = settings;
    // Room-first controllers do membership before their Bearer-only check.
    if room_first {
        match concerns::before_actions(c, Before::default().allow_agent_access()).await {
            Err(Error::InvalidAuthenticityToken(_)) if rescue => {
                agent_api::token(c)?;
            }
            result => result?,
        }
        let user = concerns::require_current_user(c)?.id;
        let room = id(c, "room_id");
        let found = c
            .app()
            .db
            .read(move |conn| Room::find_for_user(conn, user, room))
            .await
            .map_err(db_error)?;
        if found.is_none() {
            // Rails head :not_found uses HTML before any explicit JSON render.
            return Ok(c.head(StatusCode::NOT_FOUND).content_type("text/html"));
        }
    }
    let identity = if room_first {
        agent_api::token(c)?
    } else {
        agent_api::require_token(c, rescue).await?
    };
    let args = Value::Object(
        c.params
            .iter()
            .map(|(k, v)| (k.to_owned(), v.to_json()))
            .collect(),
    );
    if op == "get_context" {
        super::ensure_anywhere(c, identity.agent_id, "read_messages").await?;
    }
    if room_first {
        let room = id(c, "room_id");
        let cap = if op == "list_board_posts" {
            "read_messages"
        } else {
            "post_messages"
        };
        concerns::ensure_agent_capability(c, cap, room).await?;
        if op == "create_board_post" {
            concerns::ensure_agent_capability(c, "manage_threads", room).await?;
        }
    }
    if limit > 0 {
        agent_api::throttle(c, limit, controller, action)?;
    }
    if matches!(op, "list_board_posts" | "create_board_post") {
        let room = id(c, "room_id");
        let board = c
            .app()
            .db
            .read(move |conn| Room::find(conn, room).map(|r| r.board()))
            .await
            .map_err(db_error)?;
        if !board {
            return render_result(c, fail("Room is not a board", 422));
        }
    }
    if no_store {
        agent_api::no_store(c);
    }
    if matches!(op, "post_message" | "start_stream") {
        c.params.require("message")?;
    }

    let mut args = args;
    for key in ["message_id", "poll_id", "work_id"] {
        if matches!(
            op,
            "pin_message" | "unpin_message" | "append_stream" | "finalize_stream"
        ) && key == "message_id"
            || op == "get_poll" && key == "poll_id"
            || matches!(
                op,
                "get_work" | "update_work" | "set_result" | "handoff_work"
            ) && key == "work_id"
        {
            args[key] = json!(id(c, "id"));
        }
    }
    let result = dispatch(c, identity.agent_id, op, args, true).await?;
    // Pinned Agents::MessagesController calls an undefined render_room_not_found
    // in this rescue. Match its production 500; MCP retains the service's 404.
    if op == "post_message" && result.error.as_deref() == Some("Reply target not found") {
        return Err(Error::internal(anyhow::anyhow!(
            "Rails render_room_not_found is undefined"
        )));
    }
    render_result(c, result)
}

pub async fn contexts_show(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/contexts",
            action: "show",
            op: "get_context",
            room_first: false,
            rescue: false,
            no_store: true,
            limit: 120,
        },
    )
    .await
}

pub async fn dms_create(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/dms",
            action: "create",
            op: "open_dm",
            room_first: false,
            rescue: false,
            no_store: true,
            limit: 60,
        },
    )
    .await
}

pub async fn messages_create(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/messages",
            action: "create",
            op: "post_message",
            room_first: true,
            rescue: true,
            no_store: false,
            limit: 60,
        },
    )
    .await
}

pub async fn streaming_messages_create(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/streaming_messages",
            action: "create",
            op: "start_stream",
            room_first: true,
            rescue: true,
            no_store: true,
            limit: 60,
        },
    )
    .await
}

pub async fn streaming_messages_update(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/streaming_messages",
            action: "update",
            op: "append_stream",
            room_first: false,
            rescue: true,
            no_store: true,
            limit: 240,
        },
    )
    .await
}

pub async fn streaming_messages_finalize(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/streaming_messages",
            action: "finalize",
            op: "finalize_stream",
            room_first: false,
            rescue: true,
            no_store: true,
            limit: 60,
        },
    )
    .await
}

pub async fn pins_create(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/pins",
            action: "create",
            op: "pin_message",
            room_first: false,
            rescue: true,
            no_store: false,
            limit: 60,
        },
    )
    .await
}

pub async fn pins_destroy(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/pins",
            action: "destroy",
            op: "unpin_message",
            room_first: false,
            rescue: true,
            no_store: false,
            limit: 60,
        },
    )
    .await
}

pub async fn polls_create(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/polls",
            action: "create",
            op: "create_poll",
            room_first: false,
            rescue: true,
            no_store: false,
            limit: 60,
        },
    )
    .await
}

pub async fn polls_show(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/polls",
            action: "show",
            op: "get_poll",
            room_first: false,
            rescue: true,
            no_store: false,
            limit: 120,
        },
    )
    .await
}

pub async fn posts_index(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/posts",
            action: "index",
            op: "list_board_posts",
            room_first: true,
            rescue: true,
            no_store: true,
            limit: 0,
        },
    )
    .await
}

pub async fn posts_create(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/posts",
            action: "create",
            op: "create_board_post",
            room_first: true,
            rescue: true,
            no_store: true,
            limit: 30,
        },
    )
    .await
}

pub async fn work_index(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/work",
            action: "index",
            op: "list_work",
            room_first: false,
            rescue: false,
            no_store: true,
            limit: 0,
        },
    )
    .await
}

pub async fn work_show(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/work",
            action: "show",
            op: "get_work",
            room_first: false,
            rescue: false,
            no_store: true,
            limit: 0,
        },
    )
    .await
}

pub async fn work_update(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/work",
            action: "update",
            op: "update_work",
            room_first: false,
            rescue: false,
            no_store: true,
            limit: 0,
        },
    )
    .await
}

pub async fn work_result(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/work",
            action: "result",
            op: "set_result",
            room_first: false,
            rescue: false,
            no_store: true,
            limit: 0,
        },
    )
    .await
}

pub async fn work_handoff(c: &mut Ctx) -> Result {
    rest(
        c,
        Action {
            controller: "agents/work",
            action: "handoff",
            op: "handoff_work",
            room_first: false,
            rescue: false,
            no_store: true,
            limit: 60,
        },
    )
    .await
}
