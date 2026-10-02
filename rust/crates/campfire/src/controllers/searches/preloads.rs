//! WS8bm2 adapter for the shared WS8b-m presenter. Domain reads carry no HTML.
use crate::controllers::presenters::{DbResolver, Presenter};
use campfire_db::{
    Message, Result, User,
    models::message_rendering::{RenderingRecords, RenderingUser},
};
use campfire_richtext::markdown::{self, IconCatalog};
use campfire_richtext::{AttachableResolver, GidLookup, MentionUser, RenderContext, SignedLookup};
use rails_compat::global_id::{self, ATTACHABLE_PURPOSE, GlobalId};
use std::collections::HashMap;

pub(crate) struct GithubRendering {
    pub html: String,
    pub stamp: String,
    pub refreshes: Vec<i64>,
}

pub(crate) struct Preloads {
    pub threads: Option<campfire_db::models::message_rendering::ThreadRenderingRecords>,
    pub github: HashMap<i64, GithubRendering>,
    pub event_views: HashMap<i64, Vec<campfire_views::events::CardView>>,
    pub records: RenderingRecords,
    pub cache: crate::controllers::presenters::message_cache_preloads::CacheFacts,
    pub users: HashMap<i64, RenderingUser>,
    pub attachments: HashMap<i64, campfire_storage::Blob>,
    pub icons: IconCatalog,
    pub custom_icons: HashMap<String, String>,
    // Shared persisted provider facts, loaded before any message/quote rendering.
    pub fizzy_cards: HashMap<i64, Vec<crate::integrations::fizzy::cards::Card>>,
    pub link_references: HashMap<i64, Vec<crate::integrations::link_embed::Reference>>,
}
impl Preloads {
    pub fn load(p: &Presenter<'_>, messages: &[Message]) -> Result<Self> {
        Self::load_for(p, messages, false)
    }
    pub fn load_payload(p: &Presenter<'_>, messages: &[Message]) -> Result<Self> {
        Self::load_for(p, messages, true)
    }
    fn load_for(p: &Presenter<'_>, messages: &[Message], payload: bool) -> Result<Self> {
        let records = if payload {
            RenderingRecords::load_payload(p.conn, messages)?
        } else {
            RenderingRecords::load(p.conn, messages)?
        };
        let mut mentions = records
            .bodies
            .values()
            .flatten()
            .flat_map(|body| mention_ids(body, 0))
            .collect::<Vec<_>>();
        let threads = if payload {
            let all = messages
                .iter()
                .chain(records.sources.values())
                .cloned()
                .collect::<Vec<_>>();
            Some(
                campfire_db::models::message_rendering::ThreadRenderingRecords::load(
                    p.conn,
                    &all,
                    p.current_user_id.unwrap_or(0),
                )?,
            )
        } else {
            None
        };
        if let Some(threads) = &threads {
            mentions.extend(threads.user_ids(p.current_user_id.unwrap_or(0)));
        }
        let users = records.users(p.conn, messages, &mentions)?;
        let attachments =
            campfire_storage::Blob::attached_messages(p.conn, &records.body_ids(messages))
                .map_err(super::super::presenters::storage_error)?;
        let icons = crate::rich_text::icons(p.conn).map_err(campfire_db::Error::Other)?;
        let custom_icons = icons
            .custom
            .iter()
            .filter_map(|(name, icon)| {
                if let markdown::Icon::Custom { title, .. } = icon {
                    Some((name.clone(), title.clone()))
                } else {
                    None
                }
            })
            .collect();
        let ids = records.body_ids(messages);
        let fizzy_cards = if payload {
            HashMap::new()
        } else {
            crate::integrations::fizzy::cards::Card::for_messages(p.conn, &ids)?
        };
        let link_references = if payload {
            HashMap::new()
        } else {
            crate::integrations::link_embed::Reference::for_messages(p.conn, &ids)?
        };
        let page_messages: Vec<_> = messages
            .iter()
            .chain(records.sources.values())
            .cloned()
            .collect();
        let event_views = if payload {
            HashMap::new()
        } else {
            crate::controllers::presenters::events::for_messages(p.conn, &page_messages)?
        };
        // Use the owner's presentation/refresh policy with page-scoped persisted facts.
        // Preloading does not schedule refreshes; an actual fragment miss does.
        let mut github = HashMap::new();
        let linked = if payload || ids.is_empty() {
            Vec::new()
        } else {
            let sql = format!(
                "SELECT DISTINCT message_id FROM github_pull_request_references WHERE message_id IN ({})",
                std::iter::repeat_n("?", ids.len())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            p.conn
                .prepare(&sql)?
                .query_map(rusqlite::params_from_iter(&ids), |r| r.get::<_, i64>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        let prs =
            crate::integrations::github::pull_requests::PullRequest::for_messages(p.conn, &linked)?;
        let public_messages: Vec<_> = linked.iter().copied().filter(|id|
            prs.get(id).is_some_and(|cards| cards.iter().any(|pr| pr.private == Some(false)))
        ).collect();
        let mut discussions = HashMap::new();
        for ids in public_messages.chunks(crate::integrations::message_batches::SIZE) {
            discussions.extend(crate::integrations::github::threads::PullRequestThread::for_messages(p.conn, ids)?);
        }
        let account = if linked.is_empty() {
            None
        } else {
            campfire_db::Account::first(p.conn)?
        };
        for id in &linked {
            let message = messages
                .iter()
                .find(|m| m.id == *id)
                .or_else(|| records.sources.get(id));
            if let Some(message) = message {
                let cards = prs.get(id).map(Vec::as_slice).unwrap_or_default();
                let views = cards
                    .iter()
                    .map(|pr| {
                        crate::controllers::presenters::github::shared_card_in_discussion(
                            pr,
                            discussions.get(&(message.room_id, pr.id)).copied(),
                        )
                    })
                    .collect::<Result<Vec<_>>>()?;
                let html = crate::controllers::presenters::github::render_message_cards(
                    p.app(),
                    message,
                    &p.render_zone,
                    account.as_ref(),
                    &views,
                );
                let stamp = cards
                    .iter()
                    .map(|pr| pr.updated_at)
                    .max()
                    .map(|newest| {
                        format!(
                            "github:{}:{}",
                            newest.to_db(),
                            records
                                .pr_thread_stamps
                                .get(&message.room_id)
                                .map(|t| t.to_db())
                                .unwrap_or_default()
                        )
                    })
                    .unwrap_or_default();
                let refreshes = cards
                    .iter()
                    .filter(|pr| pr.stale(campfire_db::Timestamp::from_jiff(p.now)))
                    .map(|pr| pr.id)
                    .collect();
                github.insert(
                    *id,
                    GithubRendering {
                        html,
                        stamp,
                        refreshes,
                    },
                );
            }
        }
        let cache = if payload {
            Default::default()
        } else {
            crate::controllers::presenters::message_cache_preloads::CacheFacts::load(
                p, messages, &records,
            )?
        };
        Ok(Self {
            cache,
            threads,
            github,
            event_views,
            fizzy_cards,
            link_references,
            records,
            users,
            attachments,
            icons,
            custom_icons,
        })
    }
    pub fn plain_text(&self, p: &Presenter<'_>, m: &Message) -> Result<String> {
        let resolver = p.resolver();
        let ctx = resolver.render_context(p.request_host.clone());
        let mut text = if let Some(body) = self.records.bodies.get(&m.id).and_then(Option::as_deref)
        {
            if m.markdown() {
                markdown::plain_text(body, &ctx, &self.icons)
            } else {
                campfire_richtext::to_plain_text(body, &ctx)
            }
            .map_err(|e| campfire_db::Error::Other(e.to_string()))?
        } else {
            String::new()
        };
        if text.trim().is_empty() {
            text = self
                .attachments
                .get(&m.id)
                .map(|b| b.filename.to_string())
                .unwrap_or_default();
        }
        Ok(
            match m.forward_note.as_deref().filter(|s| !s.trim().is_empty()) {
                Some(note) if text.trim().is_empty() => note.to_owned(),
                Some(note) => format!("{note}\n\n{text}"),
                None => text,
            },
        )
    }
    pub fn details(
        &self,
        p: &Presenter<'_>,
        m: &Message,
    ) -> Result<campfire_views::messages::MessageDetails> {
        use campfire_views::messages::{
            MessageDetails, ReplyPreview, ReplySource,
            parts::{AgentStep, Poll, PollOption, PollVote},
        };
        let reply = if m.reply_to_message_id.is_some() || m.reply_target_deleted_at.is_some() {
            let source = m
                .reply_to_message_id
                .and_then(|id| self.records.sources.get(&id))
                .map(|source| -> Result<ReplySource> {
                    let url = if let Some(thread) = source.thread_id {
                        campfire_routes::ROOM.path_with(
                            &[&source.room_id],
                            None,
                            &[
                                ("thread", Some(&thread.to_string())),
                                ("message_id", Some(&source.id.to_string())),
                            ],
                        )
                    } else {
                        campfire_routes::room_at_message(source.room_id, source.id)
                    };
                    Ok(ReplySource {
                        id: source.id,
                        author: p.user(source.creator_id)?.name,
                        plain_text: self.plain_text(p, source)?,
                        url,
                    })
                })
                .transpose()?;
            Some(ReplyPreview { source })
        } else {
            None
        };
        let poll = self.records.polls.get(&m.id).map(|poll| Poll {
            id: poll.id,
            room_id: m.room_id,
            anonymous: poll.anonymous,
            multiple: poll.multiple,
            closed: poll.closed_at.is_some() || poll.closes_at.is_some_and(|t| t.jiff() <= p.now),
            closes_at: poll.closes_at.map(|t| t.jiff()),
            options: self
                .records
                .options
                .get(&poll.id)
                .into_iter()
                .flatten()
                .map(|o| PollOption {
                    id: o.id,
                    label: o.label.clone(),
                })
                .collect(),
            votes: self
                .records
                .votes
                .get(&poll.id)
                .into_iter()
                .flatten()
                .map(|(v, name)| PollVote {
                    option_id: v.poll_option_id,
                    user_id: v.user_id,
                    user_name: name.clone(),
                })
                .collect(),
            vote_error: None,
        });
        Ok(MessageDetails {
            thread_id: m.thread_id,
            system_note: m.system_note,
            action: m.action,
            edited_at: m.edited_at.map(|t| t.jiff()),
            streaming: m.streaming,
            markdown: m.markdown_source.is_some(),
            forwarded: m.forwarded_at.is_some(),
            forward_note: m.forward_note.clone(),
            pinned: self.records.pinned.contains(&m.id),
            reply_count: self.records.reply_counts.get(&m.id).copied().unwrap_or(0),
            drive_urls: self
                .records
                .drive_files
                .get(&m.id)
                .into_iter()
                .flatten()
                .map(|s| format!("https://drive.google.com/open?id={s}"))
                .collect(),
            reply,
            poll,
            agent_steps: self
                .records
                .steps
                .get(&m.id)
                .into_iter()
                .flatten()
                .map(|s| AgentStep {
                    name: s.name.clone(),
                    status: s.status.clone(),
                    duration_ms: s.duration_ms,
                    input_summary: s.input_summary.clone(),
                    output_summary: s.output_summary.clone(),
                })
                .collect(),
            ..Default::default()
        })
    }
}
// Stored ActionText nodes are canonicalized, but also scan Trix JSON and nested content.
// Decoding is the established, User-only unverified-SGID fallback, never Marshal loading.
pub(crate) fn mention_ids(body: &str, depth: usize) -> Vec<i64> {
    if depth > 32 {
        return vec![];
    }
    let Ok(content) = campfire_richtext::Content::wrap(body) else {
        return vec![];
    };
    let mut ids = vec![];
    for node in content.dom.descendants(content.root) {
        if let Some(sgid) = content.dom.attr(node, "sgid") {
            add_mention(&mut ids, sgid);
        }
        if let Some(html) = content.dom.attr(node, "content") {
            ids.extend(mention_ids(html, depth + 1));
        }
        if let Some(json) = content
            .dom
            .attr(node, "data-trix-attachment")
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        {
            if let Some(sgid) = json["sgid"].as_str() {
                add_mention(&mut ids, sgid);
            }
            if let Some(html) = json["content"].as_str() {
                ids.extend(mention_ids(html, depth + 1));
            }
        }
    }
    ids
}
fn add_mention(ids: &mut Vec<i64>, sgid: &str) {
    if let Ok(Some(gid)) = global_id::unverified_attachable_user(Some(sgid))
        && let Ok(id) = gid.id.parse()
    {
        ids.push(id);
    }
}
/// Uses exactly the established resolver for non-preloaded presenters and non-User GIDs.
pub(crate) struct PageResolver<'a> {
    pub db: DbResolver<'a>,
    pub preloads: Option<&'a Preloads>,
}
impl PageResolver<'_> {
    pub fn render_context(&self, request_host: Option<String>) -> RenderContext<'_> {
        RenderContext {
            resolver: self,
            request_host,
        }
    }
    fn user(&self, id: &str) -> Option<&User> {
        self.preloads?
            .users
            .get(&id.parse::<i64>().ok()?)
            .map(|r| &r.user)
    }
    fn mention(&self, user: &User) -> MentionUser {
        self.db.mention_user(user)
    }
}
impl AttachableResolver for PageResolver<'_> {
    fn twitter_post_exists_for_url(&self, url: &str) -> bool {
        self.db.twitter_post_exists_for_url(url)
    }

    fn embed_image_path(&self, url: &str) -> std::result::Result<String, campfire_richtext::Error> {
        self.db.embed_image_path(url)
    }
    fn locate_signed(&self, sgid: &str) -> SignedLookup {
        if self.preloads.is_none() {
            return self.db.locate_signed(sgid);
        }
        let Some(gid) =
            global_id::locate_signed(self.db.secrets, sgid, ATTACHABLE_PURPOSE, self.db.now)
        else {
            return SignedLookup::Invalid;
        };
        if gid.model_name == "User"
            && let Some(user) = self.user(&gid.id)
        {
            return SignedLookup::User(self.mention(user));
        }
        SignedLookup::MissingRecord {
            model_name: gid.model_name,
        }
    }
    fn find_gid(&self, gid: &str) -> GidLookup {
        if self.preloads.is_some()
            && let Some(parsed) = GlobalId::parse(gid)
            && parsed.model_name == "User"
        {
            return self
                .user(&parsed.id)
                .map(|u| GidLookup::User(self.mention(u)))
                .unwrap_or(GidLookup::NotFound);
        }
        self.db.find_gid(gid)
    }
}
