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
    let storage = app.storage.clone();
    let (logo, logo_animated, banner, banner_animated) = tokio::task::spawn_blocking(move || {
        let animated = |blob: &Option<campfire_storage::Blob>| {
            blob.as_ref()
                .map(|blob| storage.is_animated(blob))
                .transpose()
                .map(|value| value.unwrap_or(false))
        };
        let (logo_animated, banner_animated) = (animated(&logo)?, animated(&banner)?);
        Ok::<_, campfire_storage::Error>((logo, logo_animated, banner, banner_animated))
    })
    .await
    .map_err(Error::internal)?
    .map_err(Error::internal)?;
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
