//! `app/controllers/users/time_zones_controller.rb`: first-visit browser detection.
use crate::app::AppCtx;
use crate::concerns::{self, Before};
use campfire_db::User;
use campfire_kit::{Ctx, Error, Result, StatusCode};

pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let zone = c
        .params
        .get("time_zone")
        .and_then(|value| value.to_s())
        .unwrap_or_default();
    let result = c
        .app()
        .db
        .write(move |tx| User::detect_browser_time_zone(tx, user_id, &zone))
        .await;
    let (status, zone) = match result {
        Ok(zone) => (StatusCode::OK, zone),
        Err(campfire_db::Error::RecordInvalid(_)) => {
            let zone = c
                .app()
                .db
                .read(move |conn| User::saved_time_zone(conn, user_id))
                .await
                .map_err(Error::internal)?;
            (StatusCode::UNPROCESSABLE_ENTITY, zone)
        }
        Err(error) => return Err(Error::internal(error)),
    };
    Ok(c.render_as(
        status,
        "application/json; charset=utf-8",
        serde_json::json!({"time_zone":zone}).to_string(),
    ))
}
