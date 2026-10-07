//! Room form facts and the non-direct room writes behind classic settings pages.
use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

use crate::{Involvement, Room, RoomDetail, RoomKind, SidebarRow, StageRole, User};

/// `GET /api/v1/rooms/new?type=...` and `GET /api/v1/rooms/:id/edit`.
/// Existing forms are membership-scoped even for administrators and creators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomForm {
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub kind: RoomKind,
    pub room_id: Option<i64>,
    pub name: Option<String>,
    pub icon_name: Option<String>,
    pub display_name: String,
    /// Effective selected active users. New selected-members forms select the viewer.
    pub user_ids: Vec<i64>,
    /// All persisted members, including inactive people; empty for a new room.
    pub member_ids: Vec<i64>,
    /// Active options in classic order. Direct edit contains only nonmembers.
    pub candidate_ids: Vec<i64>,
    /// The direct edit page's other members, or the viewer for a solo direct.
    pub display_member_ids: Vec<i64>,
    pub users: Vec<User>,
    /// The types the workspace creation rule allows, including Direct.
    pub allowed_types: Vec<RoomKind>,
    pub conversion_types: Vec<RoomKind>,
    pub can_submit: bool,
    pub can_delete: bool,
    pub can_leave: bool,
    pub group_capable: bool,
    pub default_involvement: Involvement,
    pub stage_roles: Vec<RoomFormStageRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomFormStageRole {
    pub user_id: i64,
    pub role: StageRole,
}

/// `POST /api/v1/rooms`. Direct creation uses the existing `CreateDirect` endpoint.
/// No name presence/length rule is added beyond the classic domain's validations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum CreateRoom {
    Open {
        name: Option<String>,
        icon_name: Option<String>,
    },
    Closed {
        name: Option<String>,
        icon_name: Option<String>,
        user_ids: Vec<i64>,
    },
    Voice {
        name: Option<String>,
        icon_name: Option<String>,
        user_ids: Vec<i64>,
    },
    Stage {
        name: Option<String>,
        icon_name: Option<String>,
        user_ids: Vec<i64>,
    },
    Board {
        name: Option<String>,
        icon_name: Option<String>,
        user_ids: Vec<i64>,
    },
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "lowercase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum CreateRoomFields {
    Open {
        name: Option<String>,
        icon_name: Option<String>,
    },
    Closed {
        name: Option<String>,
        icon_name: Option<String>,
        user_ids: Vec<i64>,
    },
    Voice {
        name: Option<String>,
        icon_name: Option<String>,
        user_ids: Vec<i64>,
    },
    Stage {
        name: Option<String>,
        icon_name: Option<String>,
        user_ids: Vec<i64>,
    },
    Board {
        name: Option<String>,
        icon_name: Option<String>,
        user_ids: Vec<i64>,
    },
}

impl<'de> Deserialize<'de> for CreateRoom {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match CreateRoomFields::deserialize(deserializer)? {
            CreateRoomFields::Open { name, icon_name } => Self::Open { name, icon_name },
            CreateRoomFields::Closed {
                name,
                icon_name,
                user_ids,
            } => Self::Closed {
                name,
                icon_name,
                user_ids,
            },
            CreateRoomFields::Voice {
                name,
                icon_name,
                user_ids,
            } => Self::Voice {
                name,
                icon_name,
                user_ids,
            },
            CreateRoomFields::Stage {
                name,
                icon_name,
                user_ids,
            } => Self::Stage {
                name,
                icon_name,
                user_ids,
            },
            CreateRoomFields::Board {
                name,
                icon_name,
                user_ids,
            } => Self::Board {
                name,
                icon_name,
                user_ids,
            },
        })
    }
}

/// `PATCH /api/v1/rooms/:id`. Only Open and Closed can convert to one another.
/// Omit an attribute to retain it; null clears it. Member lists replace the full set.
/// Empty numeric lists follow classic absent/empty-array semantics, not its blank sentinel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum UpdateRoom {
    Open {
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Option<String>>,
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        icon_name: Option<Option<String>>,
    },
    Closed {
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Option<String>>,
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        icon_name: Option<Option<String>>,
        user_ids: Vec<i64>,
    },
    Voice {
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Option<String>>,
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        icon_name: Option<Option<String>>,
        user_ids: Vec<i64>,
    },
    Stage {
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Option<String>>,
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        icon_name: Option<Option<String>>,
        user_ids: Vec<i64>,
    },
    Board {
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<Option<String>>,
        #[ts(optional)]
        #[serde(skip_serializing_if = "Option::is_none")]
        icon_name: Option<Option<String>>,
        user_ids: Vec<i64>,
    },
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "lowercase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum UpdateRoomFields {
    Open {
        #[serde(default, deserialize_with = "present")]
        name: Option<Option<String>>,
        #[serde(default, deserialize_with = "present")]
        icon_name: Option<Option<String>>,
    },
    Closed {
        #[serde(default, deserialize_with = "present")]
        name: Option<Option<String>>,
        #[serde(default, deserialize_with = "present")]
        icon_name: Option<Option<String>>,
        user_ids: Vec<i64>,
    },
    Voice {
        #[serde(default, deserialize_with = "present")]
        name: Option<Option<String>>,
        #[serde(default, deserialize_with = "present")]
        icon_name: Option<Option<String>>,
        user_ids: Vec<i64>,
    },
    Stage {
        #[serde(default, deserialize_with = "present")]
        name: Option<Option<String>>,
        #[serde(default, deserialize_with = "present")]
        icon_name: Option<Option<String>>,
        user_ids: Vec<i64>,
    },
    Board {
        #[serde(default, deserialize_with = "present")]
        name: Option<Option<String>>,
        #[serde(default, deserialize_with = "present")]
        icon_name: Option<Option<String>>,
        user_ids: Vec<i64>,
    },
}

