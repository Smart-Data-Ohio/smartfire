//! `campfire_db::RichText` over `campfire_richtext`, for the models' Action Text needs
//! (`plain_text_body` for FTS, push and webhooks; `mentionees`).
//!
//! The models call this on the database writer thread inside their transaction, or on a reader
//! they hold. Record lookups use that same connection with the one resolver implementation the
//! controllers use (`controllers::presenters::DbResolver`): checking out another connection here
//! deadlocks once every pooled reader is waiting on the writer.

use std::sync::{Arc,LazyLock};

use campfire_db::{BasicRichText, Connection, RichText};
use campfire_kit::SharedClock;
use campfire_richtext::{RenderContext,AttachableResolver,GidLookup};
use campfire_richtext::markdown::{self,Icon,IconCatalog};
use serde::Deserialize;
use rails_compat::Secrets;

use crate::controllers::presenters::DbResolver;

pub struct AppRichText {
    secrets: Arc<Secrets>,
    clock: SharedClock,
}

impl AppRichText {
    pub fn new(secrets: Arc<Secrets>, clock: SharedClock) -> Self {
        Self { secrets, clock }
    }

    fn with_context<T>(&self, conn: &Connection, f: impl FnOnce(&RenderContext) -> T) -> T {
        let resolver = DbResolver { conn, secrets: &self.secrets, now: self.clock.now() };
        f(&resolver.render_context(None))
    }
}

#[derive(Deserialize)]
struct Brand {
    name: String,
    title: String,
    file: String,
    #[serde(default)]
    aliases: Vec<String>,
}
static BRANDS: LazyLock<Vec<Brand>> = LazyLock::new(|| {
    serde_yaml::from_str(include_str!(env!("CAMPFIRE_ICON_CONFIG")))
        .expect("reference config/icons.yml")
});
fn icons(conn: &Connection) -> Result<IconCatalog, String> {
    let mut icons = IconCatalog::default();
    for brand in BRANDS.iter() {
        let icon = Icon::Brand {
            name: brand.name.clone(),
            title: brand.title.clone(),
            url: campfire_assets::try_asset_path(&format!("icons/brands/{}", brand.file)).ok(),
        };
        for alias in std::iter::once(&brand.name).chain(&brand.aliases) {
            icons.brands.insert(alias.clone(), icon.clone());
        }
    }
    let mut stmt = conn
        .prepare_cached("SELECT name,title FROM workspace_icons")
        .map_err(|e| e.to_string())?;
    let custom = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    for (name, title) in custom {
        icons.custom.insert(
            name.clone(),
            Icon::Custom {
                url: format!("/icons/{name}"),
                name,
                title,
            },
        );
    }
    Ok(icons)
}

