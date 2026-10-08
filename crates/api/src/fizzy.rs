//! JSON adapters for the classic message-to-Fizzy flow. Scope, credentials, remote failures,
//! writes, bot deliveries and broadcasts all run through the same controller functions.
use std::collections::BTreeMap;

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_controllers::controllers::fizzy_message_cards::{self as posting, Failure};
use campfire_kit::{Ctx, Result, StatusCode};
use campfire_web::controllers::presenters::page::db_error;

use crate::{dto, endpoints::body, error::fail};

endpoint!(new => form);
endpoint!(create => post_card);

fn failure(error: Failure) -> api::ApiError {
    let message = error.message();
    match error {
        Failure::NotConnected => api::ApiError::FizzyNotConnected { message },
        Failure::Locked => api::ApiError::FizzyThreadLocked { message },
        Failure::Unreachable | Failure::VerificationUnreachable | Failure::CreateUnreachable(_) => {
            api::ApiError::FizzyUnreachable { message }
        }
        Failure::Rejected => api::ApiError::FizzyTokenRejected { message },
        Failure::ReadOnly => api::ApiError::FizzyReadOnly { message },
        Failure::Refused(_) => api::ApiError::FizzyRefused { message },
        Failure::ReplyFailed { number, url, .. } => api::ApiError::FizzyReplyFailed {
            message,
            number,
            url,
        },
        Failure::Invalid {
            board_missing,
            title_missing,
            ..
        } => {
            let mut fields = BTreeMap::new();
            if board_missing {
                fields.insert("boardId".into(), vec!["Choose a board.".into()]);
            }
            if title_missing {
                fields.insert("title".into(), vec!["Enter a title.".into()]);
            }
            api::ApiError::Validation { message, fields }
        }
    }
}

async fn form(c: &mut Ctx) -> Result {
    let source = posting::source(c).await?;
    let boards = if source.connected() {
        posting::boards(c, &source)
            .await?
            .map_err(|error| fail(c, failure(error)))?
    } else {
        serde_json::Value::Null
    };
    let view = posting::form_view(c, &source, boards);
    let boards = view
        .boards
        .as_array()
        .map(|boards| {
            boards
                .iter()
                .map(|board| api::FizzyBoard {
                    id: board["id"].as_str().unwrap_or_default().into(),
                    name: board["name"].as_str().unwrap_or_default().into(),
                })
                .collect()
        })
        .unwrap_or_default();
    c.json(
        StatusCode::OK,
        &api::FizzyMessageCardForm {
            connected: view.connected,
            boards,
            title: view.title,
            description: view.description,
            excerpt: campfire_richtext::ruby::truncate(&view.plain, 280, "..."),
            author_name: view.creator,
            room_display_name: view.room_name,
            fizzy_user_name: view.user_name,
            account_name: view.account_name,
        },
    )
}

async fn post_card(c: &mut Ctx) -> Result {
    let source = posting::source(c).await?;
    let request: api::CreateFizzyCard = body(c).await?;
    let created = posting::create_card(
        c,
        &source,
        request.board_id,
        request.title,
        request.description,
    )
    .await?
    .map_err(|error| fail(c, failure(error)))?
    .into_spa()
    .map_err(|error| fail(c, failure(error)))?;
    let app = c.app().clone();
    let reply = created.reply;
    let message = c
        .app()
        .db
        .read(move |conn| dto::message(conn, &app, &reply))
        .await
        .map_err(|error| {
            fail(
                c,
                failure(Failure::reply_failed(
                    created.number.clone(),
                    created.url.clone(),
                    db_error(error),
                )),
            )
        })?;
    c.json(
        StatusCode::CREATED,
        &api::CreatedFizzyCard {
            notice: format!("Fizzy card #{} created.", created.number),
            number: created.number.clone(),
            url: created.url.clone(),
            message,
        },
    )
    .map_err(|error| {
        fail(
            c,
            failure(Failure::reply_failed(created.number, created.url, error)),
        )
    })
}
