//! `app/models/github/pull_request_fetcher.rb`: persisted card fields and file summaries.
use super::client::{
    Error, ErrorKind, PullRequestKey, ReadClient, present, ruby_field, ruby_string, ruby_to_i,
};
use campfire_db::{Database, Timestamp};
use rusqlite::{OptionalExtension, types::Value as SqlValue};
use serde_json::{Value, json};

type Attributes = Vec<(&'static str, SqlValue)>;

/// HTTP runs outside SQLite transactions. Only the completed snapshot/error is written;
/// a failed files HTTP call preserves the prior summary, while malformed JSON/shape errors
/// discard all newly collected card attributes, as Rails' outer rescue does.
pub async fn fetch(db: &Database, client: &ReadClient, id: i64) -> campfire_db::Result<()> {
    let key = db
        .read(move |conn| {
            Ok(conn
                .query_row(
                    "SELECT owner, repo, number FROM github_pull_requests WHERE id = ?",
                    [id],
                    |row| {
                        Ok(PullRequestKey {
                            owner: row.get(0)?,
                            repo: row.get(1)?,
                            number: row.get(2)?,
                        })
                    },
                )
                .optional()?)
        })
        .await?
        .ok_or(campfire_db::Error::RecordNotFound("Github::PullRequest"))?;
    let attributes = match collect(db, client, &key, id).await {
        Ok(attributes) => attributes,
        Err(error) => {
            // Error strings are normalized at the client/shape boundary and never contain
            // tokens, response bodies or raw network error messages.
            if error.kind != ErrorKind::Fetch {
                tracing::warn!(pull_request_id = id, "Github::PullRequestFetcher failed");
            }
            vec![
                ("fetched_at", SqlValue::Text(db.env().now().to_db())),
                ("fetch_error", SqlValue::Text(error.message)),
            ]
        }
    };
    db.write(move |tx| {
        super::pull_requests::update(tx, id, &attributes).map(|_| ())
    })
    .await
}

async fn collect(
    db: &Database,
    client: &ReadClient,
    key: &PullRequestKey,
    id: i64,
) -> Result<Attributes, Error> {
    let data = client.pull_request(key).await?;
    let head = field(&data, "head")?;
    let sha = field(&head, "sha")?;
    let mut attributes = Vec::new();
    attributes.push((
        "private",
        super::boolean(&dig(&data, &["base", "repo", "private"])?)
            .map_or(SqlValue::Null, |value| SqlValue::Integer(i64::from(value))),
    ));
    for (column, path) in [
        ("title", vec!["title"]),
        ("author_login", vec!["user", "login"]),
        ("author_avatar_url", vec!["user", "avatar_url"]),
        ("base_branch", vec!["base", "ref"]),
        ("head_branch", vec!["head", "ref"]),
        ("head_sha", vec!["head", "sha"]),
        ("html_url", vec!["html_url"]),
    ] {
        attributes.push((column, string_column(dig(&data, &path)?)));
    }
    let state = if present(Some(&field(&data, "merged_at")?)) {
        "merged"
    } else if field(&data, "state")? == "closed" {
        "closed"
    } else if truthy(&field(&data, "draft")?) {
        "draft"
    } else {
        "open"
    };
    attributes.push(("state", SqlValue::Text(state.into())));
    let updated = field(&data, "updated_at")?;
    attributes.push(("github_updated_at", datetime_column(&updated)));
    attributes.push((
        "review_decision",
        SqlValue::Text(review_decision(client, key).await?.into()),
    ));
    attributes.push((
        "check_status",
        check_status(client, key, &ruby_string(&sha))
            .await?
            .map_or(SqlValue::Null, |s| SqlValue::Text(s.into())),
    ));
    // Rails' JSON column encoder disables HTML escaping; the files text's #to_json
    // below uses ActiveSupport's default HTML escaping instead.
    attributes.push(("payload", SqlValue::Text(data.to_string())));
    attributes.push(("fetched_at", SqlValue::Text(db.env().now().to_db())));
    attributes.push(("fetch_error", SqlValue::Null));
    let mapped = db.read(move |conn| Ok(conn.query_row("SELECT EXISTS(SELECT 1 FROM github_pull_request_threads WHERE github_pull_request_id = ?)", [id], |row| row.get::<_, bool>(0))?)).await
        .map_err(|_| shape_error("Statement Invalid"))?;
    if mapped {
        match changed_files(client, key, field(&data, "changed_files")?).await {
            Ok(files) => {
                attributes.push((
                    "changed_files",
                    SqlValue::Text(rails_compat::json_encode(&files)),
                ));
                attributes.push((
                    "changed_files_fetched_at",
                    SqlValue::Text(db.env().now().to_db()),
                ));
            }
            Err(error) if error.kind == ErrorKind::Fetch => {
                attributes
                    .iter_mut()
                    .find(|(column, _)| *column == "fetch_error")
                    .unwrap()
                    .1 = SqlValue::Text(error.message);
            }
            Err(error) => return Err(error),
        }
    }
    Ok(attributes)
}

async fn review_decision(client: &ReadClient, key: &PullRequestKey) -> Result<&'static str, Error> {
    let reviews = match client.reviews(key).await {
        Ok(value) => value,
        Err(error) if error.kind == ErrorKind::Fetch => return Ok("review_required"),
        Err(error) => return Err(error),
    };
    let mut latest: Vec<(Value, String, Value)> = Vec::new();
    for review in reviews.as_array().into_iter().flatten() {
        let state = field(review, "state")?;
        if state != "APPROVED" && state != "CHANGES_REQUESTED" {
            continue;
        }
        let id = dig(review, &["user", "id"])?;
        let reviewer = if truthy(&id) {
            id
        } else {
            dig(review, &["user", "login"])?
        };
        let at = ruby_string(&field(review, "submitted_at")?);
        if let Some(existing) = latest.iter_mut().find(|(who, _, _)| *who == reviewer) {
            if at > existing.1 {
                *existing = (reviewer, at, state);
            }
        } else {
            latest.push((reviewer, at, state));
        }
    }
    Ok(
        if latest
            .iter()
            .any(|(_, _, state)| state == "CHANGES_REQUESTED")
        {
            "changes_requested"
        } else if latest.iter().any(|(_, _, state)| state == "APPROVED") {
            "approved"
        } else {
            "review_required"
        },
    )
}

