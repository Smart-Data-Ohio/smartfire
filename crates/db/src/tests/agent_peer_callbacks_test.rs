use super::*;
use crate::callbacks::{Callback, Phase, Registry};
use crate::{Agent, EventSink, Message, MessageChanges, NewMessage, Session, User};
use std::sync::Mutex;
struct Sink {
    registry: Arc<Registry>,
    events: RecordingSink,
}
impl EventSink for Sink {
    fn emit(&self, event: Event) {
        self.events.emit(event);
    }
    fn model_callback(&self, tx: &mut Tx<'_>, callback: Callback) -> Result<()> {
        self.registry.call(tx, callback)
    }
}
fn adapter(t: &TestDb, registry: Arc<Registry>) -> Database {
    let mut env = t.db.env().clone();
    env.sink = Arc::new(Sink {
        registry,
        events: t.sink.clone(),
    });
    let mut config = Config::new(t.db.path());
    config.prepare = false;
    Database::open(config, env).unwrap()
}
fn phases(registry: &Registry, phases: &[Phase]) -> Arc<Mutex<Vec<(Phase, i64)>>> {
    let calls = Arc::new(Mutex::new(vec![]));
    for &phase in phases {
        let calls = calls.clone();
        registry.install(phase, move |_, call| {
            calls.lock().unwrap().push((call.phase, call.record_id));
            Ok(())
        });
    }
    calls
}
const MESSAGE_PHASES: &[Phase] = &[
    Phase::MessageActivity,
    Phase::MessageGithubReferences,
    Phase::MessageFizzyReferences,
    Phase::MessageTwitterReferences,
    Phase::MessageEventReferences,
    Phase::MessageLinkReferences,
];
#[test]
fn ws11_peer_create_edit_and_finalize_invoke_the_installed_adapters_in_order() {
    let t = super::channel_thread_test::frozen();
    let registry = Arc::new(Registry::default());
    let calls = phases(&registry, MESSAGE_PHASES);
    let db = adapter(&t, registry);
    let plain = db
        .write_blocking(|tx| {
            Ok(Message::create(
                tx,
                NewMessage {
                    room_id: id("watercooler"),
                    creator_id: id("david"),
                    markdown_source: Some("Create".into()),
                    ..Default::default()
                },
            )?
            .id)
        })
        .unwrap();
    assert_eq!(
        *calls.lock().unwrap(),
        MESSAGE_PHASES
            .iter()
            .map(|p| (*p, plain))
            .collect::<Vec<_>>()
    );
    calls.lock().unwrap().clear();
    db.write_blocking(move |tx| {
        Message::find(tx.conn(), plain)?.edit(
            tx,
            MessageChanges {
                markdown_source: Some("Edit".into()),
                ..Default::default()
            },
        )
    })
    .unwrap();
    assert_eq!(
        *calls.lock().unwrap(),
        MESSAGE_PHASES[1..]
            .iter()
            .map(|p| (*p, plain))
            .collect::<Vec<_>>()
    );
    calls.lock().unwrap().clear();
    let stream = db
        .write_blocking(|tx| {
            tx.conn().execute(
                "UPDATE agents SET suspended_at=NULL WHERE user_id=?",
                [id("bender")],
            )?;
            Ok(Message::create(
                tx,
                NewMessage {
                    room_id: id("watercooler"),
                    creator_id: id("bender"),
                    streaming: true,
                    markdown_source: Some("Draft".into()),
                    ..Default::default()
                },
            )?
            .id)
        })
        .unwrap();
    assert!(calls.lock().unwrap().is_empty());
    db.write_blocking(move |tx| {
        let mut m = Message::find(tx.conn(), stream)?;
        assert!(m.finalize_stream(tx)?);
        assert!(!m.finalize_stream(tx)?);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        *calls.lock().unwrap(),
        MESSAGE_PHASES
            .iter()
            .map(|p| (*p, stream))
            .collect::<Vec<_>>()
    );
}
#[test]
fn ws11_peer_callback_failure_rolls_back_creation_and_emits_nothing() {
    let t = super::channel_thread_test::frozen();
    t.sink.take();
    let registry = Arc::new(Registry::default());
    registry.install(Phase::MessageGithubReferences, |_, _| {
        Err(crate::Error::Other("peer callback rejected".into()))
    });
    let db = adapter(&t, registry);
    assert!(
        db.write_blocking(|tx| Message::create(
            tx,
            NewMessage {
                room_id: id("watercooler"),
                creator_id: id("david"),
                client_message_id: Some("ws11-peer-rejected".into()),
                markdown_source: Some("Rejected".into()),
                ..Default::default()
            }
        ))
        .is_err()
    );
    assert!(t.sink.events().is_empty());
    t.read(|c| {
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM messages WHERE client_message_id='ws11-peer-rejected'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            0
        );
        Ok(())
    });
}
#[test]
fn ws11_peer_removal_invokes_prepended_and_declared_dependencies_before_deleting_user() {
    let t = super::channel_thread_test::frozen();
    let registry = Arc::new(Registry::default());
    let ordered = &[
        Phase::UserHuddles,
        Phase::UserGoogleAccount,
        Phase::UserCalendarMeetingCache,
        Phase::UserGoogleIdentity,
        Phase::UserGithubAccount,
        Phase::UserFizzyAccount,
        Phase::UserSlackConnection,
        Phase::UserSlackImports,
        Phase::UserEventCalendarEntries,
        Phase::SessionHuddles,
    ];
    let calls = phases(&registry, ordered);
    let db = adapter(&t, registry.clone());
    let (user, session) = db
        .write_blocking(|tx| {
            let u = User::create_email_bot(tx)?;
            let s = Session::start(tx, u.id, None, None)?;
            Ok((u.id, s.id))
        })
        .unwrap();
    registry.install(Phase::UserHuddles, {
        let calls = calls.clone();
        move |tx, c| {
            assert!(User::find_by_id(tx.conn(), c.record_id)?.is_some());
            assert!(Agent::for_user(tx.conn(), c.record_id)?.is_none());
            calls.lock().unwrap().push((c.phase, c.record_id));
            Ok(())
        }
    });
    db.write_blocking(move |tx| User::find(tx.conn(), user)?.destroy(tx))
        .unwrap();
    assert_eq!(
        *calls.lock().unwrap(),
        ordered
            .iter()
            .map(|p| (
                *p,
                if *p == Phase::SessionHuddles {
                    session
                } else {
                    user
                }
            ))
            .collect::<Vec<_>>()
    );
}
#[test]
fn ws11_peer_missing_dependency_and_failed_dependency_preserve_the_user() {
    let t = super::channel_thread_test::frozen();
    let registry = Arc::new(Registry::default());
    let db = adapter(&t, registry.clone());
    let uid = db
        .write_blocking(|tx| Ok(User::create_email_bot(tx)?.id))
        .unwrap();
    registry.install(Phase::UserGoogleAccount, |_, _| {
        Err(crate::Error::Other("peer dependency failed".into()))
    });
    assert!(
        db.write_blocking(move |tx| User::find(tx.conn(), uid)?.destroy(tx))
            .is_err()
    );
    assert!(t.read(move |c| User::find_by_id(c, uid)).is_some());
}

