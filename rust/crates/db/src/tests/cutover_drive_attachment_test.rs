use super::*;
use crate::{DriveAttachment, Message, NewMessage};
const FILE: &str = "1AbcDefGhIjKlMnOpQrSt";
fn attrs(text: &str, key: &str, drive: Vec<String>) -> NewMessage {
    NewMessage {
        room_id: id("watercooler"),
        creator_id: id("david"),
        markdown_source: Some(text.into()),
        client_message_id: Some(key.into()),
        drive_file_ids: drive,
        ..Default::default()
    }
}
fn message(t: &TestDb, text: &str, key: &str) -> Message {
    let a = attrs(text, key, vec![]);
    t.write(move |tx| Message::create(tx, a))
}
#[test]
fn cutover_c_drive_only_message_is_valid_saved_and_reads_exact_file_ids() {
    let t = TestDb::new();
    let a = attrs("", "drive-only", vec![FILE.into()]);
    assert!(t.read(|c| Message::validate(c, &a)).is_empty());
    let saved = t.try_write(move |tx| Message::create(tx, a));
    assert!(saved.is_ok());
    let m = saved.unwrap();
    assert_eq!(t.read(|c| m.drive_file_ids(c)), vec![FILE]);
}
#[test]
fn cutover_c_textless_message_without_attachments_has_blank_source_error() {
    let t = TestDb::new();
    let a = attrs("", "blank-still-invalid", vec![]);
    let errors = t.read(|c| Message::validate(c, &a));
    assert!(!errors.is_empty());
    assert!(errors.on("markdown_source").contains(&"can't be blank"));
}
#[test]
fn cutover_c_each_bad_drive_id_is_rejected_by_the_real_message_association_validation() {
    let t = TestDb::new();
    for bad in [
        "short",
        "not a file id!!",
        "https://drive.google.com/open?id=1AbcDefGhIjKlMnOpQrSt",
        "",
    ] {
        let a = attrs("see attached", "drive-bad-id", vec![bad.into()]);
        let errors = t.read(|c| Message::validate(c, &a));
        assert!(!errors.is_empty());
        assert!(
            errors
                .on("drive_attachments.file_id")
                .contains(&"is invalid")
        );
    }
}
#[test]
fn cutover_c_saved_drive_id_duplicate_is_invalid_with_scoped_file_error() {
    let t = TestDb::new();
    let m = message(&t, "dupes", "drive-dupes");
    t.write(move |tx| DriveAttachment::create(tx, m.id, FILE));
    let duplicate = DriveAttachment::new(Some(m.id), FILE);
    let errors = t.read(|c| duplicate.validate(c));
    assert!(!errors.is_empty());
    assert!(!errors.on("file_id").is_empty());
    assert_eq!(errors.on("file_id"), vec!["has already been taken"]);
}
#[test]
fn cutover_c_same_drive_id_attaches_once_to_each_distinct_message() {
    let t = TestDb::new();
    let first = message(&t, "first", "drive-shared-1");
    let second = message(&t, "second", "drive-shared-2");
    for mid in [first.id, second.id] {
        t.write(move |tx| DriveAttachment::create(tx, mid, FILE));
    }
    assert_eq!(
        t.read(|c| DriveAttachment::for_message(c, first.id)).len(),
        1
    );
    assert_eq!(
        t.read(|c| DriveAttachment::for_message(c, second.id)).len(),
        1
    );
}
#[test]
fn cutover_c_eleventh_drive_attachment_makes_message_invalid_with_exact_limit_error() {
    let t = TestDb::new();
    let mut a = attrs(
        "many",
        "drive-eleven",
        (0..10).map(|i| format!("ten-files-{i}1")).collect(),
    );
    assert!(t.read(|c| Message::validate(c, &a)).is_empty());
    a.drive_file_ids.push("eleventh-file".into());
    let errors = t.read(|c| Message::validate(c, &a));
    assert!(!errors.is_empty());
    assert_eq!(
        errors.on("drive_attachments"),
        vec!["are limited to 10 per message"]
    );
}
#[test]
fn cutover_c_message_destroy_removes_its_persisted_drive_attachment() {
    let t = TestDb::new();
    let m = message(&t, "doomed", "drive-doomed");
    t.write(move |tx| DriveAttachment::create(tx, m.id, FILE));
    let before = t.read(|c| {
        Ok(
            c.query_row("SELECT COUNT(*) FROM drive_attachments", [], |r| {
                r.get::<_, i64>(0)
            })?,
        )
    });
    t.write(move |tx| m.destroy(tx));
    let after = t.read(|c| {
        Ok(
            c.query_row("SELECT COUNT(*) FROM drive_attachments", [], |r| {
                r.get::<_, i64>(0)
            })?,
        )
    });
    assert_eq!(after - before, -1);
}
#[test]
fn cutover_c_drive_attachment_url_is_the_open_link_for_the_id() {
    let attachment = DriveAttachment::new(None, FILE);
    assert_eq!(
        attachment.url(),
        "https://drive.google.com/open?id=1AbcDefGhIjKlMnOpQrSt"
    );
}
