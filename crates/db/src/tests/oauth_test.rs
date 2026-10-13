use super::*;
use crate::Timestamp;
use crate::models::oauth::{self, OAuthConnection, OAuthToken, RefreshUse, TokenKind};
use jiff::SignedDuration;
use rusqlite::params;

fn setup() -> (TestDb, OAuthConnection) {
    let t = super::channel_thread_test::frozen();
    let connection = t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?", [id("bender_agent")])?;
        tx.conn().execute("UPDATE agents SET kind='personal',owner_id=? WHERE id=?", params![id("david"),id("bender_agent")])?;
        let client: i64 = tx.conn().query_row("INSERT INTO oauth_clients(client_id,registration,client_name,redirect_uris,created_at,updated_at) VALUES('https://client.example/metadata','cimd','Test client','[]',?,?) RETURNING id", params![tx.now(),tx.now()], |r| r.get(0))?;
        OAuthConnection::create(tx, id("bender_agent"), id("david"), client)
    });
    (t, connection)
}

fn token(
    tx: &Tx<'_>,
    connection: i64,
    secret: &str,
    kind: &str,
    expires_at: Timestamp,
    used_at: Option<Timestamp>,
) -> Result<()> {
    tx.conn().execute("INSERT INTO oauth_tokens(oauth_connection_id,kind,token_digest,expires_at,used_at,created_at) VALUES(?,?,?,?,?,?)", params![connection,kind,crate::user::digest_bot_token(secret),expires_at,used_at,tx.now()])?;
    Ok(())
}

fn token_count(t: &TestDb) -> i64 {
    t.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM oauth_tokens", [], |r| r.get(0))?))
}

#[test]
fn oauth_identity_authentication_and_write_rechecks_share_resource_and_personal_policy() {
    let (t, connection) = setup();
    t.write(move |tx| {
        use crate::models::agent_access::{authenticate_identity, verify_identity};
        token(
            tx,
            connection.id,
            "access",
            "access",
            connection.token_expires_at(tx.now(), TokenKind::Access),
            None,
        )?;
        let identity = authenticate_identity(tx, "access", "127.0.0.1")?.unwrap();
        assert!(identity.oauth);
        assert_eq!(identity.agent_id, id("bender_agent"));
        assert_eq!(identity.credential_id, connection.agent_credential_id);
        assert!(
            verify_identity(tx.conn(), "access", tx.now(), &tx.env().default_url_origin)?
                .unwrap()
                .oauth
        );
        assert!(
            verify_identity(tx.conn(), "access", tx.now(), "https://changed.example")?.is_none()
        );
        tx.conn()
            .execute("UPDATE users SET status=1 WHERE id=?", [id("david")])?;
        assert!(authenticate_identity(tx, "access", "127.0.0.1")?.is_none());
        assert!(
            verify_identity(tx.conn(), "access", tx.now(), &tx.env().default_url_origin)?.is_none()
        );
        Ok(())
    });
}

#[test]
fn oauth_connection_is_resource_bound_absolute_and_never_a_static_credential() {
    let (t, connection) = setup();
    assert_eq!(connection.resource, "http://example.com/agents/mcp");
    assert_eq!(
        connection.expires_at,
        t.now().since(oauth::CONNECTION_LIFETIME)
    );
    let near_end = connection.expires_at.ago(SignedDuration::from_mins(1));
    for kind in [TokenKind::Access, TokenKind::Refresh] {
        assert_eq!(
            connection.token_expires_at(near_end, kind),
            connection.expires_at
        );
    }
    t.read(|conn| {
        let credential =
            crate::AgentCredential::find(conn, connection.agent_credential_id)?.unwrap();
        assert!(credential.token_digest.starts_with("oauth:"));
        assert!(
            crate::AgentCredential::authenticate(conn, &credential.token_digest, t.now())?
                .is_none()
        );
        Ok(())
    });
}

