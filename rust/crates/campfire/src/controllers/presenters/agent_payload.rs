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
