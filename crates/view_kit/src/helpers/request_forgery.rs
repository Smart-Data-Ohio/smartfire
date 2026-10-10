//! What a page carries for its own request: the authenticity tokens (`csrf_meta_tags`, a form's
//! hidden `authenticity_token`) and the Content Security Policy nonce (`csp_meta_tag`).
//!
//! The controller lends them for the length of a render ([`rendering_with`]); templates and
//! helpers read them without threading them through every view model. Outside a render that has
//! them (renders for broadcasts, and the views' own tests) there are none, and the tags are left
//! out, as with forgery protection and the policy off.

use std::cell::RefCell;
use std::rc::Rc;

use super::html::{Html, Safe};
use super::tag::{attrs, legacy_tag};

/// `form_authenticity_token` for this request's session.
pub trait AuthenticityTokens {
    /// Rails omits these tags when `allow_forgery_protection` is disabled.
    fn enabled(&self) -> bool {
        true
    }

    /// The masked global token (`csrf_meta_tags`).
    fn global(&self) -> String;
    /// The masked per-form token for a form posting to `action` (as written in the page) with
    /// `method` (`per_form_csrf_tokens`).
    fn for_form(&self, action: &str, method: &str) -> String;
}

/// This request's tokens and nonce.
pub struct RequestSecrets {
    pub tokens: Box<dyn AuthenticityTokens>,
    /// `content_security_policy_nonce`
    pub csp_nonce: Option<String>,
}

thread_local! {
    static CURRENT: RefCell<Option<Rc<RequestSecrets>>> = const { RefCell::new(None) };
}

/// Runs `render` with `secrets` available to the templates it renders.
pub fn rendering_with<R>(secrets: RequestSecrets, render: impl FnOnce() -> R) -> R {
    struct Restore(Option<Rc<RequestSecrets>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            CURRENT.with(|current| *current.borrow_mut() = previous);
        }
    }
    let previous = CURRENT.with(|current| current.replace(Some(Rc::new(secrets))));
    let _restore = Restore(previous);
    render()
}

fn current() -> Option<Rc<RequestSecrets>> {
    CURRENT.with(|current| current.borrow().clone())
}

/// `request_forgery_protection_token`
pub const PARAM: &str = "authenticity_token";

/// `token_tag(nil, form_options: { action:, method: })`: the hidden per-form token field.
pub fn token_tag(action: &str, method: &str) -> Html {
    if current().is_some_and(|secrets| !secrets.tokens.enabled()) {
        return Safe(String::new());
    }
    match current() {
        Some(secrets) => legacy_tag(
            "input",
            attrs()
                .type_("hidden")
                .name(PARAM)
                .value(secrets.tokens.for_form(action, method)),
        ),
        None => Safe(String::new()),
    }
}

/// `csrf_meta_tags`
pub fn csrf_meta_tags() -> Html {
    match current() {
        Some(secrets) if secrets.tokens.enabled() => Safe(format!(
            "{}\n{}",
            legacy_tag("meta", attrs().name("csrf-param").attr("content", PARAM)).0,
            legacy_tag(
                "meta",
                attrs()
                    .name("csrf-token")
                    .attr("content", secrets.tokens.global())
            )
            .0
        )),
        _ => Safe(String::new()),
    }
}

/// `content_security_policy_nonce`
pub fn csp_nonce() -> Option<String> {
    current().and_then(|secrets| secrets.csp_nonce.clone())
}

