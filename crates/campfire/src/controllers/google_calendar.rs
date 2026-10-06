//! `Google::CalendarNotificationsController` is an API controller without session or CSRF.
use crate::app::AppCtx;
use campfire_kit::{Ctx, Result, Error, StatusCode};
pub async fn notifications(c:&mut Ctx)->Result {
    let header=|name:&str|c.request.headers.get(name).and_then(|v|v.to_str().ok()).unwrap_or("").to_owned();
    let (id,token,state,number)=(header("x-goog-channel-id"),header("x-goog-channel-token"),header("x-goog-resource-state"),header("x-goog-message-number"));
    let status=c.app().db.write(move |tx|campfire_db::models::google_calendar::notification(tx,&id,&token,&state,&number)).await.map_err(Error::internal)?;
    Ok(c.head(StatusCode::from_u16(status).unwrap()))
}
