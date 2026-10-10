//! S3 search on `/api/v1` (`campfire_api_types::search` documents each endpoint): results,
//! filter chips and the first page's sections through `SearchQuery` (`searches#index`), and the
//! recent searches through `Search` (`searches#create/clear`).

use std::collections::BTreeMap;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
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

/// The longest `q` (and stored recent search), in characters. New: the classic page has no
/// limit; this one keeps a query's SQL (one term per word and name filter) well inside
/// SQLite's limits.
pub const MAX_QUERY_CHARS: usize = 500;
/// The most `from:` values, and separately the most `in:` values, in one query. New, as above.
pub const MAX_NAME_FILTERS: usize = 10;

/// `q`'s bounds, as `field`'s 422 when they don't hold.
fn bounded(field: &str, query: &SearchQuery) -> Option<api::ApiError> {
    let message = if query.raw.chars().count() > MAX_QUERY_CHARS {
        format!("is too long (maximum is {MAX_QUERY_CHARS} characters)")
    } else if query.from_names.len() + query.author_ids.len() > MAX_NAME_FILTERS {
        format!("has too many from: filters (maximum is {MAX_NAME_FILTERS})")
    } else if query.in_rooms.len() + query.channel_ids.len() > MAX_NAME_FILTERS {
        format!("has too many in: filters (maximum is {MAX_NAME_FILTERS})")
    } else {
        return None;
    };
    Some(validation(field, &message))
}

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
        "from_id" => api::SearchOperator::FromId,
        "in_id" => api::SearchOperator::InId,
        "mentions" => api::SearchOperator::Mentions,
        "sort" => api::SearchOperator::Sort,
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
                    RoomType::from_class_name(&row.room_type).unwrap_or_else(|| {
                        tracing::warn!(room_id = row.room_id, room_type = %row.room_type, "search section row in a room of an unknown type, shown as open");
                        RoomType::Open
                    }),
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
    let filters = typed_filters(c).map_err(|error| fail(c, error))?;
    let raw = filtered_query(&raw, &filters, c.param_str("sort").is_some());
    let query = SearchQuery::parse(&raw);
    let after = match c.param_str("before").filter(|raw| !raw.is_empty()) {
        None => None,
        Some(raw) => match decode_search_cursor(raw, query.sort) {
            Some(key) => Some(key),
            None => return Err(fail(c, validation("before", "is invalid"))),
        },
    };
    if query.sort == search_query::SearchSort::Relevance
        && query.match_expression().is_some()
        && after.is_some_and(|key| key.rank.is_none())
    {
        return Err(fail(c, validation("before", "is invalid")));
    }
    if let Some(error) = bounded("q", &query) {
        return Err(fail(c, error));
    }
    let zone = features::user_zone(c).await?.tz().clone();
    let (app, now) = (c.app().clone(), now(c));
    let (results, fetches) = c
        .app()
        .db
        .read(move |conn| {
            let sorted = query.messages_for_user_sorted(conn, viewer.id, zone, after)?;
            let next_cursor = sorted.next.map(|key| encode_search_cursor(key, query.sort));
            let page = sorted.page;
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
            let (messages, fetches) = dto::messages_and_fetches(conn, &app, &page.messages)?;
            let results = api::SearchResults {
                query: squish(&raw),
                chips: query.chips.iter().cloned().filter_map(chip).collect(),
                messages,
                users,
                conversations: dto::conversation_names(conn, &viewer, pairs)?,
                next_cursor,
                sections,
            };
            Ok((results, fetches))
        })
        .await
        .map_err(db_error)?;
    fetches.request(c.app()).await;
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
    let parsed = SearchQuery::parse(&query);
    if let Some(error) = bounded("query", &parsed) {
        return Err(fail(c, error));
    }
    if parsed.blank_query() {
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

fn typed_filters(c: &Ctx) -> std::result::Result<api::SearchFilters, api::ApiError> {
    fn id(c: &Ctx, name: &str) -> std::result::Result<Option<i64>, api::ApiError> {
        c.param_str(name)
            .map(|raw| {
                raw.parse::<i64>()
                    .ok()
                    .filter(|id| (1..=9_007_199_254_740_991).contains(id))
                    .ok_or_else(|| validation(name, "must be a positive ID"))
            })
            .transpose()
    }
    fn value<T: serde::de::DeserializeOwned>(
        name: &str,
        raw: &str,
    ) -> std::result::Result<T, api::ApiError> {
        serde_json::from_value(serde_json::Value::String(raw.into()))
            .map_err(|_| validation(name, "is invalid"))
    }
    let has = c
        .param_str("has")
        .filter(|raw| !raw.is_empty())
        .map(|raw| {
            let tokens = raw.split(',').collect::<Vec<_>>();
            if tokens.len() > 10 {
                return Err(validation("has", "has too many values"));
            }
            tokens.into_iter().map(|raw| value("has", raw)).collect()
        })
        .transpose()?
        .unwrap_or_default();
    let mentions_me = match c.param_str("mentionsMe") {
        None | Some("false") => false,
        Some("true") => true,
        _ => return Err(validation("mentionsMe", "must be true or false")),
    };
    Ok(api::SearchFilters {
        author_id: id(c, "authorId")?,
        channel_id: id(c, "channelId")?,
        has,
        mentions_me,
        sort: c
            .param_str("sort")
            .map(|raw| value("sort", raw))
            .transpose()?
            .unwrap_or_default(),
    })
}

/// Typed IDs and sort replace their equivalent query tokens; legacy names still intersect.
fn filtered_query(raw: &str, filters: &api::SearchFilters, explicit_sort: bool) -> String {
    let mut raw = raw.to_owned();
    for chip in SearchQuery::parse(&raw).chips {
        let replace = (filters.author_id.is_some() && chip.token.starts_with("from_id:"))
            || (filters.channel_id.is_some() && chip.token.starts_with("in_id:"))
            || (explicit_sort && chip.token.starts_with("sort:"));
        if replace {
            raw = raw.replacen(&chip.token, "", 1);
        }
    }
    let mut tokens = vec![raw];
    if let Some(id) = filters.author_id {
        tokens.push(format!("from_id:{id}"));
    }
    if let Some(id) = filters.channel_id {
        tokens.push(format!("in_id:{id}"));
    }
    for has in &filters.has {
        let value = match has {
            api::SearchMedia::Mention => "mention",
            api::SearchMedia::File => "file",
            api::SearchMedia::Image => "image",
            api::SearchMedia::Link => "link",
            api::SearchMedia::Audio => "audio",
            api::SearchMedia::Video => "video",
            api::SearchMedia::Pin => "pin",
        };
        tokens.push(format!("has:{value}"));
    }
    if filters.mentions_me {
        tokens.push("mentions:me".into());
    }
    if explicit_sort {
        tokens.push(format!(
            "sort:{}",
            match filters.sort {
                api::SearchSort::Newest => "newest",
                api::SearchSort::Oldest => "oldest",
                api::SearchSort::Relevance => "relevance",
            }
        ));
    }
    squish(&tokens.join(" "))
}

fn encode_search_cursor(key: search_query::SearchCursor, sort: search_query::SearchSort) -> String {
    if sort == search_query::SearchSort::Newest {
        return crate::cursor::encode(key.created_at, key.id);
    }
    let mode = if sort == search_query::SearchSort::Oldest {
        "oldest"
    } else {
        "relevance"
    };
    URL_SAFE_NO_PAD.encode(format!(
        "{mode}|{}|{}|{}",
        key.created_at.to_db(),
        key.id,
        key.rank.map(|rank| rank.to_string()).unwrap_or_default()
    ))
}

fn decode_search_cursor(
    raw: &str,
    sort: search_query::SearchSort,
) -> Option<search_query::SearchCursor> {
    if sort == search_query::SearchSort::Newest {
        let (created_at, id) = crate::cursor::decode(raw)?;
        return Some(search_query::SearchCursor {
            created_at,
            id,
            rank: None,
        });
    }
    let text = String::from_utf8(URL_SAFE_NO_PAD.decode(raw).ok()?).ok()?;
    let parts = text.split('|').collect::<Vec<_>>();
    let [mode, at, id, rank] = parts.as_slice() else {
        return None;
    };
    let expected = if sort == search_query::SearchSort::Oldest {
        "oldest"
    } else {
        "relevance"
    };
    if *mode != expected {
        return None;
    }
    let rank = if rank.is_empty() {
        None
    } else {
        Some(rank.parse::<f64>().ok().filter(|rank| rank.is_finite())?)
    };
    Some(search_query::SearchCursor {
        created_at: campfire_db::Timestamp::parse_db(at)?,
        id: id.parse().ok()?,
        rank,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorted_cursors_round_trip_and_reject_wrong_modes_and_nonfinite_ranks() {
        use search_query::{SearchCursor, SearchSort};
        let created_at = campfire_db::Timestamp::parse_db("2026-10-06 12:00:00.123456").unwrap();
        for (sort, rank) in [
            (SearchSort::Newest, None),
            (SearchSort::Oldest, None),
            (SearchSort::Relevance, Some(-0.000123456789)),
        ] {
            let key = SearchCursor {
                created_at,
                id: 42,
                rank,
            };
            let cursor = encode_search_cursor(key, sort);
            assert_eq!(decode_search_cursor(&cursor, sort), Some(key));
            if sort != SearchSort::Newest {
                assert_eq!(decode_search_cursor(&cursor, SearchSort::Newest), None);
            }
        }
        let invalid = URL_SAFE_NO_PAD.encode("relevance|2026-10-06 12:00:00|42|NaN");
        assert_eq!(decode_search_cursor(&invalid, SearchSort::Relevance), None);
        assert_eq!(decode_search_cursor("garbage", SearchSort::Oldest), None);
    }

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
