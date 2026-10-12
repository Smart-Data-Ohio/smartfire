//! Portable workspace structure. Array order defines category, room and board-tag order.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum TemplateFormat {
    #[serde(rename = "smartfire-template")]
    SmartfireTemplate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum TemplateRoomKind {
    Open,
    Closed,
    Voice,
    Stage,
    Board,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct WorkspaceTemplate {
    pub format: TemplateFormat,
    pub version: u32,
    pub categories: Vec<TemplateCategory>,
    /// Uncategorized rooms, in shared sidebar order.
    pub rooms: Vec<TemplateRoom>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct TemplateCategory {
    pub name: String,
    pub rooms: Vec<TemplateRoom>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct TemplateRoom {
    pub kind: TemplateRoomKind,
    pub name: Option<String>,
    pub icon_name: Option<String>,
    pub topic: Option<String>,
    pub announcement: bool,
    pub voice_bitrate: Option<i64>,
    pub voice_user_limit: Option<i64>,
    /// Present exactly for boards. Catalog structure only, never post tags or assignments.
    pub board: Option<TemplateBoard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct TemplateBoard {
    pub tags: Vec<TemplateBoardTag>,
    pub tags_required: bool,
    /// Zero-based index in `tags`, never a persisted id.
    pub default_tag: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct TemplateBoardTag {
    pub name: String,
    pub emoji: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct TemplateUpload {
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct TemplateImport {
    pub contents: String,
    /// The resolved structure the administrator previewed. Rechecked inside the transaction.
    pub expected: WorkspaceTemplate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum TemplateEntity {
    Category,
    Room,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TemplateChange {
    pub entity: TemplateEntity,
    pub name: Option<String>,
    pub created_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TemplatePreview {
    pub template: Option<WorkspaceTemplate>,
    pub changes: Vec<TemplateChange>,
    pub errors: Vec<String>,
    pub category_count: u32,
    pub room_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn template_document_rejects_ids_secrets_and_unknown_kinds() {
        let base = serde_json::json!({"format":"smartfire-template","version":1,"categories":[],"rooms":[]});
        let parsed: WorkspaceTemplate = serde_json::from_value(base.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), base);
        for key in ["id", "memberships", "token", "roles"] {
            let mut input = base.clone();
            input[key] = serde_json::json!([]);
            assert!(serde_json::from_value::<WorkspaceTemplate>(input).is_err());
        }
        for kind in ["direct", "forum", "unknown"] {
            let mut input = base.clone();
            input["rooms"] =
                serde_json::json!([{"kind":kind,"name":"No","announcement":false,"board":null}]);
            assert!(serde_json::from_value::<WorkspaceTemplate>(input).is_err());
        }
    }
}
