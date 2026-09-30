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

pub(super) fn events(
    data: &RenderingRecords,
    message: &Message,
) -> Vec<campfire_views::events::CardView> {
    data.event_cards
        .get(&message.id)
        .into_iter()
        .flatten()
        .map(|card| {
            let meet_link = card
                .meet_link
                .as_deref()
                .filter(|url| {
                    url.parse::<axum::http::Uri>().ok().is_some_and(|uri| {
                        uri.scheme_str() == Some("https")
                            && uri.host().is_some_and(|host| !host.is_empty())
                    })
                })
                .map(str::to_owned);
            campfire_views::events::CardView {
                id: card.id,
                room_id: card.room_id,
                title: card.title.clone(),
                organizer_name: card.organizer.clone(),
                starts_at: card.starts_at.jiff(),
                ends_at: card.ends_at.map(|t| t.jiff()),
                time_zone: card.time_zone.clone(),
                series: card.series,
                cancelled: card.cancelled,
                venue_name: card.venue.clone(),
                meet_link,
            }
        })
        .collect()
}

pub(super) fn twitter(
    data: &RenderingRecords,
    message: &Message,
) -> Vec<campfire_views::twitter::Card> {
    let present = |s: &Option<String>| {
        s.as_ref()
            .filter(|s| !campfire_richtext::ruby::is_blank(s))
            .cloned()
    };
    data.twitter_posts.get(&message.id).into_iter().flatten().map(|post| {
        static URL:LazyLock<Regex>=LazyLock::new(||Regex::new(r"https?://(?:www\.|mobile\.)?(?:twitter\.com|x\.com)/(?:i/(?:web/)?status/|(?P<handle>[A-Za-z0-9_]{1,15})/status(?:es)?/)(?P<id>[0-9]{1,25})\b").unwrap());
        let display_handle=present(&post.author_handle).or_else(||post.url.as_deref().and_then(|url|URL.captures(url)).and_then(|c|c.name("handle").map(|h|h.as_str().to_owned())));
        campfire_views::twitter::Card {
            post_id:post.post_id.clone(),
            view_url:present(&post.url).unwrap_or_else(||format!("https://x.com/i/status/{}",post.post_id)),
            display_name:present(&post.author_name).unwrap_or_else(||display_handle.as_ref().map_or_else(||"Post on X".into(),|h|format!("@{h}"))),
            profile_url:display_handle.as_ref().map(|h|format!("https://x.com/{h}")),display_handle,
            author_avatar_url:post.author_avatar_url.clone(),text:post.text.clone(),posted_at:post.posted_at.map(|t|t.jiff()),
            replies:post.replies,reposts:post.reposts,likes:post.likes,media:post.media.clone(),quote:post.quote.clone(),
            fetched_at:post.fetched_at.map(|t|t.jiff()),fetch_error:post.fetch_error.clone(),
            logo_url:Some("icons/brands/x.svg".into()),
        }
    }).collect()
}
