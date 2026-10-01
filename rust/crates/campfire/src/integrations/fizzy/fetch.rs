//! One attempt per durable job; the REST client's idempotent transport replay is separate.
use super::{
    accounts::{Account, REJECTED_TOKEN_REASON},
    cards::{Cache, Card, NOT_FOUND},
    client::{Client, ErrorKind},
};
use crate::{app::App, integrations::net::Network};
use campfire_db::{Job, User};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome, RetryPolicy};
use rails_compat::ar_encryption::ArEncryption;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchJob {
    pub card_id: i64,
    pub user_id: i64,
}
impl Job for FetchJob {
    const CLASS: &'static str = "Fizzy::FetchCardJob";
}
impl JobKind for FetchJob {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::no_retries()
    }
}
pub async fn perform(app: App, job: FetchJob, _: Execution) -> JobResult {
    fetch(
        &app,
        &Network::system(),
        &super::client::api_base_url(),
        job.card_id,
        job.user_id,
    )
    .await
    .map_err(crate::jobs::discard_missing)?;
    Ok(Outcome::Done)
}
pub async fn fetch(
    app: &App,
    network: &Network,
    base: &str,
    card_id: i64,
    user_id: i64,
) -> campfire_db::Result<()> {
    let crypto = ArEncryption::new(&app.secrets);
    let input = app
        .db
        .write(move |tx| {
            let card = Card::find(tx.conn(), card_id)?;
            User::find(tx.conn(), user_id)?;
            let Some(account) = Account::for_user(tx.conn(), user_id)? else {
                return Ok(None);
            };
            let Some(token) = account.usable_token(tx, &crypto)? else {
                return Ok(None);
            };
            let cache = Cache::for_viewer(tx, &card, user_id)?;
            Ok(Some((card, cache, account, token)))
        })
        .await?;
    let Some((card, cache, account, token)) = input else {
        return Ok(());
    };
    let result = Client::new(network.clone(), token, base)
        .card(&card.account_id, &card.number.to_string())
        .await;
    app.db
        .write(move |tx| {
            match result {
                Ok(payload) => cache.save(tx, Some(&payload), Some(tx.now()), None)?,
                Err(error) => match error.kind {
                    ErrorKind::NotFound | ErrorKind::Forbidden => {
                        cache.save(tx, None, Some(tx.now()), Some(NOT_FOUND))?
                    }
                    ErrorKind::Unauthorized => {
                        account.mark_disconnected(tx, REJECTED_TOKEN_REASON)?;
                        cache.save(tx, None, None, None)?;
                    }
                    _ => cache.save(
                        tx,
                        cache.payload.as_ref(),
                        Some(tx.now()),
                        Some(&error.message),
                    )?,
                },
            }
            card.broadcast_updates(tx);
            Ok(())
        })
        .await
}
