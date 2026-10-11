use crate::Timestamp;
use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelWebhook {
    pub id: i64,
    pub room_id: i64,
    pub creator_id: i64,
    pub name: String,
    pub avatar_url: Option<String>,
    pub last_used_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelWebhookList {
    pub webhooks: Vec<ChannelWebhook>,
}

/// Only creation and regeneration return the bearer token and execution URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChannelWebhookCredential {
    pub webhook: ChannelWebhook,
    pub token: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CreateChannelWebhook {
    pub name: String,
    pub avatar_signed_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateChannelWebhook {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Omit to retain the avatar; null clears it.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_signed_id: Option<Option<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateFields {
    name: Option<String>,
    #[serde(default, deserialize_with = "present")]
    avatar_signed_id: Option<Option<String>>,
}

fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

impl<'de> Deserialize<'de> for UpdateChannelWebhook {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let fields = UpdateFields::deserialize(d)?;
        Ok(Self {
            name: fields.name,
            avatar_signed_id: fields.avatar_signed_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct ExecuteWebhook {
    pub content: Option<String>,
    pub username: Option<String>,
    #[serde(default)]
    pub embeds: Vec<WebhookEmbed>,
}

/// Structured text and plain links only. Remote media is outside this contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WebhookEmbed {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<WebhookEmbedField>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<WebhookEmbedFooter>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WebhookEmbedField {
    pub name: String,
    pub value: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WebhookEmbedFooter {
    pub text: String,
}

/// Snapshot of attribution at delivery, retained when the webhook is renamed or deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageWebhook {
    pub name: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
}
