//! `test/models/room/push_test.rb` (who gets pushed; delivery is the app's) and
//! `test/models/push/subscription_test.rb` (validation and endpoint pinning).

use super::*;
use crate::models::push_subscription::PushSubscription;
use crate::rich_text::mention_attachment_for;
use crate::{Membership, Message, NewMessage};

const WEB_PUSH_PUBLIC_TEST_IP: &str = "142.250.185.206";

const PHONE_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148 Safari/604.1";
const DESKTOP_UA: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/130.0.0.0 Safari/537.36";

fn routing_lease(t: &TestDb, device_class: crate::DeviceClass) -> crate::WorkspacePresenceLease {
    t.write(move |tx| {
        let session = crate::Session::start(tx, id("jason"), Some(DESKTOP_UA), None)?;
        Ok(crate::WorkspacePresenceLease::establish_for_device(
            tx,
            id("jason"),
            session.id,
            Some(device_class),
        )?
        .unwrap())
    })
}

fn routing_phone(t: &TestDb) {
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE push_subscriptions SET user_agent=?, device_class='mobile' WHERE user_id=?",
            rusqlite::params![PHONE_UA, id("jason")],
        )?;
        Ok(())
    });
}

#[test]
fn push_routing_null_subscription_keeps_pre_routing_delivery() {
    let t = TestDb::new();
    routing_phone(&t);
    t.write(|tx| {
        let session = crate::Session::start(tx, id("jason"), Some(DESKTOP_UA), None)?;
        crate::WorkspacePresenceLease::establish_for_device(
            tx,
            id("jason"),
            session.id,
            Some(crate::DeviceClass::Desktop),
        )?;
        tx.conn().execute(
            "UPDATE push_subscriptions SET device_class=NULL WHERE user_id=?",
            [id("jason")],
        )?;
        Ok(())
    });
    assert_eq!(deliveries(&t, "designers", "Legacy subscription".into()), 2);
}

#[test]
fn push_routing_null_desktop_ua_lease_does_not_suppress_mobile_subscription() {
    for agent in [DESKTOP_UA, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15)"] {
        let t = TestDb::new();
        let lease = t.write(move |tx| {
            tx.conn().execute(
                "UPDATE push_subscriptions SET device_class='mobile' WHERE user_id=?",
                [id("jason")],
            )?;
            let session = crate::Session::start(tx, id("jason"), Some(agent), None)?;
            Ok(crate::WorkspacePresenceLease::establish(tx, id("jason"), session.id)?.unwrap())
        });
        assert_eq!(lease.device_class, None);
        assert_eq!(
            deliveries(&t, "designers", "Legacy tablet in use".into()),
            2,
            "{agent}"
        );
        let activity = t.read(|conn| {
            crate::WorkspacePresenceLease::activity_by_user_id(conn, &[id("jason")], t.now())
        });
        assert_eq!(
            activity[&id("jason")].overall,
            crate::models::workspace_presence_lease::Presence::Online
        );
        assert!(!activity[&id("jason")].active_desktop);
    }
}

#[test]
fn push_routing_active_desktop_suppresses_phone_in_another_room_without_desktop_push() {
    let t = TestDb::new();
    routing_phone(&t);
    routing_lease(&t, crate::DeviceClass::Desktop);
    t.write(|tx| Membership::find(tx.conn(), id("jason_hq"))?.connected(tx));
    assert_eq!(deliveries(&t, "designers", "On the computer".into()), 1);
}

#[test]
fn push_routing_idle_desktop_allows_phone_even_with_an_active_phone_lease() {
    let t = TestDb::new();
    routing_phone(&t);
    let mut desktop = routing_lease(&t, crate::DeviceClass::Desktop);
    t.clock.travel(
        crate::models::workspace_presence_lease::IDLE_AFTER + jiff::SignedDuration::from_secs(1),
    );
    t.write(move |tx| desktop.refresh(tx, false));
    routing_lease(&t, crate::DeviceClass::Mobile);
    assert_eq!(deliveries(&t, "designers", "Away from computer".into()), 2);
}