impl RichText for AppRichText {
    fn render_markdown(
        &self,
        conn: &Connection,
        source: &str,
        room_id: i64,
    ) -> Result<String, String> {
        let mut stmt=conn.prepare_cached("SELECT users.id,users.name FROM users JOIN memberships ON memberships.user_id=users.id WHERE memberships.room_id=? AND users.status=0").map_err(|e|e.to_string())?;
        let members = stmt
            .query_map([room_id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let resolver = DbResolver {
            conn,
            secrets: &self.secrets,
            now: self.clock.now(),
        };
        let mentions = |name: &str| {
            let mut matches = members
                .iter()
                .filter(|(_, member_name)| member_name == name);
            let (id, _) = matches.next()?;
            if matches.next().is_some() {
                return None;
            }
            match resolver.find_gid(&format!("gid://campfire/User/{id}")) {
                GidLookup::User(user) => Some(user),
                _ => None,
            }
        };
        markdown::render(source, &mentions, &icons(conn)?).map_err(|e| e.to_string())
    }
    fn canonicalize_html(&self, conn: &Connection, html: &str) -> String {
        match self.with_context(conn, |ctx| {
            campfire_richtext::Content::load(html, ctx).map(|content| content.to_html())
        }) {
            Ok(html) => html,
            Err(error) => {
                tracing::error!(%error,"canonicalize_html raised");
                html.into()
            }
        }
    }
    fn markdown_plain_text(
        &self,
        conn: &Connection,
        html: &str,
        names: campfire_db::rich_text::UserNames<'_>,
    ) -> String {
        let text = icons(conn).and_then(|icons| {
            self.with_context(conn, |ctx| markdown::plain_text(html, ctx, &icons))
                .map_err(|e| e.to_string())
        });
        match text {
            Ok(text) => text,
            Err(error) => {
                tracing::error!(%error,"markdown_plain_text raised");
                self.to_plain_text(conn, html, names)
            }
        }
    }


    /// `message.body.to_plain_text`. Where Rails would raise, the save would fail; the models
    /// can't fail here, so it's logged and the tag-stripped text is used instead.
    fn to_plain_text(&self, conn: &Connection, html: &str, user_names: campfire_db::rich_text::UserNames<'_>) -> String {
        match self.with_context(conn, |ctx| campfire_richtext::to_plain_text(html, ctx)) {
            Ok(text) => text,
            Err(error) => {
                tracing::error!(%error, "to_plain_text raised");
                BasicRichText.to_plain_text(conn, html, user_names)
            }
        }
    }

    /// `body.attachables.grep(User).uniq`: verified SGIDs only.
    fn mentioned_user_ids(&self, conn: &Connection, html: &str) -> Vec<i64> {
        match self.with_context(conn, |ctx| campfire_richtext::mentioned_users(html, ctx)) {
            Ok(users) => users.into_iter().map(|user| user.id).collect(),
            Err(error) => {
                tracing::error!(%error, "mentioned_users raised");
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use campfire_db::{Config, Database, Env, Timestamp, fixtures};
    use serde_json::Value;
    fn golden() -> Value {
        serde_json::from_str(include_str!("ws8_runtime_vectors.json")).unwrap()
    }
    fn fixture() -> (Database, AppRichText, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db =
            Database::open(Config::new(dir.path().join("db.sqlite3")), Env::default()).unwrap();
        let now = Timestamp::parse_db("2026-03-10 12:00:00").unwrap();
        let custom = golden()["custom"].clone();
        db.write_blocking(move|tx|{
            fixtures::load(tx.conn(),&fixtures::reference_dir(),&fixtures::Options{now,bcrypt_cost:4})?;
            tx.conn().execute("INSERT INTO workspace_icons(id,name,title,creator_id,created_at,updated_at) VALUES(?,?,?,?,?,?)",rusqlite::params![custom["id"].as_i64().unwrap(),custom["name"].as_str().unwrap(),custom["title"].as_str().unwrap(),fixtures::identify("david"),now,now])?;
            Ok(())
        }).unwrap();
        // The reference parity key is test-only and committed in parity/.env.reference.
        let secrets = rails_compat::Secrets::new(
            "5335c3b1ad35b4ad170c3413bd651ef3b6ed64e257261871a6de3f978cf3868ee417a927040935fb30b0f7debdedb34a2a403e9f34b16cf594c917c2ecd4a995",
        );
        let rich = AppRichText::new(
            Arc::new(secrets),
            Arc::new(campfire_kit::clock::FrozenClock::new(now.jiff())),
        );
        (db, rich, dir)
    }
    #[test]
    fn runtime_markdown_matches_rails_with_member_sgids_and_icons() {
        let (db, rich, _dir) = fixture();
        for row in golden()["markdown"].as_array().unwrap() {
            let actual = db
                .read_blocking(|conn| {
                    Ok(rich
                        .render_markdown(
                            conn,
                            row["source"].as_str().unwrap(),
                            fixtures::identify("designers"),
                        )
                        .unwrap())
                })
                .unwrap();
            assert_eq!(actual, row["body"].as_str().unwrap(), "{}", row["source"]);
        }
    }
    #[test]
    fn runtime_canonicalization_matches_rails() {
        let (db, rich, _dir) = fixture();
        for row in golden()["canonical"].as_array().unwrap() {
            let actual = db
                .read_blocking(|conn| {
                    Ok(rich.canonicalize_html(conn, row["input"].as_str().unwrap()))
                })
                .unwrap();
            assert_eq!(actual, row["output"].as_str().unwrap());
        }
    }
    #[test]
    fn runtime_markdown_plain_text_matches_rails() {
        let (db, rich, _dir) = fixture();
        for row in golden()["markdown"].as_array().unwrap() {
            let actual = db
                .read_blocking(|conn| {
                    Ok(rich.markdown_plain_text(conn, row["body"].as_str().unwrap(), &|_| None))
                })
                .unwrap();
            assert_eq!(actual, row["plain"].as_str().unwrap(), "{}", row["source"]);
        }
    }
}
