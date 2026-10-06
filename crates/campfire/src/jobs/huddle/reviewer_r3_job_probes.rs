use super::*;
use campfire_jobs::Execution;
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::models::huddle_invitations::RingRequest;
use campfire_db::{Job, Membership, Session};
use crate::controllers::presenters::test_support::{TestApp, DAVID, JASON, DIRECT_DAVID_JASON};

#[tokio::test]
async fn reviewer_r3_real_handler_keeps_pending_ring_after_deduped_reissue() {
    for suppressed in [false, true] {
        let clock=std::sync::Arc::new(campfire_kit::clock::FrozenClock::new("2026-01-01T12:00:00Z".parse().unwrap()));
        let test=TestApp::boot_with_huddle_and_clock(crate::huddle::Config::default(),clock.clone()).await.expect("parity seed required");
        test.booted.jobs.shutdown(std::time::Duration::from_secs(2)).await;
        let app=test.booted.app.clone();
        let grant=app.db.write(move |tx| {
            if suppressed { tx.conn().execute(r#"UPDATE users SET inbox_preferences='{"huddle_invitations":false}' WHERE id=?"#, [JASON])?; }
            let session=Session::start(tx,DAVID,None,None)?;
            let member=Membership::find_by_room_and_user(tx.conn(),DIRECT_DAVID_JASON,DAVID)?.unwrap();
            HuddleGrant::issue(tx,session.id,member.id,member.room_id,&campfire_db::models::room_delete::HuddleConfig {api_secret:Some("review-fixture-value".into()),admin_configured:false})
        }).await.unwrap();
        app.db.write(|tx| {tx.conn().execute("UPDATE background_jobs SET arguments=json_remove(arguments,'$.delivered') WHERE job_class='Notifications::HuddleRingJob'", [])?; Ok(())}).await.unwrap();
        let rows=app.db.read(campfire_jobs::inspect::all).await.unwrap();
        let queued=rows.into_iter().find(|j|j.class==RingRequest::CLASS).unwrap();
        let retained=serde_json::from_value::<RingRequest>(queued.arguments.clone()).unwrap();
        let execution=Execution {id:queued.id,executions:1,enqueued_at:queued.created_at,scheduled_at:queued.run_at};
        clock.advance(jiff::SignedDuration::from_secs(1));
        app.db.write(move |tx| HuddleGrant::issue(tx,grant.session_id,grant.membership_id,grant.room_id,&campfire_db::models::room_delete::HuddleConfig {api_secret:Some("review-fixture-value".into()),admin_configured:false})).await.unwrap();
        app.db.write(|tx| {tx.conn().execute("UPDATE background_jobs SET arguments=json_remove(arguments,'$.delivered') WHERE job_class='Notifications::HuddleRingJob'", [])?; Ok(())}).await.unwrap();
        let rows=app.db.read(campfire_jobs::inspect::all).await.unwrap();
        assert_eq!(rows.iter().filter(|j|j.class==RingRequest::CLASS).count(),1,"no replacement ring job");
        assert!(rows.iter().find(|j|j.id==queued.id).unwrap().arguments["cancelled"]!=1);

        use campfire_kit::Crypto;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let session=app.db.write(|tx| Session::start_with(tx,JASON,campfire_db::NewSession {two_factor_verified:true,..Default::default()})).await.unwrap();
        let signed=campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie("session_token",&session.token,None);
        let listener=crate::channels::tests::support::bind_listener().await;
        let addr=listener.local_addr().unwrap();
        let router=app.cable.router::<()>("/cable");
        let server=tokio::spawn(async move {axum::serve(listener,router).await.unwrap()});
        let mut request=format!("ws://{addr}/cable").into_client_request().unwrap();
        request.headers_mut().insert("origin",format!("http://{addr}").parse().unwrap());
        request.headers_mut().insert("cookie",format!("session_token={}",campfire_kit::cookies::escape(&signed)).parse().unwrap());
        request.headers_mut().insert("sec-websocket-protocol","actioncable-v1-json".parse().unwrap());
        let (socket,_)=tokio_tungstenite::connect_async(request).await.unwrap();
        let mut client=crate::channels::tests::support::Client {socket};
        assert_eq!(serde_json::from_str::<serde_json::Value>(&client.next_text().await).unwrap()["type"],"welcome");
        client.confirm(&serde_json::json!({"channel":"ActivityChannel"}).to_string()).await;
        ring(app.clone(),Ring(retained),execution).await.unwrap();
        app.cable.broadcast(&format!("user_{JASON}_activity"),&serde_json::json!({"reviewMarker":true}));
        let mut delivered=0;
        loop {
            let frame:serde_json::Value=serde_json::from_str(&client.next_text().await).unwrap();
            if frame["message"]["reviewMarker"]==true {break;}
            assert_eq!(frame["message"]["huddleInvitation"]["eventType"],"huddle_started");
            assert_eq!(frame["message"]["huddleInvitation"]["state"],"unread");
            delivered+=1;
        }
        client.socket.close(None).await.unwrap();
        server.abort();
        let _=server.await;
        assert_eq!(delivered,1,"suppressed={suppressed}: a deduped issuance discarded the only durable ring");
    }
}
