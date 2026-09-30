//! Shared request-specific message payload adapter.
//! WS8b-m owns MessagePayloadHelper; WS11-api calls the stable Presenter method.
#![allow(dead_code)] // FLAGGED: WS8b-m/WS11-api install and call this adapter at merge.
use super::{Presenter, Result};
use campfire_db::{Message, User};
use serde_json::Value;
use std::sync::{Arc, RwLock};

pub trait MessagePayload: Send + Sync {
    fn message(
        &self,
        presenter: &Presenter<'_>,
        message: &Message,
        viewer: &User,
        base_url: &str,
    ) -> Result<Value>;
}
#[derive(Default)]
pub struct State {
    adapter: RwLock<Option<Arc<dyn MessagePayload>>>,
}
impl State {
    pub fn install(&self, adapter: Arc<dyn MessagePayload>) {
        *self.adapter.write().unwrap_or_else(|p| p.into_inner()) = Some(adapter);
    }
    fn message(&self, presenter: &Presenter<'_>, message: &Message) -> Result<Value> {
        let viewer = presenter.current_user_id.ok_or_else(|| {
            campfire_db::Error::Other("message payload requires Current.user".into())
        })?;
        let viewer = presenter.user(viewer)?;
        let base = presenter.cache_base_url.as_deref().ok_or_else(|| {
            campfire_db::Error::Other("message payload requires request base URL".into())
        })?;
        let adapter = self
            .adapter
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        // FLAGGED STUB: at merge WS8b-m installs an adapter calling
        // controllers::messages::payload::message(presenter, message, viewer, base).
        // Never substitute the cached stock JSON: it omits thread/reply permissions.
        adapter
            .ok_or_else(|| {
                campfire_db::Error::Other(
                    "WS8b-m MessagePayloadHelper adapter is not installed".into(),
                )
            })?
            .message(presenter, message, &viewer, base)
    }
}
impl Presenter<'_> {
    /// Matches WS11-api's seam. Its Current.user is the authenticated bot.
    pub fn agent_message_payload(&self, message: &Message) -> Result<Value> {
        self.agent_payload.message(self, message)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{ALL_TALK, BENDER, DAVID, TestApp};
    use super::*;
    use campfire_db::NewMessage;
    struct Adapter;
    impl MessagePayload for Adapter {
        fn message(
            &self,
            p: &Presenter<'_>,
            m: &Message,
            viewer: &User,
            base: &str,
        ) -> Result<Value> {
            assert!(p.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM messages WHERE id=?)",
                [m.id],
                |r| r.get::<_, bool>(0)
            )?);
            Ok(serde_json::json!({"message":m.id,"viewer":viewer.id,"base":base}))
        }
    }
    #[tokio::test]
    async fn ws11_production_presenter_requires_current_user_and_forwards_uncached_request_context()
    {
        let test = TestApp::boot().await.expect("default seed");
        let app = test.booted.app.clone();
        let mid = app
            .db
            .write(|tx| {
                Ok(Message::create(
                    tx,
                    NewMessage {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        markdown_source: Some("Payload".into()),
                        ..Default::default()
                    },
                )?
                .id)
            })
            .await
            .unwrap();
        let copy = app.clone();
        app.db
            .read(move |conn| {
                let mut p = Presenter::new(conn, &copy, None);
                let m = Message::find(conn, mid)?;
                assert!(
                    p.agent_message_payload(&m)
                        .unwrap_err()
                        .to_string()
                        .contains("Current.user")
                );
                p.current_user_id = Some(BENDER);
                p.cache_base_url = Some("https://first.test".into());
                assert!(
                    p.agent_message_payload(&m)
                        .unwrap_err()
                        .to_string()
                        .contains("not installed")
                );
                copy.agent_message_payload.install(Arc::new(Adapter));
                assert_eq!(
                    p.agent_message_payload(&m)?,
                    serde_json::json!({"message":mid,"viewer":BENDER,"base":"https://first.test"})
                );
                p.current_user_id = Some(DAVID);
                p.cache_base_url = Some("https://second.test".into());
                assert_eq!(
                    p.agent_message_payload(&m)?,
                    serde_json::json!({"message":mid,"viewer":DAVID,"base":"https://second.test"})
                );
                Ok(())
            })
            .await
            .unwrap();
    }
}

#[cfg(test)]
mod callback_tests {
    use super::super::test_support::{ALL_TALK, DAVID, TestApp};
    use campfire_db::callbacks::Phase;
    use campfire_db::{Message, NewMessage};
    #[tokio::test]
    async fn ws11_production_jobs_dispatches_peer_hooks_and_rejects_the_originating_write() {
        let test = TestApp::boot().await.expect("default seed");
        let app = &test.booted.app;
        app.jobs
            .model_callbacks
            .install(Phase::MessageActivity, |tx, c| {
                assert!(Message::find_by_id(tx.conn(), c.record_id)?.is_some());
                Err(campfire_db::Error::Other(
                    "WS12 adapter rejected activity".into(),
                ))
            });
        assert!(
            app.db
                .write(|tx| Message::create(
                    tx,
                    NewMessage {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        client_message_id: Some("ws11-production-peer-rejected".into()),
                        markdown_source: Some("Peer callback".into()),
                        ..Default::default()
                    }
                ))
                .await
                .is_err()
        );
        app.db.read(|c|{assert_eq!(c.query_row("SELECT COUNT(*) FROM messages WHERE client_message_id='ws11-production-peer-rejected'",[],|r|r.get::<_,i64>(0))?,0);Ok(())}).await.unwrap();
    }
}