impl<'de> Deserialize<'de> for UpdateRoom {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match UpdateRoomFields::deserialize(deserializer)? {
            UpdateRoomFields::Open { name, icon_name } => Self::Open { name, icon_name },
            UpdateRoomFields::Closed {
                name,
                icon_name,
                user_ids,
            } => Self::Closed {
                name,
                icon_name,
                user_ids,
            },
            UpdateRoomFields::Voice {
                name,
                icon_name,
                user_ids,
            } => Self::Voice {
                name,
                icon_name,
                user_ids,
            },
            UpdateRoomFields::Stage {
                name,
                icon_name,
                user_ids,
            } => Self::Stage {
                name,
                icon_name,
                user_ids,
            },
            UpdateRoomFields::Board {
                name,
                icon_name,
                user_ids,
            } => Self::Board {
                name,
                icon_name,
                user_ids,
            },
        })
    }
}

fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

/// A successful room write. The viewer may have omitted or removed their own membership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomMutation {
    pub room: Room,
    pub detail: Option<RoomDetail>,
    pub row: Option<SidebarRow>,
}

/// `DELETE /api/v1/rooms/:id`: access was revoked and destruction durably requested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomRemoved {
    pub room_id: i64,
    pub deleted: bool,
}

/// `DELETE /api/v1/rooms/:id/membership`: a direct member left; the last one deletes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RoomLeft {
    pub room_id: i64,
    pub deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tagged_room_requests_reject_direct_and_unsupported_fields() {
        for kind in ["open", "closed", "voice", "stage", "board"] {
            let mut input = json!({"type":kind,"name":null,"iconName":null});
            if kind != "open" {
                input["userIds"] = json!([]);
            }
            let create: CreateRoom = serde_json::from_value(input.clone()).unwrap();
            assert_eq!(serde_json::to_value(create).unwrap(), input);
            let update: UpdateRoom = serde_json::from_value(input.clone()).unwrap();
            assert_eq!(serde_json::to_value(update).unwrap(), input);
            input["creatorId"] = json!(4);
            assert!(serde_json::from_value::<CreateRoom>(input.clone()).is_err());
            assert!(serde_json::from_value::<UpdateRoom>(input).is_err());
        }
        for input in [
            json!({"type":"direct","userIds":[]}),
            json!({"type":"open","userIds":[]}),
            json!({"type":"stage","userIds":[""]}),
            json!({"type":"closed"}),
        ] {
            assert!(serde_json::from_value::<CreateRoom>(input.clone()).is_err());
            assert!(serde_json::from_value::<UpdateRoom>(input).is_err());
        }
    }

    #[test]
    fn room_patch_distinguishes_missing_null_and_strings_for_every_type() {
        for kind in ["open", "closed", "voice", "stage", "board"] {
            for value in [None, Some(json!(null)), Some(json!(" :FIRE: "))] {
                let mut input = json!({"type":kind});
                if kind != "open" {
                    input["userIds"] = json!([]);
                }
                if let Some(value) = value {
                    input["name"] = value.clone();
                    input["iconName"] = value;
                }
                let update: UpdateRoom = serde_json::from_value(input.clone()).unwrap();
                assert_eq!(serde_json::to_value(update).unwrap(), input);
            }
        }
    }
    #[test]
    fn management_refresh_is_optional_in_existing_sidebar_payloads() {
        let mut row = crate::tests::row();
        let old = serde_json::to_value(&row).unwrap();
        assert!(old.get("refreshRoom").is_none());
        assert_eq!(
            serde_json::from_value::<crate::SidebarRow>(old).unwrap(),
            row
        );
        row.refresh_room = Some(true);
        let wire = serde_json::to_value(&row).unwrap();
        assert_eq!(wire["refreshRoom"], json!(true));
        assert_eq!(
            serde_json::from_value::<crate::SidebarRow>(wire).unwrap(),
            row
        );
        let removed: crate::SidebarRowRemoved =
            serde_json::from_value(json!({"roomId": 12})).unwrap();
        assert_eq!(removed.refresh_room, None);
        assert_eq!(
            serde_json::to_value(removed).unwrap(),
            json!({"roomId": 12})
        );
        let refreshed = crate::SidebarRowRemoved {
            room_id: 12,
            refresh_room: Some(true),
        };
        let wire = json!({"roomId": 12, "refreshRoom": true});
        assert_eq!(serde_json::to_value(refreshed).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<crate::SidebarRowRemoved>(wire).unwrap(),
            refreshed
        );
    }
}
