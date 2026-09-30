//! `app/controllers/saved_items_controller.rb`.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, cast_integer, require_current_user};
use crate::controllers::message_features as features;
use crate::controllers::presenters::{Presenter, page};
use campfire_db::{Message, Room, SavedItem, SavedItemChanges, User};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};

async fn prepare(c: &mut Ctx) -> Result<()> {
    before_actions(c, Before::default()).await?;
    c.no_store();
    c.set_header("Pragma", "no-cache");
    Ok(())
}
fn filter(c: &Ctx) -> String {
    c.param("status")
        .map(features::param_string)
        .filter(|status| campfire_db::models::saved_item::STATUSES.contains(&status.as_str()))
        .unwrap_or_else(|| "all".into())
}
fn filter_path(c: &Ctx) -> String {
    format!("{}?status={}", campfire_routes::saved_items(), filter(c))
}
async fn accessible(c: &Ctx) -> Result<SavedItem> {
    let user = require_current_user(c)?.id;
    let id = c
        .param_str("id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    c.app()
        .db
        .read(move |conn| {
            SavedItem::accessible_to(conn, user)?
                .into_iter()
                .find(|item| item.id == id)
                .ok_or(campfire_db::Error::RecordNotFound("SavedItem"))
        })
        .await
        .map_err(page::db_error)
}
pub async fn index(c: &mut Ctx) -> Result {
    prepare(c).await?;
    c.respond_to(&[&format::HTML])?;
    let user_id = require_current_user(c)?.id;
    let status = filter(c);
    let status_filter = status.clone();
    let app = c.app().clone();
    let items = c
        .app()
        .db
        .read(move |conn| {
            let presenter = Presenter::new(conn, &app, None);
            let viewer = User::find(conn, user_id)?;
            SavedItem::accessible_to(conn, user_id)?
                .into_iter()
                .filter(|item| status == "all" || item.status == status)
                .map(|item| view(&presenter, conn, &viewer, &item))
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .map_err(page::db_error)?;
    page::framed_page!(c, StatusCode::OK, |ctx| {
        campfire_views::saved_items::Index {
            ctx,
            items: &items,
            status_filter: &status_filter,
        }
    })
    .await
}
pub(crate) fn view(
    presenter: &Presenter<'_>,
    conn: &campfire_db::Connection,
    viewer: &User,
    item: &SavedItem,
) -> campfire_db::Result<campfire_views::saved_items::Item> {
    let message = Message::find(conn, item.message_id)?;
    Ok(campfire_views::saved_items::Item {
        id: item.id,
        status: item.status.clone(),
        created_at: item.created_at.jiff(),
        remind_at: item.remind_at.map(|at| at.jiff()),
        reminded_at: item.reminded_at.map(|at| at.jiff()),
        room_name: presenter
            .room_display_name(&Room::find(conn, message.room_id)?, Some(viewer))?,
        author_name: presenter.user(message.creator_id)?.name,
        body: campfire_views::helpers::truncate(&presenter.plain_text_body(&message)?, 500, "..."),
        message_path: campfire_db::message_pin::message_path(&message),
    })
}
pub async fn create(c: &mut Ctx) -> Result {
    prepare(c).await?;
    let message = features::reachable_message(c).await?;
    let nested = c.param("saved_item");
    if nested.is_some_and(|value| !value.is_null() && value.as_hash().is_none()) {
        return Err(Error::internal(anyhow::anyhow!(
            "saved_item does not support dig"
        )));
    }
    let raw = nested
        .and_then(|value| value.get("remind_at"))
        .filter(|value| value.is_present())
        .or_else(|| c.param("remind_at").filter(|value| value.is_present()));
    let zone = features::user_zone(c).await?;
    let remind_at = match raw {
        None => None,
        Some(raw) => match features::parse_time(&features::param_string(raw), &zone, c.now()) {
            Ok(Some(time)) => Some(time),
            _ => {
                return invalid(
                    c,
                    &serde_json::json!({"errors":{"remind_at":["is invalid"]}}),
                    "Reminder time is invalid",
                    true,
                )
                .await;
            }
        },
    };
    let user_id = require_current_user(c)?.id;
    let saved = c
        .app()
        .db
        .write(move |tx| SavedItem::save_for(tx, user_id, message.id, remind_at))
        .await;
    match saved {
        Ok(item) => match c.respond_to(&[&format::HTML, &format::TURBO_STREAM, &format::JSON])? {
            f if *f == format::JSON => payload(c, &item, StatusCode::CREATED),
            f if *f == format::TURBO_STREAM => Ok(c.head(StatusCode::OK)),
            _ => features::redirect(
                c,
                &campfire_routes::saved_items(),
                Some("Saved for later".into()),
                None,
                true,
                None,
            ),
        },
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            invalid(
                c,
                &features::errors_json(&errors),
                &features::sentence(&errors),
                true,
            )
            .await
        }
        Err(error) => Err(page::db_error(error)),
    }
}
pub async fn update(c: &mut Ctx) -> Result {
    prepare(c).await?;
    let mut item = accessible(c).await?;
    let status = c
        .params
        .require("saved_item")?
        .as_hash()
        .ok_or_else(|| Error::internal(anyhow::anyhow!("saved_item does not support require")))?
        .require("status")?
        .to_s()
        .unwrap_or_default();
    if !campfire_db::models::saved_item::STATUSES.contains(&status.as_str()) {
        return invalid(
            c,
            &serde_json::json!({"error":"Status must be in progress or done"}),
            "Status must be in progress or done",
            false,
        )
        .await;
    }
    item = c
        .app()
        .db
        .write(move |tx| {
            item.update(
                tx,
                SavedItemChanges {
                    status: Some(status),
                    ..Default::default()
                },
            )?;
            Ok(item)
        })
        .await
        .map_err(page::db_error)?;
    match c.respond_to(&[&format::HTML, &format::TURBO_STREAM, &format::JSON])? {
        f if *f == format::JSON => payload(c, &item, StatusCode::OK),
        f => features::redirect(
            c,
            &filter_path(c),
            None,
            None,
            false,
            (*f == format::TURBO_STREAM).then_some(StatusCode::SEE_OTHER),
        ),
    }
}
pub async fn destroy(c: &mut Ctx) -> Result {
    prepare(c).await?;
    let item = accessible(c).await?;
    c.app()
        .db
        .write(move |tx| item.destroy(tx))
        .await
        .map_err(page::db_error)?;
    match c.respond_to(&[&format::HTML, &format::TURBO_STREAM, &format::JSON])? {
        f if *f == format::JSON => Ok(c.head(StatusCode::NO_CONTENT)),
        f => features::redirect(
            c,
            &filter_path(c),
            None,
            None,
            false,
            (*f == format::TURBO_STREAM).then_some(StatusCode::SEE_OTHER),
        ),
    }
}
fn payload(c: &mut Ctx, item: &SavedItem, status: StatusCode) -> Result {
    c.json(status, &serde_json::json!({"id":item.id, "message_id":item.message_id, "status":item.status,
        "remind_at":item.remind_at.map(features::json_time), "reminded_at":item.reminded_at.map(features::json_time), "url":c.url_for(&format!("{}.json", campfire_routes::saved_item(item.id)))}))
}
async fn invalid(c: &mut Ctx, value: &serde_json::Value, alert: &str, back: bool) -> Result {
    match c.respond_to(&[&format::HTML, &format::TURBO_STREAM, &format::JSON])? {
        f if *f == format::JSON => c.json(StatusCode::UNPROCESSABLE_ENTITY, value),
        f if *f == format::TURBO_STREAM && back => Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY)),
        f => features::redirect(
            c,
            &if back {
                campfire_routes::saved_items()
            } else {
                filter_path(c)
            },
            None,
            Some(alert.into()),
            back,
            (*f == format::TURBO_STREAM).then_some(StatusCode::SEE_OTHER),
        ),
    }
}
