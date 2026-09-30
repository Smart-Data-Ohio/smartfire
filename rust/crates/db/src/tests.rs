//! Ports of `reference/test/models/**`, run against the reference fixtures.

mod account_test;
mod audit_log_test;
mod callbacks_test;
mod calendar_dispatch_test;
mod channel_thread_test;
mod differential_test;
mod direct_room_test;
mod first_run_test;
mod forwarder_test;
mod fixtures_test;
mod membership_test;
mod keyword_alert_test;
mod message_activity_test;
mod message_edit_test;
mod message_pin_test;
mod message_reference_test;
mod message_test;
mod mail_merge_test;
mod poll_test;
mod push_test;
mod notification_policy_test;
mod named_policy_test;
mod named_calendar_status_test;
mod notification_push_test;
mod named_push_gating_test;
mod status_settings_write_test;
mod workspace_presence_lease_test;
mod room_test;
mod room_delete_test;
mod retention_test;
mod rich_text_failure_test;
mod round2_test;
mod room_category_test;
mod saved_item_test;
mod scheduled_message_test;
mod save_touches_test;
mod search_query_test;
mod session_test;
mod user_test;

use std::sync::Arc;

use crate::fixtures::{self, identify};
use crate::rich_text::BasicRichText;
use crate::{Config, Connection, Database, Env, Event, RecordingSink, Result, TestClock, Tx};

/// A database loaded with the fixtures, a recording event sink and a controllable clock
/// (`fixtures :all` plus `ActiveSupport::Testing::TimeHelpers`).
pub struct TestDb {
    pub db: Database,
    pub sink: RecordingSink,
    pub clock: TestClock,
    _dir: tempfile::TempDir,
}

impl TestDb {
    pub fn new() -> Self {
        Self::with_clock(TestClock::new(), 4)
    }

    /// Fixtures loaded with `clock` and BCrypt cost `bcrypt_cost` for their password digests.
    pub fn with_clock(clock: TestClock, bcrypt_cost: u32) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let sink = RecordingSink::new();
        let env = Env {
            clock: Arc::new(clock.clone()),
            sink: Arc::new(sink.clone()),
            rich_text: Arc::new(BasicRichText),
            bcrypt_cost: 4,
        };
        let mut config = Config::new(dir.path().join("test.sqlite3"));
        config.readers = 2;
        config.environment = "test".into();
        let db = Database::open(config, env).unwrap();
        db.write_blocking(move |tx| {
            let options = fixtures::Options {
                now: tx.now(),
                bcrypt_cost,
            };
            fixtures::load(tx.conn(), &fixtures::reference_dir(), &options)
        })
        .unwrap();
        Self {
            db,
            sink,
            clock,
            _dir: dir,
        }
    }

    pub fn write<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Tx<'_>) -> Result<T> + Send + 'static,
    ) -> T {
        self.db.write_blocking(f).unwrap()
    }

    pub fn try_write<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Tx<'_>) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        self.db.write_blocking(f)
    }

    pub fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> T {
        self.db.read_blocking(f).unwrap()
    }

    pub fn now(&self) -> crate::Timestamp {
        crate::Clock::now(&self.clock)
    }

    pub fn events(&self) -> Vec<Event> {
        self.sink.events()
    }

    /// Another handle on the same database file, with its own writer connection, as a second
    /// process (another Puma worker, the periodic runner) would have: for claims that must hold
    /// across processes. It shares this one's clock and event sink.
    pub fn another_process(&self) -> Database {
        let env = Env {
            clock: Arc::new(self.clock.clone()),
            sink: Arc::new(self.sink.clone()),
            rich_text: Arc::new(BasicRichText),
            bcrypt_cost: 4,
        };
        let mut config = Config::new(self.db.path());
        config.readers = 1;
        config.prepare = false;
        Database::open(config, env).unwrap()
    }

    /// `travel_to Membership::Connectable::CONNECTION_TTL.from_now + 1`
    pub fn travel(&self, seconds: i64) {
        self.clock.travel(jiff::SignedDuration::from_secs(seconds));
    }
}

/// Golden vectors from our Rails (`bin/rails runner` in the reference image):
/// `User.digest_bot_token("BenderToken1")`, the bender fixture's `bot_token_digest`.
pub const BENDER_TOKEN_DIGEST: &str = "eca7c1486ccaf098cc637f7f8e48cad465ac9f14a4da3b2287e0f5addc62a4c2";

/// A fixture's id, by label.
pub fn id(label: &str) -> i64 {
    identify(label)
}

mod slash_commands_test;