#[test]
fn oauth_sweep_keeps_ninety_day_tombstones_and_replay_revokes_the_connection() {
    let (t, connection) = setup();
    let id = connection.id;
    t.write(move |tx| {
        token(tx,id,"old-refresh","refresh",tx.now().ago(SignedDuration::from_hours(90*24)),Some(tx.now().ago(SignedDuration::from_hours(120*24))))?;
        token(tx,id,"live-access","access",tx.now().since(oauth::ACCESS_LIFETIME),None)?;
        // An expired exchanged code remains a tombstone while its connection is live too.
        tx.conn().execute("INSERT INTO oauth_authorizations(public_id,oauth_client_id,user_id,redirect_uri,code_challenge,scope,resource,status,code_digest,oauth_connection_id,expires_at,created_at,updated_at) SELECT 'old-code',oauth_client_id,?,'https://client.example/callback','challenge',scope,resource,'exchanged','code-digest',id,?,?,? FROM oauth_connections WHERE id=?", params![super::id("david"),tx.now().ago(SignedDuration::from_hours(90*24)),tx.now(),tx.now(),id])?;
        oauth::sweep(tx)
    });
    assert_eq!(token_count(&t), 2);
    assert_eq!(
        t.read(|conn| Ok(conn.query_row(
            "SELECT COUNT(*) FROM oauth_authorizations",
            [],
            |r| r.get::<_, i64>(0)
        )?)),
        1
    );
    t.write(move |tx| {
        assert_eq!(
            OAuthToken::consume_refresh(tx, "old-refresh", id)?,
            RefreshUse::Reused
        );
        assert!(
            OAuthToken::authenticate(
                tx.conn(),
                "live-access",
                tx.now(),
                &tx.env().default_url_origin
            )?
            .is_none()
        );
        oauth::sweep(tx)
    });
    assert_eq!(
        token_count(&t),
        2,
        "one unexpired token keeps the entire family"
    );
    t.clock
        .travel_to(t.now().since(SignedDuration::from_hours(2)));
    t.write(|tx| oauth::sweep(tx));
    assert_eq!(token_count(&t), 0);
    assert_eq!(
        t.read(|conn| Ok(conn.query_row(
            "SELECT COUNT(*) FROM oauth_authorizations",
            [],
            |r| r.get::<_, i64>(0)
        )?)),
        0
    );
    t.write(move |tx| {
        assert_eq!(
            OAuthToken::consume_refresh(tx, "old-refresh", id)?,
            RefreshUse::Invalid
        );
        Ok(())
    });
}

#[test]
fn oauth_sweep_keeps_live_expired_tokens_and_cleans_only_fully_expired_dead_families() {
    for revoke in [false, true] {
        let (t, connection) = setup();
        let id = connection.id;
        t.write(move |tx| {
            token(tx, id, "expired", "refresh", tx.now(), Some(tx.now()))?;
            oauth::sweep(tx)
        });
        assert_eq!(token_count(&t), 1);
        t.write(move |tx| {
            if revoke {
                connection.revoke(tx)?;
            } else {
                tx.conn().execute(
                    "UPDATE oauth_connections SET expires_at=? WHERE id=?",
                    params![tx.now(), id],
                )?;
            }
            oauth::sweep(tx)
        });
        assert_eq!(token_count(&t), 0);
    }
}

#[test]
fn oauth_refresh_consumption_is_single_use_and_preserves_wrong_connection() {
    let (t, connection) = setup();
    let id = connection.id;
    t.write(move |tx| {
        token(
            tx,
            id,
            "refresh",
            "refresh",
            tx.now().since(oauth::REFRESH_LIFETIME),
            None,
        )?;
        assert_eq!(
            OAuthToken::consume_refresh(tx, "refresh", id + 1)?,
            RefreshUse::Invalid
        );
        assert!(
            OAuthToken::find_by_secret(tx.conn(), "refresh")?
                .unwrap()
                .used_at
                .is_none()
        );
        assert_eq!(
            OAuthToken::consume_refresh(tx, "refresh", id)?,
            RefreshUse::Consumed
        );
        assert_eq!(
            OAuthToken::consume_refresh(tx, "refresh", id)?,
            RefreshUse::Reused
        );
        assert!(
            !crate::AgentCredential::find(tx.conn(), connection.agent_credential_id)?
                .unwrap()
                .active(tx.now())
        );
        Ok(())
    });
}

