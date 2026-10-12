use super::*;

#[tokio::test]
async fn stage_audience_budget_refuses_full_joins_and_publishes_live_status() {
    let a = app(true).await.expect("frozen seeds required");
    let (room, _, _, speaker) = stage(&a).await;
    let room_id = room.id;
    a.db()
        .write(move |tx| {
            Membership::find(tx.conn(), speaker)?.change_stage_role_with_config(
                tx,
                StageRole::Listener,
                &domain(),
            )
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let workspace = "/api/v1/admin/workspace";
    let change = json!({"stageAudienceBudget":1});
    assert_eq!(
        jason
            .write(send_json(Method::PATCH, workspace, &change))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let saved = david
        .write(send_json(Method::PATCH, workspace, &change))
        .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.text());
    assert_eq!(saved.json()["stageAudienceBudget"], 1);
    for invalid in [json!(0), json!(-1), json!(9007199254740992_i64)] {
        let reply = david
            .write(send_json(
                Method::PATCH,
                workspace,
                &json!({"stageAudienceBudget":invalid}),
            ))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
    }
    let stage_path = format!("/api/v1/rooms/{room_id}/stage");
    let detail: api::StageDetail = parse(&david.send(get(&stage_path)).await);
    assert_eq!(
        (
            detail.stage.audience_count,
            detail.stage.audience_budget,
            detail.stage.audience_availability
        ),
        (0, 1, api::StageAudienceAvailability::Open)
    );
    let first = join(&mut jason, room_id).await;
    let detail: api::StageDetail = parse(&david.send(get(&stage_path)).await);
    assert_eq!(
        (
            detail.stage.audience_count,
            detail.stage.audience_availability
        ),
        (0, api::StageAudienceAvailability::Full)
    );
    let refused = kevin
        .write(empty(
            Method::POST,
            &format!("/api/v1/rooms/{room_id}/huddle"),
        ))
        .await;
    assert_eq!(
        denial(&refused),
        denied(
            StatusCode::CONFLICT,
            "StageAudienceFull",
            "This stage is full"
        )
    );
    seen(&a, first.grant_id).await;
    assert_eq!(join(&mut jason, room_id).await.grant_id, first.grant_id);
    let mut device = a.sign_in(JASON).await;
    seen(&a, join(&mut device, room_id).await.grant_id).await;
    seen(&a, join(&mut david, room_id).await.grant_id).await;
    let promoted = david
        .write(send_json(
            Method::PATCH,
            &format!("{stage_path}/members/{speaker}"),
            &json!({"role":"speaker"}),
        ))
        .await;
    assert_eq!(promoted.status, StatusCode::OK);
    seen(&a, join(&mut kevin, room_id).await.grant_id).await;
    let detail: api::StageDetail = parse(&david.send(get(&stage_path)).await);
    assert_eq!(
        (
            detail.stage.audience_count,
            detail.stage.audience_budget,
            detail.stage.audience_availability
        ),
        (1, 1, api::StageAudienceAvailability::Full)
    );
    assert_eq!(
        jason
            .write(empty(
                Method::POST,
                &format!("/api/v1/rooms/{room_id}/huddle/leave")
            ))
            .await
            .status,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(
        device
            .write(empty(
                Method::POST,
                &format!("/api/v1/rooms/{room_id}/huddle/leave")
            ))
            .await
            .status,
        StatusCode::NO_CONTENT,
    );
    let detail: api::StageDetail = parse(&david.send(get(&stage_path)).await);
    assert_eq!(
        (
            detail.stage.audience_count,
            detail.stage.audience_availability
        ),
        (0, api::StageAudienceAvailability::Open)
    );
}

#[tokio::test]
async fn stage_audience_budget_gateway_refuses_delayed_tokens_and_preserves_presence() {
    let clock = std::sync::Arc::new(campfire_kit::FrozenClock::new(
        crate::controllers::presenters::test_support::SEED_NOW
            .parse()
            .unwrap(),
    ));
    let a = TestApp::boot_with_settings(configured(), clock.clone(), &[])
        .await
        .expect("frozen seeds required");
    let (room, _, _, speaker) = stage(&a).await;
    let room_id = room.id;
    a.db()
        .write(move |tx| {
            campfire_db::Account::first_in(tx)?.update(
                tx,
                None,
                None,
                Some(&[("stage_audience_budget", "1")]),
            )?;
            Membership::find(tx.conn(), speaker)?.change_stage_role_with_config(
                tx,
                StageRole::Listener,
                &domain(),
            )
        })
        .await
        .unwrap();
    let mut kevin = a.sign_in(KEVIN).await;
    let mut jason = a.sign_in(JASON).await;
    let delayed = join(&mut kevin, room_id).await;
    clock.advance(jiff::SignedDuration::from_secs(21));
    let joined = join(&mut jason, room_id).await;
    let authorize = |token: &str| {
        empty(Method::POST, "/internal/huddle/authorize")
            .header("x-huddle-gateway-secret", "ws13-fixture-gateway-secret")
            .header("authorization", &format!("Bearer {token}"))
    };
    assert_eq!(
        jason.send(authorize(&joined.token)).await.status,
        StatusCode::OK
    );
    let refused = kevin.send(authorize(&delayed.token)).await;
    assert_eq!(refused.status, StatusCode::CONFLICT);
    assert_eq!(
        refused.json(),
        json!({"error":"StageAudienceFull","message":"This stage is full"})
    );
    let heartbeat = kevin
        .send(
            get(&format!("/internal/huddle/grants/{}", delayed.grant_id))
                .header("x-huddle-gateway-secret", "ws13-fixture-gateway-secret"),
        )
        .await;
    assert_eq!(heartbeat.status, StatusCode::CONFLICT);
    let check = kevin
        .send(
            get(&format!(
                "/internal/huddle/grants/{}?record_seen=0",
                delayed.grant_id
            ))
            .header("x-huddle-gateway-secret", "ws13-fixture-gateway-secret"),
        )
        .await;
    assert_eq!(check.status, StatusCode::OK);
    let detail: api::StageDetail = parse(
        &jason
            .send(get(&format!("/api/v1/rooms/{room_id}/stage")))
            .await,
    );
    assert_eq!(detail.stage.audience_count, 1);
    assert_eq!(
        jason
            .write(empty(
                Method::POST,
                &format!("/api/v1/rooms/{room_id}/huddle/leave")
            ))
            .await
            .status,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(
        kevin.send(authorize(&delayed.token)).await.status,
        StatusCode::OK
    );
    let grant = a
        .db()
        .read(move |conn| HuddleGrant::find_by_id(conn, delayed.grant_id))
        .await
        .unwrap()
        .unwrap();
    assert!(grant.last_seen_at.is_some());
    assert!(!grant.revoked());
}

#[tokio::test]
async fn stage_audience_budget_changes_publish_to_open_panes() {
    let a = app(true).await.expect("frozen seeds required");
    let (room, _, _, _) = stage(&a).await;
    let room_id = room.id;
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    david
        .write(send_json(
            Method::PATCH,
            "/api/v1/admin/workspace",
            &json!({"stageAudienceBudget":1}),
        ))
        .await;
    seen(&a, join(&mut jason, room_id).await.grant_id).await;
    let (addr, server) = serve(&a).await;
    let mut sync = Sync::connect(addr, &jason.cookie_header(), &[format!("room:{room_id}")]).await;
    let saved = david
        .write(send_json(
            Method::PATCH,
            "/api/v1/admin/workspace",
            &json!({"stageAudienceBudget":2}),
        ))
        .await;
    assert_eq!(saved.status, StatusCode::OK);
    let updated = sync.until(stage_of(room_id)).await;
    assert_eq!(
        (
            updated.audience_count,
            updated.audience_budget,
            updated.audience_availability
        ),
        (1, 2, api::StageAudienceAvailability::Open)
    );
    server.abort();
}

#[tokio::test]
async fn stage_audience_budget_presence_poll_publishes_expired_reservations() {
    let clock = std::sync::Arc::new(campfire_kit::FrozenClock::new(
        crate::controllers::presenters::test_support::SEED_NOW
            .parse()
            .unwrap(),
    ));
    let a = TestApp::boot_with_settings(configured(), clock.clone(), &[])
        .await
        .expect("frozen seeds required");
    let (room, _, _, _) = stage(&a).await;
    let room_id = room.id;
    let mut david = a.sign_in(DAVID).await;
    let mut jason = a.sign_in(JASON).await;
    david
        .write(send_json(
            Method::PATCH,
            "/api/v1/admin/workspace",
            &json!({"stageAudienceBudget":1}),
        ))
        .await;
    join(&mut jason, room_id).await;
    let (addr, server) = serve(&a).await;
    let mut sync = Sync::connect(addr, &jason.cookie_header(), &[format!("room:{room_id}")]).await;
    clock.advance(jiff::SignedDuration::from_secs(21));
    assert_eq!(
        jason.send(get("/api/v1/huddles")).await.status,
        StatusCode::OK
    );
    let updated = sync.until(stage_of(room_id)).await;
    assert_eq!(
        (
            updated.audience_count,
            updated.audience_budget,
            updated.audience_availability
        ),
        (0, 1, api::StageAudienceAvailability::Open)
    );
    join(&mut jason, room_id).await;
    assert_eq!(
        sync.until(stage_of(room_id)).await.audience_availability,
        api::StageAudienceAvailability::Full
    );
    assert_eq!(
        jason
            .write(empty(
                Method::POST,
                &format!("/api/v1/rooms/{room_id}/huddle/leave")
            ))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        sync.until(stage_of(room_id)).await.audience_availability,
        api::StageAudienceAvailability::Open
    );
    server.abort();
}
