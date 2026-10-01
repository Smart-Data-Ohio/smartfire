#[cfg(test)]
mod review_pr177_cache_probes {
    use crate::controllers::presenters::{self, test_support::*};
    use axum::http::StatusCode;
    async fn boundary(app: &TestApp, user: i64, count: i64) -> String {
        app.db().write(move |tx| {
            let (id,stamp):(i64,String)=tx.conn().query_row("SELECT id,created_at FROM messages WHERE room_id=? AND thread_id IS NULL ORDER BY created_at DESC,id DESC LIMIT 1 OFFSET ?",rusqlite::params![ALL_TALK,count],|r|Ok((r.get(0)?,r.get(1)?)))?;
            tx.conn().execute("UPDATE memberships SET unread_at=?,last_read_message_id=? WHERE room_id=? AND user_id=?",rusqlite::params![stamp,id,ALL_TALK,user])?;
            let following:String=tx.conn().query_row("SELECT client_message_id FROM messages WHERE room_id=? AND thread_id IS NULL ORDER BY created_at DESC,id DESC LIMIT 1 OFFSET ?",rusqlite::params![ALL_TALK,count-1],|r|r.get(0))?;
            Ok(following)
        }).await.unwrap()
    }
    fn check(html: &str, count: i64, following: &str) {
        assert_eq!(
            html.matches("id=\"unread-divider\"").count(),
            1,
            "exactly one divider, count {count}"
        );
        let start = html.find("<div id=\"unread-divider\"").unwrap();
        let end = start + html[start..].find("</div>").unwrap() + 6;
        assert!(html[start..end].contains(&format!("data-unread-count=\"{count}\"")));
        assert!(
            html[end..]
                .trim_start()
                .starts_with(&format!("<div id=\"message_{following}\"")),
            "viewer boundary must directly precede their first unread message"
        );
    }
    #[tokio::test]
    async fn review_two_viewers_keep_distinct_unread_boundaries_in_warm_cache() {
        let app = TestApp::boot_frozen().await.expect("seed required");
        let david_first = boundary(&app, DAVID, 6).await;
        let jason_first = boundary(&app, JASON, 2).await;
        assert_ne!(david_first, jason_first);
        let mut david = app.david();
        let mut jason = app.sign_in(JASON).await;
        for _ in 0..3 {
            let d = david.get(&format!("/rooms/{ALL_TALK}")).await;
            assert_eq!(d.status, StatusCode::OK);
            check(&d.text(), 6, &david_first);
            let j = jason.get(&format!("/rooms/{ALL_TALK}")).await;
            assert_eq!(j.status, StatusCode::OK);
            check(&j.text(), 2, &jason_first);
        }
        assert!(!app.booted.app.fragment_cache.is_empty());
        let shared = app.booted.app.clone();
        app.db()
            .read(move |conn| {
                let records = presenters::room_shell::find_messages(conn, ALL_TALK, None)?;
                let mut presenter =
                    presenters::Presenter::new(conn, &shared, Some("campfire.test".into()));
                presenter.cache_base_url = Some("http://campfire.test".into());
                let items = campfire_views::fragment_cache::with(&shared.fragment_cache, || {
                    presenter.messages(&records)
                })?;
                assert!(
                    items.iter().all(|item| matches!(
                        item,
                        campfire_views::messages::MessageItem::Fragment { .. }
                    )),
                    "must exercise actual cached message fragments"
                );
                for item in items {
                    if let campfire_views::messages::MessageItem::Fragment { html, .. } = item {
                        assert!(!html.contains("unread-divider"));
                    }
                }
                Ok(())
            })
            .await
            .unwrap();
        app.db().write(|tx| {tx.conn().execute("UPDATE memberships SET unread_at=NULL,last_read_message_id=NULL WHERE user_id=? AND room_id=?",rusqlite::params![JASON,ALL_TALK])?;Ok(())}).await.unwrap();
        assert_eq!(
            jason
                .get(&format!("/rooms/{ALL_TALK}"))
                .await
                .text()
                .matches("id=\"unread-divider\"")
                .count(),
            0
        );
        check(
            &david.get(&format!("/rooms/{ALL_TALK}")).await.text(),
            6,
            &david_first,
        );
    }
}
