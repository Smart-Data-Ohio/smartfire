//! Agent integration HTTP boundaries. The clients/action domains remain WS15e/g.
use super::mcp::blank;
use super::{ruby_i64, text};
use crate::app::AppCtx;
use crate::concerns::{self, Before, CurrentAgent, agent_api};
use crate::controllers::presenters::page::db_error;
use campfire_db::models::{agent_access, agent_service::ServiceResult};
use campfire_db::{Agent, Room, Tx};
use campfire_kit::{Ctx, Result, StatusCode, format, halt};
use rusqlite::OptionalExtension;
use serde_json::Value;

fn number(args: &Value, key: &str) -> i64 {
    args.get(key).map_or(0, ruby_i64)
}
fn fail(error: &str, status: u16) -> Option<ServiceResult> {
    Some(ServiceResult::fail(error, status))
}
fn preflight(
    tx: &mut Tx<'_>,
    agent_id: i64,
    op: &str,
    args: &Value,
    encryption: &rails_compat::ar_encryption::ArEncryption,
) -> campfire_db::Result<Option<ServiceResult>> {
    let agent =
        Agent::find(tx.conn(), agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
    if op == "github_pull_request_action" {
        let room = number(args, "room_id");
        if Room::find_for_user(tx.conn(), agent.user_id, room)?.is_none() {
            return Ok(fail("Room not found", 404));
        }
        let mapped:bool=tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM github_pull_requests p JOIN github_pull_request_threads t ON t.github_pull_request_id=p.id WHERE p.id=? AND t.room_id=?)",rusqlite::params![number(args,"pull_request_id"),room],|r|r.get(0))?;
        if !mapped {
            return Ok(fail("Pull request not found", 404));
        }
        if !agent_access::capability_for_agent(tx.conn(), agent_id, "external_action", Some(room))?
        {
            return Ok(fail(
                "Forbidden: agent lacks external_action capability",
                403,
            ));
        }
        let owner_app = if let Some(owner) = agent.owner_id {
            linked_account(tx, encryption, "github_connected_accounts", owner, true)?.is_some()
        } else {
            false
        };
        if !owner_app
            && linked_account(
                tx,
                encryption,
                "github_connected_accounts",
                agent.user_id,
                false,
            )?
            .is_none()
        {
            return Ok(fail("Agent has no usable GitHub account", 422));
        }
        return Ok(None);
    }
    Ok(None)
}

