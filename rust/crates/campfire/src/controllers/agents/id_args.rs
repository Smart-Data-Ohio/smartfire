//! Active Record array find_by adapters for services whose model API takes one ID.
//! Coercion and scoped selection happen here; domain authorization remains in the service.
use super::reads::lookup_ids;
use campfire_db::{Agent, Connection, Result};
use rusqlite::{OptionalExtension, params_from_iter, types::Value as SqlValue};
use serde_json::{Value, json};

pub(super) fn select(
    conn: &Connection,
    table: &str,
    raw: &Value,
    scope: &str,
    binds: &[i64],
) -> Result<i64> {
    let mut values = vec![SqlValue::Text(json!(lookup_ids(raw)).to_string())];
    values.extend(binds.iter().map(|v| SqlValue::Integer(*v)));
    Ok(conn.query_row(&format!("SELECT id FROM {table} WHERE id IN (SELECT value FROM json_each(?)) {scope} ORDER BY id LIMIT 1"),params_from_iter(values),|r|r.get(0)).optional()?.unwrap_or(0))
}
fn needs(op: &str, args: &Value) -> bool {
    let keys: &[&str] = match op {
        "post_message" | "start_stream" => &["room_id", "thread_id"],
        "append_stream" | "finalize_stream" | "pin_message" | "unpin_message" => &["message_id"],
        "open_dm" => &["user_id"],
        "get_poll" => &["room_id", "poll_id"],
        "create_poll"
        | "register_slash_command"
        | "unregister_slash_command"
        | "request_approval" => &["room_id"],
        "add_step" => &["message_id", "thread_id"],
        "update_step" => &["step_id"],
        "get_approval" | "cancel_approval" => &["approval_id"],
        _ => &[],
    };
    keys.iter()
        .any(|key| args[*key].as_array().is_some_and(|a| !a.is_empty()))
}
pub(super) async fn normalize_request(
    c: &campfire_kit::Ctx,
    agent_id: i64,
    op: &str,
    args: Value,
) -> campfire_kit::Result<Value> {
    use crate::app::AppCtx;
    if !needs(op, &args) {
        return Ok(args);
    }
    let op = op.to_owned();
    c.app()
        .db
        .read(move |conn| {
            let mut args = args;
            normalize(conn, agent_id, &op, &mut args)?;
            Ok(args)
        })
        .await
        .map_err(crate::controllers::presenters::page::db_error)
}
pub(super) fn normalize(
    conn: &Connection,
    agent_id: i64,
    op: &str,
    args: &mut Value,
) -> Result<()> {
    // Empty arrays remain blank for presence/required-argument checks.
    if !needs(op, args) {
        return Ok(());
    }
    let agent = Agent::find(conn, agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
    let room_ops = [
        "post_message",
        "start_stream",
        "create_poll",
        "get_poll",
        "register_slash_command",
        "unregister_slash_command",
    ];
    if room_ops.contains(&op) && args["room_id"].as_array().is_some_and(|a| !a.is_empty()) {
        args["room_id"] = json!(select(
            conn,
            "rooms",
            &args["room_id"],
            "AND id IN (SELECT room_id FROM memberships WHERE user_id=?)",
            &[agent.user_id]
        )?);
    }
    for (key, table, scope, binds) in match op {
        "post_message" | "start_stream" => vec![(
            "thread_id",
            "channel_threads",
            "AND room_id=?",
            vec![super::ruby_i64(&args["room_id"])],
        )],
        "append_stream" | "finalize_stream" | "pin_message" | "unpin_message" => {
            vec![("message_id", "messages", "", vec![])]
        }
        "open_dm" => vec![("user_id", "users", "", vec![])],
        "get_poll" => vec![(
            "poll_id",
            "polls",
            "AND message_id IN (SELECT id FROM messages WHERE room_id=?)",
            vec![super::ruby_i64(&args["room_id"])],
        )],
        "add_step" => vec![
            ("message_id", "messages", "", vec![]),
            ("thread_id", "channel_threads", "", vec![]),
        ],
        "update_step" => vec![("step_id", "agent_steps", "AND agent_id=?", vec![agent_id])],
        "request_approval" => vec![("room_id", "rooms", "AND deleted_at IS NULL", vec![])],
        "get_approval" | "cancel_approval" => vec![(
            "approval_id",
            "agent_approvals",
            "AND agent_id=?",
            vec![agent_id],
        )],
        _ => vec![],
    } {
        if args[key].as_array().is_some_and(|a| !a.is_empty()) {
            args[key] = json!(select(conn, table, &args[key], scope, &binds)?);
        }
    }
    Ok(())
}
