use crate::shell::escape;
use crate::{File, Served};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Favicon {
    /// The logo's PNGs, at URLs that change with the logo.
    Logo {
        /// The manifest's small icon: at most 192px a side.
        icon: String,
        /// 180×180, the logo padded onto a square tile.
        touch_icon: String,
    },
    Mark,
}

impl Favicon {
    pub(crate) fn links(&self) -> String {
        match self {
            Favicon::Logo { icon, touch_icon } => format!(
                "<link rel=\"icon\" type=\"image/png\" href=\"{}\" />\n<link rel=\"apple-touch-icon\" href=\"{}\" />\n",
                escape(icon),
                escape(touch_icon)
            ),
            // Listed after the PNG, the SVG is the icon a browser that reads SVG chooses.
            Favicon::Mark => "<link rel=\"icon\" type=\"image/png\" sizes=\"32x32\" href=\"/favicon.png\" />\n\
                <link rel=\"icon\" type=\"image/svg+xml\" href=\"/favicon.svg\" />\n\
                <link rel=\"apple-touch-icon\" href=\"/apple-touch-icon.png\" />\n"
                .to_string(),
        }
    }
}

macro_rules! mark_file {
    ($path:literal, $content_type:literal) => {
        File {
            path: $path,
            content_type: $content_type,
            immutable: false,
            identity: include_bytes!(concat!("../favicon/", $path)),
            br: None,
            gz: None,
        }
    };
}

static MARK: &[File] = &[
    mark_file!("apple-touch-icon.png", "image/png"),
    mark_file!("favicon.ico", "image/x-icon"),
    mark_file!("favicon.png", "image/png"),
    mark_file!("favicon.svg", "image/svg+xml"),
];

/// The mark's file at `path` under the root (`favicon.svg`).
pub fn file(path: &str) -> Option<Served> {
    crate::serve::find(MARK, path).map(|file| crate::serve::negotiate(file, None))
}
