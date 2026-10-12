use crate::Timestamp;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetAnnouncement {
    pub announcement: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateAnnouncementFollow {
    pub target_room_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnnouncementFollow {
    pub source_room_id: i64,
    pub target_room_id: i64,
    pub creator_id: i64,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnnouncementFollows {
    /// Incoming follows are available only to this room's managers, with visible targets.
    pub followers: Vec<AnnouncementFollow>,
    /// Outgoing follows include only source rooms the viewer can see.
    pub following: Vec<AnnouncementFollow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnnouncementSource {
    pub room_id: i64,
    pub message_id: i64,
    pub room_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announcement_message_fields_round_trip_and_default_for_existing_payloads() {
        let mut message = crate::tests::message();
        message.published = true;
        message.announcement_copy = true;
        message.announcement_source = Some(AnnouncementSource {
            room_id: 12,
            message_id: 42,
            room_name: Some("News".into()),
        });
        let mut wire = serde_json::to_value(&message).unwrap();
        assert_eq!(wire["published"], true);
        assert_eq!(wire["announcementCopy"], true);
        assert_eq!(
            wire["announcementSource"],
            serde_json::json!({"roomId":12,"messageId":42,"roomName":"News"})
        );
        assert_eq!(
            serde_json::from_value::<crate::MessageDTO>(wire.clone()).unwrap(),
            message
        );
        wire.as_object_mut().unwrap().remove("published");
        wire.as_object_mut().unwrap().remove("announcementCopy");
        wire.as_object_mut().unwrap().remove("announcementSource");
        let legacy = serde_json::from_value::<crate::MessageDTO>(wire).unwrap();
        assert!(!legacy.published);
        assert!(!legacy.announcement_copy);
        assert!(legacy.announcement_source.is_none());
    }
}
