
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct RunSummary {
    pub id: i64,
    pub kind: String,
    pub mode: String,
    pub status: String,
}
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct SetupData {
    pub client_id: Option<String>,
    pub configured: bool,
    pub configured_by: Option<String>,
    pub team_name: Option<String>,
    pub team_known: bool,
    pub connection_exists: bool,
    pub connected: bool,
    pub disconnected_reason: Option<String>,
    pub active_run: Option<RunSummary>,
    pub errors: Vec<String>,
    pub manifest: String,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Issue {
    pub level: String,
    pub slack_ref: Option<String>,
    pub message: String,
}
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct RoomTarget {
    pub id: i64,
    pub name: String,
}
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct RunData {
    pub id: i64,
    pub kind: String,
    pub mode: String,
    pub status: String,
    pub options: serde_json::Value,
    pub stats: serde_json::Value,
    pub error: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub user_name: String,
    pub issues_count: i64,
    pub queued_behind: bool,
    pub undo_reason: Option<String>,
    pub issues: Vec<Issue>,
    pub next_page: Option<i64>,
    pub sample_htmls: Vec<String>,
}
pub fn truthy(v: &serde_json::Value) -> bool {
    !matches!(v, serde_json::Value::Null | serde_json::Value::Bool(false))
}
pub fn present(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null | serde_json::Value::Bool(false) => false,
        serde_json::Value::String(s) => !campfire_richtext::ruby::is_blank(s),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}
pub fn text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}
pub fn humanize(s: &str) -> String {
    let s = s.strip_suffix("_id").unwrap_or(s).replace('_', " ");
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| format!("{}{}", c.to_uppercase(), chars.as_str()))
        .unwrap_or_default()
}
/// A conversation's kind as the plan tables name it.
pub fn conversation_type(c: &serde_json::Value) -> String {
    match c["type"].as_str().unwrap_or("") {
        "public_channel" => "Public channel".into(),
        "private_channel" => "Private channel".into(),
        "im" => "Direct message".into(),
        "mpim" => "Group DM".into(),
        s => humanize(s),
    }
}
/// A planned conversation's target as the plan's select picks it: `"skip"`, a room to merge
/// into (its id, while the room is still there), or `"new"`.
pub fn target_value(c: &serde_json::Value, rooms: &[RoomTarget]) -> String {
    let target = &c["target"];
    if target["action"] == "skip" {
        "skip".into()
    } else if target["action"] == "merge"
        && rooms
            .iter()
            .any(|r| Some(r.id) == target["room_id"].as_i64())
    {
        text(&target["room_id"])
    } else {
        "new".into()
    }
}

pub fn yes_no(v: &serde_json::Value) -> &str {
    if truthy(v) { "Yes" } else { "No" }
}
pub fn run_base(admin: &bool) -> &'static str {
    if *admin {
        "/account/slack_import/runs"
    } else {
        "/slack/imports"
    }
}
impl RunSummary {
    pub fn mode_label(&self) -> String {
        self.mode.replace('_', " ")
    }
}

impl SetupData {
    pub fn secret_placeholder(&self) -> &str {
        if self.configured {
            "Saved — paste a new secret to replace it"
        } else {
            "xoxp-…"
        }
    }
    pub fn submit_label(&self) -> &str {
        if self.configured {
            "Save new credentials"
        } else {
            "Save credentials"
        }
    }
}

impl Issue {
    pub fn has_ref(&self) -> bool {
        self.slack_ref
            .as_ref()
            .is_some_and(|s| !campfire_richtext::ruby::is_blank(s))
    }
}

impl RunData {
    pub fn title(&self) -> String {
        format!(
            "{} {} #{}",
            humanize(&self.kind),
            self.mode.replace('_', " "),
            self.id
        )
    }
    pub fn active(&self) -> bool {
        matches!(self.status.as_str(), "queued" | "running" | "undoing")
    }
    pub fn finished(&self) -> bool {
        !self.active()
    }
    pub fn cancellable(&self) -> bool {
        matches!(self.status.as_str(), "queued" | "running")
    }
    pub fn undoable(&self) -> bool {
        self.mode == "import"
            && matches!(self.status.as_str(), "completed" | "cancelled" | "failed")
            && self.undo_reason.is_none()
    }
    pub fn preview(&self) -> bool {
        self.mode == "dry_run" && self.status == "completed"
    }
    pub fn catch_up(&self) -> bool {
        self.kind == "workspace"
            && self.mode == "import"
            && self.status == "completed"
            && !present(&self.options["oldest"])
    }
    pub fn conversations(&self) -> &[serde_json::Value] {
        self.stats["conversations"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    pub fn samples(&self) -> &[serde_json::Value] {
        self.stats["samples"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    pub fn format_times(&mut self, zone: &crate::time::Zone) {
        pub fn convert(zone: &crate::time::Zone, s: &str) -> String {
            s.parse::<jiff::Timestamp>()
                .ok()
                .or_else(|| format!("{}Z", s.replace(' ', "T")).parse().ok())
                .map(|t| zone.to_fs(t, "default"))
                .unwrap_or_else(|| s.into())
        }
        self.created_at = convert(zone, &self.created_at);
        self.started_at = self.started_at.as_ref().map(|s| convert(zone, s));
        self.finished_at = self.finished_at.as_ref().map(|s| convert(zone, s));
    }
    pub fn error_present(&self) -> bool {
        self.error
            .as_ref()
            .is_some_and(|s| !campfire_richtext::ruby::is_blank(s))
    }
    pub fn next_path(&self) -> String {
        format!("{}?page={}", self.path(true), self.next_page.unwrap())
    }
    pub fn id_string(&self) -> String {
        self.id.to_string()
    }
    pub fn started(&self) -> &str {
        self.started_at.as_deref().unwrap_or(&self.created_at)
    }
    pub fn stat(&self, key: &str) -> String {
        text(&self.stats[key])
    }
    pub fn count(&self, key: &str) -> String {
        text(&self.stats["counts"][key])
    }
    pub fn users(&self, key: &str) -> String {
        text(&self.stats["users"][key])
    }
    pub fn has_users(&self) -> bool {
        self.stats["users"]
            .as_object()
            .is_some_and(|o| !o.is_empty())
    }
    pub fn has_counts(&self) -> bool {
        self.stats["counts"]
            .as_object()
            .is_some_and(|o| !o.is_empty())
    }
    pub fn issues_label(&self) -> String {
        if truthy(&self.stats["issues_count"]) {
            text(&self.stats["issues_count"])
        } else {
            self.issues_count.to_string()
        }
    }
    pub fn path(&self, admin: bool) -> String {
        format!(
            "{}/{}",
            if admin {
                "/account/slack_import/runs"
            } else {
                "/slack/imports"
            },
            self.id
        )
    }
    pub fn action(&self, admin: bool, action: &str) -> String {
        format!("{}/{action}", self.path(admin))
    }
}
