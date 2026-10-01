//! `app/controllers/searches_controller.rb`.
pub(crate) mod preloads;
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::messages::present;
use crate::controllers::presenters::page::db_error;
use askama::Template;
use campfire_db::{Search, search_query::SearchQuery};
use campfire_kit::{Ctx, Error, Param, Result, StatusCode, format};
use campfire_views::searches::{Index, IndexView, search_path};

pub fn display_query(c: &Ctx) -> Option<String> {
    let raw = c.param("q").map(super::message_features::param_string).unwrap_or_default();
    let q = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    (!q.is_empty()).then_some(q)
}
fn parsed(c: &Ctx) -> SearchQuery {
    SearchQuery::parse(&c.param("q").map(super::message_features::param_string).unwrap_or_default())
}
pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let user_id = require_current_user(c)?.id;
    let q = parsed(c);
    let query = display_query(c);
    let older = c.param("before").is_some_and(Param::is_present);
    // Rails does not resolve a cursor for a blank query: set_messages returns early.
    let before = if !q.blank_query() && older {
        Some(
            c.param("before")
                .and_then(Param::to_s)
                .and_then(|s| cast_integer(&s))
                .ok_or(Error::NotFound)?,
        )
    } else {
        None
    };
    let stream =
        older && *c.respond_to(&[&format::HTML, &format::TURBO_STREAM])? == format::TURBO_STREAM;
    let zone = super::message_features::user_zone(c).await?;
    let dbq = q.clone();
    let (window, sections, recent_searches) = c
        .app()
        .db
        .read(move |conn| {
            let window = dbq.messages_for_user(conn, user_id, zone.tz().clone(), before)?;
            let sections = if !older && !dbq.blank_query() {
                dbq.sections_for_user(conn, user_id)?
            } else {
                vec![]
            };
            let recents = Search::recent_for_user(conn, user_id)?
                .into_iter()
                .map(|row| campfire_views::layouts::RecentSearch {
                    id: row.id,
                    query: row.query,
                })
                .collect();
            Ok((window, sections, recents))
        })
        .await
        .map_err(db_error)?;
    let oldest_id = window.messages.first().map(|m| m.id);
    let messages = present(c, move |presenter| {
        search_messages(presenter, &window.messages)
    })
    .await?;
    let return_to_room = concerns::last_room_visited(c).await?;
    let user = require_current_user(c)?.clone();
    let back = if let Some(room) = return_to_room {
        Some(
            present(c, move |p| {
                Ok((room.id, p.room_display_name(&room, Some(&user))?))
            })
            .await?,
        )
    } else {
        None
    };
    let index = IndexView {
        query: query.clone(),
        q: query,
        messages,
        recent_searches,
        return_to_room: back,
        has_more: window.has_more,
        oldest_id,
        chips: q
            .chips
            .into_iter()
            .map(|chip| campfire_views::searches::Chip {
                label: chip.label,
                remove_query: chip.remove_query,
            })
            .collect(),
        sections: sections.into_iter().map(section_view).collect(),
    };
    if stream {
        let layout = super::presenters::view_context::Layout::load(c).await?;
        let body = layout.render(c, |ctx| {
            campfire_views::searches::Older { ctx, index: &index }.render()
        })?;
        return Ok(c.render(StatusCode::OK, &format::TURBO_STREAM, body));
    }
    let response = super::presenters::view_context::page_or_frame_in_any_format(
        c,
        StatusCode::OK,
        |ctx| Index { ctx, index: &index }.render(),
        |ctx| {
            let page = Index { ctx, index: &index };
            campfire_views::layouts::frame(ctx, page.as_head(), page.as_content())
        },
    )
    .await?;
    let fragments = campfire_views::messages::MessageItem::cached_fragments(
        &c.app().fragment_cache,
        &index.messages,
        &c.url_for(""),
    );
    Ok(response.with_cached_fragments(fragments))
}
pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    if parsed(c).blank_query() {
        return super::message_features::redirect(
            c,
            &campfire_routes::searches(),
            Some("Enter a word to search for.".into()),
            None,
            true,
            None,
        );
    }
    let query = display_query(c).unwrap_or_default();
    let recorded = query.clone();
    let user_id = require_current_user(c)?.id;
    c.app()
        .db
        .write(move |tx| Search::record(tx, user_id, &recorded).map(|_| ()))
        .await
        .map_err(db_error)?;
    let url = c.url_for(&search_path(&query));
    c.redirect_to(&url)
}
pub async fn clear(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let stream = *c.respond_to(&[&format::HTML, &format::TURBO_STREAM])? == format::TURBO_STREAM;
    let user_id = require_current_user(c)?.id;
    c.app()
        .db
        .write(move |tx| Search::destroy_all_for_user(tx, user_id))
        .await
        .map_err(db_error)?;
    if stream {
        let layout = super::presenters::view_context::Layout::load(c).await?;
        let body = layout.render(c, |ctx| campfire_views::searches::Clear { ctx }.render())?;
        Ok(c.render(StatusCode::OK, &format::TURBO_STREAM, body))
    } else {
        super::message_features::redirect(c, &campfire_routes::searches(), None, None, true, None)
    }
}
// The existing root presenter/partial remains owned by WS8b-m. Supply search's
// show_room_icon local only on a cache miss; a shared fragment hit stays unchanged.
fn search_messages(
    p: &super::presenters::Presenter<'_>,
    messages: &[campfire_db::Message],
) -> campfire_db::Result<Vec<campfire_views::messages::MessageItem>> {
    if messages.is_empty() { return Ok(vec![]); }
    let p = p.preload_search(messages)?;
    messages.iter().map(|message| p.search_message_item(message)).collect()
}
fn section_view(
    section: campfire_db::search_query::SearchSection,
) -> campfire_views::searches::Section {
    let kind = section.kind;
    campfire_views::searches::Section {
        id: kind.into(),
        heading: match kind {
            "board-posts" => "Board posts",
            "work-threads" => "Work threads",
            _ => "Events",
        }
        .into(),
        records: section
            .records
            .into_iter()
            .map(|r| campfire_views::searches::SectionRow {
                title: r.title,
                path: if kind == "events" {
                    campfire_routes::room_event(r.room_id, r.id)
                } else {
                    campfire_routes::room_thread(r.room_id, r.id)
                },
                room_label: if r.room_type == "Rooms::Direct" {
                    "a direct message".into()
                } else {
                    r.room_name.unwrap_or_default()
                },
                time: r.time.jiff(),
                status: if kind == "events" {
                    r.cancelled.then(|| "Cancelled".into())
                } else {
                    r.status.map(|s| match s.as_str() {
                        "planned" => "Planned".into(),
                        "in_progress" => "In progress".into(),
                        "blocked" => "Blocked".into(),
                        "done" => "Done".into(),
                        _ => s,
                    })
                },
            })
            .collect(),
    }
}
#[cfg(test)]
mod ports;