pub async fn operation(c: &Ctx, agent_id: i64, op: &str, args: Value) -> Result<ServiceResult> {
    use crate::integrations::fizzy::agent_reads::{self, Read};
    let read = match op {
        "fizzy_boards" => Some(Read::Boards { account: args["account_id"].clone() }),
        "fizzy_board" => Some(Read::Board { account: args["account_id"].clone(), board: args["board_id"].clone() }),
        "fizzy_search" => Some(Read::Search { account: args["account_id"].clone(), query: args["q"].clone() }),
        "fizzy_card" => Some(Read::Card { account: args["account_id"].clone(), number: args["number"].clone() }),
        _ => None,
    };
    if let Some(read) = read {
        let app = c.app();
        let result = agent_reads::read(app, &app.fizzy.network, &app.fizzy.base, agent_id, read).await.map_err(db_error)?;
        return Ok(ServiceResult { payload: result.payload, error: result.error, status: result.status });
    }
    if op == "fizzy_card_action" {
        let crypto = c.app().ar_encryption.clone();
        let credential = c.current::<CurrentAgent>().map(|a| a.credential_id);
        let user = concerns::require_current_user(c)?.id;
        return c.app().db.write(move |tx| {
            let zone: Option<String> = tx.conn().query_row("SELECT time_zone FROM users WHERE id=?", [user], |r| r.get(0))?;
            let zone = campfire_db::slash_commands::time_parser::zone(zone.as_deref().unwrap_or("UTC"));
            let result = crate::integrations::fizzy::agent_requests::create(tx, &crypto, agent_id, args, credential, &zone)?;
            Ok(ServiceResult { payload: result.payload, error: result.error, status: result.status })
        }).await.map_err(db_error);
    }
    let op = op.to_owned();
    let encryption = c.app().ar_encryption.clone();
    c.app()
        .db
        .write(move |tx| {
            if let Some(denial) = preflight(tx, agent_id, &op, &args, &encryption)? {
                return Ok(denial);
            }
            if op == "github_pull_request_action"
                && let Some(denial) = action_preflight(tx, agent_id, &op, &args)?
            {
                return Ok(denial);
            }
            campfire_db::models::agent_api_pending::execute(tx, agent_id, &op, args)
        })
        .await
        .map_err(db_error)
}
async fn fizzy(c: &mut Ctx, controller: &str, action: &str, op: &str, limit: u64) -> Result {
    concerns::before_actions(c, Before::default().allow_agent_access()).await?;
    let Some(identity) = c
        .current::<CurrentAgent>()
        .copied()
        .filter(|_| concerns::authenticated_by(c) == concerns::AuthenticatedBy::AgentToken)
    else {
        return halt(c.render(
            StatusCode::FORBIDDEN,
            &format::JSON,
            serde_json::json!({"error":"Forbidden: agent token required"}).to_string(),
        ));
    };
    agent_api::throttle(c, limit, controller, action)?;
    agent_api::no_store(c);
    let mut args = c.params.to_json();
    if op == "fizzy_card_action" {
        args = action_fields(args, &["account_id", "kind", "board_id", "number", "column_id", "title", "description", "body", "external_id"]);
    }
    if op == "fizzy_board" {
        args["board_id"] = args["id"].clone();
    }
    let result = operation(c, identity.agent_id, op, args).await?;
    // Fizzy preserves the error JSON even for 404, unlike most agent controllers.
    let status = StatusCode::from_u16(result.status).map_err(campfire_kit::Error::internal)?;
    let body = if result.is_ok() {
        result.payload.unwrap_or(Value::Null)
    } else {
        result.failure_body()
    };
    Ok(c.render(status, &format::JSON, body.to_string()))
}
pub async fn fizzy_boards(c: &mut Ctx) -> Result {
    fizzy(c, "agents/fizzy/boards", "index", "fizzy_boards", 120).await
}
pub async fn fizzy_board(c: &mut Ctx) -> Result {
    fizzy(c, "agents/fizzy/boards", "show", "fizzy_board", 120).await
}
pub async fn fizzy_search(c: &mut Ctx) -> Result {
    fizzy(c, "agents/fizzy/cards", "search", "fizzy_search", 120).await
}
pub async fn fizzy_card(c: &mut Ctx) -> Result {
    fizzy(c, "agents/fizzy/cards", "show", "fizzy_card", 120).await
}
pub async fn fizzy_action(c: &mut Ctx) -> Result {
    fizzy(
        c,
        "agents/fizzy/card_actions",
        "create",
        "fizzy_card_action",
        60,
    )
    .await
}
pub async fn github_action(c: &mut Ctx) -> Result {
    let identity = agent_api::require_token(c, false).await?;
    let args = c.params.to_json();
    let guard_args = args.clone();
    let encryption = c.app().ar_encryption.clone();
    if let Some(denial) = c
        .app()
        .db
        .write(move |tx| {
            preflight(
                tx,
                identity.agent_id,
                "github_pull_request_action",
                &guard_args,
                &encryption,
            )
        })
        .await
        .map_err(db_error)?
    {
        return super::render_result(c, denial);
    }
    agent_api::throttle(c, 60, "agents/github/pull_request_actions", "create")?;
    agent_api::no_store(c);
    let result = operation(c, identity.agent_id, "github_pull_request_action", args).await?;
    super::render_result(c, result)
}

