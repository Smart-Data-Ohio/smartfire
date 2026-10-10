//! Minute-tick Calendar::{Meeting,Ooo}Dispatcher. Cache reads never fetch Google.
//! Refresh jobs and conditional claims commit together for each member. A bad member is
//! isolated; after the claims, badges precede notices just as in Rails' broadcast_flips!.
use crate::models::user_status_settings::updates::MeetingRefreshJob;
use crate::{Database, Event, Result, Timestamp, UserStatusSettings};
use jiff::SignedDuration;

pub const REFRESH_STALE_AFTER: SignedDuration = SignedDuration::from_secs(15 * 60);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct DispatchStats {
    pub refreshed: usize,
    pub flipped: usize,
    pub failed_user_ids: Vec<i64>,
}
#[derive(Clone, Copy)]
enum Kind {
    Meeting,
    Ooo,
}

pub async fn dispatch_meetings(db: &Database, now: Timestamp) -> Result<DispatchStats> {
    dispatch(db, now, Kind::Meeting).await
}
pub async fn dispatch_ooo(db: &Database, now: Timestamp) -> Result<DispatchStats> {
    dispatch(db, now, Kind::Ooo).await
}

async fn dispatch(db: &Database, now: Timestamp, kind: Kind) -> Result<DispatchStats> {
    // User.active includes bots. find_each orders by the primary key, irrespective of name.
    let ids = db.read(move |conn| {
        crate::sql::query_all(conn, match kind {
            Kind::Meeting => "SELECT id FROM users WHERE status=0 AND meeting_status_enabled=1 ORDER BY id",
            Kind::Ooo => "SELECT id FROM users WHERE status=0 AND (ooo_until IS NOT NULL OR ooo_calendar_enabled=1) ORDER BY id",
        }, [], |row| row.get::<_,i64>(0))
    }).await?;
    let mut stats = DispatchStats::default();
    let mut flips = Vec::new();
    for id in ids {
        // Load individually so malformed persisted values cannot poison an entire batch.
        let outcome: Result<_> = async {
            let user = db
                .read(move |conn| UserStatusSettings::find(conn, id))
                .await?;
            let cache = user.meeting_cache.as_ref();
            let stale = cache.is_none_or(|cache| {
                cache
                    .fetched_at
                    .is_none_or(|at| at <= now.since(-REFRESH_STALE_AFTER))
            });
            let (refresh, active, claim) = match kind {
                Kind::Meeting => {
                    let active = cache.is_some_and(|cache| cache.in_meeting(now));
                    (
                        stale,
                        active,
                        cache.is_some_and(|cache| cache.in_meeting_broadcast != Some(active)),
                    )
                }
                Kind::Ooo => {
                    let active = user.out_of_office(now);
                    let no_state = user.ooo_until.is_none()
                        && cache.is_none()
                        && user.ooo_broadcast != Some(true);
                    let steady = user.ooo_broadcast.unwrap_or(false) == active
                        && (active || user.ooo_until.is_none());
                    (
                        user.ooo_calendar_enabled && !user.meeting_status_enabled && stale,
                        active,
                        !no_state && !steady,
                    )
                }
            };
            if !refresh && !claim {
                return Ok((false, None)); // Steady state never acquires the writer lock.
            }
            db.write(move |tx| {
                if refresh {
                    tx.emit_after_commit(Event::job(&MeetingRefreshJob { user_id: id }));
                }
                let won = claim
                    && match kind {
                        Kind::Meeting => user
                            .meeting_cache
                            .as_ref()
                            .expect("claim needs a cache")
                            .claim_broadcast(tx, active)?,
                        Kind::Ooo => user.claim_ooo_broadcast(tx, active, now)?,
                    };
                Ok((refresh, won.then_some(user)))
            })
            .await
        }
        .await;
        match outcome {
            Ok((refresh, user)) => {
                stats.refreshed += usize::from(refresh);
                if let Some(user) = user {
                    flips.push(user);
                }
            }
            Err(error) => {
                tracing::error!(user_id=id,%error,"Calendar dispatcher failed for member");
                stats.failed_user_ids.push(id);
            }
        }
    }
    stats.flipped = flips.len();
    if !flips.is_empty() {
        // Rails broadcasts after every conditional claim has run. The loaded snapshots are
        // retained (update_all does not reload associations or refresh the user attributes).
        db.write(move |tx| {
            UserStatusSettings::announce_badges_for(tx, &flips)?;
            Ok(())
        })
        .await?;
    }
    Ok(stats)
}
