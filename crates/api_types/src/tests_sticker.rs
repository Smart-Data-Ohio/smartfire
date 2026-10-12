use crate::tests::{assert_wire, message};
use crate::*;
use serde_json::json;

#[test]
fn sticker_catalog_and_writes_round_trip() {
    let sticker = Sticker {
        id: 1,
        name: "Wave".into(),
        description: "Hello".into(),
        tags: "👋".into(),
        image_url: "/stickers/1".into(),
        still_url: "/stickers/1?still=1".into(),
        animated: true,
        uploader_id: 7,
        created_at: "2026-10-12T03:00:00Z".into(),
        updated_at: "2026-10-12T03:00:00Z".into(),
    };
    assert_wire(
        &StickerCatalog {
            stickers: vec![sticker],
            capacity: 60,
        },
        json!({"capacity":60,"stickers":[{"id":1,"name":"Wave","description":"Hello","tags":"👋","imageUrl":"/stickers/1","stillUrl":"/stickers/1?still=1","animated":true,"uploaderId":7,"createdAt":"2026-10-12T03:00:00Z","updatedAt":"2026-10-12T03:00:00Z"}]}),
    );
    assert_wire(
        &CreateSticker {
            name: "Wave".into(),
            description: "Hello".into(),
            tags: "👋".into(),
            signed_id: Some("signed".into()),
        },
        json!({"name":"Wave","description":"Hello","tags":"👋","signedId":"signed"}),
    );
    assert_wire(
        &UpdateSticker {
            name: "Wave".into(),
            description: "Hello".into(),
            tags: "👋".into(),
        },
        json!({"name":"Wave","description":"Hello","tags":"👋"}),
    );
    assert_wire(
        &ApiErrorResponse {
            error: ApiError::StickerLimitReached {
                message: "Sticker capacity reached (limit: 60 stickers)".into(),
                capacity: 60,
            },
        },
        json!({"error":{"_tag":"StickerLimitReached","message":"Sticker capacity reached (limit: 60 stickers)","capacity":60}}),
    );
}

#[test]
fn sticker_messages_round_trip_available_deleted_and_sticker_only_create() {
    let available = MessageSticker::Available {
        id: 1,
        name: "Wave".into(),
        image_url: "/stickers/1".into(),
        still_url: "/stickers/1?still=1".into(),
        animated: true,
    };
    assert_wire(
        &available,
        json!({"kind":"available","id":1,"name":"Wave","imageUrl":"/stickers/1","stillUrl":"/stickers/1?still=1","animated":true}),
    );
    let deleted = MessageSticker::Deleted {
        name: "Wave".into(),
    };
    assert_wire(&deleted, json!({"kind":"deleted","name":"Wave"}));
    for sticker in [available, deleted] {
        let mut message = message();
        message.sticker = Some(sticker);
        let wire = serde_json::to_value(&message).unwrap();
        assert_wire(&message, wire);
    }
    assert_wire(
        &CreateMessage {
            client_message_id: "sticker-only".into(),
            sticker_id: Some(1),
            markdown_source: String::new(),
            reply_to_message_id: None,
            reply_notify_author: None,
            attachment_signed_id: None,
            attachment_signed_ids: None,
            drive_file_ids: None,
        },
        json!({"clientMessageId":"sticker-only","stickerId":1,"markdownSource":"","replyToMessageId":null,"replyNotifyAuthor":null,"attachmentSignedId":null}),
    );
}
