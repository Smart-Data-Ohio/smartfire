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

pub struct Preloads {
    pub threads: Option<campfire_db::models::message_rendering::ThreadRenderingRecords>,
    pub records: RenderingRecords,
    pub users: HashMap<i64, RenderingUser>,
    pub attachments: HashMap<i64, campfire_storage::blob::MessageAttachments>,
    pub previewed_blobs: std::collections::HashSet<i64>,
    pub icons: IconCatalog,
    pub custom_icons: HashMap<String, String>,
}
impl Preloads {
    /// Pin excerpts use the same plain-text resolver, without provider/cache/card facts.
    pub fn load_plain_text(p: &Presenter<'_>, messages: &[Message]) -> Result<Self> {
        let records = RenderingRecords::load_plain_text(p.conn, messages)?;
        let ids: Vec<_> = messages.iter().map(|m| m.id).collect();
        let mentions: Vec<_> = records.bodies.values().flatten().flat_map(|b| mention_ids(b,0))
            .chain(messages.iter().map(|m|m.creator_id)).collect();
        let users = User::where_ids(p.conn,&mentions)?.into_iter().map(|user| {
            let id=user.id;
            let icon_name=user.icon_name.clone();
            (id,RenderingUser { user, uploaded_avatar:false, icon_name })
        }).collect();
        let attachments=campfire_storage::Blob::attached_messages(p.conn,&ids)
            .map_err(super::storage_error)?;
        let icons=crate::rich_text::icons(p.conn).map_err(campfire_db::Error::Other)?;
        Ok(Self { records,users,attachments,icons,previewed_blobs:Default::default(),threads:None,custom_icons:Default::default() })
    }
    pub fn load_payload(p: &Presenter<'_>, messages: &[Message]) -> Result<Self> {
        let records = RenderingRecords::load_payload(p.conn, messages)?;
        let mut mentions = records
            .bodies
            .values()
            .flatten()
            .flat_map(|body| mention_ids(body, 0))
            .collect::<Vec<_>>();
        let threads = {
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
        };
        if let Some(threads) = &threads {
            mentions.extend(threads.user_ids(p.current_user_id.unwrap_or(0)));
        }
        let users = records.users(p.conn, messages, &mentions)?;
        let attachments =
            campfire_storage::Blob::attached_messages(p.conn, &records.body_ids(messages))
                .map_err(super::storage_error)?;
        let videos = attachments.values().flat_map(|files| &files.blobs).filter(|blob| blob.is_video()).map(|blob| blob.id).collect::<Vec<_>>();
        let previewed_blobs = campfire_storage::Blob::previewed_blob_ids(p.conn, &videos)
            .map_err(super::storage_error)?;
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
        Ok(Self {
            threads,
            records,
            users,
            attachments,
            previewed_blobs,
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
                .map(|files| files.blobs.iter().map(|blob| blob.filename.to_string()).collect::<Vec<_>>().join(", "))
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
}
// Stored ActionText nodes are canonicalized, but also scan Trix JSON and nested content.
// Decoding is the established, User-only unverified-SGID fallback, never Marshal loading.
pub fn mention_ids(body: &str, depth: usize) -> Vec<i64> {
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
pub struct PageResolver<'a> {
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