#[test]
fn oauth_migration_of_default_seed_preserves_background_jobs() {
    let dir = tempfile::tempdir().unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/seeds/frozen/default/db/production.sqlite3");
    let file = dir.path().join("default.sqlite3");
    std::fs::copy(source, file.clone()).unwrap();
    let mut conn = Connection::open(file).unwrap();
    conn.execute_batch("DROP TABLE IF EXISTS oauth_authorizations; DROP TABLE IF EXISTS oauth_tokens; DROP TABLE IF EXISTS oauth_connections; DROP TABLE IF EXISTS oauth_clients; DELETE FROM schema_migrations WHERE version='20261013120000';").unwrap();
    fn jobs(conn: &Connection) -> Vec<Vec<rusqlite::types::Value>> {
        let mut statement = conn
            .prepare("SELECT * FROM background_jobs ORDER BY id")
            .unwrap();
        let columns = statement.column_count();
        statement
            .query_map([], |row| (0..columns).map(|i| row.get(i)).collect())
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }
    let before = jobs(&conn);
    assert_eq!(
        crate::migrations::migrate_preserving_existing_foreign_key_violations(&mut conn).unwrap(),
        ["20261013120000"]
    );
    assert_eq!(jobs(&conn), before);
}

#[test]
fn oauth_deleting_credential_or_agent_cascades_with_foreign_keys_enforced() {
    for destroy_agent in [false, true] {
        let (t, connection) = setup();
        t.write(move |tx| {
            assert!(tx.conn().query_row("PRAGMA foreign_keys",[],|r|r.get::<_,bool>(0))?);
            token(tx,connection.id,"access","access",connection.expires_at,None)?;
            tx.conn().execute("INSERT INTO oauth_authorizations(public_id,oauth_client_id,user_id,redirect_uri,code_challenge,scope,resource,oauth_connection_id,expires_at,created_at,updated_at) VALUES('approval',?,?,'https://client.example/callback','challenge','agent',?,?,?,?,?)",params![connection.oauth_client_id,id("david"),connection.resource,connection.id,connection.expires_at,tx.now(),tx.now()])?;
            if destroy_agent {
                crate::Agent::find(tx.conn(),id("bender_agent"))?.unwrap().destroy(tx)?;
            } else {
                crate::AgentCredential::find(tx.conn(),connection.agent_credential_id)?.unwrap().destroy(tx)?;
            }
            for table in ["oauth_connections","oauth_tokens","oauth_authorizations"] {
                assert_eq!(tx.conn().query_row(&format!("SELECT COUNT(*) FROM {table}"),[],|r|r.get::<_,i64>(0))?,0);
            }
            Ok(())
        });
    }
}

#[test]
fn oauth_deleting_human_revokes_owned_connections_and_cascades_authorizations() {
    let (t, connection) = setup();
    t.write(move |tx| {
        assert!(tx.conn().query_row("PRAGMA foreign_keys",[],|r|r.get::<_,bool>(0))?);
        let owner = crate::User::create(tx,crate::NewUser { name: "OAuth owner".into(),..Default::default() })?;
        tx.conn().execute("UPDATE agents SET owner_id=? WHERE id=?",params![owner.id,id("bender_agent")])?;
        tx.conn().execute("UPDATE agent_credentials SET created_by_id=? WHERE id=?",params![owner.id,connection.agent_credential_id])?;
        token(tx,connection.id,"access","access",connection.expires_at,None)?;
        tx.conn().execute("INSERT INTO oauth_authorizations(public_id,oauth_client_id,user_id,redirect_uri,code_challenge,scope,resource,expires_at,created_at,updated_at) VALUES('pending',?,?,'https://client.example/callback','challenge','agent',?,?,?,?)",params![connection.oauth_client_id,owner.id,connection.resource,connection.expires_at,tx.now(),tx.now()])?;
        owner.destroy_with_dependencies(tx,|_,_,_|Ok(()))?;
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM oauth_authorizations",[],|r|r.get::<_,i64>(0))?,0);
        assert!(crate::AgentCredential::find(tx.conn(),connection.agent_credential_id)?.unwrap().revoked_at.is_some());
        Ok(())
    });
}

#[test]
fn oauth_bot_destruction_preserves_legacy_credentials_until_orphan_connection_sweep() {
    let (t, connection) = setup();
    t.write(move |tx| {
        token(
            tx,
            connection.id,
            "access",
            "access",
            connection.expires_at,
            None,
        )?;
        crate::User::find(tx.conn(), id("bender"))?
            .destroy_with_dependencies(tx, |_, _, _| Ok(()))?;
        assert!(crate::AgentCredential::find(tx.conn(), connection.agent_credential_id)?.is_some());
        oauth::sweep(tx)?;
        assert!(OAuthConnection::find(tx.conn(), connection.id)?.is_none());
        assert!(OAuthToken::find_by_secret(tx.conn(), "access")?.is_none());
        Ok(())
    });
}

