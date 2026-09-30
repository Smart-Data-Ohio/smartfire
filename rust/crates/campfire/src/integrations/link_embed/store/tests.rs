use super::*;
use crate::controllers::presenters::test_support::*;
use campfire_db::NewMessage;

#[tokio::test]
async fn ws15e_link_rows_claims_and_sync_match_rails() {
    let mut app = TestApp::boot().await.expect("build parity seed");
    app.booted.jobs.stop(std::time::Duration::from_secs(1)).await;
    app.db()
        .write(|tx| {
            let embed = Embed::for_reference(tx, "https://example.com/a")?;
            assert_eq!(Embed::for_reference(tx, "https://example.com/a")?.id, embed.id);
            assert!(embed.needs_fetch(tx.now()));
            assert!(request_fetch(tx, &embed)?);
            assert!(!request_fetch(tx, &embed)?);
            embed.save_metadata(
                tx,
                &Metadata {
                    title: Some("Title".into()),
                    ..Default::default()
                },
            )?;
            let fresh = Embed::find(tx.conn(), embed.id)?;
            assert!(fresh.usable());
            assert!(!fresh.needs_fetch(tx.now()));
            assert!((fresh.expires_at.unwrap().as_second() - tx.now().as_second() - 86400).abs() <= 1);
            let invalid = Metadata {
                title: Some("é".repeat(301)),
                ..Default::default()
            };
            assert!(embed.save_metadata(tx, &invalid).is_err());
            embed.save_negative(tx, "Could not load this link")?;
            let cached = Embed::find(tx.conn(), embed.id)?;
            assert_eq!(cached.title, Some("Title".into()));
            assert!((cached.expires_at.unwrap().as_second() - tx.now().as_second() - 3600).abs() <= 1);
            let mut message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    body: Some("<p>https://EXAMPLE.com/a#own https://example.com/b</p>".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE messages SET markdown_source=?1 WHERE id=?2",
                params!["<https://example.com/b>", message.id],
            )?;
            message = Message::find(tx.conn(), message.id)?;
            sync_message(tx, &message, true)?;
            let refs = Reference::for_message(tx.conn(), &message)?;
            assert_eq!(refs.len(), 1);
            assert_eq!(refs[0].display_url(), "https://EXAMPLE.com/a#own");
            sync_message(tx, &message, true)?;
            assert_eq!(Reference::for_message(tx.conn(), &message)?[0].id, refs[0].id);
            message.markdown_source = None;
            sync_message(tx, &message, true)?;
            assert!(Reference::for_message(tx.conn(), &message)?.is_empty());
            Ok(())
        })
        .await
        .unwrap();
    let jobs = app.db().read(campfire_jobs::inspect::all).await.unwrap();
    assert_eq!(jobs.iter().filter(|job| job.class == "LinkEmbed::FetchJob").count(), 1);
}

#[tokio::test]
async fn ws15e_link_references_preserve_room_urls_reconcile_edits_and_refresh_expired_rows() {
    let mut app=TestApp::boot().await.expect("build parity seed");
    app.booted.jobs.stop(std::time::Duration::from_secs(1)).await;
    let messages=futures_util::future::join_all((0..12).map(|index| {
        let db=app.db().clone();
        async move { db.write(move |tx| {
            let room=if index%2==0 { ALL_TALK } else { QUIET_CORNER };
            let raw=format!("https://EXAMPLE.com/shared#{index}");
            let message=Message::create(tx,NewMessage { room_id:room,creator_id:DAVID,body:Some(format!("<p>{raw}</p>")),..Default::default() })?;
            tx.conn().execute("UPDATE messages SET markdown_source=?1 WHERE id=?2",params![raw,message.id])?;
            let message=Message::find(tx.conn(),message.id)?;
            sync_message(tx,&message,true)?;
            let refs=Reference::for_message(tx.conn(),&message)?;
            assert_eq!(refs.len(),1);
            assert_eq!(refs[0].display_url(),raw);
            Ok((message,refs[0].clone()))
        }).await.unwrap() }
    })).await;
    assert!(messages.iter().all(|(_,reference)| reference.embed.id==messages[0].1.embed.id));
    let (message,reference)=messages[0].clone();
    app.db().write(move |tx| {
        let mut message=message;
        tx.conn().execute("UPDATE action_text_rich_texts SET body='<p>https://EXAMPLE.com/shared#edited</p>' WHERE record_type='Message' AND record_id=?",[message.id])?;
        sync_message(tx,&message,true)?;
        let refs=Reference::for_message(tx.conn(),&message)?;
        assert_eq!(refs[0].id,reference.id);
        assert_eq!(refs[0].display_url(),"https://EXAMPLE.com/shared#edited");
        tx.conn().execute("UPDATE link_embeds SET expires_at=?1,fetch_requested_at=?2 WHERE id=?3",params![tx.now().ago(jiff::SignedDuration::from_secs(1)),tx.now().ago(jiff::SignedDuration::from_mins(11)),reference.embed.id])?;
        message.embeds_suppressed=true;
        sync_message(tx,&message,true)?;
        assert_eq!(Reference::for_message(tx.conn(),&message)?.len(),1);
        assert!(Embed::find(tx.conn(),reference.embed.id)?.fetch_requested_at.unwrap()<tx.now().ago(FETCH_WINDOW));
        message.embeds_suppressed=false;
        sync_message(tx,&message,true)?;
        assert!(Embed::find(tx.conn(),reference.embed.id)?.fetch_requested_at.unwrap()>tx.now().ago(FETCH_WINDOW));
        tx.conn().execute("UPDATE action_text_rich_texts SET body='<p>https://example.com/new</p><pre>https://example.com/code</pre>' WHERE record_type='Message' AND record_id=?",[message.id])?;
        sync_message(tx,&message,false)?;
        let refs=Reference::for_message(tx.conn(),&message)?;
        assert_eq!(refs.len(),1);
        assert_eq!(refs[0].embed.normalized_url,"https://example.com/new");
        Ok(())
    }).await.unwrap();
    let jobs=app.db().read(campfire_jobs::inspect::all).await.unwrap();
    assert_eq!(jobs.iter().filter(|job| job.class=="LinkEmbed::FetchJob").count(),2,"twelve writers share the initial claim; only the expired row refetches");
}
