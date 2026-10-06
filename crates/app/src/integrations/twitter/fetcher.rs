//! `app/models/twitter/post_fetcher.rb`: bounded read-only fxtwitter JSON and untrusted facts.
use super::{post::Post, urls};
use crate::{
    app::App,
    net::{
        Network,
        http::{self, Body, Endpoint, HttpError, Request, Timeouts},
    },
};
use campfire_db::{Job, Timestamp};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome, RetryPolicy};
use campfire_richtext::{
    ruby::{is_blank, json_value_to_s, strip},
    uri,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;
pub const API_HOST: &str = "api.fxtwitter.com";
pub const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
pub const TIMEOUTS: Timeouts = Timeouts {
    open: Duration::from_secs(5),
    read: Duration::from_secs(10),
    write: Duration::from_secs(60),
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchJob {
    pub post_id: i64,
}
impl Job for FetchJob {
    const CLASS: &'static str = "Twitter::FetchPostJob";
}
impl JobKind for FetchJob {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::no_retries()
    }
}
pub async fn perform(app: App, job: FetchJob, _: Execution) -> JobResult {
    fetch(&app, &Network::system(), job.post_id)
        .await
        .map_err(crate::queue::discard_missing)?;
    Ok(Outcome::Done)
}
pub async fn fetch(app: &App, net: &Network, id: i64) -> campfire_db::Result<()> {
    let post = app.db.read(move |conn| Post::find(conn, id)).await?;
    let result = fetch_card(net, &post, &TIMEOUTS).await;
    app.db
        .write(move |tx| {
            let post = Post::find(tx.conn(), id)?;
            match result {
                Ok(card) => post.save_card(tx, &card),
                Err(error) => post.save_error(tx, &error),
            }
        })
        .await
}
#[derive(Debug)]
pub struct Card {
    pub url: String,
    pub author_handle: Option<String>,
    pub author_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub text: Option<String>,
    pub posted_at: Option<Timestamp>,
    pub replies: Option<i64>,
    pub reposts: Option<i64>,
    pub likes: Option<i64>,
    pub media: Value,
    pub quote: Option<Value>,
}
pub fn request_path(post: &Post) -> String {
    let handle = post
        .url
        .as_deref()
        .and_then(|url| urls::extract(url).first().and_then(|r| r.handle.clone()))
        .unwrap_or_else(|| "i".into());
    format!("/{handle}/status/{}", post.post_id)
}
async fn fetch_card(net: &Network, post: &Post, timeouts: &Timeouts) -> Result<Card, String> {
    // Net::HTTP's default max_retries=1 applies to this idempotent GET, separately from the job.
    for attempt in 0..=1 {
        match get(net, &request_path(post), timeouts).await {
            Ok((status, body)) => return decode(post, status, &body),
            Err(Failure::Transport(error)) if attempt == 0 && retryable(&error) => (),
            Err(Failure::Transport(error)) => return Err(transport_error(&error)),
            Err(Failure::TooLarge) => return Err("Post response too large".into()),
        }
    }
    unreachable!("the final attempt returns")
}
pub enum Failure {
    Transport(HttpError),
    TooLarge,
}
impl From<HttpError> for Failure {
    fn from(error: HttpError) -> Self {
        Self::Transport(error)
    }
}
pub async fn get(net: &Network, path: &str, timeouts: &Timeouts) -> Result<(u16, Vec<u8>), Failure> {
    // DNS, all address attempts and TLS share Net::HTTP's one open_timeout.
    let endpoint = Endpoint {
        https: true,
        host: API_HOST.into(),
        port: 443,
        pinned_ip: None,
    };
    let request = Request::net_http(
        hyper::Method::GET,
        path.into(),
        None,
        vec![
            ("User-Agent".into(), "Smartfire-X-Post-Cards".into()),
            ("Accept".into(), "application/json".into()),
        ],
    )
    .transport(false, &endpoint);
    let response = http::exchange(net, &endpoint, request, timeouts).await?;
    if response
        .header("content-length")
        .is_some_and(|header| ruby_to_i(&header) > MAX_BODY_BYTES as i128)
    {
        return Err(Failure::TooLarge);
    }
    let status = response.status;
    match response.read_body(MAX_BODY_BYTES).await? {
        Body::Complete(body) => Ok((status, body)),
        Body::TooLarge => Err(Failure::TooLarge),
    }
}
fn ruby_to_i(value: &str) -> i128 {
    let value = strip(value);
    let (negative, value) = if let Some(v) = value.strip_prefix('-') {
        (true, v)
    } else {
        (false, value.strip_prefix('+').unwrap_or(value))
    };
    let digits = value
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    let number = digits
        .parse()
        .unwrap_or(if digits.is_empty() { 0 } else { i128::MAX });
    if negative { -number } else { number }
}
fn retryable(error: &HttpError) -> bool {
    match error {
        HttpError::ReadTimeout | HttpError::ConnectionClosed => true,
        HttpError::Io(e) => matches!(
            e.kind(),
            std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe
                | std::io::ErrorKind::TimedOut
        ),
        _ => false,
    }
}
fn transport_error(error: &HttpError) -> String {
    let class = match error {
        HttpError::OpenTimeout => "Open Timeout",
        HttpError::ReadTimeout => "Read Timeout",
        HttpError::WriteTimeout => "Write Timeout",
        HttpError::ConnectionClosed => "Eof Error",
        HttpError::Unresolvable(_) => "Socket Error",
        HttpError::Tls(_) => "Ssl Error",
        HttpError::Io(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => "Econnrefused",
        HttpError::Io(_) => "Io Error",
        HttpError::Http(_) => "Http Error",
        HttpError::Inflate(_) => "Error",
    };
    format!("Could not reach X ({class})")
}
pub fn decode(post: &Post, status: u16, body: &[u8]) -> Result<Card, String> {
    match status {
        404 => return Err("Post not found on X".into()),
        200..=299 => (),
        _ => return Err(format!("fxtwitter returned {status}")),
    }
    let data = if super::super::fizzy::error_body::within_nesting_limit(body) {
        serde_json::from_slice::<Value>(body).ok()
    } else {
        None
    };
    let data = data.ok_or("Could not load this post")?;
    let Some(tweet) = data.get("tweet").filter(|v| v.is_object()) else {
        return Err(if data.get("code") == Some(&json!(404)) {
            "Post not found on X"
        } else {
            "Could not load this post"
        }
        .into());
    };
    card_attributes(post, tweet).map_err(|_| "Could not reach X (Argument Error)".into())
}
fn card_attributes(post: &Post, tweet: &Value) -> Result<Card, ()> {
    let author = &tweet["author"];
    let handle = clean_handle(&author["screen_name"]);
    let id = json_value_to_s(&tweet["id"]);
    let url = handle
        .as_ref()
        .map(|h| {
            format!(
                "https://x.com/{h}/status/{}",
                if is_blank(&id) { &post.post_id } else { &id }
            )
        })
        .unwrap_or_else(|| post.view_url());
    Ok(Card {
        url,
        author_handle: handle,
        author_name: clean_text(&author["name"], 4000)?,
        author_avatar_url: twimg_url(&author["avatar_url"]),
        text: clean_text(&tweet["text"], 4000)?,
        posted_at: posted_at(tweet),
        replies: clean_count(&tweet["replies"]),
        reposts: clean_count(&tweet["retweets"]),
        likes: clean_count(&tweet["likes"]),
        media: clean_media(&tweet["media"])?,
        quote: clean_quote(&tweet["quote"])?,
    })
}
fn clean_text(value: &Value, limit: usize) -> Result<Option<String>, ()> {
    let text = crate::integrations::opengraph::metadata::strip_tags(&json_value_to_s(value))
        .map_err(|_| ())?;
    let text = strip(&text);
    if is_blank(text) {
        Ok(None)
    } else {
        Ok(Some(text.chars().take(limit).collect()))
    }
}
fn clean_handle(value: &Value) -> Option<String> {
    let handle = json_value_to_s(value);
    let handle = strip(&handle);
    (!handle.is_empty()
        && handle.len() <= 15
        && handle
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_'))
    .then(|| handle.into())
}
fn clean_count(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|v| v.trunc() as i64))
}
fn posted_at(tweet: &Value) -> Option<Timestamp> {
    if let Some(number) = tweet["created_timestamp"].as_f64() {
        let micros = (number * 1_000_000.0).trunc();
        return jiff::Timestamp::from_microsecond(micros as i64)
            .ok()
            .map(Timestamp::from_jiff);
    }
    let value = json_value_to_s(&tweet["created_at"]);
    value
        .parse::<jiff::Timestamp>()
        .ok()
        .or_else(|| jiff::Timestamp::strptime("%a %b %d %H:%M:%S %z %Y", &value).ok())
        .map(Timestamp::from_jiff)
}
fn twimg_url(value: &Value) -> Option<String> {
    let string = json_value_to_s(value);
    let parsed = uri::parse(&string).ok()?;
    (parsed
        .scheme
        .as_deref()
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https"))
        && matches!(
            parsed.host.as_deref(),
            Some("pbs.twimg.com" | "video.twimg.com")
        ))
    .then_some(string)
}
fn positive_integer(value: &Value) -> Option<i64> {
    if value.is_number() {
        return clean_count(value).filter(|v| *v > 0);
    }
    let text = json_value_to_s(value);
    let text = strip(&text);
    let text = text.strip_prefix('+').unwrap_or(text);
    if text.is_empty()
        || text.starts_with('-')
        || text.starts_with('_')
        || text.ends_with('_')
        || text.contains("__")
    {
        return None;
    }
    let (radix, digits) =
        if let Some(s) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            (16, s)
        } else if let Some(s) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
            (2, s)
        } else if let Some(s) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
            (8, s)
        } else if let Some(s) = text.strip_prefix("0d").or_else(|| text.strip_prefix("0D")) {
            (10, s)
        } else if text.starts_with('0') && text.len() > 1 {
            (8, text)
        } else {
            (10, text)
        };
    i64::from_str_radix(&digits.replace('_', ""), radix)
        .ok()
        .filter(|v| *v > 0)
}
fn array(value: &Value) -> Vec<&Value> {
    match value {
        Value::Null => vec![],
        Value::Array(a) => a.iter().collect(),
        Value::Object(o) => {
            let _ = o;
            vec![]
        }
        _ => vec![value],
    }
}
fn clean_media(media: &Value) -> Result<Value, ()> {
    if !media.is_object() {
        return Ok(json!([]));
    }
    let mut entries = Vec::new();
    for entry in array(&media["photos"])
        .into_iter()
        .chain(array(&media["videos"]))
    {
        if !entry.is_object() {
            continue;
        }
        let Some(kind) = entry["type"]
            .as_str()
            .filter(|s| matches!(*s, "photo" | "video" | "gif"))
        else {
            continue;
        };
        let Some(url) = twimg_url(&entry["url"]) else {
            continue;
        };
        let thumb = if kind == "photo" {
            None
        } else {
            twimg_url(&entry["thumbnail_url"])
        };
        if kind != "photo" && thumb.is_none() {
            continue;
        }
        let alt = if !super::super::fizzy::blank(&entry["altText"]) {
            &entry["altText"]
        } else if !super::super::fizzy::blank(&entry["alt"]) {
            &entry["alt"]
        } else {
            &Value::Null
        };
        entries.push(json!({"type":kind,"url":url,"thumbnail_url":thumb,"width":positive_integer(&entry["width"]),"height":positive_integer(&entry["height"]),"alt":clean_text(alt,1000)?}));
        if entries.len() == 4 {
            break;
        }
    }
    Ok(json!(entries))
}
fn clean_quote(quote: &Value) -> Result<Option<Value>, ()> {
    if !quote.is_object() {
        return Ok(None);
    }
    let Some(text) = clean_text(&quote["text"], 4000)? else {
        return Ok(None);
    };
    let url = json_value_to_s(&quote["url"]);
    let author = &quote["author"];
    Ok(Some(
        json!({"url":urls::starts_with_post_url(&url).then_some(url),"author_name":clean_text(&author["name"],4000)?,"author_handle":clean_handle(&author["screen_name"]),"text":text}),
    ))
}
