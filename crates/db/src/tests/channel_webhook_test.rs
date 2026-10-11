use super::{TestDb, id};
use crate::models::channel_webhook::{ChannelWebhook, WebhookMessage};
use crate::{Message, NewMessage, Room, RoomType, User};
use serde_json::json;

#[test]
fn channel_webhook_identity_digest_rotation_and_avatar() {
    let t = TestDb::new();
    let room = id("watercooler");
    let (hook, token) = t.write(move |tx| ChannelWebhook::create(tx, room, id("david"), "Builds"));
    let digest = ChannelWebhook::digest(&token);
    assert_eq!(hook.token_digest, digest);
    assert_ne!(hook.token_digest, token);
    assert_eq!(
        ChannelWebhook::digest("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        t.read(move |conn| ChannelWebhook::find_by_token_digest(conn, &digest))
            .unwrap()
            .id,
        hook.id
    );
    assert_eq!(
        t.read(move |conn| ChannelWebhook::list_for_room(conn, room))
            .len(),
        1
    );
    let author = t.read(move |conn| User::find(conn, hook.author_id));
    assert!(author.is_bot() && !author.is_active());
    assert!(
        author.email_address.is_none()
            && author.password_digest.is_none()
            && author.bot_token_digest.is_none()
    );
    assert!(t.read(move |conn| author.memberships(conn)).is_empty());
    let avatar = t.write(|tx| {
        Ok(crate::Blob::create(
            tx,
            &crate::Blob {
                id: 0,
                key: "webhook-avatar".into(),
                filename: "avatar.png".into(),
                content_type: Some("image/png".into()),
                metadata: None,
                service_name: "local".into(),
                byte_size: 1,
                checksum: None,
                created_at: tx.now(),
            },
        )?
        .id)
    });
    t.write(move |tx| {
        ChannelWebhook::set_avatar(tx, hook.id, Some(avatar))?;
        ChannelWebhook::rename(tx, hook.id, "Renamed")
    });
    let updated = t
        .read(move |conn| ChannelWebhook::find(conn, hook.id))
        .unwrap();
    assert_eq!(updated.name, "Renamed");
    assert_eq!(t.read(|conn| updated.avatar_blob_id(conn)), Some(avatar));
    t.write(move |tx| ChannelWebhook::set_avatar(tx, hook.id, None));
    assert_eq!(t.read(|conn| updated.avatar_blob_id(conn)), None);
    let old_digest = ChannelWebhook::digest(&token);
    let (_, new) = t.write(move |tx| ChannelWebhook::regenerate(tx, hook.id));
    assert_ne!(new, token);
    assert!(
        t.read(move |conn| ChannelWebhook::find_by_token_digest(conn, &old_digest))
            .is_none()
    );
    t.write(move |tx| ChannelWebhook::delete(tx, hook.id));
    assert!(
        t.read(move |conn| ChannelWebhook::find(conn, hook.id))
            .is_none()
    );
    assert!(
        t.read(move |conn| User::find(conn, hook.author_id)?.memberships(conn))
            .is_empty()
    );
}

#[test]
fn channel_webhook_message_snapshot_and_token_recheck() {
    let t = TestDb::new();
    let (hook, token) =
        t.write(|tx| ChannelWebhook::create(tx, id("watercooler"), id("david"), "Builds"));
    let author_id = hook.author_id;
    let hook_id = hook.id;
    let digest = ChannelWebhook::digest(&token);
    let payload = WebhookMessage {
        webhook_id: hook_id,
        token_digest: Some(digest.clone()),
        username: Some("CI".into()),
        embeds: json!([{"title":"Passed"}]),
    };
    let attributes = NewMessage {
        room_id: hook.room_id,
        creator_id: author_id,
        markdown_source: Some(String::new()),
        webhook: Some(payload),
        ..Default::default()
    };
    let first = attributes.clone();
    let message = t.write(move |tx| Message::create(tx, first));
    t.write(move |tx| ChannelWebhook::rename(tx, hook_id, "Later"));
    t.write(move |tx| ChannelWebhook::regenerate(tx, hook_id));
    assert!(
        t.try_write(move |tx| Message::create(tx, attributes))
            .is_err()
    );
    t.write(move |tx| ChannelWebhook::delete(tx, hook_id));
    let snapshot = t
        .read(move |conn| ChannelWebhook::message_payload(conn, message.id))
        .unwrap();
    assert_eq!(snapshot.name, "Builds");
    assert_eq!(snapshot.username.as_deref(), Some("CI"));
    assert_eq!(snapshot.embeds, json!([{"title":"Passed"}]));
    t.write(move |tx| Message::find(tx.conn(), message.id)?.destroy(tx));
    assert!(
        t.read(move |conn| ChannelWebhook::message_payload(conn, message.id))
            .is_none()
    );
}

#[test]
fn channel_webhook_room_cascade_and_failed_creation_are_atomic() {
    let t = TestDb::new();
    let room = t.write(|tx| {
        Room::create_for(
            tx,
            RoomType::Closed,
            Some("Temporary"),
            id("david"),
            &[id("david")],
        )
    });
    let (hook, _) = t.write(move |tx| ChannelWebhook::create(tx, room.id, id("david"), "Builds"));
    t.write(move |tx| {
        tx.conn()
            .execute("DELETE FROM memberships WHERE room_id=?", [room.id])?;
        tx.conn()
            .execute("DELETE FROM rooms WHERE id=?", [room.id])?;
        Ok(())
    });
    assert!(
        t.read(move |conn| ChannelWebhook::find(conn, hook.id))
            .is_none()
    );
    let before = t
        .read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0))?));
    for name in [String::new(), "  ".into(), "x".repeat(81)] {
        assert!(
            t.try_write(move |tx| ChannelWebhook::create(
                tx,
                id("watercooler"),
                id("david"),
                &name
            ))
            .is_err()
        );
    }
    assert!(
        t.try_write(|tx| ChannelWebhook::create(tx, -1, id("david"), "Builds"))
            .is_err()
    );
    assert_eq!(
        t.read(
            |conn| Ok(conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0))?)
        ),
        before
    );
}