#[test]
fn push_routing_expired_or_deleted_desktop_allows_phone_at_send_time() {
    for delete in [false, true] {
        let t = TestDb::new();
        routing_phone(&t);
        let mut desktop = routing_lease(&t, crate::DeviceClass::Desktop);
        if delete {
            t.write(move |tx| desktop.delete(tx));
        } else {
            t.clock.travel(
                crate::models::workspace_presence_lease::TTL + jiff::SignedDuration::from_micros(1),
            );
        }
        assert_eq!(deliveries(&t, "designers", "Desktop gone".into()), 2);
    }
}

#[test]
fn push_routing_active_desktop_preserves_desktop_subscriptions() {
    let t = TestDb::new();
    routing_lease(&t, crate::DeviceClass::Desktop);
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE push_subscriptions SET device_class='desktop' WHERE user_id=?",
            [id("jason")],
        )?;
        Ok(())
    });
    assert_eq!(
        deliveries(&t, "designers", "Desktop notification".into()),
        2
    );
}

#[test]
fn push_routing_one_active_desktop_wins_over_an_idle_desktop() {
    let t = TestDb::new();
    routing_phone(&t);
    let mut idle = routing_lease(&t, crate::DeviceClass::Desktop);
    t.clock.travel(
        crate::models::workspace_presence_lease::IDLE_AFTER + jiff::SignedDuration::from_secs(1),
    );
    t.write(move |tx| idle.refresh(tx, false));
    routing_lease(&t, crate::DeviceClass::Desktop);
    assert_eq!(deliveries(&t, "designers", "Another computer".into()), 1);
}

#[test]
fn push_routing_idle_threshold_is_inclusive_and_uses_send_time() {
    let t = TestDb::new();
    routing_phone(&t);
    let mut desktop = routing_lease(&t, crate::DeviceClass::Desktop);
    t.clock.travel_to(
        desktop
            .last_active_at
            .unwrap()
            .since(crate::models::workspace_presence_lease::IDLE_AFTER),
    );
    desktop = t.write(move |tx| {
        desktop.refresh(tx, false)?;
        Ok(desktop)
    });
    assert_eq!(deliveries(&t, "designers", "At threshold".into()), 1);
    t.clock.travel(jiff::SignedDuration::from_micros(1));
    t.write(move |tx| desktop.refresh(tx, false));
    assert_eq!(deliveries(&t, "designers", "Past threshold".into()), 2);
}

#[test]
fn push_routing_revoked_or_mismatched_desktop_session_does_not_suppress_phone() {
    for revoke in [true, false] {
        let t = TestDb::new();
        routing_phone(&t);
        let desktop = routing_lease(&t, crate::DeviceClass::Desktop);
        t.write(move |tx| {
            if revoke {
                tx.conn()
                    .execute("DELETE FROM sessions WHERE id=?", [desktop.session_id])?;
            } else {
                tx.conn().execute(
                    "UPDATE sessions SET user_id=? WHERE id=?",
                    rusqlite::params![id("david"), desktop.session_id],
                )?;
            }
            Ok(())
        });
        assert_eq!(deliveries(&t, "designers", "Session ended".into()), 2);
    }
}

#[test]
fn push_routing_unknown_device_identity_keeps_existing_delivery() {
    let t = TestDb::new();
    routing_phone(&t);
    routing_lease(&t, crate::DeviceClass::Unknown);
    assert_eq!(deliveries(&t, "designers", "Unknown computer".into()), 2);

    routing_lease(&t, crate::DeviceClass::Desktop);
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE push_subscriptions SET user_agent=NULL, device_class=NULL WHERE user_id=?",
            [id("jason")],
        )?;
        Ok(())
    });
    assert_eq!(
        deliveries(&t, "designers", "Unknown subscription".into()),
        2
    );
}