/// `csp_meta_tag`
pub fn csp_meta_tag() -> Html {
    match csp_nonce() {
        Some(nonce) => legacy_tag("meta", attrs().name("csp-nonce").attr("content", nonce)),
        None => Safe(String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed;

    impl AuthenticityTokens for Fixed {
        fn global(&self) -> String {
            "GLOBAL".into()
        }

        fn for_form(&self, action: &str, method: &str) -> String {
            format!("{method}:{action}")
        }
    }

    fn secrets(nonce: Option<&str>) -> RequestSecrets {
        RequestSecrets {
            tokens: Box::new(Fixed),
            csp_nonce: nonce.map(str::to_string),
        }
    }

    #[test]
    fn disabled_forgery_omits_tokens_and_preserves_the_real_csp_nonce() {
        struct Disabled;
        impl AuthenticityTokens for Disabled {
            fn enabled(&self) -> bool {
                false
            }
            fn global(&self) -> String {
                panic!("disabled token must not be requested")
            }
            fn for_form(&self, _: &str, _: &str) -> String {
                panic!("disabled form token must not be requested")
            }
        }
        let (meta, field, nonce) = rendering_with(
            RequestSecrets {
                tokens: Box::new(Disabled),
                csp_nonce: Some("real-nonce".into()),
            },
            || {
                (
                    csrf_meta_tags().0,
                    token_tag("/session", "post").0,
                    csp_meta_tag().0,
                )
            },
        );
        assert_eq!(meta, "");
        assert_eq!(field, "");
        assert_eq!(nonce, "<meta name=\"csp-nonce\" content=\"real-nonce\" />");
    }

    #[test]
    fn tags_render_only_inside_a_render_with_secrets() {
        assert_eq!(csrf_meta_tags().0, "");
        assert_eq!(token_tag("/session", "post").0, "");
        assert_eq!(csp_meta_tag().0, "");
        let (meta, field, nonce) = rendering_with(secrets(Some("n+/=")), || {
            (
                csrf_meta_tags().0,
                token_tag("/session", "post").0,
                csp_meta_tag().0,
            )
        });
        assert_eq!(
            meta,
            "<meta name=\"csrf-param\" content=\"authenticity_token\" />\n<meta name=\"csrf-token\" content=\"GLOBAL\" />"
        );
        assert_eq!(
            field,
            "<input type=\"hidden\" name=\"authenticity_token\" value=\"post:/session\" />"
        );
        assert_eq!(nonce, "<meta name=\"csp-nonce\" content=\"n+/=\" />");
        assert_eq!(csrf_meta_tags().0, "", "restored after the render");
    }

    #[test]
    fn renders_nest_and_restore() {
        rendering_with(secrets(Some("outer")), || {
            rendering_with(secrets(None), || assert_eq!(csp_nonce(), None));
            assert_eq!(csp_nonce().as_deref(), Some("outer"));
        });
    }

    #[test]
    fn forms_carry_their_per_form_token() {
        use crate::helpers::forms::{button_to, form_with};
        use crate::helpers::tag::attrs;

        let (post, patch, get, delete_button, post_button) = rendering_with(secrets(None), || {
            (
                form_with("/rooms").open().0,
                form_with("/rooms/1").method("patch").open().0,
                form_with("/searches").method("get").open().0,
                button_to("/rooms/1", attrs().method("delete"), "X").0,
                button_to("/rooms/1/ban", attrs(), "Ban").0,
            )
        });
        assert_eq!(
            post,
            "<form action=\"/rooms\" accept-charset=\"UTF-8\" method=\"post\"><input type=\"hidden\" name=\"authenticity_token\" value=\"post:/rooms\" />"
        );
        assert_eq!(
            patch,
            "<form action=\"/rooms/1\" accept-charset=\"UTF-8\" method=\"post\"><input type=\"hidden\" name=\"_method\" value=\"patch\" /><input type=\"hidden\" name=\"authenticity_token\" value=\"patch:/rooms/1\" />"
        );
        assert_eq!(
            get,
            "<form action=\"/searches\" accept-charset=\"UTF-8\" method=\"get\">"
        );
        assert_eq!(
            delete_button,
            "<form class=\"button_to\" method=\"post\" action=\"/rooms/1\"><input type=\"hidden\" name=\"_method\" value=\"delete\" /><button type=\"submit\">X</button><input type=\"hidden\" name=\"authenticity_token\" value=\"delete:/rooms/1\" /></form>"
        );
        assert!(post_button.ends_with("<button type=\"submit\">Ban</button><input type=\"hidden\" name=\"authenticity_token\" value=\"post:/rooms/1/ban\" /></form>"), "{post_button}");
        assert!(
            !form_with("/rooms").open().0.contains("authenticity_token"),
            "none outside a render"
        );
    }

}
