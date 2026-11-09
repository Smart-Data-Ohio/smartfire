use super::{rpc_error, rpc_result};
use crate::controllers::presenters::page::db_error;
use crate::{app::AppCtx, concerns::CurrentAgent};
use campfire_db::models::mcp_events::{self as domain, Grant, GrantResult};
use campfire_kit::{Ctx, Result, StatusCode};
use serde_json::{Value, json};

fn error(c: &mut Ctx, id: Value, code: i64, message: &str, data: Option<Value>) -> Result {
    Ok(rpc_error(c, id, code, message, data, StatusCode::OK))
}
pub fn descriptors() -> Value {
    let events=[("mention","A message mentions the agent."),("reply","A reply to the agent's message."),("direct_message","A direct message to the agent.")].into_iter().map(|(name,description)|json!({"name":name,"description":description,"delivery":["webhook"],"inputSchema":{"type":"object","properties":{},"additionalProperties":false},"payloadSchema":{"type":"object","required":["id","event_type","outcome","created_at","message"],"properties":{"id":{"type":"integer"},"event_type":{"type":"string","enum":["mention","reply","direct_message"]},"outcome":{"type":"string"},"created_at":{"type":"string","format":"date-time"},"room":{"type":"object"},"actor":{"type":"object"},"message":{"type":"object"}}}})).collect::<Vec<_>>();
    json!({"resultType":"complete","events":events})
}
pub async fn dispatch(
    c: &mut Ctx,
    id: Value,
    method: &str,
    p: &Value,
    identity: CurrentAgent,
) -> Result {
    if method == "events/list" {
        return Ok(rpc_result(c, id, descriptors()));
    }
    if matches!(method, "events/poll" | "events/stream") {
        return error(
            c,
            id,
            -32026,
            "Unsupported",
            Some(
                json!({"feature":"deliveryMode","value":if method=="events/poll" {"poll"} else {"push"}}),
            ),
        );
    }
    if !matches!(method, "events/subscribe" | "events/unsubscribe") {
        return error(c, id, -32601, "Method not found", None);
    }
    let Some(name) = p["name"].as_str() else {
        return error(c, id, -32602, "Invalid params", None);
    };
    if method == "events/subscribe"
        && !campfire_db::models::agent_delivery::MESSAGE_TYPES.contains(&name)
    {
        return error(c, id, -32023, "NotFound", Some(json!({"kind":"event"})));
    }
    if p.get("arguments").is_some_and(|v| v != &json!({})) {
        return error(c, id, -32602, "Arguments must be an empty object", None);
    }
    if method == "events/subscribe" && p["delivery"]["mode"] != "webhook" {
        return error(
            c,
            id,
            -32026,
            "Unsupported",
            Some(json!({"feature":"deliveryMode","value":p["delivery"]["mode"]})),
        );
    }
    let Some(url) = p["delivery"]["url"].as_str() else {
        return error(c, id, -32602, "Invalid callback URL", None);
    };
    let valid = url::Url::parse(url).ok().filter(|u| {
        u.host_str().is_some()
            && u.username().is_empty()
            && u.password().is_none()
            && u.fragment().is_none()
            && (u.scheme() == "https"
                || (matches!(c.app().config.environment.as_str(), "test" | "development")
                    && u.scheme() == "http"))
    });
    let Some(parsed_url) = valid else {
        return error(c, id, -32602, "Callback URL must be absolute HTTPS", None);
    };
    let name = name.to_owned();
    let url = url.to_owned();
    let credential = identity.credential_id;
    if method == "events/unsubscribe" {
        let deleted = c
            .app()
            .db
            .write(move |tx| domain::unsubscribe(tx, credential, &url, &name))
            .await
            .map_err(db_error)?;
        return if deleted {
            Ok(rpc_result(c, id, json!({"resultType":"complete"})))
        } else {
            error(
                c,
                id,
                -32023,
                "NotFound",
                Some(json!({"kind":"subscription"})),
            )
        };
    }
    let Some(secret) = p["delivery"]["secret"]
        .as_str()
        .filter(|s| rails_compat::webhook::standard_key(s).is_some())
    else {
        return error(c, id, -32602, "Invalid webhook secret", None);
    };
    let cursor = match p.get("cursor") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) => {
            match s.parse::<i64>() {
                Ok(n) => Some(n),
                Err(_) => return error(c, id, -32602, "Invalid cursor", None),
            }
        }
        _ => return error(c, id, -32602, "Invalid cursor", None),
    };
    let ttl = match p.get("ttlMs") {
        None => 3_600_000,
        Some(Value::Null) => 86_400_000,
        Some(v) => match v.as_i64() {
            Some(n) => n.clamp(60_000, 86_400_000),
            None => return error(c, id, -32602, "Invalid ttlMs", None),
        },
    };
    let max_age = match p.get("maxAgeMs") {
        None => 86_400_000,
        Some(v) => match v.as_i64() {
            Some(n) if n >= 0 => n.min(86_400_000),
            _ => return error(c, id, -32602, "Invalid maxAgeMs", None),
        },
    };
    let agent = identity.agent_id;
    let check_url = url.clone();
    let check_name = name.clone();
    let (allowed, limit, verified) = c
        .app()
        .db
        .read(move |conn| {
            Ok((
                domain::can_subscribe(conn, agent)?,
                domain::at_limit(
                    conn,
                    credential,
                    &domain::subscription_id(credential, &check_url, &check_name),
                )?,
                domain::endpoint_verified(conn, credential, &check_url)?,
            ))
        })
        .await
        .map_err(db_error)?;
    if !allowed {
        return error(c, id, -32024, "Forbidden", None);
    }
    if limit {
        return error(
            c,
            id,
            -32025,
            "ResourceExhausted",
            Some(json!({"limit":"subscriptions","max":20})),
        );
    }
    if !verified {
        let key = format!(
            "mcp:verification:{}",
            parsed_url.host_str().unwrap().to_ascii_lowercase()
        );
        if c.kit()
            .rate_limits()
            .increment(&key, jiff::SignedDuration::from_secs(60), c.now())
            > 10
        {
            return error(
                c,
                id,
                -32025,
                "ResourceExhausted",
                Some(json!({"limit":"verification","max":10})),
            );
        }
        let clock = c.app().clock.clone();
        if let Err(reason) = campfire_app::integrations::mcp_events::verify(
            &c.app().subscription_network,
            &url,
            secret,
            move || clock.now(),
        )
        .await
        {
            return error(
                c,
                id,
                -32027,
                "CallbackEndpointError",
                Some(json!({"reason":reason.as_str()})),
            );
        }
    }
    let grant = Grant {
        agent,
        credential,
        name,
        url,
        secret: secret.into(),
        ttl_ms: ttl,
        cursor,
        max_age_ms: max_age,
    };
    let encryption = c.app().ar_encryption.clone();
    let result = c
        .app()
        .db
        .write(move |tx| domain::subscribe(tx, grant, &encryption))
        .await
        .map_err(db_error)?;
    match result {
        GrantResult::Granted(s, truncated) => Ok(rpc_result(
            c,
            id,
            s.result(
                campfire_db::Timestamp::from_jiff(c.app().clock.now()),
                truncated,
            ),
        )),
        GrantResult::Forbidden => error(c, id, -32024, "Forbidden", None),
        GrantResult::Limit => error(
            c,
            id,
            -32025,
            "ResourceExhausted",
            Some(json!({"limit":"subscriptions","max":20})),
        ),
    }
}
