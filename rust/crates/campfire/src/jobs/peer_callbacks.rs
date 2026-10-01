//! Available has-one dependencies in User's Rails declaration order.
//! The reference account models have no destroy callbacks; remote preparations stay outside Tx.
use campfire_db::callbacks::{Phase, Registry};
pub(super) fn install(registry: &Registry) {
    registry.install(Phase::UserGithubAccount, |tx, c| {
        tx.conn().execute(
            "DELETE FROM github_connected_accounts WHERE user_id=?",
            [c.record_id],
        )?;
        Ok(())
    });
    registry.install(Phase::UserFizzyAccount, |tx, c| {
        tx.conn().execute(
            "DELETE FROM fizzy_connected_accounts WHERE user_id=?",
            [c.record_id],
        )?;
        Ok(())
    });
}
