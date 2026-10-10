//! The icon catalog (`config/icons.yml`'s brands, then the workspace's custom icons) that
//! Markdown rendering, autocompletion, room validation and the Slack importer share.

use std::sync::LazyLock;

use campfire_db::Connection;
use campfire_richtext::markdown::{Icon, IconCatalog};
use serde::Deserialize;

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
/// Icons.client_icon_names: canonical brands and their aliases in YAML order, then custom
/// names in database name order. Read each request so uploads/removals reach the live layout.
pub fn client_icon_names(conn: &Connection) -> campfire_db::Result<Vec<String>> {
    let mut names = BRANDS
        .iter()
        .flat_map(|brand| std::iter::once(&brand.name).chain(&brand.aliases))
        .cloned()
        .collect::<Vec<_>>();
    names.extend(
        conn.prepare_cached("SELECT name FROM workspace_icons ORDER BY name")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    Ok(names)
}
pub fn builtin_icon(name: &str) -> bool {
    let name = campfire_richtext::ruby::strip(name)
        .trim_matches(':')
        .trim()
        .to_lowercase();
    BRANDS
        .iter()
        .any(|b| b.name == name || b.aliases.contains(&name))
}
pub fn icons(conn: &Connection) -> Result<IconCatalog, String> {
    let mut icons = IconCatalog::default();
    for brand in BRANDS.iter() {
        let icon = Icon::Brand {
            name: brand.name.clone(),
            title: brand.title.clone(),
            url: campfire_static_assets::try_asset_path(&format!("icons/brands/{}", brand.file)).ok(),
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

/// WS8br seam: reuse WS5's data resolver for Room#icon_name_must_resolve, on the
/// caller's transaction connection; this does not render or alter rich-text fragments.
pub fn room_icon_resolves(conn: &Connection, name: &str) -> campfire_db::Result<bool> {
    Ok(icons(conn)
        .map_err(campfire_db::Error::Other)?
        .find_normalized(name)
        .is_some())
}