#[cfg(test)]
mod finalization_tests {
    use super::super::test_support::{ALL_TALK, BENDER, TestApp};
    use campfire_db::{Agent, Message, NewMessage};
    #[tokio::test]
    async fn ws11_finalization_durable_job_failure_rolls_back_claim_and_earlier_effects() {
        let test = TestApp::boot().await.expect("default seed");
        let app = &test.booted.app;
        let mid=app.db.write(|tx|{
            let mut a=Agent::for_user(tx.conn(),BENDER)?.unwrap();a.set_working_presence(tx,Some("Keep working"))?;
            let m=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:BENDER,markdown_source:Some("Draft".into()),streaming:true,..Default::default()})?;
            tx.conn().execute_batch("CREATE TRIGGER ws11_reject_finalize BEFORE INSERT ON background_jobs WHEN NEW.job_class='Room::PushMessageJob' BEGIN SELECT RAISE(ABORT,'WS11 rejected finalize queue'); END;")?;Ok(m.id)
        }).await.unwrap();
        assert!(
            app.db
                .write(move |tx| Message::find(tx.conn(), mid)?.finalize_stream(tx))
                .await
                .is_err()
        );
        app.db
            .read(move |c| {
                assert!(Message::find(c, mid)?.streaming);
                assert_eq!(
                    c.query_row(
                        "SELECT COUNT(*) FROM message_search_index WHERE rowid=?",
                        [mid],
                        |r| r.get::<_, i64>(0)
                    )?,
                    0
                );
                assert_eq!(
                    Agent::for_user(c, BENDER)?
                        .unwrap()
                        .working_presence
                        .as_deref(),
                    Some("Keep working")
                );
                Ok(())
            })
            .await
            .unwrap();
    }
}

#[cfg(test)]
mod reference_callback_tests {
    use super::super::test_support::{ALL_TALK, BENDER, TestApp};
    use campfire_db::{Message, MessageChanges, NewMessage};
    fn expected(phase: &str) -> i64 {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../vectors/agents_stream_references_contract.json"
        ))
        .unwrap();
        oracle[phase].as_i64().unwrap()
    }
    #[tokio::test]
    async fn ws11_stream_case_link_references_sync_only_at_finalize() {
        let (app, _dir) = TestApp::boot()
            .await
            .expect("default seed")
            .stop_jobs()
            .await;
        let mid = app
            .db
            .write(|tx| {
                let m = Message::create(
                    tx,
                    NewMessage {
                        room_id: ALL_TALK,
                        creator_id: BENDER,
                        streaming: true,
                        markdown_source: Some("See https://example.test/some/page".into()),
                        ..Default::default()
                    },
                )?;
                assert_eq!(
                    tx.conn().query_row(
                        "SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",
                        [m.id],
                        |r| r.get::<_, i64>(0)
                    )?,
                    expected("start")
                );
                Ok(m.id)
            })
            .await
            .unwrap();
        app.db.write(move|tx| {
            let mut m=Message::find(tx.conn(),mid)?;
            m.update(tx,MessageChanges{markdown_source:Some("See https://example.test/some/page and more".into()),..Default::default()})?;
            assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",[mid],|r|r.get::<_,i64>(0))?,expected("append"));
            assert!(m.finalize_stream(tx)?);
            assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",[mid],|r|r.get::<_,i64>(0))?,expected("finalize"));
            assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='LinkEmbed::FetchJob'",[],|r|r.get::<_,i64>(0))?,1);
            assert!(!m.finalize_stream(tx)?);
            assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",[mid],|r|r.get::<_,i64>(0))?,expected("repeat"));
            assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='LinkEmbed::FetchJob'",[],|r|r.get::<_,i64>(0))?,1);Ok(())
        }).await.unwrap();
        app.db
            .write(|tx| {
                let mut quiet = Message::create(
                    tx,
                    NewMessage {
                        room_id: ALL_TALK,
                        creator_id: BENDER,
                        streaming: true,
                        markdown_source: Some("https://example.test/quiet-page".into()),
                        ..Default::default()
                    },
                )?;
                assert!(quiet.finalize_stream_quietly(tx)?);
                assert_eq!(
                    tx.conn().query_row(
                        "SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",
                        [quiet.id],
                        |r| r.get::<_, i64>(0)
                    )?,
                    expected("quiet")
                );
                Ok(())
            })
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn ws11_stream_reference_fetch_failure_keeps_claim_and_reference_atomic() {
        let (app, _dir) = TestApp::boot()
            .await
            .expect("default seed")
            .stop_jobs()
            .await;
        let mid=app.db.write(|tx| {
            let m=Message::create(tx,NewMessage{room_id:ALL_TALK,creator_id:BENDER,streaming:true,markdown_source:Some("https://example.test/rejected-fetch".into()),..Default::default()})?;
            tx.conn().execute_batch("CREATE TRIGGER ws11_reject_ref_fetch BEFORE INSERT ON background_jobs WHEN NEW.job_class='LinkEmbed::FetchJob' BEGIN SELECT RAISE(ABORT,'reference fetch queue failure'); END")?;Ok(m.id)
        }).await.unwrap();
        assert!(
            app.db
                .write(move |tx| Message::find(tx.conn(), mid)?.finalize_stream(tx))
                .await
                .is_err()
        );
        app.db.read(move|c| {
            assert!(Message::find(c,mid)?.streaming);
            assert_eq!(c.query_row("SELECT COUNT(*) FROM link_embed_references WHERE message_id=?",[mid],|r|r.get::<_,i64>(0))?,0);
            assert_eq!(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='LinkEmbed::FetchJob'",[],|r|r.get::<_,i64>(0))?,0);Ok(())
        }).await.unwrap();
    }
}