#[test]
fn push_routing_explicit_mobile_subscription_with_desktop_user_agent_waits_for_computer() {
    let t = TestDb::new();
    routing_lease(&t, crate::DeviceClass::Desktop);
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE push_subscriptions SET device_class='mobile' WHERE user_id=?",
            [id("jason")],
        )?;
        Ok(())
    });
    assert_eq!(deliveries(&t, "designers", "Tablet subscription".into()), 1);
}

fn desktop_site_reported_class() -> crate::DeviceClass {
    let reported =
        std::env::var("PUSH_ROUTING_DESKTOP_SITE_CLASS").unwrap_or_else(|_| "mobile".into());
    crate::DeviceClass::from_name(&reported)
        .expect("browser classifier must report a valid device class")
}

#[test]
fn push_routing_desktop_site_phone_is_suppressed_while_a_laptop_is_active() {
    let t = TestDb::new();
    let class = desktop_site_reported_class();
    routing_lease(&t, crate::DeviceClass::Desktop);
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE push_subscriptions SET user_agent=?, device_class=? WHERE user_id=?",
            rusqlite::params![DESKTOP_UA, class, id("jason")],
        )?;
        Ok(())
    });
    assert_eq!(deliveries(&t, "designers", "Laptop in use".into()), 1);
}

#[test]
fn push_routing_desktop_site_phone_or_tablet_cannot_suppress_another_phone() {
    for agent in [DESKTOP_UA, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15)"] {
        let t = TestDb::new();
        let class = desktop_site_reported_class();
        routing_phone(&t);
        t.write(move |tx| {
            let session = crate::Session::start(tx, id("jason"), Some(agent), None)?;
            crate::WorkspacePresenceLease::establish_for_device(
                tx,
                id("jason"),
                session.id,
                Some(class),
            )?;
            Ok(())
        });
        assert_eq!(
            deliveries(&t, "designers", "Only a mobile device in use".into()),
            2
        );
    }
}

#[test]
fn push_routing_mobile_or_unknown_lease_never_counts_as_a_computer() {
    for class in [crate::DeviceClass::Mobile, crate::DeviceClass::Unknown] {
        let t = TestDb::new();
        routing_phone(&t);
        let lease = t.write(move |tx| {
            let session = crate::Session::start(tx, id("jason"), Some(DESKTOP_UA), None)?;
            Ok(crate::WorkspacePresenceLease::establish_for_device(
                tx,
                id("jason"),
                session.id,
                Some(class),
            )?
            .unwrap())
        });
        let activity = t.read(|conn| {
            crate::WorkspacePresenceLease::activity_by_user_id(conn, &[id("jason")], t.now())
        });
        assert_eq!(
            activity[&id("jason")].overall,
            crate::models::workspace_presence_lease::Presence::Online
        );
        assert!(!activity[&id("jason")].active_desktop);
        assert_eq!(lease.device_class, Some(class));
        assert_eq!(deliveries(&t, "designers", "Using a tablet".into()), 2);
    }
}

#[test]
fn push_routing_explicit_unknown_subscription_keeps_existing_delivery() {
    let t = TestDb::new();
    routing_phone(&t);
    routing_lease(&t, crate::DeviceClass::Desktop);
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE push_subscriptions SET device_class='unknown' WHERE user_id=?",
            [id("jason")],
        )?;
        Ok(())
    });
    assert_eq!(deliveries(&t, "designers", "Uncertain device".into()), 2);
}

// WS17 regressions: these run through the actual room-recipient query, not a policy mock.
#[test]
fn ws17_room_push_respects_dnd_and_non_allowed_sender() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn()
            .execute("UPDATE users SET dnd_enabled=1 WHERE id=?", [id("jason")])?;
        Ok(())
    });
    assert_eq!(deliveries(&t, "designers", "Quiet please".into()), 1);
}

