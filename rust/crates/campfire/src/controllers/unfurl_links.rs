//! `UnfurlLinksController` (reference/app/controllers/unfurl_links_controller.rb): the composer
//! asks for a pasted URL's OpenGraph metadata.

use campfire_kit::{Ctx, Error, Result, StatusCode};

use crate::concerns::{Before, before_actions};
use crate::integrations::net::Network;
use crate::integrations::opengraph::{self, Unfurl};

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    // A hash or array passes `require`, but `URI.parse` can't take it (`InvalidURIError`,
    // rescued), so the metadata has no title and isn't valid.
    let Some(url) = url_param(c)? else { return Ok(c.head(StatusCode::NO_CONTENT)) };
    match opengraph_json(c, &url).await? {
        // `render json: opengraph`
        Some(json) => Ok(c.render_as(StatusCode::OK, campfire_kit::response::JSON_UTF8, json)),
        None => Ok(c.head(StatusCode::NO_CONTENT)),
    }
}

/// `params.require(:url)`
fn url_param(c: &Ctx) -> Result<Option<String>> {
    Ok(c.params.require("url")?.as_str().map(str::to_string))
}

/// `Opengraph::Metadata.from_url(url)`, as JSON when `valid?` (`crate::integrations::opengraph`).
/// The network is the system's unless the request carries another one (tests).
async fn opengraph_json(c: &Ctx, url: &str) -> Result<Option<String>> {
    let net = c.current::<Network>().cloned().unwrap_or_else(Network::system);
    match opengraph::unfurl(&net, url).await.map_err(Error::internal)? {
        Unfurl::Json(json) => Ok(Some(json)),
        Unfurl::NoContent => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{Method, StatusCode};

    use crate::controllers::presenters::test_support::*;

    #[tokio::test]
    async fn unfurls_nothing_from_private_addresses_and_needs_a_url() {
        let Some(app) = TestApp::boot().await else { return };
        let mut david = app.david();
        let private = david.write(Req::new(Method::POST, "/unfurl_link").form(&[("url", "http://127.0.0.1/secret")])).await;
        assert_eq!(private.status, StatusCode::NO_CONTENT);
        let missing = david.write(Req::new(Method::POST, "/unfurl_link").form(&[("url", "")])).await;
        assert_eq!(missing.status, StatusCode::BAD_REQUEST);
        let hash = david.write(Req::new(Method::POST, "/unfurl_link").form(&[("url[a]", "http://example.com")])).await;
        assert_eq!(hash.status, StatusCode::NO_CONTENT);
        let array = david.write(Req::new(Method::POST, "/unfurl_link").form(&[("url[]", "http://example.com")])).await;
        assert_eq!(array.status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn ws15e_composer_http_skips_special_cards_before_dns() {
        use std::sync::Arc;
        use crate::integrations::{net::Network,test_support::FakeResolver};
        #[derive(Clone)]
        struct NetworkAction(Network);
        impl<'a> campfire_kit::ActionFn<'a> for NetworkAction {
            type Fut=futures_util::future::BoxFuture<'a,campfire_kit::Result>;
            fn call(&self,c: &'a mut campfire_kit::Ctx) -> Self::Fut {
                c.set_current(self.0.clone());
                Box::pin(crate::controllers::dispatch(c))
            }
        }
        let mut app = TestApp::boot().await.expect("build parity seed");
        app.booted.jobs.stop(std::time::Duration::from_secs(1)).await;
        let resolver=Arc::new(FakeResolver::default());
        let action=NetworkAction(Network { resolver:resolver.clone(),..Network::system() });
        let kit=campfire_kit::Kit::new(campfire_kit::KitConfig::production(true),Arc::new(campfire_kit::RailsCrypto::new(app.booted.app.secrets.clone())),seed_clock(),app.booted.app.clone());
        let methods=campfire_kit::get(action.clone()).merge(campfire_kit::post(action.clone()));
        let routes=axum::Router::new().route("/{*path}",methods).route("/",campfire_kit::get(action));
        app.booted.router=campfire_kit::app(routes,kit);
        let request = |url: &str| Req::new(Method::POST, "/unfurl_link").form(&[("url", url)]);
        assert_eq!(
            app.anonymous()
                .send(request("https://github.com/basecamp/once/pull/1"))
                .await
                .status,
            StatusCode::FOUND
        );
        let mut david = app.david();
        for url in ["https://github.com/basecamp/once/pull/1", "https://app.fizzy.do/123/cards/42"] {
            let response = david.write(request(url)).await;
            assert_eq!(response.status, StatusCode::NO_CONTENT);
            assert!(response.body.is_empty());
        }
        assert!(resolver.lookups().is_empty());
    }
}
