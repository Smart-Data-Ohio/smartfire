//! Reserved SPA auth routes. Historic auth URLs still serve their retained pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedOutRoute {
    SignIn,
    Transfer,
    Challenge,
    FirstRun,
}

pub fn signed_out_route(path: &str) -> Option<SignedOutRoute> {
    match path {
        "/app/session/new" => Some(SignedOutRoute::SignIn),
        "/app/two_factor/challenge" => Some(SignedOutRoute::Challenge),
        "/app/first_run" => Some(SignedOutRoute::FirstRun),
        _ => path
            .strip_prefix("/app/session/transfers/")
            .filter(|id| !id.is_empty() && !id.contains('/'))
            .map(|_| SignedOutRoute::Transfer),
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
            "/first_run",
            "/app/first_run/extra",
        ] {
            assert_eq!(signed_out_route(path), None, "{path}");
        }
    }
}
