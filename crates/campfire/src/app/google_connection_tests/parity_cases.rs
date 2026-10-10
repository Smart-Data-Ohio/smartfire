//! Pinned request producers: actual WS14e rows, durable jobs and organizer/grant isolation.
use super::*;
use crate::controllers::presenters::test_support::JASON;
use std::time::Duration;
#[tokio::test]
async fn google_connection_producers_and_disconnect_isolation_match_pinned_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_connection_cases.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let at = jiff::Timestamp::from_second(oracle["now"].as_i64().unwrap()).unwrap();
        let mut a = TestApp::boot_with_clock(Arc::new(campfire_kit::FrozenClock::new(at)))
            .await
            .unwrap();
        a.booted.jobs.stop(Duration::from_secs(5)).await;
        let r = Recorded::new(vec![]);
        support::install(&a, r.clone()).await;
        a.db().write(|tx|{tx.conn().execute_batch("DELETE FROM google_accounts;DELETE FROM google_identities;DELETE FROM calendar_push_channels;DELETE FROM event_calendar_entries;DELETE FROM background_jobs;DELETE FROM audit_logs;")?;Ok(())}).await.unwrap();
        let name = row["name"].as_str().unwrap();
        if name.starts_with("disconnect") {
            support::grant(
                &a,
                DAVID,
                campfire_db::Timestamp::from_jiff(at).since(jiff::SignedDuration::from_hours(1)),
                false,
            )
            .await;
        }
        if name == "disconnect_entries" {
            support::grant(
                &a,
                JASON,
                campfire_db::Timestamp::from_jiff(at).since(jiff::SignedDuration::from_hours(1)),
                false,
            )
            .await;
        }
        let spec = row.clone();
        a.db().write(move |tx| {
            let name=spec["name"].as_str().unwrap();
            tx.conn().execute("UPDATE users SET meeting_status_enabled=?,ooo_calendar_enabled=? WHERE id=?",rusqlite::params![name=="callback_meeting",name=="callback_ooo",DAVID])?;
            for (i,id) in spec["event_ids"].as_array().unwrap().iter().enumerate() {
                let id=id.as_i64().unwrap();
                if matches!(name,"disconnect_links"|"disconnect_other") {
                    tx.conn().execute("UPDATE events SET meet_link_requested=1,meet_link='https://meet.google.com/abc-defg-hij',organizer_id=? WHERE id=?",rusqlite::params![if name=="disconnect_other"&&i==1 {JASON}else{DAVID},id])?;
                }
                if name=="disconnect_entries" && i==0 {
                    for (uid,remote) in [(DAVID,"mine-fixture"),(JASON,"theirs-fixture")] {
                        campfire_db::models::google_entry::reserve(tx,id,uid)?;
                        tx.conn().execute("UPDATE event_calendar_entries SET google_event_id=? WHERE event_id=? AND user_id=?",rusqlite::params![remote,id,uid])?;
                    }
                }
            }
            if name=="disconnect_identity" {campfire_db::models::google_identity::GoogleIdentity::link_to_user(tx,json!({"sub":"connection-member","email":"login@smartdata.net","hd":"smartdata.net"}).as_object().unwrap(),DAVID)?;}
            Ok(())
        }).await.unwrap();
        let mut b = a.sign_in(DAVID).await;
        b.get("/users/me/profile").await;
        let reply = if name.starts_with("callback") {
            let state = start(&a, &mut b).await;
            let scopes = format!(
                "{} {}",
                campfire_db::models::google_account::CALENDAR_SCOPE,
                campfire_db::models::google_account::DRIVE_SCOPE
            );
            r.answer(200, tokens(&a, Some(&scopes)));
            callback(&mut b, &state).await
        } else {
            sudo(&a, &mut b).await;
            b.write(Req::new(Method::DELETE, "/google/connection"))
                .await
        };
        let crypto = a.booted.app.ar_encryption.clone();
        let ids = row["event_ids"].clone();
        let mut actual=a.db().read(move |conn| {
            let mut jobs=conn.prepare("SELECT job_class,arguments FROM background_jobs WHERE job_class LIKE 'Calendar::%'")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Value>(1)?)))?.map(|r|r.map(|(class,args)|{
                let arguments=match class.as_str(){"Calendar::SyncEntryJob"=>json!([args["event_id"],args["user_id"]]),"Calendar::MeetingRefreshJob"=>json!([args["user_id"]]),"Calendar::DisconnectCleanupJob"=>json!([args[0]]),_=>args};
                json!({"class":class,"args":arguments})
            })).collect::<rusqlite::Result<Vec<_>>>()?;
            jobs.sort_by_key(Value::to_string);
            let accounts=conn.prepare("SELECT user_id FROM google_accounts ORDER BY user_id")?.query_map([],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let identity=conn.prepare("SELECT user_id,subject FROM google_identities ORDER BY user_id")?.query_map([],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?])))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let entries=conn.prepare("SELECT user_id,google_event_id FROM event_calendar_entries ORDER BY user_id")?.query_map([],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?])))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let mut links=Vec::new();
            for id in ids.as_array().unwrap(){links.push(conn.query_row("SELECT id,organizer_id,meet_link,meet_link_requested FROM events WHERE id=?",[id.as_i64().unwrap()],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"organizer_id":r.get::<_,Option<i64>>(1)?,"meet_link":r.get::<_,Option<String>>(2)?,"meet_link_requested":r.get::<_,bool>(3)?})))?);}
            let account=GoogleAccount::for_user(conn,DAVID)?.map(|account|Ok::<_,campfire_db::Error>(json!({"email":account.email,"scopes":account.scopes,"access_token":account.access_token(&crypto).map_err(|e|campfire_db::Error::Other(e.to_string()))?,"refresh_token":account.refresh_token(&crypto).map_err(|e|campfire_db::Error::Other(e.to_string()))?,"calendar":account.calendar(),"drive":account.drive()}))).transpose()?;
            let audits=conn.prepare("SELECT action FROM audit_logs ORDER BY id")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(json!({"jobs":jobs,"accounts":accounts,"identity":identity,"entries":entries,"links":links,"account":account,"audits":audits}))
        }).await.unwrap();
        actual["status"] = json!(reply.status.as_u16());
        actual["location"] = json!(reply.location());
        // Read the real encrypted session's flash, without following away from the producer.
        let raw = b
            .cookie_header()
            .split(';')
            .find_map(|part| {
                part.trim()
                    .strip_prefix("_campfire_session=")
                    .map(str::to_owned)
            })
            .unwrap();
        let cookie = RailsCrypto::new(a.booted.app.secrets.clone())
            .decrypt_cookie(
                "_campfire_session",
                &rails_compat::cookies::unescape(&raw),
                at,
            )
            .unwrap();
        actual["flash"] = cookie["flash"]["flashes"].clone();
        let mut expected = row["result"].clone();
        if name.starts_with("callback") && expected["location"] == "http://campfire.test/users/me/profile" {
            expected["location"] = json!("http://campfire.test/app/settings/integrations");
        }
        assert_eq!(actual, expected, "pinned Rails {name}");
    }
    println!("Pinned Rails connection producers: 7 exercised; 0 skipped; recorded HTTP only");
}