#[test]
fn ws17_room_push_respects_quiet_hours() {
    let t = TestDb::new();
    t.clock
        .travel_to(crate::Timestamp::parse_db("2026-03-08 07:15:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("UPDATE users SET quiet_hours_enabled=1, quiet_hours_start_minute=180, quiet_hours_end_minute=240, time_zone='America/New_York' WHERE id=?", [id("jason")])?;
        Ok(())
    });
    assert_eq!(deliveries(&t, "designers", "Quiet please".into()), 1);
}

#[test]
fn ws17_room_push_respects_out_of_office() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE users SET ooo_until=? WHERE id=?",
            rusqlite::params![
                tx.now().since(jiff::SignedDuration::from_hours(1)),
                id("jason")
            ],
        )?;
        Ok(())
    });
    assert_eq!(deliveries(&t, "designers", "Quiet please".into()), 1);
}

#[test]
fn ws17_replies_notify_only_opted_in_authors_and_merge_duplicate_scopes() {
    let t = TestDb::new();
    t.write(|tx| {
        Membership::find(tx.conn(), id("jason_designers"))?
            .update_involvement(tx, crate::Involvement::Mentions)
    });
    let source = t.write(|tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("jason"),
                body: Some("Original".into()),
                ..Default::default()
            },
        )
    });
    let send = |notify, body: String| {
        t.write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: id("david"),
                    body: Some(body),
                    reply_to_message_id: Some(source.id),
                    reply_notify_author: Some(notify),
                    ..Default::default()
                },
            )
        })
    };
    let recipients = |message: &Message| {
        t.read(|c| Ok(PushSubscription::pushes_for(c, &BasicRichText, message, t.now())?.1))
    };
    assert_eq!(recipients(&send(false, "Quiet reply".into())).len(), 1);
    assert_eq!(recipients(&send(true, "Reply".into())).len(), 2);
    let message = send(
        true,
        format!("Reply {}", mention_attachment_for(id("jason"))),
    );
    assert_eq!(recipients(&message).len(), 2);
    t.write(|tx| {
        Membership::find(tx.conn(), id("jason_designers"))?
            .update_involvement(tx, crate::Involvement::Everything)
    });
    assert_eq!(
        recipients(&message).len(),
        2,
        "everything + mention + reply must send once per device"
    );
    t.write(|tx| {
        Membership::find(tx.conn(), id("jason_designers"))?
            .update_involvement(tx, crate::Involvement::Muted)
    });
    assert_eq!(recipients(&send(true, "Muted reply".into())).len(), 1);
    assert_eq!(
        recipients(&message).len(),
        2,
        "muted rooms still push mentions"
    );
}

/// How many deliveries `Room::MessagePusher#push` would queue for a new message.
fn deliveries(t: &TestDb, room: &str, body: String) -> usize {
    let attributes = NewMessage {
        room_id: id(room),
        creator_id: id("david"),
        client_message_id: Some("earth".into()),
        body: Some(body),
        attachment_blob_id: None,
        ..Default::default()
    };
    let message = t.write(move |tx| Message::create(tx, attributes));
    let now = t.now();
    let (_, pushes) = t.read(|c| PushSubscription::pushes_for(c, &BasicRichText, &message, now));
    pushes.len()
}

#[test]
fn deliver_new_message_to_other_room_users_with_push_subscriptions() {
    let t = TestDb::new();
    let all = t.read(PushSubscription::count);
    let davids = t.read(|c| PushSubscription::for_user(c, id("david"))).len() as i64;
    assert_eq!(
        deliveries(&t, "hq", "This is from earth".into()) as i64,
        all - davids
    );
}

#[test]
fn notifies_subscribed_users() {
    let t = TestDb::new();
    assert_eq!(deliveries(&t, "designers", "This is from earth".into()), 2);
    assert_eq!(
        deliveries(
            &t,
            "designers",
            format!("Hey {}", mention_attachment_for(id("kevin")))
        ),
        3
    );
}

