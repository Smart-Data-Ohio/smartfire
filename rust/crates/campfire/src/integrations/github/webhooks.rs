//! Inbound delivery authentication and selection (`Github::WebhooksController` and
//! `Github::WebhookDelivery`). A delivery claim, privacy writes and its durable jobs commit
//! together. No network calls or HTML enter this transaction.
use std::collections::BTreeSet;

use campfire_db::{Database, Event, Tx};
use jiff::SignedDuration;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use super::{
    blank,
    jobs::{DeliverSubscriptionEventJob, FetchPullRequestJob},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authentication {
    Accepted,
    Unauthorized,
    Unavailable,
}

pub fn authenticate(
    secret: Option<&str>,
    signature: Option<&str>,
    guid: &str,
    raw: &[u8],
) -> Authentication {
    let Some(secret) = secret.filter(|secret| !blank(secret)) else {
        return Authentication::Unavailable;
    };
    if !rails_compat::webhook::verify_github_signature(secret, raw, signature) || blank(guid) {
        return Authentication::Unauthorized;
    }
    Authentication::Accepted
}

/// Authentication must precede this call. Duplicate deliveries never parse the payload or
/// prune rows. A signed, malformed *JSON shape* keeps its claim before returning Rails' 500;
/// invalid JSON itself is the controller's empty payload and is acknowledged.
pub async fn receive(
    db: &Database,
    guid: String,
    event: String,
    raw: Vec<u8>,
) -> anyhow::Result<()> {
    db.write(move |tx| {
        if !claim(tx, &guid, &event)? {
            return Ok(Ok(()));
        }
        let payload = serde_json::from_slice::<Value>(&raw).unwrap_or_else(|_| json!({}));
        let plan = match plan(tx.conn(), &event, &payload) {
            Ok(plan) => plan,
            Err(error) => return Ok(Err(error)),
        };
        for id in plan.pull_requests {
            if let Some(private) = payload
                .get("repository")
                .and_then(Value::as_object)
                .and_then(|repository| repository.get("private"))
            {
                store_privacy(tx, id, private)?;
            }
            tx.emit_after_commit(Event::job(&FetchPullRequestJob {
                pull_request_id: id,
            }));
        }
        if plan.subscription {
            tx.emit_after_commit(Event::job(&DeliverSubscriptionEventJob { event, payload }));
        }
        Ok(Ok(()))
    })
    .await?
}

/// The unique index is the claim, including across processes. Targeting this index rather than
/// `INSERT OR IGNORE` prevents other constraints or storage failures being mistaken for dedupe.
pub fn claim(tx: &mut Tx<'_>, guid: &str, event: &str) -> campfire_db::Result<bool> {
    if blank(guid) {
        return Ok(false);
    }
    let now = tx.now();
    let inserted = tx.conn().execute("INSERT INTO github_webhook_deliveries (delivery_guid, event, created_at, updated_at) VALUES (?, ?, ?, ?) ON CONFLICT(delivery_guid) DO NOTHING", params![guid, event, now, now])?;
    if inserted == 0 {
        return Ok(false);
    }
    tx.conn().execute(
        "DELETE FROM github_webhook_deliveries WHERE created_at < ?",
        [tx.now().ago(SignedDuration::from_hours(7 * 24))],
    )?;
    Ok(true)
}

struct Plan {
    pull_requests: Vec<i64>,
    subscription: bool,
}

fn plan(conn: &Connection, event: &str, payload: &Value) -> anyhow::Result<Plan> {
    let mut candidates = Vec::new();
    match event {
        "pull_request" | "pull_request_review" => {
            if let Some(pr) = get(payload, "pull_request")?.filter(|pr| truthy(pr)) {
                let full_name =
                    match dig(pr, &["base", "repo", "full_name"])?.filter(|name| truthy(name)) {
                        Some(name) => Some(name),
                        None => dig(payload, &["repository", "full_name"])?,
                    };
                if let (Some(full_name), Some(number)) = (
                    full_name.filter(|name| truthy(name)),
                    get(pr, "number")?.filter(|number| truthy(number)),
                ) && let Some((owner, repo)) = owner_and_repo(full_name)?
                {
                    candidates.push((owner, repo, number.clone()));
                }
            }
        }
        "issue_comment" => {
            if let Some(issue) = get(payload, "issue")?.filter(|issue| truthy(issue)) {
                let number = get(issue, "number")?.filter(|number| truthy(number));
                let is_pr = issue
                    .as_object()
                    .ok_or_else(shape_error)?
                    .contains_key("pull_request");
                if let Some(number) = number.filter(|_| is_pr)
                    && let Some(full_name) =
                        dig(payload, &["repository", "full_name"])?.filter(|name| truthy(name))
                    && let Some((owner, repo)) = owner_and_repo(full_name)?
                {
                    candidates.push((owner, repo, number.clone()));
                }
            }
        }
        "check_suite" | "check_run" => {
            let full_name = dig(payload, &["repository", "full_name"])?;
            let check = get(payload, event)?;
            let numbers = check
                .filter(|check| !check.is_null())
                .map(|check| dig(check, &["pull_requests"]))
                .transpose()?
                .flatten()
                .filter(|numbers| truthy(numbers));
            if let Some(full_name) = full_name.filter(|name| truthy(name))
                && let Some((owner, repo)) = owner_and_repo(full_name)?
                && let Some(numbers) = numbers
            {
                for pr in array(numbers)? {
                    if let Some(number) = get(pr, "number")?.filter(|number| truthy(number)) {
                        candidates.push((owner.clone(), repo.clone(), number.clone()));
                    }
                }
            }
        }
        "status" => {
            let full_name = dig(payload, &["repository", "full_name"])?;
            let branches = get(payload, "branches")?.filter(|branches| truthy(branches));
            let mut names = Vec::new();
            if let Some(branches) = branches {
                for branch in array(branches)? {
                    if let Some(name) = get(branch, "name")?.filter(|name| truthy(name)) {
                        names.push(name.clone());
                    }
                }
            }
            if let Some(full_name) = full_name.filter(|name| truthy(name))
                && !names.is_empty()
                && let Some((owner, repo)) = owner_and_repo(full_name)?
            {
                let mut statement = conn.prepare("SELECT number, head_branch FROM github_pull_requests WHERE owner = ? AND repo = ?")?;
                for row in statement.query_map(params![owner, repo], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
                })? {
                    let (number, branch) = row?;
                    if branch.is_some_and(|branch| {
                        names.iter().any(|name| name.as_str() == Some(&branch))
                    }) {
                        candidates.push((owner.clone(), repo.clone(), json!(number)));
                    }
                }
            }
        }
        _ => {}
    }
    let mut ids = BTreeSet::new();
    let mut pull_requests = Vec::new();
    for (owner, repo, number) in candidates {
        let number = integer_for_query(&number);
        let id: Option<i64> = conn.query_row("SELECT id FROM github_pull_requests WHERE owner = ? AND repo = ? AND number = ? AND EXISTS (SELECT 1 FROM github_pull_request_references WHERE github_pull_request_id = github_pull_requests.id)", params![owner, repo, number], |row| row.get(0)).optional()?;
        if let Some(id) = id.filter(|id| ids.insert(*id)) {
            pull_requests.push(id);
        }
    }
    let subscription = if event == "issue_comment" {
        false
    } else if let Some((owner, repo)) = repository_owner_and_repo(payload)? {
        conn.query_row("SELECT EXISTS (SELECT 1 FROM github_repository_subscriptions WHERE owner = ? AND repo = ?)", params![owner, repo], |row| row.get(0))?
    } else {
        false
    };
    Ok(Plan {
        pull_requests,
        subscription,
    })
}

