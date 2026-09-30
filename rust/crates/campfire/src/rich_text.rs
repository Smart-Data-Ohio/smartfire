//! `campfire_db::RichText` over `campfire_richtext`, for the models' Action Text needs
//! (`plain_text_body` for FTS, push and webhooks; `mentionees`).
//!
//! The models call this on the database writer thread inside their transaction, or on a reader
//! they hold. Record lookups use that same connection with the one resolver implementation the
//! controllers use (`controllers::presenters::DbResolver`): checking out another connection here
//! deadlocks once every pooled reader is waiting on the writer.

use std::sync::{Arc, LazyLock};

use campfire_db::{BasicRichText, Connection, RichText};
use campfire_kit::SharedClock;
use campfire_richtext::markdown::{self, Icon, IconCatalog};
use campfire_richtext::{AttachableResolver, GidLookup, RenderContext};
use rails_compat::Secrets;
use serde::Deserialize;

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
        let resolver = DbResolver {
            conn,
            secrets: &self.secrets,
            now: self.clock.now(),
        };
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
const ICON_CONFIG: &str = include_str!("../vendor/icons.yml");
static BRANDS: LazyLock<Vec<Brand>> =
    LazyLock::new(|| serde_yaml::from_str(ICON_CONFIG).expect("vendored config/icons.yml"));
