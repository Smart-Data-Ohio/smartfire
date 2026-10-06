//! `ActionDispatch::ContentSecurityPolicy`: the policy an app declares
//! (`config.content_security_policy`), its nonce generator and nonce directives, and the
//! middleware's rule for which responses get the header.
//!
//! The middleware sits below the session middleware and above `Rack::ConditionalGet`, and only
//! sees what the router answers: every controller response (HTML, JSON, redirects, `head`), but
//! not public files, assets or the exception pages `ShowExceptions` renders. It leaves a 304 and a
//! response that already carries a policy alone. [`crate::Ctx`] applies it when finishing.

use std::sync::Arc;

/// A directive's source: a literal (`'self'`, `https:`, a host) or a lambda evaluated for each
/// response (`-> { ContentSecurityPolicySources.livekit }`), whose list is flattened in.
#[derive(Clone)]
pub enum Source {
    Static(String),
    Dynamic(Arc<dyn Fn() -> Vec<String> + Send + Sync>),
}

impl From<&str> for Source {
    fn from(source: &str) -> Self {
        Source::Static(source.to_string())
    }
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Static(source) => write!(f, "{source:?}"),
            Source::Dynamic(_) => f.write_str("<dynamic>"),
        }
    }
}

/// `content_security_policy_nonce_generator`: the session id Rails would report (see
/// [`crate::Session::public_id`]) to a nonce.
pub type NonceGenerator = Arc<dyn Fn(Option<&str>) -> String + Send + Sync>;

#[derive(Clone, Default)]
pub struct ContentSecurityPolicy {
    directives: Vec<(String, Vec<Source>)>,
    nonce_directives: Vec<String>,
    nonce_generator: Option<NonceGenerator>,
    report_only: bool,
}

impl std::fmt::Debug for ContentSecurityPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContentSecurityPolicy")
            .field("directives", &self.directives)
            .field("nonce_directives", &self.nonce_directives)
            .field("report_only", &self.report_only)
            .finish()
    }
}

impl ContentSecurityPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    /// `policy.<directive> *sources`, in declaration order.
    pub fn directive(mut self, name: &str, sources: impl IntoIterator<Item = Source>) -> Self {
        self.directives.push((name.to_string(), sources.into_iter().collect()));
        self
    }

    /// `content_security_policy_nonce_generator` and `content_security_policy_nonce_directives`.
    pub fn nonce(mut self, generator: NonceGenerator, directives: &[&str]) -> Self {
        self.nonce_generator = Some(generator);
        self.nonce_directives = directives.iter().map(|d| d.to_string()).collect();
        self
    }

    /// `content_security_policy_report_only`
    pub fn report_only(mut self, report_only: bool) -> Self {
        self.report_only = report_only;
        self
    }

    /// The header the middleware sets.
    pub fn header_name(&self) -> &'static str {
        if self.report_only { "content-security-policy-report-only" } else { "content-security-policy" }
    }

    /// `request.content_security_policy_nonce`: `None` without a generator.
    pub fn generate_nonce(&self, session_id: Option<&str>) -> Option<String> {
        self.nonce_generator.as_ref().map(|generate| generate(session_id))
    }

    /// `policy.build(context, nonce, nonce_directives)`
    pub fn build(&self, nonce: Option<&str>) -> String {
        let directives: Vec<String> = self
            .directives
            .iter()
            .map(|(name, sources)| {
                // `build_directive` flattens the lambdas' lists, then joins with spaces.
                let sources: Vec<String> = sources
                    .iter()
                    .flat_map(|source| match source {
                        Source::Static(source) => vec![source.clone()],
                        Source::Dynamic(resolve) => resolve(),
                    })
                    .collect();
                match nonce {
                    Some(nonce) if self.nonce_directives.contains(name) => format!("{name} {} 'nonce-{nonce}'", sources.join(" ")),
                    _ => format!("{name} {}", sources.join(" ")),
                }
            })
            .collect();
        directives.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ContentSecurityPolicy {
        ContentSecurityPolicy::new()
            .directive("default-src", ["'self'".into()])
            .directive("script-src", ["'self'".into(), "https://apis.google.com".into()])
            .directive("connect-src", ["'self'".into(), Source::Dynamic(Arc::new(|| vec!["wss://lk.test".into(), "https://lk.test".into()]))])
            .directive("worker-src", [Source::Dynamic(Arc::new(Vec::new))])
            .directive("report-uri", ["/csp_reports".into()])
            .nonce(Arc::new(|id: Option<&str>| format!("n-{}", id.unwrap_or("random"))), &["script-src"])
    }

    #[test]
    fn builds_directives_in_order_with_the_nonce_on_nonce_directives() {
        assert_eq!(
            policy().build(Some("abc")),
            "default-src 'self'; script-src 'self' https://apis.google.com 'nonce-abc'; \
             connect-src 'self' wss://lk.test https://lk.test; worker-src ; report-uri /csp_reports"
        );
        assert_eq!(
            policy().build(None),
            "default-src 'self'; script-src 'self' https://apis.google.com; \
             connect-src 'self' wss://lk.test https://lk.test; worker-src ; report-uri /csp_reports"
        );
    }

    #[test]
    fn nonces_come_from_the_generator() {
        assert_eq!(policy().generate_nonce(Some("sid")).as_deref(), Some("n-sid"));
        assert_eq!(policy().generate_nonce(None).as_deref(), Some("n-random"));
        assert_eq!(ContentSecurityPolicy::new().generate_nonce(Some("sid")), None);
    }

    #[test]
    fn report_only_changes_the_header() {
        assert_eq!(policy().header_name(), "content-security-policy");
        assert_eq!(policy().report_only(true).header_name(), "content-security-policy-report-only");
    }
}
