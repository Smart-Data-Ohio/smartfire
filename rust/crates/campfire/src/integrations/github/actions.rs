//! `Github::AgentPullRequestAction` and `Github::ReviewLogins`, shared by human and agent writes.
use super::client::{Error, PullRequestKey, WriteClient, ruby_string, ruby_strip};
use campfire_db::Errors;
use serde_json::{Value, json};

pub const INVALID_REVIEWERS: &str = "Enter GitHub usernames separated by commas.";
/// Ruby's /[\s,]+/ uses ASCII whitespace, unlike String#blank?.
pub fn normalize_reviewers(submitted: &Value) -> Option<Vec<String>> {
    let items = match submitted {
        Value::Array(items) => items.clone(),
        value => vec![value.clone()],
    };
    let mut logins = Vec::new();
    for item in items {
        let text = ruby_string(&item);
        for token in text
            .split([' ', '\t', '\r', '\n', '\u{b}', '\u{c}', ','])
            .filter(|s| !s.is_empty())
        {
            let login = token.strip_prefix('@').unwrap_or(token).to_lowercase();
            // After downcase, Ruby's case-insensitive [a-z] also admits the long s.
            let letter = |c: char| c.is_ascii_alphanumeric() || c == '\u{17f}';
            if login.is_empty()
                || login.chars().count() > 39
                || !login.chars().next().is_some_and(letter)
                || !login.chars().next_back().is_some_and(letter)
                || login.contains("--")
                || login.chars().any(|c| !letter(c) && c != '-')
            {
                return None;
            }
            if !logins.contains(&login) {
                logins.push(login);
            }
        }
    }
    (logins.len() <= 15).then_some(logins)
}
#[derive(Clone)]
pub struct Action {
    pub pull_request_id: i64,
    pub key: PullRequestKey,
    pub kind: Value,
    pub body: Value,
    pub reviewers: Value,
}
impl Action {
    pub fn from_payload(pull_request_id: i64, key: PullRequestKey, payload: &Value) -> Self {
        Self {
            pull_request_id,
            key,
            kind: payload["kind"].clone(),
            body: payload["body"].clone(),
            reviewers: payload["reviewers"].clone(),
        }
    }
    pub fn normalized_body(&self) -> Option<String> {
        let body = ruby_string(&self.body);
        let body = ruby_strip(&body);
        (!super::blank(body)).then(|| body.to_owned())
    }
    pub fn normalized_reviewers(&self) -> Option<Vec<String>> {
        normalize_reviewers(&self.reviewers)
    }
    pub fn errors(&self) -> Errors {
        let mut errors = Errors::default();
        if !matches!(
            self.kind.as_str(),
            Some("comment" | "approve" | "request_changes" | "request_review")
        ) {
            errors.add(
                "kind",
                "must be one of: comment, approve, request_changes, request_review",
            );
        }
        let kind = ruby_string(&self.kind);
        let body = self.normalized_body();
        if body.is_none() {
            match kind.as_str() {
                "comment" => errors.add("body", "is required for a comment"),
                "request_changes" => errors.add("body", "is required when requesting changes"),
                _ => (),
            }
        }
        if body.as_ref().is_some_and(|s| s.chars().count() > 3500) {
            errors.add("body", "is too long (maximum is 3500 characters)");
        }
        if kind == "request_review" && self.normalized_reviewers().is_none_or(|v| v.is_empty()) {
            errors.add("reviewers", INVALID_REVIEWERS);
        }
        errors
    }
    pub fn action_name(&self) -> String {
        format!("github.{}", ruby_string(&self.kind))
    }
    pub fn summary(&self) -> Option<String> {
        let reference = format!("{}/{}#{}", self.key.owner, self.key.repo, self.key.number);
        match ruby_string(&self.kind).as_str() {
            "comment" => Some(format!(
                "Comment on {reference}: {}",
                self.normalized_body()
                    .unwrap_or_default()
                    .chars()
                    .take(120)
                    .collect::<String>()
            )),
            "approve" => Some(format!("Approve {reference}")),
            "request_changes" => Some(format!("Request changes on {reference}")),
            "request_review" => Some(campfire_richtext::ruby::truncate(
                &format!(
                    "Request review on {reference} from {}",
                    self.normalized_reviewers()
                        .unwrap_or_default()
                        .iter()
                        .map(|s| format!("@{s}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                500,
                "...",
            )),
            _ => None,
        }
    }
    pub fn payload(&self) -> Value {
        let kind = ruby_string(&self.kind);
        let body = matches!(kind.as_str(), "comment" | "approve" | "request_changes")
            .then(|| self.normalized_body())
            .flatten();
        let reviewers = (kind == "request_review")
            .then(|| self.normalized_reviewers().filter(|v| !v.is_empty()))
            .flatten();
        json!({"pull_request_id":self.pull_request_id,"kind":kind,"body":body,"reviewers":reviewers})
    }
    pub fn payload_json(&self) -> String {
        // Active Support's text JSON encoder preserves the Ruby hash insertion order.
        let payload = self.payload();
        let fields = ["pull_request_id", "kind", "body", "reviewers"]
            .map(|key| format!("\"{key}\":{}", rails_compat::json_encode(&payload[key])));
        format!("{{{}}}", fields.join(","))
    }
    pub async fn perform(&self, client: &WriteClient) -> Result<Value, Error> {
        let body = self.normalized_body();
        match ruby_string(&self.kind).as_str() {
            "comment" => {
                client
                    .create_issue_comment(&self.key, body.as_deref().unwrap_or(""))
                    .await
            }
            "approve" => {
                client
                    .create_review(&self.key, "APPROVE", body.as_deref())
                    .await
            }
            "request_changes" => {
                client
                    .create_review(&self.key, "REQUEST_CHANGES", body.as_deref())
                    .await
            }
            "request_review" => {
                client
                    .request_reviewers(&self.key, &self.normalized_reviewers().unwrap_or_default())
                    .await
            }
            _ => unreachable!("the caller validates the action before execution"),
        }
    }
}