#[test]
fn does_not_notify_for_connected_rooms() {
    let t = TestDb::new();
    t.write(|tx| Membership::find(tx.conn(), id("kevin_designers"))?.connected(tx));
    assert_eq!(
        deliveries(
            &t,
            "designers",
            format!("Hey {}", mention_attachment_for(id("kevin")))
        ),
        2
    );
}

#[test]
fn does_not_notify_for_invisible_rooms() {
    let t = TestDb::new();
    t.write(|tx| {
        Membership::find(tx.conn(), id("kevin_designers"))?
            .update_involvement(tx, crate::Involvement::Invisible)
    });
    assert_eq!(
        deliveries(
            &t,
            "designers",
            format!("Hey {}", mention_attachment_for(id("kevin")))
        ),
        2
    );
}

#[test]
fn payloads() {
    let t = TestDb::new();
    let attributes = NewMessage {
        room_id: id("designers"),
        creator_id: id("david"),
        body: Some("Hi".into()),
        ..Default::default()
    };
    let message = t.write(move |tx| Message::create(tx, attributes));
    let room = t.read(|c| message.room(c));
    let payload = t.read(|c| PushSubscription::payload_for(c, &BasicRichText, &room, &message));
    assert_eq!(
        (payload.title.as_str(), payload.body.as_str()),
        ("Designers", "David: Hi")
    );
    assert_eq!(payload.path, format!("/rooms/{}", id("designers")));

    let attributes = NewMessage {
        room_id: id("david_and_jason"),
        creator_id: id("david"),
        body: Some("Hi".into()),
        ..Default::default()
    };
    let message = t.write(move |tx| Message::create(tx, attributes));
    let room = t.read(|c| message.room(c));
    let payload = t.read(|c| PushSubscription::payload_for(c, &BasicRichText, &room, &message));
    assert_eq!(
        (payload.title.as_str(), payload.body.as_str()),
        ("David", "Hi")
    );
}

#[test]
fn long_payloads_are_preserved_like_rails() {
    let t = TestDb::new();
    let body = format!("{}\"{}", "é".repeat(1000), "x".repeat(2000));
    let attributes = NewMessage {
        room_id: id("designers"),
        creator_id: id("david"),
        body: Some(body),
        ..Default::default()
    };
    let message = t.write(move |tx| Message::create(tx, attributes));
    let room = t.read(|c| message.room(c));
    let payload = t.read(|c| PushSubscription::payload_for(c, &BasicRichText, &room, &message));

    assert_eq!(
        payload.body,
        format!("David: {}\"{}", "é".repeat(1000), "x".repeat(2000))
    );
    assert_eq!(payload.tag, Some(format!("room-{}", room.id)));
}

fn build(endpoint: &str) -> PushSubscription {
    PushSubscription::new(
        id("david"),
        Some(endpoint),
        Some("test_key"),
        Some("test_auth"),
        None,
    )
}

fn public(_: &str) -> Option<String> {
    Some(WEB_PUSH_PUBLIC_TEST_IP.into())
}

fn private(_: &str) -> Option<String> {
    None
}

fn errors(subscription: &PushSubscription, resolve: fn(&str) -> Option<String>) -> Vec<String> {
    subscription
        .validate(&resolve)
        .on("endpoint")
        .into_iter()
        .map(String::from)
        .collect()
}

#[test]
fn valid_subscription_with_permitted_endpoint() {
    assert!(errors(&build("https://fcm.googleapis.com/fcm/send/abc123"), public).is_empty());
}

#[test]
fn rejects_endpoint_with_non_https_scheme() {
    assert!(
        errors(&build("http://fcm.googleapis.com/fcm/send/abc123"), public)
            .contains(&"must use HTTPS".into())
    );
}

#[test]
fn rejects_endpoint_with_non_permitted_host() {
    assert!(
        errors(&build("https://attacker.example.com/webhook"), public)
            .contains(&"is not a permitted push service".into())
    );
}

