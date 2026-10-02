//! Agent integration HTTP boundaries. The clients/action domains remain WS15e/g.
use crate::app::AppCtx;
use crate::concerns::{self, Before, CurrentAgent, agent_api};
use crate::controllers::presenters::page::db_error;
use campfire_db::models::agent_service::ServiceResult;
use campfire_kit::{Ctx, Result, StatusCode, format, halt};
use serde_json::Value;

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
    Err(campfire_kit::Error::internal(anyhow::anyhow!("Unknown Fizzy operation: {op}")))
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
/// Each tool and REST controller supplies exactly the fields its Rails caller constructs.
pub(super) fn action_fields(args: Value, keys: &[&str]) -> Value {
    Value::Object(keys.iter().map(|key| ((*key).into(), args[*key].clone())).collect())
}
