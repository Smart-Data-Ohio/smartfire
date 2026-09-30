//! `app/jobs/retention/prune_job.rb`: bounded, resumable maintenance with Rails cutoffs.
use crate::models::room_delete;
use crate::sql::{placeholders, query_all};
use crate::{Database, Result, Timestamp, Tx};
use serde::{Deserialize, Serialize};

pub const BATCH_SIZE: usize = 1000; // ActiveRecord::Batches default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneJob {}
impl crate::Job for PruneJob {
    const CLASS: &'static str = "Retention::PruneJob";
}

pub fn audit_cutoff(now: Timestamp) -> Result<Timestamp> {
    let date = now
        .jiff()
        .to_zoned(jiff::tz::TimeZone::UTC)
        .checked_sub(jiff::Span::new().years(1))
        .map_err(|e| crate::Error::Other(e.to_string()))?;
    Ok(Timestamp::from_jiff(date.timestamp()))
}
async fn prune(
    db: &Database,
    table: &'static str,
    predicate: String,
    cutoff: Timestamp,
) -> Result<()> {
    loop {
        let sql = format!(
            "DELETE FROM {table} WHERE id IN (SELECT id FROM {table} WHERE {predicate} ORDER BY id LIMIT {BATCH_SIZE})"
        );
        if db
            .write(move |tx| Ok(tx.conn().execute(&sql, [cutoff])?))
            .await?
            == 0
        {
            break;
        }
    }
    Ok(())
}
pub async fn perform(db: &Database) -> Result<()> {
    let now = db.env().now();
    for (table, predicate, cutoff) in [
        (
            "agent_events",
            "created_at < ?",
            now.ago(jiff::SignedDuration::from_secs(90 * 86400)),
        ),
        ("two_factor_remembered_devices", "expires_at <= ?", now),
        ("two_factor_setup_secrets", "expires_at <= ?", now),
        (
            "activity_items",
            "read_at IS NOT NULL AND updated_at < ?",
            now.ago(jiff::SignedDuration::from_secs(180 * 86400)),
        ),
        (
            "github_webhook_deliveries",
            "created_at < ?",
            now.ago(jiff::SignedDuration::from_secs(14 * 86400)),
        ),
        (
            "huddle_cleanups",
            "completed_at IS NOT NULL AND completed_at < ?",
            now.ago(jiff::SignedDuration::from_secs(7 * 86400)),
        ),
        ("audit_logs", "created_at < ?", audit_cutoff(now)?),
        (
            "fizzy_card_caches",
            "updated_at < ?",
            now.ago(jiff::SignedDuration::from_secs(86400)),
        ),
    ] {
        prune(db, table, predicate.to_owned(), cutoff).await?;
    }
    let cutoff = now.ago(jiff::SignedDuration::from_secs(30 * 86400));
    loop {
        let ids:Vec<i64>=db.read(move |conn|query_all(conn,&format!("SELECT id FROM huddle_grants WHERE revoked_at < ? ORDER BY id LIMIT {BATCH_SIZE}"),[cutoff],|r|r.get(0))).await?;
        if ids.is_empty() {
            break;
        }
        let unlink = ids.clone();
        // One unlink per batch, outside each grant's destroy transaction, like Rails.
        db.write(move |tx| unlink_grants(tx, &unlink)).await?;
        for id in ids {
            db.write(move |tx| room_delete::destroy_grant(tx, id))
                .await?;
        }
    }
    db.write(|tx| room_delete::reenqueue_stuck(tx, 3600))
        .await?;
    Ok(())
}
fn unlink_grants(tx: &Tx<'_>, ids: &[i64]) -> Result<()> {
    tx.conn().execute(
        &format!(
            "UPDATE huddle_cleanups SET huddle_grant_id=NULL WHERE huddle_grant_id IN ({})",
            placeholders(ids.len())
        ),
        rusqlite::params_from_iter(ids),
    )?;
    Ok(())
}
