//! Retained-page fixture context and HTTP contract helpers.
use super::super::presenters::test_support::*;
use campfire_view_kit::helpers as h;

pub(crate) fn render_with(
    _app: &TestApp,
    configure: impl FnOnce(&mut campfire_retained::Context),
    f: impl FnOnce(&campfire_retained::Context) -> String,
) -> String {
    struct Tokens;
    impl h::request_forgery::AuthenticityTokens for Tokens {
        fn global(&self) -> String {
            "GLOBAL".into()
        }
        fn for_form(&self, action: &str, method: &str) -> String {
            format!("{method}:{action}")
        }
    }
    let asset = |name: &str| campfire_static_assets::asset_path(name);
    let mut ctx = campfire_retained::Context {
        current_user: Some(campfire_view_kit::CurrentUser {
            id: DAVID,
            name: "David".into(),
            administrator: true,
            bot: false,
            avatar_url: String::new(),
            preferences: Default::default(),
        }),
        account: campfire_view_kit::AccountSummary {
            name: "Signal".into(),
            logo_url: String::new(),
            has_logo: false,
        },
        flash_notice: None,
        flash_alert: None,
        platform: Default::default(),
        asset_path: &asset,
        custom_styles: None,
        base_url: "http://campfire.test".into(),
        app_version: "parity".into(),
        chrome: Default::default(),
    };
    configure(&mut ctx);
    h::request_forgery::rendering_with(
        h::request_forgery::RequestSecrets {
            tokens: Box::new(Tokens),
            csp_nonce: Some("NONCE".into()),
        },
        || f(&ctx),
    )
}
pub(crate) fn retained<'a>(ctx: &campfire_retained::Context<'a>) -> campfire_retained::Context<'a> {
    campfire_retained::Context {
        current_user: ctx.current_user.clone(), account: ctx.account.clone(),
        flash_notice: ctx.flash_notice.clone(), flash_alert: ctx.flash_alert.clone(),
        custom_styles: ctx.custom_styles.clone(), platform: ctx.platform.clone(),
        app_version: ctx.app_version.clone(), base_url: ctx.base_url.clone(), asset_path: ctx.asset_path,
        chrome: campfire_retained::Chrome {
            service_worker_auto_register: ctx.chrome.service_worker_auto_register,
            service_worker_url: ctx.chrome.service_worker_url.clone(),
        },
    }
}
