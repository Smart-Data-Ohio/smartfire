//! Users::StarsController: only the signed-in human's own private preference rows.
use crate::app::AppCtx;
use crate::concerns::{self, Before};
use campfire_db::UserStar;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};

pub async fn create(c: &mut Ctx) -> Result {
    change(c, true).await
}
pub async fn destroy(c: &mut Ctx) -> Result {
    change(c, false).await
}

async fn change(c: &mut Ctx, starred: bool) -> Result {
    // Rails overrides request_authentication for JSON; preserve the rest of the shared chain.
    match concerns::before_actions(c, Before::default()).await {
        Err(Error::Halt(response))
            if response.status == StatusCode::FOUND
                && !concerns::signed_in(c)
                && c.format()? == Some(&format::JSON) =>
        {
            return Ok(head(StatusCode::UNAUTHORIZED));
        }
        result => result?,
    }
    let target = match super::find_user(c, "user_id").await {
        Err(Error::NotFound) => return Ok(head(StatusCode::NOT_FOUND)),
        result => result?,
    };
    let viewer = concerns::require_current_user(c)?;
    // set_starred_user runs before ensure_human_starrer, including for a bot's own card.
    if viewer.id == target.id {
        return Ok(head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    if !viewer.is_active() || viewer.is_bot() {
        return Ok(head(StatusCode::FORBIDDEN));
    }
    let (viewer_id, target_id) = (viewer.id, target.id);
    let result = c
        .app()
        .db
        .write(move |tx| {
            if starred {
                UserStar::find_or_create(tx, viewer_id, target_id)?;
            } else {
                UserStar::remove(tx, viewer_id, target_id)?;
            }
            Ok(())
        })
        .await;
    if let Err(error) = result
        && !(starred
            && (error.is_record_not_unique()
                || matches!(error, campfire_db::Error::RecordInvalid(_))))
    {
        return Err(Error::internal(error));
    }
    match c.respond_to(&[&format::JSON, &format::HTML])? {
        f if *f == format::JSON => Ok(c.render(
            StatusCode::OK,
            &format::JSON,
            format!("{{\"starred\":{starred}}}"),
        )),
        _ => c.redirect_back_or_to(&campfire_routes::user(target_id)),
    }
}

// Head runs before respond_to has set formats: Rails chooses bare text/html even for JSON.
fn head(status: StatusCode) -> campfire_kit::Response {
    campfire_kit::Response::new(status).content_type("text/html")
}
