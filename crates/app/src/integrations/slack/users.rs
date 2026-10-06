//! Workspace-wide Slack author mapping (`app/models/slack/user_mapper.rb`).
use std::collections::{HashMap, HashSet};

use super::runner::{present, string, truthy};
use campfire_db::models::slack::SlackConnection;
use campfire_db::models::slack_import::{IssueLevel, SlackImport};
use campfire_db::{Connection, NewUser, Result, Tx, User};
use indexmap::{IndexMap, IndexSet};
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

pub const SLACKBOT_ID: &str = "USLACKBOT";

pub fn mapped_id(conn: &Connection, workspace: i64, kind: &str, key: &str) -> Result<Option<i64>> {
    Ok(conn.query_row("SELECT record_id FROM slack_import_records WHERE slack_workspace_id=? AND slack_kind=? AND slack_key=?",
        params![workspace,kind,key],|r|r.get(0)).optional()?)
}
pub fn record(
    tx: &Tx<'_>,
    run: &SlackImport,
    kind: &str,
    key: &str,
    record_type: &str,
    id: i64,
    created: bool,
) -> Result<()> {
    tx.conn().execute("INSERT INTO slack_import_records (slack_workspace_id,slack_import_id,slack_kind,slack_key,record_type,record_id,created_record,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?)",
        params![run.slack_workspace_id,run.id,kind,key,record_type,id,created,tx.now(),tx.now()])?;
    Ok(())
}
pub fn users_for(
    conn: &Connection,
    run: &SlackImport,
    ids: &[Value],
) -> Result<IndexMap<String, User>> {
    let ids: IndexSet<_> = ids
        .iter()
        .map(string)
        .filter(|id| !id.trim().is_empty())
        .collect();
    let mut users = IndexMap::new();
    if ids.is_empty() {
        return Ok(users);
    }
    let (sql, bindings) = users_query(run.slack_workspace_id, &ids);
    let mut statement = conn.prepare(&sql)?;
    let mappings = statement
        .query_map(rusqlite::params_from_iter(bindings), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (key, id) in mappings {
        if let Some(user) = User::find_by_id(conn, id)? {
            users.insert(key, user);
        }
    }
    Ok(users)
}

pub fn users_query(workspace: i64, ids: &IndexSet<String>) -> (String, Vec<rusqlite::types::Value>) {
    records_query(
        workspace,
        ids,
        r#""slack_import_records"."slack_key", "slack_import_records"."record_id""#,
    )
}

fn records_query(
    workspace: i64,
    ids: &IndexSet<String>,
    columns: &str,
) -> (String, Vec<rusqlite::types::Value>) {
    use rusqlite::types::Value as SqlValue;

    // UserMapper#records_for, with the pinned SQLite adapter's 999-bind limit.
    // Rails inlines the entire relation above that limit. The SQL shape matters:
    // after ANALYZE, large IN lists can scan rows instead of the identity index.
    let bound = ids.len() + 2 <= 999;
    let (workspace, kind, bindings, values) = if bound {
        let mut bindings = vec![SqlValue::Integer(workspace), SqlValue::Text("user".into())];
        bindings.extend(ids.iter().cloned().map(SqlValue::Text));
        (
            "?".to_owned(),
            "?".to_owned(),
            bindings,
            vec!["?".to_owned(); ids.len()],
        )
    } else {
        // SQLite3::Database.quote doubles single quotes; backslashes stay literal.
        let values = ids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect();
        (
            workspace.to_string(),
            "'user'".to_owned(),
            Vec::new(),
            values,
        )
    };
    let predicate = if values.len() == 1 {
        format!("= {}", values[0])
    } else {
        format!("IN ({})", values.join(", "))
    };
    let sql = format!(
        r#"SELECT {columns} FROM "slack_import_records" WHERE "slack_import_records"."slack_workspace_id" = {workspace} AND "slack_import_records"."slack_kind" = {kind} AND "slack_import_records"."slack_key" {predicate}"#
    );
    (sql, bindings)
}

fn existing_keys(conn: &Connection, workspace: i64, members: &[Value]) -> Result<HashSet<String>> {
    let ids: IndexSet<_> = members
        .iter()
        .filter(|member| truthy(&member["id"]))
        .map(|member| string(&member["id"]))
        .collect();
    if ids.is_empty() {
        return Ok(HashSet::new());
    }
    let (sql, bindings) = records_query(workspace, &ids, r#""slack_import_records"."slack_key""#);
    Ok(conn
        .prepare(&sql)?
        .query_map(rusqlite::params_from_iter(bindings), |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

fn matchable_users(conn: &Connection, members: &[&Value]) -> Result<HashMap<String, User>> {
    // Rails deduplicates the original emails before String#downcase, and indexes
    // users in id order so the last matching user wins, including deactivated rows.
    let emails: IndexSet<_> = members
        .iter()
        .filter(|member| !placeholder_only(member))
        .filter_map(|member| email(member))
        .collect();
    let mut users = HashMap::new();
    if emails.is_empty() {
        return Ok(users);
    }
    let emails: Vec<_> = emails
        .iter()
        .map(|email| super::payload::downcase(email))
        .collect();
    let bound = emails.len() <= 999;
    let values = if bound {
        vec!["?".to_owned(); emails.len()]
    } else {
        emails
            .iter()
            .map(|email| format!("'{}'", email.replace('\'', "''")))
            .collect()
    };
    let sql = format!(
        r#"SELECT "users".* FROM "users" WHERE (LOWER(email_address) IN ({})) ORDER BY "users"."id" ASC"#,
        values.join(", ")
    );
    let bindings = if bound { emails } else { Vec::new() };
    for user in conn
        .prepare(&sql)?
        .query_map(rusqlite::params_from_iter(bindings), User::from_row)?
    {
        let user = user?;
        users.insert(
            super::payload::downcase(user.email_address.as_deref().unwrap_or_default()),
            user,
        );
    }
    Ok(users)
}
pub fn ensure_author(tx: &mut Tx<'_>, run: &SlackImport, key: &str) -> Result<User> {
    if let Some(id) = mapped_id(tx.conn(), run.slack_workspace_id, "user", key)?
        && let Some(user) = User::find_by_id(tx.conn(), id)?
    {
        return Ok(user);
    }
    placeholder(tx, run, key, "Unknown Slack user")
}
pub fn bot_user(tx: &mut Tx<'_>, run: &SlackImport, message: &Value) -> Result<User> {
    let key = bot_key(message);
    if let Some(id) = mapped_id(tx.conn(), run.slack_workspace_id, "user", &key)?
        && let Some(user) = User::find_by_id(tx.conn(), id)?
    {
        return Ok(user);
    }
    placeholder(
        tx,
        run,
        &key,
        present(&message["username"])
            .as_deref()
            .unwrap_or("Slack bot"),
    )
}
fn placeholder(tx: &mut Tx<'_>, run: &SlackImport, key: &str, name: &str) -> Result<User> {
    let user = User::create_slack_placeholder(
        tx,
        NewUser {
            name: name.into(),
            ..Default::default()
        },
        false,
        None,
        false,
    )?;
    record(tx, run, "user", key, "User", user.id, true)?;
    Ok(user)
}
pub fn bot_key(message: &Value) -> String {
    present(&message["bot_id"]).map_or_else(
        || {
            format!(
                "botname:{}",
                present(&message["username"])
                    .as_deref()
                    .unwrap_or("unknown")
            )
        },
        |id| format!("bot:{id}"),
    )
}
pub fn display_name(member: &Value) -> Option<String> {
    [
        &member["profile"]["real_name"],
        &member["profile"]["display_name"],
        &member["real_name"],
        &member["name"],
    ]
    .into_iter()
    .find_map(present)
}
fn member_name(member: &Value) -> String {
    display_name(member).unwrap_or_else(|| "Slack user".into())
}
fn bot_like(member: &Value) -> bool {
    member["id"] == SLACKBOT_ID || truthy(&member["is_bot"]) || truthy(&member["is_app_user"])
}
fn placeholder_only(member: &Value) -> bool {
    bot_like(member)
        || ["deleted", "is_restricted", "is_ultra_restricted"]
            .iter()
            .any(|key| truthy(&member[*key]))
}
fn email(member: &Value) -> Option<String> {
    present(&member["profile"]["email"])
        .or_else(|| present(&member["email"]))
        .map(|s| {
            s.trim_matches(|c: char| c.is_ascii_whitespace() || c == '\0')
                .to_owned()
        })
}

pub fn map_page(
    tx: &mut Tx<'_>,
    run: &SlackImport,
    members: &Value,
    dry: bool,
    allowed_domains: &HashSet<String>,
) -> Result<Value> {
    let mut delta = json!({"matched":0,"placeholders":0,"deactivated":0,"bots":0,"total":0});
    let members = super::payload::array(members);
    if members.is_empty() {
        return Ok(delta);
    }
    for member in &members {
        super::payload::at(member, "id")?;
    }
    // Rails preloads known keys and email matches once, before creating any new placeholder.
    // Duplicate fresh Slack ids/emails therefore remain genuine uniqueness failures.
    let known = existing_keys(tx.conn(), run.slack_workspace_id, &members)?;
    // Rails excludes already mapped identities before looking up profile emails. Dry
    // previews also inspect blank-id rows while building their email index; real imports
    // discard those rows first.
    for member in &members {
        if known.contains(&string(&member["id"]))
            || (!dry && present(&member["id"]).is_none())
            || placeholder_only(member)
        {
            continue;
        }
        if member.is_string() {
            return Err(super::payload::no_method("dig", member).into());
        }
        if let Some(profile) = member.get("profile").filter(|p| !p.is_null())
            && !profile.is_object()
        {
            return Err(if profile.is_array() {
                super::payload::type_error()
            } else {
                super::payload::Error {
                    class: "TypeError",
                    message: format!(
                        "{} does not have #dig method",
                        match profile {
                            Value::Bool(false) => "FalseClass",
                            Value::Bool(true) => "TrueClass",
                            Value::String(_) => "String",
                            Value::Number(n) if n.is_f64() => "Float",
                            _ => "Integer",
                        }
                    ),
                }
            }
            .into());
        }
    }
    let fresh: Vec<_> = members
        .iter()
        .filter(|m| present(&m["id"]).is_some() && !known.contains(&string(&m["id"])))
        .collect();
    let candidates: Vec<_> = members
        .iter()
        .filter(|member| {
            !known.contains(&string(&member["id"])) && (dry || present(&member["id"]).is_some())
        })
        .collect();
    let email_index = matchable_users(tx.conn(), &candidates)?;
    let connection = run
        .slack_connection_id
        .map(|id| SlackConnection::find(tx.conn(), id))
        .transpose()?
        .flatten();
    let owner = connection
        .as_ref()
        .map(|c| User::find_by_id(tx.conn(), c.user_id))
        .transpose()?
        .flatten();
    for member in fresh {
        let key = string(&member["id"]);
        let name = member_name(member);
        let email = email(member);
        let (bucket, existing) = if connection.as_ref().is_some_and(|c| c.slack_user_id == key)
            && owner.is_some()
        {
            ("matched", owner.clone())
        } else if bot_like(member) {
            ("bots", None)
        } else if placeholder_only(member) || email.as_ref().is_none_or(|s| s.trim().is_empty()) {
            ("deactivated", None)
        } else if let Some(user) =
            email_index.get(&super::payload::downcase(email.as_ref().unwrap()))
        {
            ("matched", Some(user.clone()))
        } else {
            ("placeholders", None)
        };
        if !dry {
            let created = existing.is_none();
            let user = if let Some(user) = existing {
                user
            } else {
                let active = bucket == "placeholders";
                let name = if key == SLACKBOT_ID {
                    "Slackbot".to_owned()
                } else {
                    name.clone()
                };
                let domain = super::payload::downcase(
                    email
                        .as_ref()
                        .and_then(|s| s.rsplit('@').next())
                        .unwrap_or("")
                        .trim(),
                );
                User::create_slack_placeholder(
                    tx,
                    NewUser {
                        name,
                        email_address: active.then(|| email.clone()).flatten(),
                        bio: active
                            .then(|| present(&member["profile"]["title"]))
                            .flatten(),
                        ..Default::default()
                    },
                    active,
                    present(&member["tz"]).as_deref(),
                    active && allowed_domains.contains(&domain),
                )?
            };
            record(tx, run, "user", &key, "User", user.id, created)?;
            if bucket == "deactivated"
                && !placeholder_only(member)
                && email.as_ref().is_none_or(|s| s.trim().is_empty())
            {
                SlackImport::record_issue(
                    tx,
                    run.id,
                    IssueLevel::Warning,
                    Some(&format!("user:{key}")),
                    &format!("Slack user {name} has no email address; imported as deactivated"),
                )?;
            }
        }
        super::runner::add(&mut delta[bucket], 1);
        super::runner::add(&mut delta["total"], 1);
    }
    Ok(delta)
}
