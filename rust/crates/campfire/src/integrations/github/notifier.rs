//! `app/models/github/notifier.rb`: plans contain text/data; the cable sink renders messages.
use super::{
    blank,
    client::{ruby_string, ruby_strip},
    webhooks::{dig, get, integer_for_query, truthy},
};
use campfire_db::{ActivityItem, ChannelThread, Database, Event, Message, NewMessage, Tx, User};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone)]
struct Post {
    event: &'static str,
    owner: String,
    repo: String,
    number: Value,
    url: String,
    line: String,
    redacted: String,
    suffix: String,
    reviewer: Option<Value>,
}
#[derive(Clone)]
struct Subscription {
    id: i64,
    room_id: i64,
    owner: String,
    repo: String,
    verified: bool,
}

/// Claims survive a later post failure, as in Rails. All claims precede bot creation or posts;
/// each successful post, its notification link, inbox row and queued jobs commit together.
pub async fn deliver(db: &Database, event: String, payload: Value) -> campfire_db::Result<()> {
    let (posts,subscriptions,public)=db.read(move|conn| {
        let posts=plan(conn,&event,&payload).map_err(|e|campfire_db::Error::Other(e.to_string()))?;
        let Some(first)=posts.first() else {return Ok((posts,Vec::new(),false))};
        let public=dig(&payload,&["repository","private"]).map_err(|e|campfire_db::Error::Other(e.to_string()))?==Some(&Value::Bool(false));
        let mut stmt=conn.prepare("SELECT s.id,s.room_id,s.owner,s.repo,s.reader_verified,s.events FROM github_repository_subscriptions s JOIN rooms r ON r.id=s.room_id WHERE r.deleted_at IS NULL AND s.owner=? AND s.repo=? ORDER BY s.id")?;
        let subscriptions=stmt.query_map(params![first.owner,first.repo],|r| {
            let events:String=r.get(5)?;
            Ok((Subscription {id:r.get(0)?,room_id:r.get(1)?,owner:r.get(2)?,repo:r.get(3)?,verified:r.get(4)?},serde_json::from_str::<Value>(&events).unwrap_or(Value::Null)))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((posts,subscriptions,public))
    }).await?;
    let mut claims = Vec::new();
    for (subscription, events) in subscriptions {
        let subscribed = match &events {
            Value::Array(events) => events.iter().any(|e| e.as_str() == Some(posts[0].event)),
            Value::String(events) => events.contains(posts[0].event),
            _ => {
                return Err(campfire_db::Error::Other(
                    "Invalid subscription events".into(),
                ));
            }
        };
        if !subscribed {
            continue;
        }
        for post in &posts {
            let key = format!(
                "{}:{}/{}#{}{}",
                post.event,
                subscription.owner,
                subscription.repo,
                ruby_string(&post.number),
                post.suffix
            );
            let id = subscription.id;
            // Rails commits each claim independently, even if a later claim or post fails.
            if let Some(id) = db.write(move |tx| super::subscriptions::claim_notification(tx, id, &key)).await? {
                claims.push((
                    subscription.clone(),
                    id,
                    post.clone(),
                    !public && !subscription.verified,
                ));
            }
        }
    }
    if claims.is_empty() {
        return Ok(());
    }
    let bot = db.write(bot_user).await?;
    for (subscription, notification, post, redact) in claims {
        db.write(move|tx| {
            let thread_id:Option<i64>=tx.conn().query_row("SELECT t.channel_thread_id FROM github_pull_request_threads t JOIN github_pull_requests p ON p.id=t.github_pull_request_id WHERE t.room_id=? AND p.owner=? AND p.repo=? AND p.number=? LIMIT 1",params![subscription.room_id,post.owner,post.repo,integer_for_query(&post.number)],|r|r.get(0)).optional()?;
            let source=format!("{}\n{}",if redact {&post.redacted}else{&post.line},post.url);
            let attributes=NewMessage {room_id:subscription.room_id,creator_id:bot,markdown_source:Some(source.clone()),..Default::default()};
            let message=match thread_id.map(|id|ChannelThread::find_by_id(tx.conn(),id)).transpose()?.flatten() {
                Some(mut thread) if thread.locked_at.is_none()=>thread.post_message(tx,bot,attributes)?,
                _=>Message::create_markdown(tx,attributes,&source)?,
            };
            tx.emit_after_commit(Event::broadcast(&MessageCreated {room_id:message.room_id,message_id:message.id,thread_id:message.thread_id}));
            tx.conn().execute("UPDATE github_notifications SET message_id=?,updated_at=? WHERE id=?",params![message.id,tx.now(),notification])?;
            if post.event=="review_requested" {record_review_request(tx,&message,post.reviewer.as_ref())?;}
            Ok(())
        }).await?;
    }
    Ok(())
}
pub(super) fn bot_user(tx: &mut Tx<'_>) -> campfire_db::Result<i64> {
    if let Some(id) = tx
        .conn()
        .query_row(
            "SELECT id FROM users WHERE status=0 AND role=2 AND name='GitHub' ORDER BY id LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    Ok(User::create_integration_bot(tx, "GitHub")?.id)
}
fn record_review_request(
    tx: &mut Tx<'_>,
    message: &Message,
    login: Option<&Value>,
) -> campfire_db::Result<()> {
    let Some(login) = login.filter(|v| !value_blank(v)) else {
        return Ok(());
    };
    let login = ruby_strip(&ruby_string(login)).to_lowercase();
    let reviewer:Option<(i64,String)>=tx.conn().query_row("SELECT u.id,u.inbox_preferences FROM users u JOIN memberships m ON m.user_id=u.id AND m.room_id=? WHERE u.status=0 AND u.role!=2 AND u.github_login=? AND (m.involvement IS NULL OR m.involvement!='invisible') ORDER BY u.id LIMIT 1",params![message.room_id,login],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let Some((id, raw)) = reviewer else {
        return Ok(());
    };
    let preferences: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    if [json!(false), json!(0), json!("0"), json!("false")]
        .contains(&preferences["github_review_requests"])
    {
        return Ok(());
    }
    // create_or_find leaves an existing inbox item's read/type state alone.
    if ActivityItem::find_by_user_and_source(tx.conn(), id, "Message", message.id)?.is_none() {
        ActivityItem::refresh_unread(tx, id, "Message", message.id, "pr_review_request")?;
    }
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageCreated {
    pub room_id: i64,
    pub message_id: i64,
    pub thread_id: Option<i64>,
}
impl campfire_db::Broadcast for MessageCreated {
    const KIND: &'static str = "Github::Notifier#broadcast_create";
}

fn field<'a>(value: &'a Value, key: &str) -> anyhow::Result<&'a Value> {
    Ok(get(value, key)?.unwrap_or(&Value::Null))
}
fn value_blank(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => true,
        Value::String(v) => blank(v),
        Value::Array(v) => v.is_empty(),
        Value::Object(v) => v.is_empty(),
        _ => false,
    }
}
fn inline(value: &Value) -> String {
    let text = ruby_string(value).replace("@[", "@\u{200b}[");
    let text = text
        .split(['\r', '\n'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let text = ruby_strip(&text);
    if blank(text) {
        "Someone".into()
    } else {
        text.into()
    }
}
fn pr_ref(number: &Value, title: &Value) -> String {
    if value_blank(title) {
        format!("#{}", ruby_string(number))
    } else {
        format!("#{}: {}", ruby_string(number), inline(title))
    }
}
fn check_line(number: &Value, title: &Value, name: &Value) -> String {
    let mut line = format!("Checks failed on {}", pr_ref(number, title));
    if !value_blank(name) {
        line.push_str(&format!(" (`{}`)", inline(name)));
    }
    line
}
fn plan(conn: &Connection, event: &str, payload: &Value) -> anyhow::Result<Vec<Post>> {
    if !matches!(
        event,
        "pull_request" | "pull_request_review" | "check_run" | "check_suite" | "status"
    ) {
        return Ok(Vec::new());
    }
    let mut posts = Vec::new();
    // Match the source's event gates before repository lookup, including shape errors.
    match event {
        "pull_request" => {
            let pr = field(payload, "pull_request")?;
            if !truthy(pr) || !truthy(field(pr, "number")?) {
                return Ok(posts);
            }
            let Some((owner, repo)) = super::webhooks::repository_owner_and_repo(payload)? else {
                return Ok(posts);
            };
            let number = field(pr, "number")?.clone();
            let title = field(pr, "title")?;
            let url = format!(
                "https://github.com/{owner}/{repo}/pull/{}",
                ruby_string(&number)
            );
            let sender = dig(payload, &["sender", "login"])?.unwrap_or(&Value::Null);
            let action = field(payload, "action")?.as_str().unwrap_or("");
            let (key, actor, verb, suffix, reviewer) = match action {
                "opened" => ("opened", sender, "opened pull request", "".into(), None),
                "reopened" => ("opened", sender, "reopened pull request", "".into(), None),
                "ready_for_review" => ("opened", sender, "marked", "".into(), None),
                "closed" if truthy(field(pr, "merged")?) => (
                    "merged",
                    dig(pr, &["merged_by", "login"])?
                        .filter(|v| truthy(v))
                        .unwrap_or(sender),
                    "merged",
                    "".into(),
                    None,
                ),
                "closed" => (
                    "closed",
                    sender,
                    "closed",
                    format!(":{}", ruby_string(field(pr, "closed_at")?)),
                    None,
                ),
                "review_requested" => {
                    let reviewer =
                        dig(payload, &["requested_reviewer", "login"])?.unwrap_or(&Value::Null);
                    if value_blank(reviewer) {
                        return Ok(posts);
                    }
                    (
                        "review_requested",
                        sender,
                        "requested a review from",
                        format!(":{}", ruby_string(reviewer).to_lowercase()),
                        Some(reviewer.clone()),
                    )
                }
                _ => return Ok(posts),
            };
            let line = |title: &Value| match action {
                "ready_for_review" => format!(
                    "**{}** {verb} {} ready for review",
                    inline(actor),
                    pr_ref(&number, title)
                ),
                "review_requested" => format!(
                    "**{}** {verb} **{}** on {}",
                    inline(actor),
                    inline(reviewer.as_ref().unwrap()),
                    pr_ref(&number, title)
                ),
                _ => format!("**{}** {verb} {}", inline(actor), pr_ref(&number, title)),
            };
            posts.push(Post {
                event: key,
                owner,
                repo,
                line: line(title),
                redacted: line(&Value::Null),
                number,
                url,
                suffix,
                reviewer,
            });
        }
        "pull_request_review" => {
            if field(payload, "action")? != "submitted" {
                return Ok(posts);
            }
            let empty = json!({});
            let review = field(payload, "review")?;
            let review = if truthy(review) { review } else { &empty };
            let verb = match field(review, "state")?.as_str() {
                Some("approved") => "approved",
                Some("changes_requested") => "requested changes on",
                Some("commented") => "commented on",
                _ => return Ok(posts),
            };
            let pr = field(payload, "pull_request")?;
            let pr = if truthy(pr) { pr } else { &empty };
            let number = field(pr, "number")?;
            if !truthy(number) {
                return Ok(posts);
            }
            let Some((owner, repo)) = super::webhooks::repository_owner_and_repo(payload)? else {
                return Ok(posts);
            };
            let actor = match dig(review, &["user", "login"])?.filter(|v| truthy(v)) {
                Some(actor) => actor,
                None => dig(payload, &["sender", "login"])?.unwrap_or(&Value::Null),
            };
            let title = field(pr, "title")?;
            posts.push(Post {
                event: "review_submitted",
                url: format!(
                    "https://github.com/{owner}/{repo}/pull/{}",
                    ruby_string(number)
                ),
                owner,
                repo,
                number: number.clone(),
                line: format!("**{}** {verb} {}", inline(actor), pr_ref(number, title)),
                redacted: format!(
                    "**{}** {verb} {}",
                    inline(actor),
                    pr_ref(number, &Value::Null)
                ),
                suffix: format!(":{}", ruby_string(field(review, "id")?)),
                reviewer: None,
            });
        }
        "check_run" | "check_suite" => {
            let name = dig(
                payload,
                if event == "check_run" {
                    &["check_run", "name"]
                } else {
                    &["check_suite", "app", "name"]
                },
            )?
            .unwrap_or(&Value::Null);
            let check = field(payload, event)?;
            if !truthy(check)
                || !["failure", "timed_out", "cancelled"]
                    .contains(&field(check, "conclusion")?.as_str().unwrap_or(""))
            {
                return Ok(posts);
            }
            let sha = field(check, "head_sha")?;
            let empty = Vec::new();
            let numbers = field(check, "pull_requests")?;
            let numbers = if truthy(numbers) {
                numbers
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Malformed pull_requests"))?
            } else {
                &empty
            };
            let numbers = numbers
                .iter()
                .map(|pr| field(pr, "number"))
                .collect::<anyhow::Result<Vec<_>>>()?
                .into_iter()
                .filter(|n| truthy(n))
                .collect::<Vec<_>>();
            if value_blank(sha) || numbers.is_empty() {
                return Ok(posts);
            }
            let Some((owner, repo)) = super::webhooks::repository_owner_and_repo(payload)? else {
                return Ok(posts);
            };
            let full = dig(payload, &["repository", "full_name"])?.unwrap_or(&Value::Null);
            for number in numbers {
                let title:Option<String>=conn.query_row("SELECT title FROM github_pull_requests WHERE owner=? AND repo=? AND number=? ORDER BY id LIMIT 1",params![owner,repo,integer_for_query(number)],|r|r.get(0)).optional()?.flatten();
                let title = title.map(Value::String).unwrap_or(Value::Null);
                posts.push(Post {
                    event: "checks_failed",
                    owner: owner.clone(),
                    repo: repo.clone(),
                    number: number.clone(),
                    url: format!(
                        "https://github.com/{}/pull/{}",
                        ruby_string(full),
                        ruby_string(number)
                    ),
                    line: check_line(number, &title, name),
                    redacted: check_line(number, &Value::Null, name),
                    suffix: format!(":{}", ruby_string(sha)),
                    reviewer: None,
                });
            }
        }
        "status" => {
            if !["failure", "error"].contains(&field(payload, "state")?.as_str().unwrap_or("")) {
                return Ok(posts);
            }
            let Some((owner, repo)) = super::webhooks::repository_owner_and_repo(payload)? else {
                return Ok(posts);
            };
            let branches = field(payload, "branches")?;
            let empty = Vec::new();
            let branches = if truthy(branches) {
                branches
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Malformed branches"))?
            } else {
                &empty
            };
            let branches = branches
                .iter()
                .map(|b| field(b, "name"))
                .collect::<anyhow::Result<Vec<_>>>()?
                .into_iter()
                .filter(|n| truthy(n))
                .collect::<Vec<_>>();
            let sha = field(payload, "sha")?;
            if branches.is_empty() || value_blank(sha) {
                return Ok(posts);
            }
            let full = dig(payload, &["repository", "full_name"])?.unwrap_or(&Value::Null);
            let context = field(payload, "context")?;
            let mut stmt=conn.prepare("SELECT number,title,html_url,head_branch FROM github_pull_requests WHERE owner=? AND repo=? ORDER BY id")?;
            let rows = stmt
                .query_map(params![owner, repo], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for (number, title, url, branch) in rows {
                if !branches
                    .iter()
                    .any(|b| branch.as_deref() == Some(ruby_string(b).as_str()))
                {
                    continue;
                }
                let number = json!(number);
                let title = title.map(Value::String).unwrap_or(Value::Null);
                posts.push(Post {
                    event: "checks_failed",
                    owner: owner.clone(),
                    repo: repo.clone(),
                    url: url.filter(|url| !blank(url)).unwrap_or_else(|| {
                        format!(
                            "https://github.com/{}/pull/{}",
                            ruby_string(full),
                            ruby_string(&number)
                        )
                    }),
                    line: check_line(&number, &title, context),
                    redacted: check_line(&number, &Value::Null, context),
                    number,
                    suffix: format!(":{}", ruby_string(sha)),
                    reviewer: None,
                });
            }
        }
        _ => {}
    }
    Ok(posts)
}
#[cfg(test)]
mod tests;