#[test]
fn ws11_finalization_callback_failure_preserves_the_rails_claim_and_earlier_effects() {
    let t = super::channel_thread_test::frozen();
    let registry = Arc::new(Registry::default());
    let db = adapter(&t, registry.clone());
    let mid = db
        .write_blocking(|tx| {
            Agent::find(tx.conn(), id("bender_agent"))?
                .unwrap()
                .set_working_presence(tx, Some("Finishing"))?;
            Ok(Message::create(
                tx,
                NewMessage {
                    room_id: id("watercooler"),
                    creator_id: id("bender"),
                    markdown_source: Some("Before activity failure".into()),
                    streaming: true,
                    ..Default::default()
                },
            )?
            .id)
        })
        .unwrap();
    registry.install(Phase::MessageActivity, |_, _| {
        Err(crate::Error::Other("WS12 adapter rejected activity".into()))
    });
    let error = db
        .write_blocking(move |tx| Message::find(tx.conn(), mid)?.finalize_stream(tx))
        .unwrap_err()
        .to_string();
    let repeated = db
        .write_blocking(move |tx| Message::find(tx.conn(), mid)?.finalize_stream(tx))
        .unwrap();
    t.read(move|c|{
        let results=serde_json::json!({"error":error,"streaming":Message::find(c,mid)?.streaming,"indexed":c.query_row("SELECT COUNT(*) FROM message_search_index WHERE rowid=?",[mid],|r|r.get::<_,i64>(0))?,"posted":c.query_row("SELECT COUNT(*) FROM agent_events WHERE message_id=? AND event_type='posted'",[mid],|r|r.get::<_,i64>(0))?,"presence":Agent::find(c,id("bender_agent"))?.unwrap().working_presence,"repeated":repeated});
        let gold:serde_json::Value=serde_json::from_str(include_str!("../../../../vectors/agents_finalization_failure_contract.json")).unwrap();assert_eq!(results,gold["results"]);Ok(())
    });
}
#[test]
fn ws11_removal_uninstalled_peer_fk_and_rescued_error_preserve_rows_and_callbacks() {
    let t = super::channel_thread_test::frozen();
    let uid=t.write(|tx|{let u=User::create_email_bot(tx)?;tx.conn().execute("INSERT INTO github_connected_accounts(user_id,github_login,access_token,created_at,updated_at) VALUES(?,'ws11-removal','public fake material',?,?)",rusqlite::params![u.id,tx.now(),tx.now()])?;Ok(u.id)});
    t.sink.take();
    t.write(move |tx| {
        assert!(User::find(tx.conn(), uid)?.destroy(tx).is_err());
        assert!(User::find_by_id(tx.conn(), uid)?.is_some());
        assert_eq!(
            tx.conn().query_row(
                "SELECT COUNT(*) FROM github_connected_accounts WHERE user_id=?",
                [uid],
                |r| r.get::<_, i64>(0)
            )?,
            1
        );
        Ok(())
    });
    assert!(t.sink.events().is_empty());
}