pub(crate) fn icons(conn: &Connection) -> Result<IconCatalog, String> {
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

/// `MessagesHelper#markdown_message_presentation`, using the same icon catalog as writes.
pub(crate) fn markdown_presentation(conn: &Connection, body: &str, ctx: &RenderContext<'_>) -> Result<String, String> {
    markdown::presentation(body, ctx, &icons(conn)?, None).map_err(|error| error.to_string())
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
    fn try_canonicalize_html(&self, conn: &Connection, html: &str) -> Result<String, String> {
        self.with_context(conn, |ctx| {
            campfire_richtext::Content::load(html, ctx).map(|c| c.to_html())
        })
        .map_err(|e| e.to_string())
    }
    fn try_to_plain_text(
        &self,
        conn: &Connection,
        html: &str,
        _: campfire_db::rich_text::UserNames<'_>,
    ) -> Result<String, String> {
        self.with_context(conn, |ctx| campfire_richtext::to_plain_text(html, ctx))
            .map_err(|e| e.to_string())
    }
    fn try_markdown_plain_text(
        &self,
        conn: &Connection,
        html: &str,
        _: campfire_db::rich_text::UserNames<'_>,
    ) -> Result<String, String> {
        let icons = icons(conn)?;
        self.with_context(conn, |ctx| markdown::plain_text(html, ctx, &icons))
            .map_err(|e| e.to_string())
    }
    fn try_mentioned_user_ids(&self, conn: &Connection, html: &str) -> Result<Vec<i64>, String> {
        self.with_context(conn, |ctx| campfire_richtext::mentioned_users(html, ctx))
            .map(|users| users.into_iter().map(|u| u.id).collect())
            .map_err(|e| e.to_string())
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

    /// Compatibility callers still receive a logged fallback. Message writes and reads use
    /// the fallible methods above, so renderer errors reach their transaction.
    fn to_plain_text(
        &self,
        conn: &Connection,
        html: &str,
        user_names: campfire_db::rich_text::UserNames<'_>,
    ) -> String {
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
    fn check_icon_reference(root: &std::path::Path, ci: bool) -> Result<bool, String> {
        let path = root.join("config/icons.yml");
        match std::fs::read(&path) {
            Ok(reference) if reference == ICON_CONFIG.as_bytes() => Ok(true),
            Ok(_) => Err(format!(
                "vendored icon catalog differs from {}; refresh crates/campfire/vendor/icons.yml",
                path.display()
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !ci => Ok(false),
            Err(error) => Err(format!("cannot read reference {}: {error}", path.display())),
        }
    }

    #[test]
    fn vendored_icon_catalog_matches_reference() {
        let root = std::env::var_os("CAMPFIRE_REFERENCE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(fixtures::reference_root);
        if !check_icon_reference(&root, std::env::var_os("CI").is_some()).unwrap() {
            eprintln!("SKIPPED icon reference comparison: no config/icons.yml at {}", root.display());
        }
    }

    #[test]
    fn changed_icon_catalog_is_rejected() {
        let reference = tempfile::tempdir().unwrap();
        std::fs::create_dir(reference.path().join("config")).unwrap();
        // Equal YAML with an extra newline still violates byte-identical vendoring.
        let mut changed = ICON_CONFIG.as_bytes().to_vec();
        changed.push(b'\n');
        std::fs::write(reference.path().join("config/icons.yml"), changed).unwrap();
        assert!(check_icon_reference(reference.path(), false).unwrap_err().contains("differs"));
    }

    #[test]
    fn missing_icon_reference_fails_in_ci() {
        let reference = tempfile::tempdir().unwrap();
        assert!(check_icon_reference(reference.path(), true).unwrap_err().contains("config/icons.yml"));
    }

    #[test]
    fn missing_icon_reference_can_skip_locally() {
        let reference = tempfile::tempdir().unwrap();
        assert!(!check_icon_reference(reference.path(), false).unwrap());
    }

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
            assert!(crate::app::asset_goldens::compare("runtime_markdown", &actual, row["body"].as_str().unwrap()));
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
            assert!(crate::app::asset_goldens::compare("runtime_canonicalization", &actual, row["output"].as_str().unwrap()));
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
    #[test]
    fn runtime_review_markdown_create_and_edit_store_canonical_rails_html() {
        let cases: Value = serde_json::from_str(include_str!("../../db/src/tests/ws8_review_vectors.json")).unwrap();
        let (db, adapter, _dir) = fixture();
        let mut config = Config::new(db.path());
        config.prepare = false;
        let db = Database::open(config, Env { rich_text: Arc::new(adapter), ..Env::default() }).unwrap();
        let source = cases["saved"]["source"].as_str().unwrap().to_owned();
        let mut message = db.write_blocking(move |tx| campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: fixtures::identify("designers"), creator_id: fixtures::identify("david"),
            markdown_source: Some(source), ..Default::default()
        })).unwrap();
        assert!(crate::app::asset_goldens::compare("saved", &db.read_blocking(|conn| message.body_html(conn)).unwrap().unwrap(), cases["saved"]["body"].as_str().unwrap()));
        let edited = cases["edited"]["source"].as_str().unwrap().to_owned();
        let message = db.write_blocking(move |tx| {
            message.edit(tx, campfire_db::MessageChanges { markdown_source: Some(edited), ..Default::default() })?;
            Ok(message)
        }).unwrap();
        assert!(crate::app::asset_goldens::compare("edited", &db.read_blocking(|conn| message.body_html(conn)).unwrap().unwrap(), cases["edited"]["body"].as_str().unwrap()));
    }
    #[test]
    fn runtime_scheduled_edit_and_forward_match_rails() {
        use campfire_db::models::forwarder::{self, BlobCopier, Destination};
        use campfire_db::{Message, MessageChanges, NewScheduledMessage, ScheduledMessage};
        struct NoAttachments;
        impl BlobCopier for NoAttachments {
            fn copy(
                &self,
                _: &mut campfire_db::Tx<'_>,
                _: &campfire_db::Blob,
            ) -> campfire_db::Result<campfire_db::Blob> {
                panic!("no attachment")
            }
            fn discard(&self, _: &[campfire_db::Blob]) {}
        }
        let (db, adapter, _dir) = fixture();
        let mut config = Config::new(db.path());
        config.prepare = false;
        let now = Timestamp::parse_db("2026-03-10 12:00:00").unwrap();
        let db = Database::open(
            config,
            Env {
                clock: Arc::new(campfire_db::TestClock::frozen_at(now)),
                rich_text: Arc::new(adapter),
                ..Env::default()
            },
        )
        .unwrap();
        let g = golden()["flows"].clone();
        let source = g["scheduled"]["source"].as_str().unwrap().to_owned();
        let sent = db
            .write_blocking(move |tx| {
                let item = ScheduledMessage::create(
                    tx,
                    NewScheduledMessage {
                        user_id: fixtures::identify("david"),
                        room_id: fixtures::identify("designers"),
                        markdown_source: source,
                        send_at: now.since(jiff::SignedDuration::from_secs(3600)),
                        thread_id: None,
                        reply_to_message_id: None,
                    },
                )?;
                assert!(ScheduledMessage::dispatch(tx, item.id, now, true)?);
                Message::find(
                    tx.conn(),
                    ScheduledMessage::find(tx.conn(), item.id)?
                        .sent_message_id
                        .unwrap(),
                )
            })
            .unwrap();
        let observation = |msg: &Message| {
            db.read_blocking(|c|Ok(serde_json::json!({"body":msg.body_html(c)?.unwrap(),"plain":msg.plain_text_body(c,&*db.env().rich_text)?}))).unwrap()
        };
        assert!(crate::app::asset_goldens::compare("scheduled", observation(&sent)["body"].as_str().unwrap(), g["scheduled"]["body"].as_str().unwrap()));
        assert_eq!(observation(&sent)["plain"], g["scheduled"]["plain"]);
        let edit = g["edited"]["source"].as_str().unwrap().to_owned();
        let edited = db
            .write_blocking(move |tx| {
                let mut msg = sent;
                msg.edit(
                    tx,
                    MessageChanges {
                        markdown_source: Some(edit),
                        ..Default::default()
                    },
                )?;
                Ok(msg)
            })
            .unwrap();
        assert!(crate::app::asset_goldens::compare("edited", observation(&edited)["body"].as_str().unwrap(), g["edited"]["body"].as_str().unwrap()));
        assert_eq!(observation(&edited)["plain"], g["edited"]["plain"]);
        let forwarded = db
            .write_blocking(move |tx| {
                Ok(forwarder::forward(
                    tx,
                    &edited,
                    &[Destination::room(fixtures::identify("watercooler"))],
                    Some("Note @[David]"),
                    fixtures::identify("david"),
                    &NoAttachments,
                )?
                .unwrap()
                .remove(0)
                .message)
            })
            .unwrap();
        let mut obs = observation(&forwarded);
        obs["markdown_source"] = serde_json::to_value(&forwarded.markdown_source).unwrap();
        obs["forwarded_markdown"] = serde_json::json!(forwarded.forwarded_markdown);
        obs["mentionees"] = db
            .read_blocking(|c| {
                Ok(serde_json::json!(
                    forwarded
                        .mentionees(c, &*db.env().rich_text)?
                        .into_iter()
                        .map(|u| u.id)
                        .collect::<Vec<_>>()
                ))
            })
            .unwrap();
        assert!(crate::app::asset_goldens::compare("forwarded", obs["body"].as_str().unwrap(), g["forwarded"]["body"].as_str().unwrap()));
        obs["body"] = g["forwarded"]["body"].clone();
        assert_eq!(obs, g["forwarded"]);
    }
}