#[test]
fn rejects_endpoint_whose_host_only_suffix_matches_a_permitted_host() {
    assert!(
        errors(
            &build("https://evilfcm.googleapis.com.attacker.example/webhook"),
            public
        )
        .contains(&"is not a permitted push service".into())
    );
}

#[test]
fn rejects_blank_endpoint() {
    assert!(errors(&build(""), public).contains(&"can't be blank".into()));
}

#[test]
fn rejects_endpoint_on_a_non_default_port() {
    assert!(
        errors(
            &build("https://fcm.googleapis.com:8443/fcm/send/abc123"),
            public
        )
        .contains(&"must use the default HTTPS port".into())
    );
}

#[test]
fn rejects_endpoint_that_resolves_to_private_ip() {
    let subscription = build("https://fcm.googleapis.com/fcm/send/abc123");
    assert!(
        errors(&subscription, private)
            .contains(&"resolves to a private or invalid IP address".into())
    );
    assert_eq!(subscription.resolved_endpoint_ip(&private), None);
}

#[test]
fn resolved_endpoint_ip_returns_the_pinned_public_ip() {
    assert_eq!(
        build("https://fcm.googleapis.com/fcm/send/abc123")
            .resolved_endpoint_ip(&public)
            .as_deref(),
        Some(WEB_PUSH_PUBLIC_TEST_IP)
    );
}

#[test]
fn delivery_is_skipped_for_a_non_permitted_host_or_port() {
    assert_eq!(
        build("https://attacker.example.com/collect").resolved_endpoint_ip(&public),
        None
    );
    assert_eq!(
        build("https://fcm.googleapis.com:22/fcm/send/abc123").resolved_endpoint_ip(&public),
        None
    );
}

#[test]
fn accepts_all_permitted_push_service_domains() {
    for endpoint in [
        "https://fcm.googleapis.com/fcm/send/token123",
        "https://jmt17.google.com/fcm/send/token123",
        "https://updates.push.services.mozilla.com/wpush/v2/token123",
        "https://web.push.apple.com/QaBC123",
        "https://wns2-db5p.notify.windows.com/w/?token=abc123",
    ] {
        assert!(errors(&build(endpoint), public).is_empty(), "{endpoint}");
    }
}

#[test]
fn create_validates() {
    let t = TestDb::new();
    assert!(
        t.try_write(|tx| PushSubscription::create(
            tx,
            &build("http://fcm.googleapis.com/x"),
            &public
        ))
        .is_err()
    );
    let created =
        t.write(|tx| PushSubscription::create(tx, &build("https://fcm.googleapis.com/x"), &public));
    assert_eq!(
        t.read(|c| PushSubscription::find(c, created.id))
            .endpoint
            .as_deref(),
        Some("https://fcm.googleapis.com/x")
    );
}

#[test]
fn a_note_to_self_never_pushes_to_or_unreads_its_author() {
    let t = TestDb::new();
    let david = id("david");
    assert!(
        !t.read(|c| PushSubscription::for_user(c, david)).is_empty(),
        "David has devices"
    );
    let room = t.write(move |tx| crate::Room::find_or_create_direct_for(tx, &[david], david));
    for body in [
        "Buy milk".to_owned(),
        format!("Remind {}", mention_attachment_for(david)),
    ] {
        let attributes = NewMessage {
            room_id: room.id,
            creator_id: david,
            body: Some(body),
            ..Default::default()
        };
        let message = t.write(move |tx| Message::create(tx, attributes));
        let now = t.now();
        let (_, pushes) =
            t.read(|c| PushSubscription::pushes_for(c, &BasicRichText, &message, now));
        assert!(pushes.is_empty(), "{pushes:?}");
        let membership = t
            .read(|c| Membership::find_by_room_and_user(c, room.id, david))
            .expect("David is its member");
        assert_eq!(membership.unread_at, None);
    }
}
