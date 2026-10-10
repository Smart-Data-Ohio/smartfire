use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{ActivityItem, KeywordAlert, Message};
#[tokio::test]
async fn ws17_keyword_http_records_inbox_during_dnd_and_mention_wins() {
    let app = TestApp::boot().await.expect("parity seed");
    let mut browser = app.david();
    app.db()
        .write(|tx| {
            KeywordAlert::create(tx, JASON, "deploy")?;
            tx.conn()
                .execute("UPDATE users SET dnd_enabled=1 WHERE id=?", [JASON])?;
            Ok(())
        })
        .await
        .unwrap();
    let rich_text = app.db().env().rich_text.clone();
    let mention = app
        .db()
        .read(move |conn| {
            campfire_db::RichText::render_markdown(&*rich_text, conn, "Deploy @[Jason]", ALL_TALK)
                .map_err(campfire_db::Error::Other)
        })
        .await
        .unwrap();
    for (body, event) in [
        ("Deploy now".to_owned(), "keyword_alert"),
        (mention, "mention"),
    ] {
        let reply = browser
            .write(
                Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                    .header("accept", "text/vnd.turbo-stream.html")
                    .form(&[("message[body]", body.as_str())]),
            )
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let message = app
            .db()
            .read(|c| Ok(Message::last(c)?.unwrap()))
            .await
            .unwrap();
        assert_eq!(
            app.db()
                .read(move |c| ActivityItem::find_by_user_and_source(
                    c, JASON, "Message", message.id
                ))
                .await
                .unwrap()
                .unwrap()
                .event_type,
            event
        );
    }
}
#[tokio::test]
async fn ws17_keyword_http_insert_failure_rolls_back_message_index_and_jobs() {
    let app = TestApp::boot().await.expect("parity seed").without_job_runner().await;
    let mut browser = app.david();
    browser.authenticity_token().await;
    app.db().write(|tx| {KeywordAlert::create(tx,JASON,"deploy")?;tx.conn().execute_batch("CREATE TRIGGER ws17_reject_activity BEFORE INSERT ON activity_items BEGIN SELECT RAISE(ABORT,'ws17 rejected activity'); END")?;Ok(())}).await.unwrap();
    let snapshot = |c: &campfire_db::Connection| -> campfire_db::Result<(i64, i64, i64)> {
        Ok((
            Message::count(c)?,
            c.query_row("SELECT count(*) FROM message_search_index", [], |r| {
                r.get(0)
            })?,
            c.query_row("SELECT count(*) FROM background_jobs", [], |r| r.get(0))?,
        ))
    };
    let before = app.db().read(snapshot).await.unwrap();
    let reply = browser
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                .header("accept", "text/vnd.turbo-stream.html")
                .form(&[("message[body]", "Deploy now")]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(app.db().read(snapshot).await.unwrap(), before);
}
