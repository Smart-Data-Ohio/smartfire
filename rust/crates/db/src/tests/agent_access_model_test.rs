use super::*;
use crate::{
    AgentCredential, AgentGrant, Error, Membership, NewCredential, NewGrant, Status, User,
    UserChanges,
};
use serde_json::{Value, json};

fn grant() -> NewGrant {
    NewGrant {
        agent_id: id("bender_agent"),
        granted_by_id: id("david"),
        capability: "post_messages".into(),
        ..Default::default()
    }
}
fn credential() -> NewCredential {
    NewCredential {
        agent_id: id("bender_agent"),
        created_by_id: id("david"),
        name: "Test".into(),
        token_digest: "ws11-public-digest".into(),
        token_last_four: "disp".into(),
        ..Default::default()
    }
}
fn errors_hash(errors: crate::Errors) -> Value {
    let mut map = serde_json::Map::new();
    for (key, error) in errors.0 {
        map.entry(key.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(json!(error));
    }
    Value::Object(map)
}

#[test]
fn ws11_grant_and_credential_validation_match_rails_vectors() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        let vectors:Value=serde_json::from_str(include_str!("../../../../vectors/agents_grant_credential_contract.json")).unwrap();
        tx.conn().execute("DELETE FROM agent_grants",[])?;
        for (key,a) in [("blank",NewGrant {capability:"".into(),..grant()}),("unknown",NewGrant {capability:"launch_missiles".into(),..grant()}),("dm_room",NewGrant {capability:"dm_anyone".into(),room_id:Some(id("watercooler")),..grant()}),("missing_room",NewGrant {room_id:Some(0),..grant()}),("missing_nonzero_room",NewGrant {room_id:Some(-1),..grant()}),("missing_agent",NewGrant {agent_id:0,..grant()}),("missing_granter",NewGrant {granted_by_id:0,..grant()}),("dm_workspace",NewGrant {capability:"dm_anyone".into(),..grant()})] {assert_eq!(errors_hash(AgentGrant::validate(tx.conn(),&a,None)?),vectors["grants"][key],"{key}");}
        let mut existing=AgentGrant::create(tx,NewGrant {room_id:Some(id("watercooler")),..grant()})?;
        assert_eq!(errors_hash(AgentGrant::validate(tx.conn(),&NewGrant {room_id:Some(id("watercooler")),..grant()},None)?),vectors["grants"]["duplicate"]);
        assert_eq!(errors_hash(AgentGrant::validate(tx.conn(),&grant(),None)?),vectors["grants"]["other_scope"]);
        existing.revoke(tx)?;let stamp=existing.revoked_at;existing.revoke(tx)?;assert_eq!(existing.revoked_at,stamp);
        assert_eq!(errors_hash(AgentGrant::validate(tx.conn(),&NewGrant {room_id:Some(id("watercooler")),..grant()},None)?),vectors["grants"]["regrant"]);
        for capability in crate::models::agent_access::CAPABILITIES {assert!(AgentGrant::validate(tx.conn(),&NewGrant {capability:capability.into(),..grant()},None)?.is_empty());}
        for (key,a) in [("blank_name",NewCredential {name:"".into(),..credential()}),("blank_digest",NewCredential {token_digest:"".into(),..credential()}),("blank_display",NewCredential {token_last_four:"".into(),..credential()}),("missing_agent",NewCredential {agent_id:0,..credential()}),("missing_creator",NewCredential {created_by_id:0,..credential()})] {assert_eq!(errors_hash(AgentCredential::validate(tx.conn(),&a,None)?),vectors["credentials"][key],"{key}");}
        AgentCredential::create(tx,credential())?;
        assert_eq!(errors_hash(AgentCredential::validate(tx.conn(),&credential(),None)?),vectors["credentials"]["duplicate"]);
        let (generated,secret)=AgentCredential::create_with_secret(tx,id("bender_agent"),"Generated",id("david"),None)?;
        assert_eq!(json!({"secret_length":secret.len(),"digest_matches":generated.token_digest==crate::user::digest_bot_token(&secret),"display_from_digest":generated.token_last_four==generated.token_digest[..4],"plaintext_stored":generated.token_digest==secret || generated.token_last_four==secret}),vectors["credentials"]["generated"]);
        Ok(())
    });
}
#[test]
fn ws11_credential_revoke_expiry_and_usage_stamps_are_fresh() {
    let t = super::channel_thread_test::frozen();
    let (mut credential, secret) = t.write(|tx| {
        AgentCredential::create_with_secret(tx, id("bender_agent"), "Runner", id("david"), None)
    });
    let input = format!("  {secret}  ");
    assert_eq!(
        t.read(|c| AgentCredential::authenticate(c, &input, t.now()))
            .unwrap()
            .id,
        credential.id
    );
    let id = credential.id;
    t.write(move |tx| {
        credential.record_use(tx, Some("203.0.113.7"))?;
        let stamp = credential.last_used_at;
        credential.record_use(tx, Some("198.51.100.9"))?;
        assert_eq!(credential.last_used_at, stamp);
        assert_eq!(credential.last_used_ip.as_deref(), Some("203.0.113.7"));
        Ok(())
    });
    t.clock.travel(jiff::SignedDuration::from_mins(1));
    t.write(move |tx| {
        let mut c = AgentCredential::find(tx.conn(), id)?.unwrap();
        c.record_use(tx, Some("198.51.100.9"))?;
        assert_eq!(c.last_used_ip.as_deref(), Some("198.51.100.9"));
        c.revoke(tx)?;
        let stamp = c.revoked_at;
        c.revoke(tx)?;
        assert_eq!(c.revoked_at, stamp);
        Ok(())
    });
    assert!(
        t.read(|c| AgentCredential::authenticate(c, &secret, t.now()))
            .is_none()
    );
    let secret = secret.clone();
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE agent_credentials SET revoked_at=NULL,expires_at=? WHERE id=?",
            rusqlite::params![tx.now(), id],
        )?;
        assert!(AgentCredential::authenticate(tx.conn(), &secret, tx.now())?.is_none());
        Ok(())
    });
}
#[test]
fn ws11_membership_removal_revokes_room_grants_atomically() {
    let t = super::channel_thread_test::frozen();
    let (room_grant, workspace) = t.write(|tx| {
        Ok((
            AgentGrant::create(
                tx,
                NewGrant {
                    room_id: Some(id("watercooler")),
                    ..grant()
                },
            )?,
            AgentGrant::create(
                tx,
                NewGrant {
                    capability: "react".into(),
                    ..grant()
                },
            )?,
        ))
    });
    let room_id = room_grant.id;
    let workspace_id = workspace.id;
    let rolled = t.try_write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), id("watercooler"), id("bender"))?
            .unwrap()
            .destroy(tx)?;
        assert!(
            AgentGrant::find(tx.conn(), room_id)?
                .unwrap()
                .revoked_at
                .is_some()
        );
        assert!(
            AgentGrant::find(tx.conn(), workspace_id)?
                .unwrap()
                .revoked_at
                .is_none()
        );
        Err::<(), _>(Error::Other("WS11 rollback".into()))
    });
    assert!(rolled.is_err());
    assert!(
        t.read(|c| AgentGrant::find(c, room_id))
            .unwrap()
            .revoked_at
            .is_none()
    );
    t.write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), id("watercooler"), id("bender"))?
            .unwrap()
            .destroy(tx)
    });
    assert!(
        t.read(|c| AgentGrant::find(c, room_id))
            .unwrap()
            .revoked_at
            .is_some()
    );
}
#[test]
fn ws11_inactive_user_status_revokes_grants_in_the_same_write() {
    let t = super::channel_thread_test::frozen();
    let grant = t.write(|tx| AgentGrant::create(tx, grant()));
    t.write(move |tx| {
        let mut user = User::find(tx.conn(), id("bender"))?;
        user.update(
            tx,
            UserChanges {
                status: Some(Status::Banned),
                ..Default::default()
            },
        )?;
        assert!(
            AgentGrant::find(tx.conn(), grant.id)?
                .unwrap()
                .revoked_at
                .is_some()
        );
        Ok(())
    });
}
