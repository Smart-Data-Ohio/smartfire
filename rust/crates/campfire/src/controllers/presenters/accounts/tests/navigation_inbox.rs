use super::*;
#[tokio::test]
async fn public_bot_profile_and_sidebar_preserve_management_privacy() {
    let test = boot_seed("agents_ui").await.expect("agents_ui seed");
    let mut member = test.browser("198.51.100.140");
    member.sign_in(&test.label("emails.kevin")).await;
    let bot = test.label("users.bender");
    let bot_id = bot.parse::<i64>().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            let mut agent = campfire_db::Agent::for_user(tx.conn(), bot_id)?.unwrap();
            agent.update(
                tx,
                campfire_db::AgentChanges {
                    status: Some("working".into()),
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let page = member.get(&format!("/users/{bot}")).await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    for expected in [
        "Bender Bot",
        "Workspace agent, managed by David",
        "Working",
        "Grants:",
    ] {
        assert!(page.text().contains(expected), "{expected}");
    }
    for private in [
        "Today's usage:",
        "Last 24 hours:",
        "Manage capability grants",
        "View activity ledger",
        "View approval requests",
    ] {
        assert!(!page.text().contains(private), "{private}");
    }
    let sidebar = member.get("/users/me/sidebar").await;
    assert_eq!(sidebar.status, StatusCode::OK);
    assert!(sidebar.text().contains("href=\"/agents\""));
    assert!(sidebar.text().contains("Activity inbox"));
    let mut admin = test.browser("198.51.100.141");
    admin.sign_in(&test.label("emails.david")).await;
    let page = admin.get(&format!("/users/{bot}")).await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    for expected in [
        "Today's usage:",
        "Last 24 hours:",
        "Manage capability grants",
        "View activity ledger",
        "View approval requests",
    ] {
        assert!(page.text().contains(expected), "{expected}");
    }
}
#[tokio::test]
async fn inbox_reader_excludes_other_owners_and_revoked_message_sources() {
    let test = boot_seed("agents_ui").await.expect("agents_ui seed");
    let viewer_id = test.label("users.kevin").parse::<i64>().unwrap();
    let items = test
        .booted
        .app
        .db
        .read(move |conn| {
            let viewer = campfire_db::User::find(conn, viewer_id)?;
            crate::controllers::presenters::activity::accessible(conn, &viewer)
        })
        .await
        .unwrap();
    assert!(items.iter().any(|i| i.source_type == "AgentApproval"));
    test.booted
        .app
        .db
        .read(move |conn| {
            for item in &items {
                assert_eq!(item.user_id, viewer_id);
                if item.source_type == "AgentApproval" {
                    let a = campfire_db::AgentApproval::find(conn, item.source_id)?.unwrap();
                    assert_eq!(
                        campfire_db::Agent::find(conn, a.agent_id)?
                            .unwrap()
                            .owner_id,
                        Some(viewer_id)
                    );
                }
            }
            Ok(())
        })
        .await
        .unwrap();
    let new_owner = test.label("users.david").parse::<i64>().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            let items = crate::controllers::presenters::activity::accessible(
                tx.conn(),
                &campfire_db::User::find(tx.conn(), viewer_id)?,
            )?;
            for item in items
                .into_iter()
                .filter(|i| i.source_type == "AgentApproval")
            {
                let approval =
                    campfire_db::AgentApproval::find(tx.conn(), item.source_id)?.unwrap();
                let mut agent = campfire_db::Agent::find(tx.conn(), approval.agent_id)?.unwrap();
                agent.update(
                    tx,
                    campfire_db::AgentChanges {
                        owner_id: Some(Some(new_owner)),
                        ..Default::default()
                    },
                )?;
            }
            tx.conn()
                .execute("DELETE FROM memberships WHERE user_id=?", [viewer_id])?;
            Ok(())
        })
        .await
        .unwrap();
    let items = test
        .booted
        .app
        .db
        .read(move |conn| {
            crate::controllers::presenters::activity::accessible(
                conn,
                &campfire_db::User::find(conn, viewer_id)?,
            )
        })
        .await
        .unwrap();
    assert!(
        !items
            .iter()
            .any(|i| ["Message", "AgentApproval"].contains(&i.source_type.as_str()))
    );
    let mut member = test.browser("198.51.100.142");
    member.sign_in(&test.label("emails.kevin")).await;
    let inbox = member.get("/activity?type=agents").await;
    assert_eq!(inbox.status, StatusCode::OK, "{}", inbox.text());
    assert_eq!(inbox.header("cache-control"), Some("no-store"));
    assert_eq!(inbox.header("pragma"), Some("no-cache"));
    assert!(inbox.text().contains("activity-inbox-title"));
}
