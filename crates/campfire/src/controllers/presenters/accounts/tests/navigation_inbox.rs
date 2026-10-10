use super::*;
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
            campfire_db::ActivityItem::accessible_to(conn, &viewer)
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
            let items = campfire_db::ActivityItem::accessible_to(
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
            campfire_db::ActivityItem::accessible_to(
                conn,
                &campfire_db::User::find(conn, viewer_id)?,
            )
        })
        .await
        .unwrap();
    assert!(
        !items
            .iter()
            .any(|i| ["Message", "AgentApproval"].contains(&i.source_type.as_str())),
        "inbox must exclude revoked message sources and approvals owned by another user"
    );
    let mut member = test.browser("198.51.100.142");
    member.sign_in(&test.label("emails.kevin")).await;
    let inbox = member.request(axum::http::Method::GET, "/activity?type=agents", &[("accept", "application/json")], None).await;
    assert_eq!(inbox.status, StatusCode::OK, "{}", inbox.text());
    assert_eq!(inbox.header("cache-control"), Some("no-store"));
    assert_eq!(inbox.header("pragma"), Some("no-cache"));
    let payload: serde_json::Value = serde_json::from_str(&inbox.text()).unwrap();
    assert!(payload["activity_items"].as_array().unwrap().is_empty());
}
