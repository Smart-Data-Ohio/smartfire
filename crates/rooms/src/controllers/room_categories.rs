//! `app/controllers/room_categories_controller.rb`: sidebar categories belong to the viewer.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, cast_integer, require_current_user};
use crate::controllers::presenters::page::db_error;
use campfire_db::RoomCategory;
use campfire_kit::{Ctx, Error, Param, Result, StatusCode};

pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let user = require_current_user(c)?.id;
    let rows = c
        .app()
        .db
        .read(move |conn| RoomCategory::ordered_for_user(conn, user))
        .await
        .map_err(db_error)?;
    c.json(
        StatusCode::OK,
        &rows
            .iter()
            .map(|row| serde_json::json!({"id": row.id, "name": row.name}))
            .collect::<Vec<_>>(),
    )
}

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let user = require_current_user(c)?.id;
    let params = category_params(c)?;
    let name = params.get("name").and_then(Param::to_s).unwrap_or_default();
    let collapsed = params
        .get("collapsed")
        .map(boolean)
        .transpose()?
        .unwrap_or(false);
    let result = c
        .app()
        .db
        .write(move |tx| {
            let position = RoomCategory::next_position_for(tx.conn(), user)?;
            RoomCategory::create(tx, user, &name, position, collapsed)
        })
        .await;
    // Rails calls create/update (not bang): invalid names still reload the sidebar.
    if let Err(error) = result
        && !matches!(error, campfire_db::Error::RecordInvalid(_))
    {
        return Err(db_error(error));
    }
    redirect_sidebar(c)
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let mut row = category(c).await?;
    let params = category_params(c)?;
    let name = params
        .get("name")
        .map(|value| value.to_s().unwrap_or_default())
        .unwrap_or_else(|| row.name.clone());
    let collapsed = params
        .get("collapsed")
        .map(boolean)
        .transpose()?
        .unwrap_or(row.collapsed);
    let result = c
        .app()
        .db
        .write(move |tx| row.update(tx, &name, row.position, collapsed))
        .await;
    if let Err(error) = result
        && !matches!(error, campfire_db::Error::RecordInvalid(_))
    {
        return Err(db_error(error));
    }
    redirect_sidebar(c)
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let row = category(c).await?;
    c.app()
        .db
        .write(move |tx| row.destroy(tx))
        .await
        .map_err(db_error)?;
    redirect_sidebar(c)
}

fn category_params(c: &Ctx) -> Result<campfire_kit::ParamMap> {
    Ok(c.params
        .require("room_category")?
        .permit(&campfire_kit::permit_keys(&["name", "collapsed"])))
}

pub(crate) async fn find_for_user(c: &Ctx, id: i64) -> Result<RoomCategory> {
    let user = require_current_user(c)?.id;
    c.app()
        .db
        .read(move |conn| Ok(RoomCategory::find_by_id(conn, id)?.filter(|row| row.user_id == user)))
        .await
        .map_err(db_error)?
        .ok_or(Error::NotFound)
}

async fn category(c: &Ctx) -> Result<RoomCategory> {
    let id = c
        .param_str("id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    find_for_user(c, id).await
}

fn redirect_sidebar(c: &mut Ctx) -> Result {
    c.redirect_to(&c.url_for(&campfire_routes::user_sidebar()))
}

/// ActiveModel::Type::Boolean's FALSE_VALUES; nil/empty casts to SQL NULL, rejected by schema.
fn boolean(value: &Param) -> Result<bool> {
    match value {
        Param::Null => Err(Error::internal(anyhow::anyhow!("collapsed cannot be NULL"))),
        Param::Str(s) if s.is_empty() => {
            Err(Error::internal(anyhow::anyhow!("collapsed cannot be NULL")))
        }
        Param::Bool(b) => Ok(*b),
        Param::Number(n) => Ok(n.as_i64() != Some(0)),
        Param::Str(s) => Ok(!matches!(
            s.as_str(),
            "0" | "f" | "F" | "false" | "FALSE" | "off" | "OFF"
        )),
        _ => Ok(true),
    }
}
