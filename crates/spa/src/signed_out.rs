//! Reserved SPA auth routes. Historic auth URLs still serve their retained pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedOutRoute {
    SignIn,
    Transfer,
    Challenge,
    /// `/app/join/:join_code`, the retained `/join/:join_code`.
    Join,
    /// `/app/invite/:token`, the retained `/invite/:token`.
    Invite,
    FirstRun,
}

pub fn signed_out_route(path: &str) -> Option<SignedOutRoute> {
    match path {
        "/app/session/new" => Some(SignedOutRoute::SignIn),
        "/app/two_factor/challenge" => Some(SignedOutRoute::Challenge),
        "/app/first_run" => Some(SignedOutRoute::FirstRun),
        _ => [
            ("/app/session/transfers/", SignedOutRoute::Transfer),
            ("/app/join/", SignedOutRoute::Join),
            ("/app/invite/", SignedOutRoute::Invite),
        ]
        .into_iter()
        .find_map(|(prefix, route)| {
            path.strip_prefix(prefix)
                .filter(|segment| !segment.is_empty() && !segment.contains('/'))
                .map(|_| route)
        }),
    }
}

/// About, Privacy and Terms, which anyone may read: the signed-out shell draws them for everyone,
/// as the retained pages show a member and a visitor the same page.
pub fn is_public_page(path: &str) -> bool {
    matches!(path, "/app/about" | "/app/privacy" | "/app/terms")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_out_routes_are_a_fixed_allow_list() {
        assert_eq!(
            signed_out_route("/app/session/new"),
            Some(SignedOutRoute::SignIn)
        );
        assert_eq!(
            signed_out_route("/app/session/transfers/signed-id"),
            Some(SignedOutRoute::Transfer)
        );
        assert_eq!(
            signed_out_route("/app/two_factor/challenge"),
            Some(SignedOutRoute::Challenge)
        );
        assert_eq!(
            signed_out_route("/app/join/abc-123"),
            Some(SignedOutRoute::Join)
        );
        assert_eq!(
            signed_out_route("/app/invite/token"),
            Some(SignedOutRoute::Invite)
        );
        assert_eq!(
            signed_out_route("/app/first_run"),
            Some(SignedOutRoute::FirstRun)
        );
        for path in [
            "/session/new",
            "/app/",
            "/app/session/new/extra",
            "/app/session/transfers/",
            "/app/session/transfers/a/b",
            "/app/two_factor/challenge/extra",
            "/app/join/",
            "/app/join/a/b",
            "/app/invite/",
            "/app/invite/a/b",
            "/first_run",
            "/app/first_run/extra",
        ] {
            assert_eq!(signed_out_route(path), None, "{path}");
        }
    }

    #[test]
    fn public_pages_are_a_fixed_allow_list() {
        for path in ["/app/about", "/app/privacy", "/app/terms"] {
            assert!(is_public_page(path), "{path}");
            assert_eq!(signed_out_route(path), None, "{path}");
        }
        for path in ["/about", "/app/about/", "/app/terms/extra", "/app/privacy.json", "/app/"] {
            assert!(!is_public_page(path), "{path}");
        }
    }
}
