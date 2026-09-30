//! app/controllers/autocompletable/icons_controller.rb and Icons.search.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions};
use crate::controllers::presenters::page::db_error;
use campfire_kit::{Ctx, Param, Result, StatusCode, format};
use campfire_richtext::markdown::Icon;
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap},
    sync::LazyLock,
};
#[derive(Clone, Serialize)]
struct Suggestion {
    name: String,
    title: String,
    kind: &'static str,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    character: Option<String>,
}
fn normalize(raw: &str) -> String {
    campfire_richtext::ruby::strip(raw).to_lowercase()
}
fn rank(name: &str, query: &str) -> Option<u8> {
    if name == query {
        Some(0)
    } else if name.starts_with(query) {
        Some(1)
    } else if name.contains(query) {
        Some(2)
    } else {
        None
    }
}
fn image(name: &str, title: &str, kind: &'static str, url: Option<String>) -> Suggestion {
    Suggestion {
        name: name.into(),
        title: title.into(),
        kind,
        value: format!("{kind}:{name}"),
        image: url,
        character: None,
    }
}
fn suggestions(
    conn: &campfire_db::Connection,
    query: &str,
    custom: bool,
) -> campfire_db::Result<Vec<Suggestion>> {
    let catalog = crate::rich_text::icons(conn).map_err(campfire_db::Error::Other)?;
    let mut custom_entries = catalog
        .custom
        .values()
        .filter_map(|i| {
            if let Icon::Custom { name, title, url } = i {
                Some(image(name, title, "custom", Some(url.clone())))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    custom_entries.sort_by(|a, b| a.name.cmp(&b.name));
    if custom {
        return Ok(custom_entries);
    }
    let query = normalize(query);
    if query.is_empty() {
        return Ok(vec![]);
    }
    let mut best: BTreeMap<String, (u8, Suggestion)> = BTreeMap::new();
    for (alias, record) in catalog.brands {
        if let Icon::Brand { name, title, url } = record
            && let Some(rank) = rank(&alias, &query)
        {
            let suggestion = image(&name, &title, "brand", url);
            let entry = best.entry(name).or_insert((rank, suggestion.clone()));
            if rank < entry.0 {
                *entry = (rank, suggestion);
            }
        }
    }
    let mut matches = best.into_values().collect::<Vec<_>>();
    matches.extend(
        custom_entries
            .into_iter()
            .filter_map(|s| rank(&s.name, &query).map(|r| (r, s))),
    );
    static EMOJI: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../../../../richtext/data/gemoji-4.1.0.json"))
            .expect("pinned gemoji aliases")
    });
    matches.extend(EMOJI.iter().filter_map(|(name, character)| {
        rank(name, &query).map(|r| {
            let mut title = name.replace('_', " ");
            if let Some(ch) = title.chars().next() {
                title = ch.to_uppercase().to_string() + &title[ch.len_utf8()..].to_lowercase();
            }
            (
                r,
                Suggestion {
                    name: name.clone(),
                    title,
                    kind: "emoji",
                    value: format!("emoji:{name}"),
                    image: None,
                    character: Some(character.clone()),
                },
            )
        })
    }));
    matches.sort_by(|(ar, a), (br, b)| {
        ar.cmp(br)
            .then_with(|| (a.kind == "emoji").cmp(&(b.kind == "emoji")))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(matches.into_iter().take(8).map(|(_, s)| s).collect())
}
pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let custom = c.param("custom").is_some_and(Param::is_present);
    let query = c
        .param("q")
        .filter(|p| p.is_present())
        .or_else(|| c.param("query"))
        .and_then(Param::to_s)
        .unwrap_or_default();
    let icons = c
        .app()
        .db
        .read(move |conn| suggestions(conn, &query, custom))
        .await
        .map_err(db_error)?;
    c.respond_to(&[&format::JSON])?;
    Ok(c.render(
        StatusCode::OK,
        &format::JSON,
        campfire_views::helpers::to_rails_json(&icons),
    ))
}
