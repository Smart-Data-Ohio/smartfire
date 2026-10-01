//! Native child partials for the complete room compositor. Cross-room quotes remain
//! viewer-authorized lazy frames; the cached parent carries no private source data.
use super::Presenter;
use campfire_db::{Message, Result, Room, User};

/// Rails message_quote_stamp/message_quote_names_digest: edits and source renames
/// invalidate a quoting parent's cached bytes even when that parent is untouched.
pub fn cache_stamp(conn: &campfire_db::Connection, message: &Message) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut query = conn.prepare("SELECT source.updated_at,source.edited_at,author.name,room.name FROM message_references ref JOIN messages source ON source.id=ref.referenced_message_id JOIN users author ON author.id=source.creator_id JOIN rooms room ON room.id=source.room_id WHERE ref.message_id=? ORDER BY ref.id")?;
    let dependencies = query
        .query_map([message.id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if dependencies.is_empty() {
        return Ok(String::new());
    }
    Ok(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&dependencies).expect("quote cache dependencies serialize")
        )
    ))
}

pub fn cards(presenter: &Presenter<'_>, message: &Message) -> Result<Vec<String>> {
    let mut query = presenter.conn.prepare(
        "SELECT id,referenced_message_id FROM message_references WHERE message_id=? ORDER BY id",
    )?;
    let references = query
        .query_map([message.id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    references
        .into_iter()
        .map(|(id, source_id)| {
            let source = Message::find(presenter.conn, source_id)?;
            let body = if source.room_id != message.room_id {
                campfire_views::message_links::lazy(id, message.room_id)
            } else {
                let user = User::find(presenter.conn, source.creator_id)?;
                let room = Room::find(presenter.conn, source.room_id)?;
                let room_label = if room.direct() {
                    "a direct message"
                } else {
                    room.name.as_deref().unwrap_or_default()
                };
                let plain_text = presenter.plain_text_body(&source)?;
                let path = if let Some(thread) = source.thread_id {
                    format!(
                        "/rooms/{}?message_id={}&thread={thread}",
                        source.room_id, source.id
                    )
                } else {
                    format!("/rooms/{}/@{}", source.room_id, source.id)
                };
                super::page::render_detached_at(
                    presenter.app,
                    None,
                    presenter
                        .cache_base_url
                        .as_deref()
                        .unwrap_or(campfire_views::message_links::ORIGIN_SLOT),
                    |ctx| {
                        campfire_views::message_links::Card {
                            ctx,
                            author: &user.name,
                            room_label,
                            created_at: source.created_at.jiff(),
                            plain_text: &plain_text,
                            path: &path,
                        }
                        .html()
                    },
                )
            };
            Ok(format!("\n    {body}\n"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controllers::presenters::{
        page,
        test_support::{DAVID, KEVIN, TestApp},
    };
    use campfire_db::NewMessage;
    #[tokio::test]
    async fn same_room_quote_children_match_three_complete_rails_partials() {
        let test = TestApp::boot().await.expect("build pinned parity seed");
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("quote_child_vectors.json")).unwrap();
        assert_eq!(vectors["cases"].as_array().unwrap().len(), 3);
        for case in vectors["cases"].as_array().unwrap() {
            let input = &case["input"];
            let html = page::render_detached_at(
                &test.booted.app,
                None,
                "http://campfire.test",
                |ctx| {
                    let card = campfire_views::message_links::Card {
                        ctx,
                        author: input["author"].as_str().unwrap(),
                        room_label: input["room_label"].as_str().unwrap(),
                        created_at: input["created_at"].as_str().unwrap().parse().unwrap(),
                        plain_text: input["plain_text"].as_str().unwrap(),
                        path: input["path"].as_str().unwrap(),
                    }
                    .html();
                    format!(
                        "<div id=\"message_link_cards_message_{}\" class=\"message-link-cards\">\n    {card}\n</div>\n",
                        input["client_message_id"].as_str().unwrap()
                    )
                },
            );
            assert_eq!(html, case["html"].as_str().unwrap());
        }
    }
    #[tokio::test]
    async fn warm_quote_parent_refreshes_legacy_edits_and_source_names() {
        let test = TestApp::boot().await.expect("build pinned parity seed");
        let (room, source, quoting) = test
            .db()
            .write(|tx| {
                let room = Room::create_for(
                    tx,
                    campfire_db::RoomType::Closed,
                    Some("Old quote room"),
                    DAVID,
                    &[DAVID, super::super::test_support::JASON],
                )?;
                let source = Message::create(
                    tx,
                    NewMessage {
                        room_id: room.id,
                        creator_id: super::super::test_support::JASON,
                        body: Some("<p>Old quoted excerpt</p>".into()),
                        ..Default::default()
                    },
                )?;
                let quoting = Message::create(
                    tx,
                    NewMessage {
                        room_id: room.id,
                        creator_id: DAVID,
                        markdown_source: Some(format!("/rooms/{}/@{}", room.id, source.id)),
                        ..Default::default()
                    },
                )?;
                Ok((room, source, quoting))
            })
            .await
            .unwrap();
        // Build once without a request, then render under two real view contexts.
        // Quote jumps follow the parent renderer's origin, including job defaults.
        let app = test.booted.app.clone();
        let quoting_id = quoting.id;
        let view = test
            .db()
            .read(move |conn| {
                let parent = Message::find(conn, quoting_id)?;
                Presenter::new(conn, &app, None).message(&parent)
            })
            .await
            .unwrap();
        for base in ["http://example.org", "https://other.example:445"] {
            let html = page::render_detached_at(&test.booted.app, None, base, |ctx| {
                campfire_views::messages::message(ctx, &view)
            });
            assert!(html.contains(&format!("href=\"{base}/rooms/{}/@{}\"", room.id, source.id)));
        }
        let mut browser = test.sign_in(DAVID).await;
        let path = format!("/rooms/{}", room.id);
        let card = |html: String| {
            html.split(&format!(
                "id=\"message_link_cards_message_{}\"",
                quoting.client_message_id
            ))
            .nth(1)
            .unwrap()
            .split("</blockquote>")
            .next()
            .unwrap()
            .to_owned()
        };
        let old = card(browser.get(&path).await.text());
        assert!(old.contains("Old quoted excerpt"));
        assert!(old.contains("Old quote room"));
        assert!(old.contains(">Jason</span>"));
        assert_eq!(card(browser.get(&path).await.text()), old);
        let source_id = source.id;
        test.db().write(move |tx| {
            // The frozen helper explicitly includes edited_at for legacy rich-text edits
            // that leave source.updated_at untouched, plus names not touched by that edit.
            tx.conn().execute("UPDATE action_text_rich_texts SET body='<p>New quoted excerpt</p>' WHERE record_type='Message' AND record_id=?",[source_id])?;
            tx.conn().execute("UPDATE messages SET edited_at='2026-03-02 16:00:01' WHERE id=?",[source_id])?;
            Ok(())
        }).await.unwrap();
        let edited = card(browser.get(&path).await.text());
        assert!(edited.contains("New quoted excerpt"));
        assert!(!edited.contains("Old quoted excerpt"));
        let room_id = room.id;
        test.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE users SET name='Renamed quote author' WHERE id=?",
                    [super::super::test_support::JASON],
                )?;
                tx.conn().execute(
                    "UPDATE rooms SET name='Renamed quote room' WHERE id=?",
                    [room_id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let renamed = card(browser.get(&path).await.text());
        assert!(renamed.contains("Renamed quote author"));
        assert!(renamed.contains("Renamed quote room"));
        assert!(!renamed.contains("Old quote room"));
        assert!(!renamed.contains(">Jason</span>"));
    }
    #[tokio::test]
    async fn cross_room_quote_parent_never_renders_private_source_facts() {
        let test = TestApp::boot().await.expect("build pinned parity seed");
        let message = test
            .db()
            .write(|tx| {
                let private = Room::create_for(
                    tx,
                    campfire_db::RoomType::Closed,
                    Some("Private source room"),
                    KEVIN,
                    &[KEVIN],
                )?;
                let source = Message::create(
                    tx,
                    NewMessage {
                        room_id: private.id,
                        creator_id: KEVIN,
                        markdown_source: Some("Secret source body must stay private".into()),
                        ..Default::default()
                    },
                )?;
                let quoting = Message::create(
                    tx,
                    NewMessage {
                        room_id: crate::controllers::presenters::test_support::ALL_TALK,
                        creator_id: DAVID,
                        markdown_source: Some(format!("/rooms/{}/@{}", private.id, source.id)),
                        ..Default::default()
                    },
                )?;
                Ok(quoting)
            })
            .await
            .unwrap();
        let app = test.booted.app.clone();
        test.db()
            .read(move |conn| {
                let html = cards(&Presenter::new(conn, &app, None), &message)?.concat();
                assert!(html.contains("loading=\"lazy\""));
                assert!(html.contains("message-link-frame"));
                assert!(!html.contains("Secret source body"));
                assert!(!html.contains("Private source room"));
                assert!(!html.contains("Kevin"));
                Ok(())
            })
            .await
            .unwrap();
    }
}