async fn check_status(
    client: &ReadClient,
    key: &PullRequestKey,
    sha: &str,
) -> Result<Option<&'static str>, Error> {
    match collect_checks(client, key, sha).await {
        Err(error) if error.kind == ErrorKind::Fetch => Ok(None),
        result => result,
    }
}
async fn collect_checks(
    client: &ReadClient,
    key: &PullRequestKey,
    sha: &str,
) -> Result<Option<&'static str>, Error> {
    let data = client.check_runs(key, sha).await?;
    let runs = fetch_field(&data, "check_runs", json!([]))?;
    let runs = runs.as_array().ok_or_else(|| {
        shape_error(if runs.is_object() {
            "Type Error"
        } else {
            "No Method Error"
        })
    })?;
    let mut statuses = Vec::new();
    for run in runs {
        let status = if field(run, "status")? == "completed" {
            field(run, "conclusion")?
        } else {
            Value::String("pending".into())
        };
        if truthy(&status) {
            statuses.push(status);
        }
    }
    let combined = client.status(key, sha).await?;
    let state = fetch_field(&combined, "state", Value::Null)?;
    let has_statuses = to_i(&fetch_field(&combined, "total_count", json!(0))?)? > 0;
    if present(Some(&state)) && (state != "pending" || (statuses.is_empty() && has_statuses)) {
        statuses.push(state);
    }
    let has = |names: &[&str]| {
        statuses
            .iter()
            .any(|status| status.as_str().is_some_and(|s| names.contains(&s)))
    };
    Ok(
        if has(&["failure", "error", "timed_out", "action_required"]) {
            Some("failing")
        } else if has(&["pending", "in_progress", "queued", "requested", "waiting"]) {
            Some("pending")
        } else if has(&["success", "neutral", "skipped"]) {
            Some("passing")
        } else {
            None
        },
    )
}

async fn changed_files(
    client: &ReadClient,
    key: &PullRequestKey,
    total: Value,
) -> Result<Value, Error> {
    let data = client.files(key).await?;
    let files = data.as_array().into_iter().flatten().take(100).map(|file| Ok(json!({"filename":field(file,"filename")?,"additions":to_i(&field(file,"additions")?)?,"deletions":to_i(&field(file,"deletions")?)?,"status":field(file,"status")?}))).collect::<Result<Vec<_>,Error>>()?;
    let total = to_i(&total)?;
    let total = if total > 0 { total } else { files.len() as i64 };
    Ok(json!({"files":files,"total_count":total}))
}

fn truthy(value: &Value) -> bool {
    !matches!(value, Value::Null | Value::Bool(false))
}
fn shape_error(class: &str) -> Error {
    Error::new(
        ErrorKind::Other,
        format!("Could not reach GitHub ({class})"),
    )
}
fn field(value: &Value, key: &str) -> Result<Value, Error> {
    Ok(ruby_field(value, key)?.unwrap_or(Value::Null))
}
fn fetch_field(value: &Value, key: &str, default: Value) -> Result<Value, Error> {
    match value {
        Value::Object(object) => Ok(object.get(key).cloned().unwrap_or(default)),
        Value::Array(_) => Err(shape_error("Type Error")),
        _ => Err(shape_error("No Method Error")),
    }
}
fn dig(value: &Value, path: &[&str]) -> Result<Value, Error> {
    let mut value = value;
    for (index, key) in path.iter().enumerate() {
        match value {
            Value::Object(object) => value = object.get(*key).unwrap_or(&Value::Null),
            Value::Null if index > 0 => return Ok(Value::Null),
            Value::Array(_) => return Err(shape_error("Type Error")),
            _ => {
                return Err(shape_error(if index == 0 {
                    "No Method Error"
                } else {
                    "Type Error"
                }));
            }
        }
    }
    Ok(value.clone())
}
fn to_i(value: &Value) -> Result<i64, Error> {
    match value {
        Value::Null => Ok(0),
        Value::String(s) => Ok(ruby_to_i(s)),
        Value::Number(n) => Ok(n
            .as_i64()
            .unwrap_or_else(|| n.as_f64().unwrap_or(0.0) as i64)),
        _ => Err(shape_error("No Method Error")),
    }
}
fn string_column(value: Value) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Text(if b { "t" } else { "f" }.into()),
        value => SqlValue::Text(ruby_string(&value)),
    }
}
fn datetime_column(value: &Value) -> SqlValue {
    value
        .as_str()
        .and_then(|s| {
            s.parse::<jiff::Timestamp>()
                .ok()
                .map(Timestamp::from_jiff)
                .or_else(|| Timestamp::parse_db(s))
        })
        .map_or(SqlValue::Null, |at| SqlValue::Text(at.to_db()))
}
#[cfg(test)]
mod tests;
