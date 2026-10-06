//! Original room and account audit clauses: actual router, actual writer, reloaded rows.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Room, RoomType};
use serde_json::{Value, json};
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/room_audit_originals.json"
    ))
    .unwrap()
}
async fn observation(app: &TestApp, action: String) -> Value {
    app.db().read(move|c|{
        let mut q=c.prepare("SELECT actor_id,actor_label,target_id,target_type,target_label,details FROM audit_logs WHERE action=? ORDER BY id")?;
        let entries=q.query_map([action],|r|Ok(json!({"actor_id":r.get::<_,Option<i64>>(0)?,"actor_label":r.get::<_,Option<String>>(1)?,"target_id":r.get::<_,Option<i64>>(2)?,"target_type":r.get::<_,Option<String>>(3)?,"target_label":r.get::<_,Option<String>>(4)?,"details":serde_json::from_str::<Value>(&r.get::<_,String>(5)?).unwrap()})))?.collect::<Result<Vec<_>,_>>()?;
        Ok(json!(entries))
    }).await.unwrap()
}
async fn run(name: &'static str) {
    let app = TestApp::boot_frozen()
        .await
        .expect("CI seed required")
        .without_job_runner()
        .await;
    let case = oracle()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
        .clone();
    let setup = case["setup"].clone();
    app.db()
        .write(move |tx| {
            if let Some(kind) = setup["type"].as_str() {
                Room::create_for(
                    tx,
                    if kind == "direct" {
                        RoomType::Direct
                    } else {
                        RoomType::Closed
                    },
                    setup["name"].as_str(),
                    DAVID,
                    &setup["members"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|id| id.as_i64().unwrap())
                        .collect::<Vec<_>>(),
                )?;
            }
            if setup["replace"] == true {
                tx.conn().execute(
                    "DELETE FROM memberships WHERE room_id=654632876 AND user_id=?",
                    [KEVIN],
                )?;
            }
            if let Some(styles) = setup["styles"].as_str() {
                tx.conn()
                    .execute("UPDATE accounts SET custom_styles=?", [styles])?;
            }
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut browsers = std::collections::BTreeMap::new();
    for (step, expected) in case["requests"]
        .as_array()
        .unwrap()
        .iter()
        .zip(case["responses"].as_array().unwrap())
    {
        let who = step["viewer"].as_i64().unwrap();
        if let std::collections::btree_map::Entry::Vacant(entry) = browsers.entry(who) {
            let mut browser = if who == DAVID {
                app.david()
            } else {
                app.sign_in(who).await
            };
            if who == DAVID {
                assert_eq!(
                    browser
                        .write(
                            Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")])
                        )
                        .await
                        .status,
                    StatusCode::FOUND
                );
            }
            entry.insert(browser);
        }
        let before = observation(&app, case["action"].as_str().unwrap().into())
            .await
            .as_array()
            .unwrap()
            .len();
        let reply = browsers
            .get_mut(&who)
            .unwrap()
            .write(
                Req::new(
                    step["method"].as_str().unwrap().parse().unwrap(),
                    step["path"].as_str().unwrap(),
                )
                .header("content-type", "application/json")
                .header("accept", "text/html")
                .body(serde_json::to_vec(&step["params"]).unwrap()),
            )
            .await;
        assert!(
            reply.status.as_u16() < 500,
            "{name}: invalid transport {} {}",
            reply.status,
            reply.text()
        );
        let entries = observation(&app, case["action"].as_str().unwrap().into()).await;
        let mut actual = json!({"delta":entries.as_array().unwrap().len() as i64-before as i64,"entries":entries});
        if expected.get("status").is_some() {
            actual["status"] = json!(reply.status.as_u16());
        }
        assert_eq!(
            actual, *expected,
            "{name}: original audit delta and exact row fields"
        );
    }
}
macro_rules! cases {($($name:ident=>$case:literal),+$(,)?)=>{$(#[tokio::test]async fn $name(){run($case).await;})+};}
cases!(
 original_open_creation_actor_target_label_and_details=>"create",
 original_refused_closed_creation_has_no_audit=>"failed",
 original_group_creation_then_reopening_has_one_audit=>"direct_reuse",
 original_room_destroy_label_and_actor=>"destroy",
 original_membership_granted_and_revoked_names=>"membership",
 original_unchanged_membership_has_no_audit=>"unchanged",
 original_group_add_jz_has_actor_and_granted_name=>"add",
 original_group_add_existing_jason_has_no_audit=>"add_none",
 original_last_leaver_kevin_destroys_weekend_plans=>"last_leave",
 original_other_leaver_has_no_destroy_audit=>"leave_no_destroy",
 original_jz_channel_leave_actor_label_and_revocation=>"leave_channel",
 original_group_leave_david_actor_and_revocation=>"leave_group",
 original_combined_account_settings_audit_fields=>"settings",
 original_styles_audit_sizes_and_digests=>"styles",
 original_same_styles_have_no_audit=>"styles_same",
);
