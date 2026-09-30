//! Views for `reference/app/views/public_pages`.

use crate::helpers as h;
use askama::Template;

#[derive(Clone, Copy)]
pub enum Page {
    About,
    Privacy,
    Terms,
}

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Self::About => "Smartfire | About",
            Self::Privacy => "Smartfire | Privacy Policy",
            Self::Terms => "Smartfire | Terms of Service",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::About => {
                "What Smartfire is: self-hosted, open-source team chat run by the organization hosting each workspace."
            }
            Self::Privacy => {
                "Smartfire privacy policy: what each self-hosted workspace stores, who can see it, and how Google sign-in, Calendar, and Drive data is handled."
            }
            Self::Terms => {
                "Smartfire terms of service: the open-source software license and the rules for using a self-hosted workspace."
            }
        }
    }
}

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
