//! Workspace branding shared by boot, admin responses and live updates.

use campfire_app::cable::sync::WorkspaceBranding;
use campfire_db::Account;
use campfire_kit::{Error, Result};

use super::{accounts, attachments};
use crate::app::App;

pub async fn for_account(app: &App, account: &Account) -> Result<WorkspaceBranding> {
    let id = account.id;
    let (logo, banner) = app
        .db
        .read(move |conn| {
            Ok((
                attachments::attached_blob(conn, "Account", id, "logo")?,
                attachments::attached_blob(conn, "Account", id, "banner")?,
            ))
        })
        .await
        .map_err(Error::internal)?;
    let logo_animated = logo
        .as_ref()
        .is_some_and(campfire_storage::branding::animated);
    let banner_animated = banner
        .as_ref()
        .is_some_and(campfire_storage::branding::animated);
    Ok(WorkspaceBranding {
        name: account.name.clone(),
        logo_url: logo.as_ref().map(|blob| {
            if logo_animated {
                campfire_routes::workspace_logo(blob.id, false)
            } else {
                format!(
                    "{}&blob={}",
                    accounts::fresh_account_logo_path(Some(account), None),
                    blob.id
                )
            }
        }),
        logo_still_url: logo
            .as_ref()
            .filter(|_| logo_animated)
            .map(|blob| campfire_routes::workspace_logo(blob.id, true)),
        banner_url: banner
            .as_ref()
            .map(|blob| campfire_routes::fresh_account_banner(blob.id, false)),
        banner_still_url: banner
            .as_ref()
            .filter(|_| banner_animated)
            .map(|blob| campfire_routes::fresh_account_banner(blob.id, true)),
    })
}

/// Public workspace identity, shared by joining, the signed-out boot and About.
pub async fn public_for_account(
    app: &App,
    account: Option<&Account>,
) -> Result<campfire_api_types::SignInWorkspace> {
    let Some(account) = account else {
        return Ok(campfire_api_types::SignInWorkspace {
            name: None,
            logo_url: None,
            description: String::new(),
            invite_splash_url: None,
        });
    };
    let branding = for_account(app, account).await?;
    Ok(campfire_api_types::SignInWorkspace {
        name: Some(branding.name),
        logo_url: branding.logo_url,
        description: account.settings().description().to_owned(),
        invite_splash_url: invite_splash_url(app, account.id).await?,
    })
}

pub async fn invite_splash_url(app: &App, account_id: i64) -> Result<Option<String>> {
    let blob = app
        .db
        .read(move |conn| attachments::attached_blob(conn, "Account", account_id, "invite_splash"))
        .await
        .map_err(Error::internal)?;
    Ok(blob.map(|blob| {
        campfire_routes::fresh_account_invite_splash(
            blob.id,
            campfire_storage::branding::animated(&blob),
        )
    }))
}

/// Called after the mutation and its audit have committed. A socket failure cannot undo a save.
pub async fn publish(app: &App) {
    if !app.cable.sync_wanted() {
        return;
    }
    let read = async {
        let account = app.db.read(Account::first).await.map_err(Error::internal)?;
        match account {
            Some(account) => for_account(app, &account).await.map(Some),
            None => Ok(None),
        }
    }
    .await;
    match read {
        Ok(Some(branding)) => campfire_app::cable::sync::workspace_updated(&app.cable, branding),
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "sync: workspace branding not read"),
    }
}
