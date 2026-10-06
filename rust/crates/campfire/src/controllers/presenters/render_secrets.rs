//! Fixed rendering entropy for the tests: the authenticity tokens and CSP nonce a page renders
//! with, inside [`with_fixed_render_secrets`]. The test harness re-exports it.

tokio::task_local! {
    static FIXED_RENDER_SECRETS: ();
}

/// Fix only rendering entropy, before the real router/controller runs. No HTML inputs
/// or response rewrites; authentication and forgery verification keep their real tokens.
pub async fn with_fixed_render_secrets<T>(request: impl std::future::Future<Output = T>) -> T {
    FIXED_RENDER_SECRETS.scope((), request).await
}

pub(crate) fn fixed_render_secrets()
-> Option<campfire_views::helpers::request_forgery::RequestSecrets> {
    use campfire_views::helpers::request_forgery::{AuthenticityTokens, RequestSecrets};
    struct Tokens;
    impl AuthenticityTokens for Tokens {
        fn global(&self) -> String {
            "GLOBAL".into()
        }
        fn for_form(&self, action: &str, method: &str) -> String {
            format!("{method}:{action}")
        }
    }
    FIXED_RENDER_SECRETS
        .try_with(|()| RequestSecrets {
            tokens: Box::new(Tokens),
            csp_nonce: Some("NONCE".into()),
        })
        .ok()
}
