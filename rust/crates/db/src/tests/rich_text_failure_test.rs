//! Fault-inject only the external renderer; model/SQLite writes are real.
use super::*;
use crate::rich_text::UserNames;
use crate::{BasicRichText, Message, MessageChanges, NewMessage, RichText};
#[derive(Clone, Copy)]
enum Fault {
    Canonical,
    Plain,
    Mentions,
}
struct Renderer(Fault);
impl RichText for Renderer {
    fn to_plain_text(&self, c: &Connection, s: &str, n: UserNames<'_>) -> String {
        BasicRichText.to_plain_text(c, s, n)
    }
    fn mentioned_user_ids(&self, c: &Connection, s: &str) -> Vec<i64> {
        BasicRichText.mentioned_user_ids(c, s)
    }
    fn try_canonicalize_html(
        &self,
        c: &Connection,
        s: &str,
    ) -> std::result::Result<String, String> {
        if matches!(self.0, Fault::Canonical) {
            Err("canonical renderer failed".into())
        } else {
            Ok(BasicRichText.canonicalize_html(c, s))
        }
    }
    fn try_to_plain_text(
        &self,
        c: &Connection,
        s: &str,
        n: UserNames<'_>,
    ) -> std::result::Result<String, String> {
        if matches!(self.0, Fault::Plain) {
            Err("plain renderer failed".into())
        } else {
            Ok(self.to_plain_text(c, s, n))
        }
    }
    fn try_markdown_plain_text(
        &self,
        c: &Connection,
        s: &str,
        n: UserNames<'_>,
    ) -> std::result::Result<String, String> {
        self.try_to_plain_text(c, s, n)
    }
    fn try_mentioned_user_ids(
        &self,
        c: &Connection,
        s: &str,
    ) -> std::result::Result<Vec<i64>, String> {
        if matches!(self.0, Fault::Mentions) {
            Err("mention renderer failed".into())
        } else {
            Ok(self.mentioned_user_ids(c, s))
        }
    }
}
fn failures(fault: Fault) {
    let t = TestDb::new();
    let mut config = crate::Config::new(t.db.path());
    config.prepare = false;
    let env = Env {
        rich_text: Arc::new(Renderer(fault)),
        ..t.db.env().clone()
    };
    let db = Database::open(config, env).unwrap();
    let before = t.read(|c| crate::Room::find(c, id("designers")));
    let count = t
        .read(|c| Ok(c.query_row::<i64, _, _>("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?));
    let result = db.write_blocking(|tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                body: Some("<p>new body</p>".into()),
                ..Default::default()
            },
        )
    });
    assert!(result.is_err(), "renderer errors must abort creates");
    assert_eq!(
        t.read(
            |c| Ok(c.query_row::<i64, _, _>("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?)
        ),
        count
    );
    let before_message = t.read(|c| Message::find(c, id("first")));
    let result = db.write_blocking(|tx| {
        let mut message = Message::find(tx.conn(), id("first"))?;
        message.edit(
            tx,
            MessageChanges {
                body: Some("<p>edited body</p>".into()),
                ..Default::default()
            },
        )
    });
    assert!(result.is_err(), "renderer errors must abort edits");
    assert_eq!(t.read(|c| Message::find(c, id("first"))), before_message);
    assert_eq!(t.read(|c| crate::Room::find(c, id("designers"))), before);
}
#[test]
fn canonical_errors_abort_message_writes() {
    failures(Fault::Canonical);
}
#[test]
fn plain_text_errors_abort_message_writes() {
    failures(Fault::Plain);
}
#[test]
fn mention_errors_abort_message_writes() {
    failures(Fault::Mentions);
}
