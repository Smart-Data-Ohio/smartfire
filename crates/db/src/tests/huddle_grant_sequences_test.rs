//! Real, multi-step Rails grant transitions, not mocked issuance/revocation.
use crate::models::huddle_grant::HuddleGrant;
use crate::models::room_delete::HuddleConfig;
use crate::tests::{TestDb, huddle_notices_test, huddle_revocation_test};
use crate::{Membership, StageRole, Timestamp};
use rusqlite::params;
use serde_json::{Value, json};

fn run_case(name: &str, test: &str) {
    if std::env::var("WS13B_GRANT_CHILD").as_deref() != Ok(test) {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!("tests::huddle_grant_sequences_test::{test}"),
                "--nocapture",
            ])
            .env("WS13B_GRANT_CHILD", test)
            .env("LIVEKIT_API_KEY", "ws13b-fixture-api-key")
            .env("LIVEKIT_API_SECRET", "ws13b-fixture-api-secret")
            .env_remove("LIVEKIT_INTERNAL_URL")
            .env_remove("LIVEKIT_URL")
            .env_remove("LIVEKIT_GATEWAY_SECRET")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let vectors: Value =
        serde_json::from_str(include_str!("huddle_grant_sequence_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 13);
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    let db = TestDb::new();
    db.clock
        .travel_to(Timestamp::from_second(vectors["now"].as_i64().unwrap()));
    let input = case["input"].clone();
    db.write(move |tx| {
        tx.conn().execute_batch("DELETE FROM huddle_cleanups; DELETE FROM huddle_grants; DELETE FROM sqlite_sequence WHERE name IN ('huddle_grants','huddle_cleanups');")?;
        for table in ["rooms", "memberships", "sessions"] {
            for row in input[table].as_array().unwrap() {
                huddle_notices_test::insert(tx, table, row)?;
            }
        }
        Ok(())
    });
    let mut identities = std::collections::BTreeMap::<i64, String>::new();
    for expected in case["results"].as_array().unwrap() {
        let operation = expected["operation"].clone();
        let actual = if operation["op"] == "travel" {
            db.travel(operation["seconds"].as_i64().unwrap());
            Ok(Value::Null)
        } else {
            db.try_write(move |tx| {
                let config = HuddleConfig {
                    api_secret: Some("ws13b-fixture-api-secret".into()),
                    admin_configured: false,
                };
                let id = operation["id"].as_i64().unwrap_or_default();
                match operation["op"].as_str().unwrap() {
                    "issue" => HuddleGrant::issue(
                        tx,
                        operation["session_id"].as_i64().unwrap(),
                        operation["membership_id"].as_i64().unwrap(),
                        operation["room_id"].as_i64().unwrap(),
                        &config,
                    )
                    .map(|g| json!(g.id)),
                    "seen" => {
                        tx.conn().execute(
                            "UPDATE huddle_grants SET last_seen_at=? WHERE id=?",
                            params![
                                tx.now().ago(jiff::SignedDuration::from_secs(
                                    operation["age"].as_i64().unwrap()
                                )),
                                id
                            ],
                        )?;
                        Ok(Value::Null)
                    }
                    "destroy_member" => Membership::find(tx.conn(), id)?
                        .destroy(tx)
                        .map(|_| Value::Null),
                    "delete_member" => {
                        tx.conn()
                            .execute("DELETE FROM memberships WHERE id=?", [id])?;
                        Ok(Value::Null)
                    }
                    // Restoring the association is setup for issue(), which must keep the
                    // captured old membership/token denied. Membership creation is WS8.
                    "restore_member" => {
                        huddle_notices_test::insert(tx, "memberships", &operation["row"])
                            .map(|_| Value::Null)
                    }
                    "role" => {
                        let role = match operation["role"].as_str().unwrap() {
                            "host" => StageRole::Host,
                            "speaker" => StageRole::Speaker,
                            "listener" => StageRole::Listener,
                            _ => panic!("unknown stage role"),
                        };
                        Membership::find(tx.conn(), id)?
                            .change_stage_role_with_config(tx, role, &config)
                            .map(|_| Value::Null)
                    }
                    "raw_role" => {
                        tx.conn().execute(
                            "UPDATE memberships SET stage_role=? WHERE id=?",
                            params![operation["role"].as_str(), id],
                        )?;
                        Ok(Value::Null)
                    }
                    "authorize" => HuddleGrant::authorize_or_revoke(tx, id, &config)
                        .map(|g| json!(g.is_some())),
                    "revoke" => HuddleGrant::find_by_id(tx.conn(), id)?
                        .unwrap()
                        .revoke(tx, true, &config)
                        .map(|_| Value::Null),
                    "participants" => HuddleGrant::participants_for(tx.conn(), 9001, tx.now())
                        .map(|users| json!(users.iter().map(|u| u.id).collect::<Vec<_>>())),
                    _ => panic!("unknown sequence operation"),
                }
            })
        };
        match actual {
            Ok(value) => {
                assert!(expected["error"].is_null(), "{name} {expected}");
                assert_eq!(value, expected["value"], "{name} operation result");
            }
            Err(error) => {
                assert_eq!(error.to_string(), "HuddleGrant::Ineligible", "{name}");
                assert_eq!(expected["error"], "HuddleGrant::Ineligible");
            }
        }
        let mut actual = db.read(huddle_revocation_test::snapshot);
        for row in actual["grants"].as_array_mut().unwrap() {
            let id = row["id"].as_i64().unwrap();
            let grant = db.read(|conn| Ok(HuddleGrant::find_by_id(conn, id)?.unwrap()));
            row["authorized"] = json!(db.read(|conn| grant.authorized(conn)));
            row["in_call"] = json!(grant.in_call(db.now()));
            if let Some(identity) = identities.get(&id) {
                assert_eq!(&grant.identity, identity, "{name} reused identity changed");
            } else {
                assert!(
                    !identities
                        .values()
                        .any(|identity| *identity == grant.identity),
                    "{name} resurrected identity"
                );
                assert!(grant.identity.starts_with("campfire-participant-"));
                let random = &grant.identity["campfire-participant-".len()..];
                assert_eq!(random.len(), 64);
                assert!(
                    random
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                );
                identities.insert(id, grant.identity);
            }
        }
        let mut expected = json!({"grants": expected["grants"], "cleanups": expected["cleanups"]});
        huddle_revocation_test::normalize(&mut actual);
        huddle_revocation_test::normalize(&mut expected);
        assert_eq!(actual, expected, "{name} intermediate rows");
    }
}

macro_rules! grant_case {
    ($test:ident, $case:literal) => {
        #[test]
        fn $test() {
            run_case($case, stringify!($test));
        }
    };
}
grant_case!(active_session_reuses_random_authorized_grant, "reuse");
grant_case!(
    restoring_membership_never_resurrects_old_identity,
    "restore_membership"
);
grant_case!(
    issuance_rejects_stale_and_cross_user_memberships,
    "ineligible"
);
grant_case!(create_and_reuse_stamp_last_issued_at, "stamp");
grant_case!(
    room_switch_revokes_only_the_sessions_live_grant,
    "switch_room"
);
grant_case!(same_room_rejoin_keeps_live_grant, "same_room");
grant_case!(
    participants_deduplicate_sort_and_drop_revoked_quiet_grants,
    "participants"
);
grant_case!(
    stage_issuance_records_host_and_listener_roles,
    "stage_roles"
);
grant_case!(nonstage_issuance_has_no_role, "nonstage_role");
grant_case!(demotion_revokes_only_target_and_persists_cleanup, "demote");
grant_case!(
    host_speaker_transitions_keep_identity_and_authorization,
    "host_speaker"
);
grant_case!(
    gateway_check_revokes_missed_role_mismatch,
    "authorize_mismatch"
);
grant_case!(role_rejoin_creates_new_authorized_identity, "rejoin_role");
