//! Four one-to-one assertions from the pinned FizzyConnectedAccountTest.
use super::*;
use crate::controllers::presenters::test_support::{DAVID, TestApp};
use std::time::Duration;

const FIXTURE_TOKEN: &str = "fizzy-model-fixture-token";
fn input() -> Input<'static> {
    Input {
        user_id: DAVID,
        account_id: "897362094",
        account_name: Some("Fixture workspace"),
        fizzy_user_id: Some("fixture-user"),
        fizzy_user_name: Some("Fixture person"),
        token: FIXTURE_TOKEN,
    }
}
async fn app() -> (TestApp, ArEncryption) {
    let mut app = TestApp::boot().await.expect("pinned parity seed required");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(|tx| Account::disconnect(tx, DAVID))
        .await
        .unwrap();
    (app, crypto)
}
#[tokio::test]
async fn rails_a_linked_account_is_connected_and_usable() {
    let (app, crypto) = app().await;
    app.db()
        .write(move |tx| {
            let account = Account::create(tx, &crypto, &input())?;
            assert!(account.connected());
            assert_eq!(
                account.usable_token(tx, &crypto)?.as_deref(),
                Some(FIXTURE_TOKEN)
            );
            let ciphertext: String = tx.conn().query_row(
                "SELECT access_token FROM fizzy_connected_accounts WHERE id=?",
                [account.id],
                |r| r.get(0),
            )?;
            assert_ne!(ciphertext, FIXTURE_TOKEN);
            assert_eq!(crypto.decrypt(&ciphertext).unwrap(), FIXTURE_TOKEN);
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn rails_one_account_per_user() {
    let (app, crypto) = app().await;
    app.db()
        .write(move |tx| {
            let first = Account::create(tx, &crypto, &input())?;
            let mut duplicate = input();
            duplicate.token = "another-fizzy-model-fixture-token";
            let result = Account::create(tx, &crypto, &duplicate);
            let Err(campfire_db::Error::RecordInvalid(errors)) = result else {
                panic!("duplicate must fail model validation")
            };
            assert_eq!(
                errors.0,
                vec![("user_id", "has already been taken".into())]
            );
            assert_eq!(Account::for_user(tx.conn(), DAVID)?.unwrap().id, first.id);
            assert_eq!(
                first.usable_token(tx, &crypto)?.as_deref(),
                Some(FIXTURE_TOKEN)
            );
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn rails_marking_disconnected_reads_as_unusable() {
    let (app, crypto) = app().await;
    app.db()
        .write(move |tx| {
            let account = Account::create(tx, &crypto, &input())?;
            account.mark_disconnected(tx, REJECTED_TOKEN_REASON)?;
            let account = Account::for_user(tx.conn(), DAVID)?.unwrap();
            assert!(!account.connected());
            assert!(account.usable_token(tx, &crypto)?.is_none());
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn rails_deactivating_the_user_disconnects_the_account() {
    let (app, crypto) = app().await;
    app.db()
        .write(move |tx| {
            Account::create(tx, &crypto, &input())?;
            campfire_db::User::find(tx.conn(), DAVID)?.deactivate(tx)?;
            let account = Account::for_user(tx.conn(), DAVID)?.unwrap();
            assert_eq!(
                account.disconnected_reason.as_deref(),
                Some("Account deactivated")
            );
            assert!(!account.connected());
            assert!(account.usable_token(tx, &crypto)?.is_none());
            Ok(())
        })
        .await
        .unwrap();
}
