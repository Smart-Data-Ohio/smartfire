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
}

pub fn signed_out_route(path: &str) -> Option<SignedOutRoute> {
    match path {
        "/app/session/new" => Some(SignedOutRoute::SignIn),
        "/app/two_factor/challenge" => Some(SignedOutRoute::Challenge),
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
        ] {
            assert_eq!(signed_out_route(path), None, "{path}");
        }
    }
}
