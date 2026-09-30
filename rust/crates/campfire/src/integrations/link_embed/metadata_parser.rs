//! `LinkEmbed::MetadataParser`: first-selector precedence, plain text and image URI joins.
use crate::integrations::{
    net::redirect,
    opengraph::{html, metadata::strip_tags},
};
use campfire_richtext::{dom::Dom, sanitizer::cgi_unescape_html, uri};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub image_url: Option<String>,
}

pub fn parse(body: &[u8], base: &str) -> Result<Metadata, String> {
    let decoded = html::decode(body);
    let metas = html::meta_elements(&decoded);
    let first = |selectors: &[(&str, &str)]| -> Option<String> {
        for (attribute, name) in selectors {
            // Rails at_xpath selects the first matching tag even when its content is blank.
            if let Some(meta) = metas
                .iter()
                .find(|tag| tag.attr(attribute).is_some_and(|value| value.to_ascii_lowercase() == *name))
            {
                let content = strip(meta.attr("content").unwrap_or(""));
                if !content.chars().all(char::is_whitespace) {
                    return Some(content.to_string());
                }
            }
        }
        None
    };
    let clean = |value: Option<String>, limit: usize| -> Result<Option<String>, String> {
        let stripped = strip_tags(value.as_deref().unwrap_or("")).map_err(|e| e.to_string())?;
        let text = cgi_unescape_html(strip(&stripped));
        Ok((!text.chars().all(char::is_whitespace)).then(|| text.chars().take(limit).collect()))
    };
    let title = first(&[
        ("property", "og:title"),
        ("name", "og:title"),
        ("property", "twitter:title"),
        ("name", "twitter:title"),
    ]);
    let title = match title {
        Some(title) => Some(title),
        None => {
            let mut dom = Dom::new();
            let root = dom.parse_fragment(&decoded).map_err(|e| e.to_string())?;
            dom.descendants(root)
                .into_iter()
                .find(|node| dom.name(*node) == "title")
                .map(|node| dom.text_content(node))
        }
    };
    let description = first(&[
        ("property", "og:description"),
        ("name", "og:description"),
        ("property", "twitter:description"),
        ("name", "twitter:description"),
        ("name", "description"),
    ]);
    let site = first(&[
        ("property", "og:site_name"),
        ("name", "og:site_name"),
        ("property", "twitter:site"),
        ("name", "twitter:site"),
    ])
    .or_else(|| uri::parse(base).ok()?.host);
    let image = first(&[
        ("property", "og:image:secure_url"),
        ("name", "og:image:secure_url"),
        ("property", "og:image"),
        ("name", "og:image"),
        ("property", "twitter:image:src"),
        ("name", "twitter:image:src"),
        ("property", "twitter:image"),
        ("name", "twitter:image"),
    ]);
    let image_url = image
        .and_then(|value| redirect::resolve(&uri::parse(base).ok()?, Some(strip(&value))))
        .filter(|url| url.host.as_ref().is_some_and(|host| !host.is_empty()))
        .map(|url| url.to_s());
    Ok(Metadata {
        title: clean(title, 300)?,
        description: clean(description, 1000)?,
        site_name: clean(site, 100)?,
        image_url,
    })
}

fn strip(value: &str) -> &str {
    value.trim_matches(['\0', ' ', '\t', '\n', '\r', '\u{b}', '\u{c}'])
}

#[cfg(test)]
mod tests {
    #[test]
    fn ws15e_link_metadata_matches_rails() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/ws15e_link_embed.json")).unwrap();
        for case in vectors["metadata"].as_array().unwrap() {
            assert_eq!(
                serde_json::to_value(super::parse(case["html"].as_str().unwrap().as_bytes(), case["base"].as_str().unwrap()).unwrap())
                    .unwrap(),
                case["expected"],
                "{case}"
            );
        }
    }
}
