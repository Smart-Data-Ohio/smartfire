//! Action Text's record lookups for `campfire_richtext`, backed by the database: mention
//! attachments resolve to users through their signed (or, for users only, unverified) GlobalIDs
//! (`reference/lib/rails_ext/action_text_attachables.rb`).

use campfire_db::{Connection, User};
use campfire_richtext::{AttachableResolver, GidLookup, MentionUser, RenderContext, SignedLookup};
use rails_compat::Secrets;
use rails_compat::global_id::{self, ATTACHABLE_PURPOSE, GlobalId};
use std::{cell::RefCell, collections::HashMap};

use super::avatar_path;

pub struct DbResolver<'a> {
    pub conn: &'a Connection,
    pub secrets: &'a Secrets,
    pub now: jiff::Timestamp,
    twitter_existence: RefCell<HashMap<String, bool>>,
    shared_twitter_existence: Option<&'a RefCell<HashMap<String, bool>>>,
}

impl<'a> DbResolver<'a> {
    pub fn new(conn: &'a Connection, secrets: &'a Secrets, now: jiff::Timestamp) -> Self {
        Self {
            conn,
            secrets,
            now,
            twitter_existence: RefCell::default(),
            shared_twitter_existence: None,
        }
    }
    pub fn with_twitter_cache(
        conn: &'a Connection,
        secrets: &'a Secrets,
        now: jiff::Timestamp,
        cache: &'a RefCell<HashMap<String, bool>>,
    ) -> Self {
        let mut resolver = Self::new(conn, secrets, now);
        resolver.shared_twitter_existence = Some(cache);
        resolver
    }

    pub(crate) fn mention_user(&self, user: &User) -> MentionUser {
        MentionUser {
            id: user.id,
            name: user.name.clone(),
            title: user.title(),
            attachable_sgid: global_id::attachable_sgid(
                self.secrets,
                &GlobalId::new("User", user.id),
            ),
            user_path: campfire_routes::user(user.id),
            avatar_path: avatar_path(self.secrets, user),
        }
    }

    /// `GlobalID::Locator` finds records by the model's primary key; ids that aren't integers
    /// never match a row.
    fn find_user(&self, id: &str) -> Option<User> {
        let id: i64 = id.parse().ok()?;
        User::find_by_id(self.conn, id).ok().flatten()
    }

    pub fn render_context(&self, request_host: Option<String>) -> RenderContext<'_> {
        RenderContext {
            resolver: self,
            request_host,
        }
    }
}

impl campfire_richtext::markdown::IconResolver for DbResolver<'_> {
    fn find(&self, name: &str) -> Option<campfire_richtext::markdown::Icon> {
        use campfire_richtext::markdown::{Icon, IconCatalog};
        use campfire_views::{
            helpers::AvatarIcon,
            messages::reactions::{static_icon, static_icon_name},
        };
        use rusqlite::OptionalExtension;
        let name = name.trim().to_lowercase();
        let icon = static_icon(&name);
        if let Some(AvatarIcon::Image {
            title,
            url,
            brand: true,
        }) = icon
        {
            return Some(Icon::Brand {
                name: static_icon_name(&name).unwrap_or(name),
                title,
                url: Some(url),
            });
        }
        let title: Option<String> = self
            .conn
            .query_row(
                "SELECT title FROM workspace_icons WHERE name=?",
                [&name],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten();
        if let Some(title) = title {
            return Some(Icon::Custom {
                url: format!("/icons/{name}"),
                name,
                title,
            });
        }
        if let Some(AvatarIcon::Emoji { character, .. }) = icon {
            return Some(Icon::Emoji(character));
        }
        IconCatalog::default().find(&name)
    }
}

impl AttachableResolver for DbResolver<'_> {
    fn twitter_post_exists_for_url(&self, url: &str) -> bool {
        let Some(post) = crate::integrations::twitter::urls::extract(url)
            .into_iter()
            .next()
        else {
            return false;
        };
        let cache = self
            .shared_twitter_existence
            .unwrap_or(&self.twitter_existence);
        if let Some(exists) = cache.borrow().get(&post.post_id) {
            return *exists;
        }
        let exists = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM twitter_posts WHERE post_id=?)",
                [&post.post_id],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false);
        cache.borrow_mut().insert(post.post_id, exists);
        exists
    }

    fn embed_image_path(&self, url: &str) -> Result<String, campfire_richtext::Error> {
        Ok(crate::integrations::image_proxy::signed_path(
            self.secrets,
            url,
        ))
    }

    fn locate_signed(&self, sgid: &str) -> SignedLookup {
        let Some(gid) = global_id::locate_signed(self.secrets, sgid, ATTACHABLE_PURPOSE, self.now)
        else {
            return SignedLookup::Invalid;
        };
        match gid.model_name.as_str() {
            "User" => match self.find_user(&gid.id) {
                Some(user) => SignedLookup::User(self.mention_user(&user)),
                None => SignedLookup::MissingRecord {
                    model_name: gid.model_name,
                },
            },
            _ => SignedLookup::MissingRecord {
                model_name: gid.model_name,
            },
        }
    }

    fn find_gid(&self, gid: &str) -> GidLookup {
        let Some(gid) = GlobalId::parse(gid) else {
            return GidLookup::NotFound;
        };
        match gid.model_name.as_str() {
            "User" => match self.find_user(&gid.id) {
                Some(user) => GidLookup::User(self.mention_user(&user)),
                None => GidLookup::NotFound,
            },
            "Message" | "Room" | "Rooms::Open" | "Rooms::Closed" | "Rooms::Direct" | "Boost"
            | "Account" => GidLookup::OtherModel,
            _ => GidLookup::Raises,
        }
    }
}
