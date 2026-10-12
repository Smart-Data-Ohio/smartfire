use super::{TestDb, id};
use crate::{
    Error, Message, NewMessage,
    models::sticker::{CAPACITY, Sticker, StickerValues},
};

fn values(name: &str) -> StickerValues {
    StickerValues {
        name: name.into(),
        description: "Greeting".into(),
        tags: "👋 hello".into(),
    }
}

#[test]
fn sticker_names_are_bounded_trimmed_and_case_folded() {
    let t = TestDb::new();
    for name in ["", " ", "a", &"a".repeat(31), &"👋".repeat(31)] {
        let values = values(name);
        assert!(matches!(
            t.try_write(move |tx| Sticker::create(tx, id("david"), values)),
            Err(Error::RecordInvalid(_))
        ));
    }
    let sticker = t.write(|tx| Sticker::create(tx, id("david"), values("  Straße  ")));
    assert_eq!(sticker.name, "Straße");
    assert_eq!(sticker.description, "Greeting");
    assert!(matches!(
        t.try_write(|tx| Sticker::create(tx, id("david"), values("STRASSE"))),
        Err(Error::RecordInvalid(_))
    ));
    t.write(move |tx| {
        let edited = sticker.update(tx, values("Hello"))?;
        assert_eq!(edited.name, "Hello");
        edited.destroy(tx)
    });
    assert!(t.read(Sticker::list).is_empty());
}

#[test]
fn sticker_has_sixty_slots_and_deletion_frees_one() {
    let t = TestDb::new();
    t.write(|tx| {
        for index in 0..CAPACITY {
            Sticker::create(tx, id("david"), values(&format!("Sticker {index}")))?;
        }
        Ok(())
    });
    assert!(matches!(
        t.try_write(|tx| Sticker::create(tx, id("david"), values("Overflow"))),
        Err(Error::StickerLimitReached)
    ));
    t.write(|tx| {
        Sticker::list(tx.conn())?[0].destroy(tx)?;
        Sticker::create(tx, id("david"), values("Replacement"))?;
        Ok(())
    });
    assert_eq!(t.read(Sticker::list).len(), CAPACITY as usize);
}

#[test]
fn sticker_only_messages_validate_the_reference_and_survive_rename_delete_and_edit() {
    let t = TestDb::new();
    let mut input = NewMessage {
        room_id: id("watercooler"),
        creator_id: id("david"),
        markdown_source: Some(String::new()),
        sticker_id: Some(-1),
        ..Default::default()
    };
    assert!(matches!(
        t.try_write({
            let input = input.clone();
            move |tx| Message::create(tx, input)
        }),
        Err(Error::RecordInvalid(_))
    ));
    let sticker = t.write(|tx| Sticker::create(tx, id("david"), values("Hello")));
    input.sticker_id = Some(sticker.id);
    let message = t.write(move |tx| Message::create(tx, input));
    let message_id = message.id;
    t.write(move |tx| {
        assert_eq!(
            message.notification_text(tx.conn(), tx.rich_text())?,
            "Sticker: Hello"
        );
        let sticker = sticker.update(tx, values("Wave"))?;
        assert_eq!(
            Message::find(tx.conn(), message_id)?.plain_text_body(tx.conn(), tx.rich_text())?,
            "Sticker: Wave"
        );
        sticker.destroy(tx)?;
        let mut deleted = Message::find(tx.conn(), message_id)?;
        assert_eq!(deleted.sticker_id, None);
        assert_eq!(deleted.sticker_name.as_deref(), Some("Wave"));
        deleted.update(
            tx,
            crate::MessageChanges {
                markdown_source: Some(String::new()),
                ..Default::default()
            },
        )?;
        assert_eq!(
            deleted.notification_text(tx.conn(), tx.rich_text())?,
            "Sticker: Wave"
        );
        assert!(
            Message::search_in_room(tx.conn(), id("watercooler"), "Wave")?
                .iter()
                .any(|row| row.id == message_id)
        );
        Ok(())
    });
}