/// `Github::Notifier.repository_owner_and_repo`: strict repository identity for subscription
/// lookup, in contrast to the webhook's looser PR candidate splitter.
pub fn repository_owner_and_repo(payload: &Value) -> anyhow::Result<Option<(String, String)>> {
    let full_name = dig(payload, &["repository", "full_name"])?.filter(|name| truthy(name));
    let full_name = match full_name {
        Some(name) => Some(name),
        None => dig(payload, &["pull_request", "base", "repo", "full_name"])?,
    };
    let Some(full_name) = full_name.and_then(Value::as_str) else {
        return Ok(None);
    };
    let Some((owner, repo)) = full_name.split_once('/') else {
        return Ok(None);
    };
    if blank(owner) || blank(repo) || repo.contains('/') {
        return Ok(None);
    }
    Ok(Some((owner.to_lowercase(), repo.to_lowercase())))
}

fn shape_error() -> anyhow::Error {
    anyhow::anyhow!("Malformed GitHub webhook payload shape")
}
pub(super) fn truthy(value: &Value) -> bool {
    !matches!(value, Value::Null | Value::Bool(false))
}
pub(super) fn get<'a>(value: &'a Value, key: &str) -> anyhow::Result<Option<&'a Value>> {
    Ok(value.as_object().ok_or_else(shape_error)?.get(key))
}
pub(super) fn dig<'a>(value: &'a Value, keys: &[&str]) -> anyhow::Result<Option<&'a Value>> {
    let Some((key, rest)) = keys.split_first() else {
        return Ok(Some(value));
    };
    match get(value, key)? {
        Some(Value::Null) if !rest.is_empty() => Ok(None),
        Some(value) if !rest.is_empty() => dig(value, rest),
        value => Ok(value),
    }
}
fn array(value: &Value) -> anyhow::Result<&Vec<Value>> {
    value.as_array().ok_or_else(shape_error)
}
fn owner_and_repo(value: &Value) -> anyhow::Result<Option<(String, String)>> {
    let name = value.as_str().ok_or_else(shape_error)?;
    Ok(name
        .split_once('/')
        .map(|(owner, repo)| (owner.to_lowercase(), repo.to_lowercase())))
}
pub(super) fn integer_for_query(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|number| number as i64)),
        Value::String(number) if !blank(number) => Some(crate::ruby::ruby_to_i(number)),
        Value::Bool(value) => Some(i64::from(*value)),
        _ => None,
    }
}
fn store_privacy(tx: &mut Tx<'_>, id: i64, value: &Value) -> campfire_db::Result<()> {
    let private = super::boolean(value);
    let changed:bool=tx.conn().query_row("SELECT private IS NOT ? FROM github_pull_requests WHERE id=?",params![private,id],|r|r.get(0))?;
    if changed {super::pull_requests::update(tx,id,&[("private",private.map_or(rusqlite::types::Value::Null,|v|rusqlite::types::Value::Integer(i64::from(v))))])?;}
    Ok(())
}
