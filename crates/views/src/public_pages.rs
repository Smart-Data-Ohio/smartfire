//! Views for `reference/app/views/public_pages`.

use crate::helpers as h;
use askama::Template;

macro_rules! public_template {
    ($name:ident, $path:literal) => {
        #[derive(Template)]
        #[template(path = $path)]
        pub struct $name<'a> {
            pub operator_name: &'a Option<String>,
            pub contact_email: &'a Option<String>,
            pub effective_date: &'a str,
        }
    };
}

public_template!(About, "public_pages/about.html");
public_template!(Privacy, "public_pages/privacy.html");
public_template!(Terms, "public_pages/terms.html");

pub fn render(
    page: Page,
    operator_name: &Option<String>,
    contact_email: &Option<String>,
    effective_date: &str,
    public_stylesheet: h::Html,
) -> askama::Result<String> {
    let content = match page {
        Page::About => About {
            operator_name,
            contact_email,
            effective_date,
        }
        .render()?,
        Page::Privacy => Privacy {
            operator_name,
            contact_email,
            effective_date,
        }
        .render()?,
        Page::Terms => Terms {
            operator_name,
            contact_email,
            effective_date,
        }
        .render()?,
    };
    crate::layouts::Public {
        page_title: Some(page.title().into()),
        page_description: Some(page.description().into()),
        public_stylesheet,
        content: h::raw(content),
    }
    .render()
}
pub use campfire_presentation::public_pages::*;