#[test]
fn oauth_workspace_data_removal_clears_all_rows_under_enforced_foreign_keys() {
    let (t, connection) = setup();
    t.write(move |tx| {
        token(tx,connection.id,"access","access",connection.expires_at,None)?;
        let tables: Vec<String> = crate::sql::query_all(tx.conn(),"SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'message_search_index_%' AND name NOT IN ('schema_migrations','ar_internal_metadata')",[],|r|r.get(0))?;
        tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
        for table in tables { tx.conn().execute(&format!("DELETE FROM \"{table}\""),[])?; }
        Ok(())
    });
    assert_eq!(token_count(&t), 0);
}

#[test]
fn oauth_dcr_sweep_removes_only_old_unused_clients_and_keeps_pending_consent() {
    let (t, connection) = setup();
    let connection_id = connection.id;
    t.write(move |tx| {
        let old = tx.now().ago(SignedDuration::from_hours(25));
        token(tx, connection_id, "dcr-tombstone", "refresh", old, Some(old))?;
        token(tx, connection_id, "dcr-access", "access", tx.now().since(SignedDuration::from_mins(5)), None)?;
        // A client with a connection survives, even after revocation and token-family cleanup.
        tx.conn().execute("UPDATE oauth_clients SET registration='dcr',created_at=? WHERE id=?", params![old,connection.oauth_client_id])?;
        for (id, kind, created) in [("old-unused","dcr",old),("fresh-unused","dcr",tx.now()),("boundary-unused","dcr",tx.now().ago(SignedDuration::from_hours(24))),("old-cimd","cimd",old),("pending-unused","dcr",old)] {
            tx.conn().execute("INSERT INTO oauth_clients(client_id,registration,client_name,redirect_uris,created_at,updated_at) VALUES(?,?,'Test client','[]',?,?)", params![id,kind,created,created])?;
        }
        tx.conn().execute("INSERT INTO oauth_authorizations(public_id,oauth_client_id,user_id,redirect_uri,code_challenge,scope,resource,expires_at,created_at,updated_at) SELECT 'pending-dcr',id,?,'https://client.example/callback','challenge','agent','http://example.com/agents/mcp',?,?,? FROM oauth_clients WHERE client_id='pending-unused'", params![super::id("david"),tx.now().since(SignedDuration::from_mins(10)),tx.now(),tx.now()])?;
        oauth::sweep(tx)
    });
    let clients = || {
        t.read(|conn| {
            let mut stmt =
                conn.prepare("SELECT client_id FROM oauth_clients ORDER BY client_id")?;
            Ok(stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
    };
    assert_eq!(
        clients(),
        [
            "fresh-unused",
            "https://client.example/metadata",
            "old-cimd",
            "pending-unused"
        ]
    );
    t.write(move |tx| {
        OAuthConnection::find(tx.conn(), connection_id)?
            .unwrap()
            .revoke(tx)?;
        oauth::sweep(tx)
    });
    assert_eq!(
        clients(),
        [
            "fresh-unused",
            "https://client.example/metadata",
            "old-cimd",
            "pending-unused"
        ]
    );
    assert_eq!(
        token_count(&t),
        2,
        "revoked family retains its tombstones until every token expires"
    );
    t.clock
        .travel_to(t.now().since(SignedDuration::from_mins(10)));
    t.write(|tx| oauth::sweep(tx));
    assert_eq!(
        clients(),
        [
            "fresh-unused",
            "https://client.example/metadata",
            "old-cimd"
        ]
    );
    assert_eq!(token_count(&t), 0);
    t.write(move |tx| {
        crate::AgentCredential::find(tx.conn(), connection.agent_credential_id)?
            .unwrap()
            .destroy(tx)?;
        oauth::sweep(tx)
    });
    assert_eq!(clients(), ["fresh-unused", "old-cimd"]);
    t.clock
        .travel_to(t.now().since(SignedDuration::from_hours(24)));
    t.write(|tx| oauth::sweep(tx));
    assert_eq!(clients(), ["old-cimd"]);
}
