
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub avatar_path: String,
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Row {
    pub id: i64,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub raw_name: Option<String>,
    #[serde(default)]
    pub category_row: bool,
    pub epoch: String,
    /// Rails collection cache keys only the membership, participant IDs and admin flag.
    #[serde(default)]
    pub direct_cache_key: Option<String>,
    pub members: Vec<Person>,
    pub call: crate::rooms::calls::CallRow,
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub collapsed: bool,
    pub rows: Vec<Row>,
}
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Sidebar {
    pub account_name: String,
    pub logo_path: Option<String>,
    pub actor: Person,
    pub configured: bool,
    pub can_create: bool,
    pub favorites: Vec<Row>,
    pub channels: Vec<Row>,
    pub boards: Vec<Row>,
    pub voice: Vec<Row>,
    pub stage: Vec<Row>,
    pub direct: Vec<Row>,
    pub placeholders: Vec<Person>,
    pub categories: Vec<Category>,
}
impl Person {
    pub fn first_name(&self) -> &str {
        self.name.split_whitespace().next().unwrap_or("")
    }
}

impl Row {
    pub fn param_key(&self) -> String {
        format!("rooms_{}", self.kind)
    }
    pub fn classes(&self) -> String {
        format!(
            "sidebar-item room btn{}{}{}",
            if self.kind == "board" {
                " board-room"
            } else {
                ""
            },
            if self.call.unread { " unread" } else { "" },
            if self.call.muted { " muted" } else { "" }
        )
    }
}

impl Category {
    pub fn action(&self) -> String {
        format!("/room_categories/{}", self.id)
    }
    pub fn toggle_label(&self) -> &str {
        if self.collapsed { "Expand" } else { "Collapse" }
    }
}
