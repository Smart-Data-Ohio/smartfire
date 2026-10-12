//! Public pages inherit ActionController::Base, bypassing the workspace callback chain. Their
//! retained URLs (`/about`, `/privacy`, `/terms`) have moved to the SPA's.
//!
//! `GET /api/v1/public_pages/{page}` is the SPA's copy of the same pages: the article each
//! template (templates/public_pages) renders, with its links on the SPA's pages, and the `LEGAL_*`
//! settings it read. Like the HTML pages it answers anyone, signed in or not, and runs no
//! before-action.

use crate::app::AppCtx;
use askama::Template;
use campfire_api_types::{PublicPage, PublicPageName, PublicPagePolicy};
use campfire_app::public_policy::PublicPolicy;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};

pub async fn about(c: &mut Ctx) -> Result {
    show(c, "/app/about")
}
pub async fn privacy(c: &mut Ctx) -> Result {
    show(c, "/app/privacy")
}
pub async fn terms(c: &mut Ctx) -> Result {
    show(c, "/app/terms")
}

/// The pages have moved for good to the SPA (`/app/about`, `/app/privacy`, `/app/terms`), which
/// draws them for anyone. A request that doesn't want a page still gets the bare 404.
fn show(c: &mut Ctx, path: &str) -> Result {
    let formats = c.formats()?;
    if !formats
        .first()
        .is_some_and(|value| **value == format::HTML || **value == format::ALL)
    {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    c.respond_to(&[&format::HTML])?;
    crate::concerns::redirect_to_signed_out_spa(c, path, StatusCode::MOVED_PERMANENTLY)
}

/// `GET /api/v1/public_pages/{page}`.
pub async fn show_json(c: &mut Ctx) -> Result {
    let page = match c.param_str("page").unwrap_or_default() {
        "about" => PublicPageName::About,
        "privacy" => PublicPageName::Privacy,
        "terms" => PublicPageName::Terms,
        _ => return Ok(c.head(StatusCode::NOT_FOUND)),
    };
    let mut body = public_page(page, &c.app().config.public_policy).map_err(Error::internal)?;
    if page == PublicPageName::About {
        let account = c.app().db.read(campfire_db::Account::first).await.map_err(Error::internal)?;
        body.workspace = Some(crate::controllers::presenters::workspace_branding::public_for_account(c.app(), account.as_ref()).await?);
    }
    let body = serde_json::to_string(&body).map_err(Error::internal)?;
    Ok(c.render_as(StatusCode::OK, "application/json; charset=utf-8", body))
}

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

/// Where the pages' in-text links go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Links {
    pub privacy: String,
    pub terms: String,
    pub sign_in: String,
}

macro_rules! public_template {
    ($name:ident, $path:literal) => {
        #[derive(Template)]
        #[template(path = $path)]
        pub struct $name<'a> {
            pub operator_name: &'a Option<String>,
            pub contact_email: &'a Option<String>,
            pub effective_date: &'a str,
            pub links: &'a Links,
        }
    };
}

public_template!(About, "public_pages/about.html");
public_template!(Privacy, "public_pages/privacy.html");
public_template!(Terms, "public_pages/terms.html");

/// The page's article: what the SPA puts in its reading column.
pub fn render_article(
    page: Page,
    operator_name: &Option<String>,
    contact_email: &Option<String>,
    effective_date: &str,
    links: &Links,
) -> askama::Result<String> {
    match page {
        Page::About => About { operator_name, contact_email, effective_date, links }.render(),
        Page::Privacy => Privacy { operator_name, contact_email, effective_date, links }.render(),
        Page::Terms => Terms { operator_name, contact_email, effective_date, links }.render(),
    }
}

/// The few Rails helpers the templates call, with ERB's escaping and attribute order.
pub mod h {
    use std::fmt::{self, Write};

    use askama::filters::Safe;
    use campfire_presentation::helpers::serialization::{escape, push_escaped};
    use campfire_presentation::helpers::url::cgi_escape;

    /// `ERB::Util.html_escape` as the templates' escaper: `&amp; &lt; &gt; &quot; &#39;`.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct ErbEscaper;

    impl askama::filters::Escaper for ErbEscaper {
        fn write_escaped_str<W: Write>(&self, mut dest: W, string: &str) -> fmt::Result {
            let mut out = String::with_capacity(string.len());
            push_escaped(&mut out, string);
            dest.write_str(&out)
        }
    }

    /// A tag's options, in the order they were given.
    #[derive(Default)]
    pub struct Attrs(Vec<(&'static str, String)>);

    pub fn attrs() -> Attrs {
        Attrs::default()
    }

    impl Attrs {
        pub fn attr(mut self, name: &'static str, value: &str) -> Self {
            self.0.push((name, value.to_owned()));
            self
        }
    }

    /// `link_to(text, url, options)`: the options, then the `href`.
    pub fn link_to_text(text: &str, url: &str, options: Attrs) -> Safe<String> {
        let mut out = String::from("<a");
        for (name, value) in options.attr("href", url).0 {
            out.push_str(&format!(" {name}=\""));
            push_escaped(&mut out, &value);
            out.push('"');
        }
        out.push('>');
        push_escaped(&mut out, text);
        out.push_str("</a>");
        Safe(out)
    }

    /// `mail_to(email)`: the address percent-escaped (`ERB::Util.url_encode`, keeping "@") in the
    /// href, and the plain address as the link text.
    pub fn mail_to(email: &str) -> Safe<String> {
        let encoded = cgi_escape(email).replace('+', "%20").replace("%40", "@");
        Safe(format!("<a href=\"mailto:{}\">{}</a>", escape(&encoded), escape(email)))
    }
}

/// The SPA's links: its own copies of these pages and its sign-in page.
pub fn spa_links() -> Links {
    Links {
        privacy: "/app/privacy".into(),
        terms: "/app/terms".into(),
        sign_in: "/app/session/new".into(),
    }
}

pub fn public_page(name: PublicPageName, policy: &PublicPolicy) -> askama::Result<PublicPage> {
    let page = match name {
        PublicPageName::About => Page::About,
        PublicPageName::Privacy => Page::Privacy,
        PublicPageName::Terms => Page::Terms,
    };
    Ok(PublicPage {
        page: name,
        workspace: None,
        title: page.title().into(),
        description: page.description().into(),
        policy: PublicPagePolicy {
            operator_name: policy.operator_name.clone(),
            contact_email: policy.contact_email.clone(),
            effective_date: policy.effective_date.clone(),
        },
        html: render_article(
            page,
            &policy.operator_name,
            &policy.contact_email,
            &policy.effective_date,
            &spa_links(),
        )?,
    })
}
