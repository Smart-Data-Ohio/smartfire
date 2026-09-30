//! WS8bm2 read-only provider adapters. WS15 retains fetch/write and private-card endpoints.
use campfire_db::{Message, models::message_rendering::RenderingRecords};
use campfire_views::message_providers::{EmbedEntry, GithubEntry};
use regex::Regex;
use std::sync::LazyLock;

pub(super) fn github(data: &RenderingRecords, message: &Message) -> Vec<GithubEntry> {
    let frame = |id: i64, _provider: &str, _param: &str, _class: &str, _indent: &str| {
        let html = campfire_views::helpers::turbo_frame_tag(
            &format!("card_for_message_{}_github_pull_request_{id}", message.id),
            Some(&format!(
                "/rooms/{}/github/pull_requests/{id}/card?message_id={}",
                message.room_id, message.id
            )),
            None,
            campfire_views::helpers::attrs()
                .attr("loading", "lazy")
                .class("github-pr-card-frame"),
            "",
        )
        .0;
        format!("\n    {html}\n")
    };
    data.providers
        .github
        .get(&message.id)
        .into_iter()
        .flatten()
        .map(|card| {
            use campfire_views::message_providers::{GithubCard, GithubEntry};
            if card.private != Some(false) {
                return GithubEntry::Private(frame(
                    card.id,
                    "github/pull_requests",
                    "github_pull_request",
                    "github-pr-card-frame",
                    "    ",
                ));
            }
            let full_name = format!("{}/{}", card.owner, card.repo);
            let payload = card
                .payload
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
            let cased = payload
                .as_ref()
                .and_then(|v| v.pointer("/base/repo/full_name"))
                .and_then(|v| v.as_str())
                .filter(|s| {
                    let pieces = s.split('/').collect::<Vec<_>>();
                    pieces.len() == 2
                        && pieces
                            .iter()
                            .all(|s| !s.is_empty() && !s.chars().any(char::is_whitespace))
                });
            let display_name = cased
                .map(str::to_owned)
                .or_else(|| {
                    static NAME: LazyLock<Regex> =
                        LazyLock::new(|| Regex::new(r"github\.com/([^/\s]+/[^/\s]+)").unwrap());
                    card.html_url
                        .as_deref()
                        .and_then(|s| NAME.captures(s))
                        .map(|c| c[1].to_owned())
                })
                .unwrap_or_else(|| full_name.clone());
            GithubEntry::Public(Box::new(GithubCard {
                id: card.id,
                display_name,
                number: card.number,
                state: card.state.clone().unwrap_or_else(|| "unknown".into()),
                title: card.title.clone().unwrap_or_default(),
                author: card.author_login.clone().unwrap_or_default(),
                avatar: card.author_avatar_url.clone().unwrap_or_default(),
                base: card.base_branch.clone().unwrap_or_default(),
                head: card.head_branch.clone().unwrap_or_default(),
                review: card.review_decision.clone().unwrap_or_default(),
                checks: card.check_status.clone().unwrap_or_else(|| "none".into()),
                updated_at: card.github_updated_at.map(|t| t.jiff()),
                url: card.html_url.clone().unwrap_or_else(|| {
                    format!("https://github.com/{full_name}/pull/{}", card.number)
                }),
                full_name,
                error: card.fetch_error.clone().unwrap_or_default(),
                thread_id: card.discussion_thread_id,
            }))
        })
        .collect()
}

pub(super) fn embeds(data: &RenderingRecords, message: &Message) -> Vec<EmbedEntry> {
    if message.embeds_suppressed {
        Vec::new()
    } else {
        data.providers.embeds.get(&message.id).into_iter().flatten().map(|card| {
                use campfire_views::message_providers::{EmbedCard,EmbedEntry};
                static POST:LazyLock<Regex>=LazyLock::new(||Regex::new(r#"https?://(?:www\.)?linkedin\.com/(?:posts/[^/?#\s<>"'()\]]+|feed/update/(urn:li:(?:activity|share|ugcPost):[0-9]+)(?:$|[/?#\s<>"'()\]\.,;:!?}]))"#).unwrap());
                let player_url=POST.captures(&card.url).and_then(|c|c.get(1)).map(|urn|format!("https://www.linkedin.com/embed/feed/update/{}",urn.as_str()));
                EmbedEntry { linkedin:POST.is_match(&card.normalized_url),card:EmbedCard { url:card.url.clone(),title:card.title.clone().unwrap_or_default(),description:card.description.clone().unwrap_or_default(),site:card.site_name.clone().unwrap_or_default(),image:card.image_url.clone().unwrap_or_default(),player_url } }
            }).collect()
    }
}
