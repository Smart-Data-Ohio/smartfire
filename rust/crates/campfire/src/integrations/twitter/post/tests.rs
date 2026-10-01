use super::*;
use crate::integrations::test_support::TestDb;
use campfire_db::{BasicRichText, Env, RecordingSink, TestClock};
use std::sync::Arc;
#[test]
fn ws15e_x_post_identity_fetch_windows_and_quiet_claims() {
    let clock = Arc::new(TestClock::frozen_at(Timestamp::from_second(1_700_000_000)));
    let sink = Arc::new(RecordingSink::new());
    let db = TestDb::with_env(
        Env {
            clock,
            sink: sink.clone(),
            rich_text: Arc::new(BasicRichText),
            bcrypt_cost: 4,
            ..Env::default()
        },
        &std::env::temp_dir(),
    );
    db.db
        .write_blocking(|tx| {
            let first = Post::for_reference(
                tx,
                "266031293945503744",
                Some("https://x.com/a/status/266031293945503744"),
            )?;
            let second = Post::for_reference(
                tx,
                "266031293945503744",
                Some("https://x.com/b/status/266031293945503744"),
            )?;
            assert_eq!(first.id, second.id);
            assert_eq!(second.post_id, "266031293945503744");
            assert_eq!(second.url, first.url);
            assert_eq!(
                tx.conn().query_row(
                    "SELECT COUNT(*) FROM twitter_posts WHERE post_id=?",
                    [&first.post_id],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            assert!(Post::for_reference(tx, "  ", None).is_err());
            assert!(first.needs_fetch(tx.now()));
            for (minutes, error, needs) in [
                (60, None, false),
                (11, Some("Post not found on X"), true),
                (9, Some("Post not found on X"), false),
                (10, Some("Post not found on X"), false),
                (11, Some(" "), false),
            ] {
                tx.conn().execute(
                    "UPDATE twitter_posts SET fetched_at=?,fetch_error=? WHERE id=?",
                    params![
                        tx.now().ago(SignedDuration::from_mins(minutes)),
                        error,
                        first.id
                    ],
                )?;
                assert_eq!(
                    Post::find(tx.conn(), first.id)?.needs_fetch(tx.now()),
                    needs
                );
            }
            assert!(first.claim_fetch(tx)?);
            assert!(!first.claim_fetch(tx)?);
            tx.conn().execute(
                "UPDATE twitter_posts SET fetch_requested_at=? WHERE id=?",
                params![tx.now().ago(FETCH_WINDOW), first.id],
            )?;
            assert!(
                !first.claim_fetch(tx)?,
                "exact ten-minute boundary is still recent"
            );
            tx.conn().execute(
                "UPDATE twitter_posts SET fetch_requested_at=? WHERE id=?",
                params![
                    tx.now()
                        .ago(FETCH_WINDOW)
                        .ago(SignedDuration::from_micros(1)),
                    first.id
                ],
            )?;
            assert!(first.claim_fetch(tx)?);
            Ok(())
        })
        .unwrap();
    assert!(
        sink.events().is_empty(),
        "upsert and callback-free claims emit neither jobs nor broadcasts"
    );
}
