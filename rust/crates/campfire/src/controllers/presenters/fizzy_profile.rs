//! Profile display uses WS15e's token reader, including its unreadable-token disconnection.
use crate::app::AppState;
use crate::integrations::fizzy::accounts::Account;
use campfire_db::Result;
use campfire_views::users::ConnectionPanel;
use rails_compat::ar_encryption::ArEncryption;

pub async fn connection(app: &AppState, user_id: i64) -> Result<ConnectionPanel> {
    let crypto = ArEncryption::new(&app.secrets);
    app.db
        .write(move |tx| {
            let Some(account) = Account::for_user(tx.conn(), user_id)? else {
                return Ok(ConnectionPanel::Missing);
            };
            let usable = account.usable_token(tx, &crypto)?.is_some();
            // Rails usable? updates the same model; reload the owner's write before rendering.
            let account = Account::for_user(tx.conn(), user_id)?.expect("linked account");
            Ok(if usable {
                ConnectionPanel::Connected {
                    name: account.fizzy_user_name.unwrap_or_default(),
                    workspace: account.account_name,
                    app_token: false,
                }
            } else {
                ConnectionPanel::Rejected {
                    reason: account
                        .disconnected_reason
                        .filter(|s| !campfire_richtext::ruby::is_blank(s)),
                }
            })
        })
        .await
}
