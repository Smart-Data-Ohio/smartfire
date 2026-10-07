use crate::controllers::presenters::{self, test_support::*};
use askama::Template;
use campfire_views::users;
use serde_json::Value;
async fn replay(name: &str) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_agent_profiles.json"
    ))
    .unwrap();
    let row = vectors["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()
        .clone();
    let agent_id = vectors["agent_id"].as_i64().unwrap();
    let setup = row.clone();
    app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM agent_events WHERE agent_id=?", [agent_id])?;
        if setup["original_memberships"] == true { tx.conn().execute("DELETE FROM memberships WHERE user_id=? AND room_id NOT IN (?,?)", [BENDER,ALL_TALK,DIRECT_KEVIN_BENDER])?; }
        if let Some(attrs)=setup["attrs"].as_object() {
            for (key,value) in attrs {
                let value=match value { Value::String(s) if key.ends_with("_at")=>rusqlite::types::Value::Text(campfire_db::Timestamp::from_jiff(s.parse().unwrap()).to_db()),Value::String(s)=>rusqlite::types::Value::Text(s.clone()),Value::Number(n)=>rusqlite::types::Value::Integer(n.as_i64().unwrap()),_=>panic!("unsupported setup") };
                tx.conn().execute(&format!("UPDATE agents SET {key}=? WHERE id=?"), rusqlite::params![value,agent_id])?;
            }
        }
        for (table,id,key) in [("users",BENDER,"user_attrs"),("rooms",setup["room_id"].as_i64().unwrap(),"room_attrs")] {
            if let Some(attrs)=setup[key].as_object() { for (field,value) in attrs { tx.conn().execute(&format!("UPDATE {table} SET {field}=? WHERE id=?"),rusqlite::params![value.as_str().unwrap(),id])?; } }
        }
        if setup["activity"]==true {
            tx.conn().execute("INSERT INTO agent_events(agent_id,event_type,room_id,message_id,outcome,created_at) SELECT ?,'mention',room_id,id,'delivered',? FROM messages ORDER BY id LIMIT 1", rusqlite::params![agent_id,tx.now()])?;
        }
        if setup["no_agent"]==true {tx.conn().execute("DELETE FROM agents WHERE id=?", [agent_id])?;}
        Ok(())
    }).await.unwrap();
    let viewer_id = row["viewer_id"].as_i64().unwrap();
    let now: jiff::Timestamp = SEED_NOW.parse().unwrap();
    let secrets = app.booted.app.secrets.clone();
    let (user, viewer, agent, manage) = app
        .db()
        .read(move |c| {
            let user = campfire_db::User::find(c, BENDER)?;
            let viewer = campfire_db::User::find(c, viewer_id)?;
            let zone: Option<String> =
                c.query_row("SELECT time_zone FROM users WHERE id=?", [viewer.id], |r| {
                    r.get(0)
                })?;
            let agent = presenters::agents::profile(
                c,
                &secrets,
                user.id,
                &viewer,
                campfire_db::Timestamp::from_jiff(now),
                &campfire_views::time::Zone::for_user(zone.as_deref()),
            )?;
            let manage = user.is_bot()
                && (viewer.is_administrator()
                    || agent.as_ref().is_some_and(|a| a.management.is_some()));
            Ok((user, viewer, agent, manage))
        })
        .await
        .unwrap();
    let summary = presenters::user_summary(&app.booted.app.secrets, &user);
    let current = presenters::view_context::current_user(&app.booted.app.secrets, &viewer);
    let transfer = presenters::accounts::transfer_id(&app.booted.app.secrets, BENDER, now);
    for (part, expected) in [(false, "html"), (true, "nav")] {
        let actual = super::people_tests::render_with(
            &app,
            |ctx| ctx.current_user = Some(current.clone()),
            |ctx| {
                let page = users::Show {
                    ctx,
                    user: summary.clone(),
                    transfer_id: transfer.clone(),
                    profile_status: None,
                    agent_profile: agent.clone(),
                    now,
                };
                if part {
                    page.as_nav().render().unwrap()
                } else {
                    page.as_content().render().unwrap()
                }
            },
        );
        if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(format!("{dir}/{name}-{expected}.actual"), &actual).unwrap();
            std::fs::write(
                format!("{dir}/{name}-{expected}.expected"),
                row[expected].as_str().unwrap(),
            )
            .unwrap();
        }
        assert!(
            actual == row[expected].as_str().unwrap(),
            "{name}: complete {expected} differs; WS8BR2_DIFF_DIR writes raw bytes"
        );
    }
    let mut browser = app.sign_in(viewer_id).await;
    let response = browser.get(&format!("/users/{BENDER}")).await;
    assert_eq!(response.status, axum::http::StatusCode::OK);
    let body = response.text();
    super::people_tests::assert_http_fragment(&body, row["html"].as_str().unwrap(), "section", "class", "panel txt-align-center");
    assert!(body.contains(&campfire_views::helpers::escape(&summary.name)));
    assert_eq!(body.contains("Manage capability grants"), manage);
    assert_eq!(
        body.contains("Last 24 hours:"),
        agent.as_ref().is_some_and(|a| a.management.is_some())
    );
    if let Some(agent) = agent {
        for (id, _) in agent.rooms {
            assert!(body.contains(&format!("href=\"/rooms/{id}\"")));
        }
    }
}
macro_rules! scenario {
    ($test:ident,$name:literal) => {
        #[tokio::test]
        async fn $test() {
            replay($name).await;
        }
    };
}
scenario!(admin_grants, "admin_grants");
scenario!(owner_grants, "owner_grants");
scenario!(peer_hides_grants, "peer_hides_grants");
scenario!(identity_status_rooms_grants, "identity_status_rooms_grants");
scenario!(owner_activity, "owner_activity");
scenario!(admin_activity, "admin_activity");
scenario!(peer_hides_activity, "peer_hides_activity");
scenario!(private_rooms_hidden, "private_rooms_hidden");
scenario!(suspended, "suspended");
scenario!(minimal_bot, "minimal_bot");

scenario!(review_agent_markup_matches_rails, "review_markup");

scenario!(original_identity_and_two_room_visibility_match_rails, "original_identity");
scenario!(original_nonmember_hides_both_rooms_match_rails, "original_private_rooms");