// Local GitHub usability only; App-token refresh and remote access remain WS15g.
fn linked_account(
    tx: &Tx<'_>,
    encryption: &rails_compat::ar_encryption::ArEncryption,
    table: &str,
    user: i64,
    app_only: bool,
) -> campfire_db::Result<Option<String>> {
    let extra = if app_only {
        " AND token_source='app'"
    } else {
        ""
    };
    let column = if table == "fizzy_connected_accounts" {
        "fizzy_account_id"
    } else {
        "github_login"
    };
    let row=tx.conn().query_row(&format!("SELECT id,access_token,disconnected_reason,{column} FROM {table} WHERE user_id=?{extra}"),[user],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?))).optional()?;
    let Some((id, encrypted, disconnected, account)) = row else {
        return Ok(None);
    };
    if disconnected
        .as_deref()
        .is_some_and(|s| !campfire_richtext::ruby::is_blank(s))
    {
        return Ok(None);
    }
    match encryption.decrypt(&encrypted) {
        Ok(token) if !campfire_richtext::ruby::is_blank(&token) => Ok(Some(account)),
        Ok(_) => Ok(None),
        Err(_) => {
            tx.conn().execute(&format!("UPDATE {table} SET disconnected_reason='The stored token could not be read; link it again',updated_at=? WHERE id=?"),rusqlite::params![tx.now(),id])?;
            Ok(None)
        }
    }
}
fn normalized(args: &Value, key: &str) -> String {
    text(args.get(key))
        .unwrap_or_default()
        .trim_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
        .to_owned()
}
fn action_preflight(
    tx: &mut Tx<'_>,
    agent_id: i64,
    op: &str,
    args: &Value,
) -> campfire_db::Result<Option<ServiceResult>> {
    if let Some(external) = args.get("external_id").filter(|v| !blank(v))
        && let Some(mut existing) = campfire_db::AgentApproval::find_by_external_id(
            tx.conn(),
            agent_id,
            &text(Some(external)).unwrap_or_default(),
        )?
    {
        existing.expire_if_due(tx)?;
        return Ok(Some(ServiceResult::ok(
            existing.created_payload(tx.now()),
            200,
        )));
    }
    let kind = text(args.get("kind")).unwrap_or_default();
    let mut errors = campfire_db::Errors::default();
    if op == "github_pull_request_action" {
        if !["comment", "approve", "request_changes", "request_review"].contains(&kind.as_str()) {
            errors.add(
                "kind",
                "must be one of: comment, approve, request_changes, request_review",
            );
        }
        let body = normalized(args, "body");
        if body.is_empty() {
            if kind == "comment" {
                errors.add("body", "is required for a comment");
            }
            if kind == "request_changes" {
                errors.add("body", "is required when requesting changes");
            }
        }
        if body.chars().count() > 3500 {
            errors.add("body", "is too long (maximum is 3500 characters)");
        }
        if kind == "request_review" {
            let submitted = args.get("reviewers");
            let array = submitted
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_else(|| submitted.into_iter().cloned().collect());
            let mut logins = std::collections::HashSet::new();
            let mut valid = true;
            for part in array {
                for login in text(Some(&part))
                    .unwrap_or_default()
                    .split(|c: char| c.is_whitespace() || c == ',')
                    .filter(|s| !s.is_empty())
                {
                    let login = login
                        .strip_prefix('@')
                        .unwrap_or(login)
                        .to_ascii_lowercase();
                    valid &= !login.is_empty()
                        && login.len() <= 39
                        && login
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                        && !login.starts_with('-')
                        && !login.ends_with('-')
                        && !login.contains("--");
                    logins.insert(login);
                }
            }
            if !valid || logins.is_empty() || logins.len() > 15 {
                errors.add("reviewers", "Enter GitHub usernames separated by commas.");
            }
        }
    }
    if !errors.is_empty() {
        let mut fields = serde_json::Map::new();
        for (key, message) in &errors.0 {
            fields
                .entry((*key).to_owned())
                .or_insert_with(|| Value::Array(vec![]))
                .as_array_mut()
                .unwrap()
                .push(message.clone().into());
        }
        return Ok(Some(ServiceResult {
            error: Some(campfire_db::slash_commands::sentence(
                errors.full_messages(),
            )),
            status: 422,
            payload: Some(serde_json::json!({"errors":fields})),
        }));
    }
    if let Some(budget) = campfire_db::models::agent_posting::check_budget(
        tx,
        agent_id,
        campfire_db::models::agent_posting::Cap::ExternalActions,
    )? {
        return Ok(Some(ServiceResult::budget(budget)));
    }
    Ok(None)
}

/// Each tool and REST controller supplies exactly the fields its Rails caller constructs.
pub(super) fn action_fields(args: Value, keys: &[&str]) -> Value {
    Value::Object(keys.iter().map(|key| ((*key).into(), args[*key].clone())).collect())
}
