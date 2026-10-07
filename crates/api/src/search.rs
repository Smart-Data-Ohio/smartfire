//! S3 search on `/api/v1` (`campfire_api_types::search` documents each endpoint): results,
//! filter chips and the first page's sections through `SearchQuery` (`searches#index`), and the
//! recent searches through `Search` (`searches#create/clear`).

use std::collections::BTreeMap;

use campfire_api_types as api;
use campfire_app::app::AppCtx;
use campfire_db::models::search_query::{self, SearchQuery, SearchSection};
use campfire_db::{RoomType, Search};
use campfire_kit::{Ctx, Result, StatusCode};
use campfire_messages::controllers::message_features as features;
use campfire_web::concerns;
use campfire_web::controllers::presenters::page::db_error;

use crate::dto;
use crate::endpoints::{before_actions, body, now};
use crate::error::{fail, validation};

endpoint!(
    /// `GET /api/v1/search`
    index => index_search
);
endpoint!(
    /// `GET /api/v1/search/recents`
    recents => index_recents
);
endpoint!(
    /// `POST /api/v1/search/recents`
    record => create_recent
);
endpoint!(
    /// `DELETE /api/v1/search/recents`
    clear => clear_recents
);

/// `searches#create`'s refusal of a blank query.
const BLANK: &str = "Enter a word to search for.";

/// `display_query`: runs of whitespace collapsed.
fn squish(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A parsed operator as its chip. `Chip::label` is `"{operator}: {value}"`.
fn chip(chip: search_query::Chip) -> Option<api::SearchChip> {
    let (operator, value) = chip.label.split_once(": ")?;
    let operator = match operator {
        "from" => api::SearchOperator::From,
        "in" => api::SearchOperator::In,
        "has" => api::SearchOperator::Has,
        "before" => api::SearchOperator::Before,
        "after" => api::SearchOperator::After,
        "on" => api::SearchOperator::On,
        "is" => api::SearchOperator::Is,
        _ => return None,
    };
    Some(api::SearchChip {
        operator,
        value: value.to_owned(),
        token: chip.token,
        label: chip.label.clone(),
        remove_query: chip.remove_query,
    })
}

fn section(section: SearchSection) -> api::SearchSection {
    let kind = match section.kind {
        "board-posts" => api::SearchSectionKind::BoardPosts,
        "work-threads" => api::SearchSectionKind::WorkThreads,
        _ => api::SearchSectionKind::Events,
    };
    api::SearchSection {
        kind,
        rows: section
            .records
            .into_iter()
            .map(|row| api::SearchSectionRow {
                id: row.id,
                room_id: row.room_id,
                room_kind: dto::room_kind(
                    RoomType::from_class_name(&row.room_type).unwrap_or(RoomType::Open),
                ),
                title: row.title,
                time: dto::time(row.time),
                work_status: row.status.as_deref().and_then(|status| match status {
                    "planned" => Some(api::WorkStatus::Planned),
                    "in_progress" => Some(api::WorkStatus::InProgress),
                    "blocked" => Some(api::WorkStatus::Blocked),
                    "done" => Some(api::WorkStatus::Done),
                    _ => None,
                }),
                cancelled: row.cancelled,
            })
            .collect(),
    }
}

async fn index_search(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    let raw = c.param_str("q").unwrap_or_default().to_owned();
    let after = match c.param_str("before").filter(|raw| !raw.is_empty()) {
        None => None,
        Some(raw) => match crate::cursor::decode(raw) {
            Some(key) => Some(key),
            None => return Err(fail(c, validation("before", "is invalid"))),
        },
    };
    let zone = features::user_zone(c).await?.tz().clone();
    let (app, now) = (c.app().clone(), now(c));
    let results = c
        .app()
        .db
        .read(move |conn| {
            let query = SearchQuery::parse(&raw);
            let page = query.messages_for_user_after(conn, viewer.id, zone, after)?;
            // The page is oldest first; the next one starts past its oldest.
            let next_cursor = page
                .messages
                .first()
                .filter(|_| page.has_more)
                .map(|message| crate::cursor::encode(message.created_at, message.id));
            let sections = if after.is_none() && !query.blank_query() {
                query
                    .sections_for_user(conn, viewer.id)?
                    .into_iter()
                    .map(section)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let pairs = page
                .messages
                .iter()
                .map(|message| (message.room_id, message.thread_id))
                .chain(sections.iter().flat_map(|section| {
                    section.rows.iter().map(move |row| match section.kind {
                        api::SearchSectionKind::Events => (row.room_id, None),
                        _ => (row.room_id, Some(row.id)),
                    })
                }))
                .collect::<Vec<_>>();
            let users = dto::users(
                conn,
                &app.secrets,
                page.messages.iter().map(|message| message.creator_id),
                now,
            )?;
            Ok(api::SearchResults {
                query: squish(&raw),
                chips: query.chips.iter().cloned().filter_map(chip).collect(),
                messages: dto::messages(conn, &app, &page.messages)?,
                users,
                conversations: dto::conversation_names(conn, &viewer, pairs)?,
                next_cursor,
                sections,
            })
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &results)
}

fn recent_list(
    conn: &campfire_db::Connection,
    user_id: i64,
) -> campfire_db::Result<api::RecentSearchList> {
    Ok(api::RecentSearchList {
        searches: Search::recent_for_user(conn, user_id)?
            .into_iter()
            .map(|row| api::RecentSearch {
                id: row.id,
                query: row.query,
                searched_at: dto::time(row.updated_at),
            })
            .collect(),
    })
}

async fn index_recents(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let list = c
        .app()
        .db
        .read(move |conn| recent_list(conn, user_id))
        .await
        .map_err(db_error)?;
    c.json(StatusCode::OK, &list)
}

async fn create_recent(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    let api::RecordSearch { query } = body(c).await?;
    if SearchQuery::parse(&query).blank_query() {
        return Err(fail(
            c,
            api::ApiError::Validation {
                message: BLANK.into(),
                fields: BTreeMap::from([("query".to_owned(), vec![BLANK.to_owned()])]),
            },
        ));
    }
    let query = squish(&query);
    let list = c
        .app()
        .db
        .write(move |tx| {
            Search::record(tx, user_id, &query)?;
            recent_list(tx.conn(), user_id)
        })
        .await
        .map_err(db_error)?;
    c.json(StatusCode::CREATED, &list)
}

async fn clear_recents(c: &mut Ctx) -> Result {
    before_actions(c).await?;
    let user_id = concerns::require_current_user(c)?.id;
    c.app()
        .db
        .write(move |tx| Search::destroy_all_for_user(tx, user_id))
        .await
        .map_err(db_error)?;
    Ok(c.head(StatusCode::NO_CONTENT))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chips_name_their_operator_and_value() {
        let query = SearchQuery::parse("launch from:@ada, has:FILE is:thread on:2026-01-02");
        let chips = query.chips.into_iter().filter_map(chip).collect::<Vec<_>>();
        let seen = chips
            .iter()
            .map(|chip| (chip.operator, chip.value.as_str(), chip.token.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            seen,
            [
                (api::SearchOperator::From, "ada", "from:@ada,"),
                (api::SearchOperator::Has, "file", "has:FILE"),
                (api::SearchOperator::Is, "true", "is:thread"),
                (api::SearchOperator::On, "2026-01-02", "on:2026-01-02"),
            ]
        );
        assert_eq!(chips[0].label, "from: ada");
        assert_eq!(
            chips[0].remove_query,
            "launch has:FILE is:thread on:2026-01-02"
        );
    }
}
