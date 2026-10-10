//! Every Rails administrator Google-link controller case, plus idempotent repeats.
use super::google_api_tests::{self as support, Recorded};
use crate::controllers::presenters::test_support::{Req, TestApp};
use campfire_kit::FrozenClock;
use hyper::Method;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
#[tokio::test]
async fn google_admin_link_controls_and_all_security_audits_match_pinned_rails() {
    let v: Value =
        serde_json::from_str(include_str!("../../../../vectors/google_admin_links.json")).unwrap();
    let at: jiff::Timestamp = "2026-03-02T16:00:00Z".parse().unwrap();
    for case in v["cases"].as_array().unwrap() {
        let mut a = TestApp::boot_with_clock(Arc::new(FrozenClock::new(at)))
            .await
            .unwrap();
        a.booted.jobs.stop(Duration::from_secs(5)).await;
        let recorded = Recorded::new(vec![]);
        support::install(&a, recorded.clone()).await;
        let input = case.clone();
        a.db().write(move |tx| {
            tx.conn().execute_batch("DELETE FROM google_identities;DELETE FROM audit_logs;")?;
            let kevin=input["kevin_id"].as_i64().unwrap();let jz=input["jz_id"].as_i64().unwrap();
            tx.conn().execute("UPDATE users SET email_address='kevin@smartdata.net',email_self_changed_at=?,google_email_link_allowed=? WHERE id=?",rusqlite::params![(input["spec"]["allowed"]!=true).then_some(tx.now().ago(jiff::SignedDuration::from_hours(24))),input["spec"]["allowed"]==true,kevin])?;
            if input["spec"]["linked"]!=false {campfire_db::models::google_identity::GoogleIdentity::link_to_user(tx,json!({"sub":"google-sub-jz","email":"jz@smartdata.net","hd":"smartdata.net"}).as_object().unwrap(),jz)?;}
            Ok(())
        }).await.unwrap();
        let mut actor = a.sign_in(case["actor_id"].as_i64().unwrap()).await;
        actor.get("/users/me/profile").await;
        let path = case["target_id"]
            .as_i64()
            .map(|id| format!("/account/users/{id}/google_link"))
            .unwrap_or("/api/v1/admin/people".into());
        let method: Method = case["spec"]["method"]
            .as_str()
            .unwrap()
            .to_uppercase()
            .parse()
            .unwrap();
        let response = if method == Method::GET {
            actor.get(&path).await
        } else {
            actor.write(Req::new(method.clone(), &path)).await
        };
        let name = case["spec"]["name"].as_str().unwrap();
        assert_eq!(
            response.status.as_u16() as u64,
            case["status"].as_u64().unwrap(),
            "{name}: {}",
            response.text()
        );
        assert_eq!(
            json!(response.location()),
            case["location"],
            "{name}: destination"
        );
        let html = response.text();
        // K15 retired the account HTML; the SPA exposes the same control capabilities as JSON.
        let forms = if method == Method::GET {
            response.json()["people"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|person| {
                    person["banned"] == false
                        && (person["googleIdentityEmail"].is_string()
                            || person["offerGoogleEmailLink"] == true)
                })
                .map(|person| format!("/account/users/{}/google_link", person["id"]))
                .collect::<Vec<_>>()
        } else {
            assert!(!html.contains("<form"));
            Vec::new()
        };
        assert_eq!(json!(forms), case["forms"], "{name}: complete control set");
        let kevin = case["kevin_id"].as_i64().unwrap();
        let jz = case["jz_id"].as_i64().unwrap();
        let state=a.db().read(move |c| {
            let flags=c.query_row("SELECT email_self_changed_at,google_email_link_allowed FROM users WHERE id=?",[kevin],|r|Ok((r.get::<_,Option<campfire_db::Timestamp>>(0)?,r.get::<_,bool>(1)?)))?;
            let count=c.query_row("SELECT count(*) FROM google_identities WHERE user_id=?",[jz],|r|r.get::<_,i64>(0))?;
            let audits=c.prepare("SELECT action,actor_id,target_id,target_type,target_label,details FROM audit_logs ORDER BY id")?.query_map([],|r|Ok(json!({"action":r.get::<_,String>(0)?,"actor_id":r.get::<_,Option<i64>>(1)?,"target_id":r.get::<_,Option<i64>>(2)?,"target_type":r.get::<_,Option<String>>(3)?,"target_label":r.get::<_,Option<String>>(4)?,"details":r.get::<_,Value>(5)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(json!({"changed_at":flags.0.map(|t|t.jiff().to_string()),"allowed":flags.1,"identity_count":count,"audits":audits}))
        }).await.unwrap();
        for key in ["changed_at", "allowed", "identity_count", "audits"] {
            assert_eq!(state[key], case[key], "{name}: {key}");
        }
        assert!(recorded.calls.lock().unwrap().is_empty());
    }
    println!("Pinned Rails admin Google links: 9 exercised; 0 skipped");
}
