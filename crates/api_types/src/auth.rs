//! Signed-out authentication contracts. Retained forms use the same operations and cookies.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PasswordSignIn {
    pub email_address: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GoogleSignInStart {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TransferSignIn {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChallengeSubmission {
    pub code: String,
    pub remember_device: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SignOut {
    pub push_subscription_endpoint: Option<String>,
}

/// Credential refusals keep their HTTP status, but are next actions rather than expired sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum AuthResponse {
    SignedIn {
        location: String,
    },
    SecondFactorRequired {
        challenge: ChallengeState,
    },
    Navigate {
        location: String,
    },
    Error {
        field_errors: BTreeMap<String, Vec<String>>,
    },
}

/// No pending identity, secret, recovery codes or session token is sent to the browser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChallengeState {
    pub methods: Vec<ChallengeMethod>,
    pub remember_device: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ChallengeMethod {
    Totp,
    RecoveryCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SignedOut {
    SignedOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SignedOutBoot {
    pub kind: SignedOut,
    pub workspace: SignInWorkspace,
    pub sign_in_methods: SignInMethods,
    pub first_run_pending: bool,
    /// The first administrator, whom the retained sign-in page names for help.
    pub help_contact: Option<SignInHelpContact>,
    /// The version the retained sign-in page prints under the help contact.
    pub version: String,
    pub csrf_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SignInHelpContact {
    pub name: String,
    pub email_address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SignInWorkspace {
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SignInMethods {
    pub password: bool,
    pub google: bool,
    /// The Google Workspace domains Google sign-in accepts, named under its button.
    pub google_domains: Vec<String>,
}

/// `POST /api/v1/join/:join_code` and `POST /api/v1/invites/:token`: the retained join form's
/// fields. Sent as multipart form parts beside an optional `avatar` file (a JSON body without the
/// avatar is read the same way).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceJoin {
    pub name: String,
    pub email_address: String,
    pub password: String,
}

/// `GET /api/v1/join/:join_code` and `GET /api/v1/invites/:token`: what the retained join page
/// shows. A dead invite answers `inviteInvalid` with the retained page's status (410, or 404 for
/// an unknown token), as does a submission to one. Access gates and a wrong join code answer an
/// [`AuthResponse`], as the session endpoints do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum JoinPage {
    Join {
        workspace: SignInWorkspace,
        help_contact: Option<SignInHelpContact>,
    },
    InviteInvalid {
        workspace: SignInWorkspace,
        help_contact: Option<SignInHelpContact>,
        refusal: InviteRefusal,
        /// The retained page's sentence for the refusal ("It has expired.").
        reason: String,
    },
}

/// Why a workspace invite can't enroll anyone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum InviteRefusal {
    Expired,
    Exhausted,
    Revoked,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::assert_wire;
    use serde_json::json;

    #[test]
    fn auth_actions_round_trip_without_pending_identity() {
        assert_wire(
            &AuthResponse::SignedIn {
                location: "/app/".into(),
            },
            json!({"kind":"signedIn","location":"/app/"}),
        );
        assert_wire(
            &AuthResponse::Navigate {
                location: "/session/new".into(),
            },
            json!({"kind":"navigate","location":"/session/new"}),
        );
        assert_wire(
            &AuthResponse::SecondFactorRequired {
                challenge: ChallengeState {
                    methods: vec![ChallengeMethod::Totp, ChallengeMethod::RecoveryCode],
                    remember_device: true,
                },
            },
            json!({"kind":"secondFactorRequired","challenge":{"methods":["totp","recoveryCode"],"rememberDevice":true}}),
        );
        assert_wire(
            &AuthResponse::Error {
                field_errors: [("code".into(), vec!["That code didn't work.".into()])].into(),
            },
            json!({"kind":"error","fieldErrors":{"code":["That code didn't work."]}}),
        );
    }

    #[test]
    fn public_boot_and_auth_requests_round_trip() {
        assert_wire(
            &SignedOutBoot {
                kind: SignedOut::SignedOut,
                workspace: SignInWorkspace {
                    name: None,
                    logo_url: None,
                    description: String::new(),
                },
                sign_in_methods: SignInMethods {
                    password: true,
                    google: true,
                    google_domains: vec!["example.com".into()],
                },
                first_run_pending: true,
                help_contact: Some(SignInHelpContact {
                    name: "Ada".into(),
                    email_address: "ada@example.com".into(),
                }),
                version: "1.2.3".into(),
                csrf_token: "masked".into(),
            },
            json!({"kind":"signedOut","workspace":{"name":null,"logoUrl":null,"description":""},"signInMethods":{"password":true,"google":true,"googleDomains":["example.com"]},"firstRunPending":true,"helpContact":{"name":"Ada","emailAddress":"ada@example.com"},"version":"1.2.3","csrfToken":"masked"}),
        );
        assert_wire(
            &PasswordSignIn {
                email_address: "ada@example.com".into(),
                password: "secret".into(),
            },
            json!({"emailAddress":"ada@example.com","password":"secret"}),
        );
        assert_wire(
            &ChallengeSubmission {
                code: "123456".into(),
                remember_device: false,
            },
            json!({"code":"123456","rememberDevice":false}),
        );
        assert_wire(
            &SignOut {
                push_subscription_endpoint: None,
            },
            json!({"pushSubscriptionEndpoint":null}),
        );
        assert_wire(
            &WorkspaceJoin {
                name: "Ada".into(),
                email_address: "ada@example.com".into(),
                password: "secret".into(),
            },
            json!({"name":"Ada","emailAddress":"ada@example.com","password":"secret"}),
        );
        assert_wire(&GoogleSignInStart {}, json!({}));
        assert_wire(&TransferSignIn {}, json!({}));
    }

    #[test]
    fn join_pages_round_trip() {
        let workspace = SignInWorkspace {
            name: Some("Harbor".into()),
            logo_url: None,
            description: "Crew".into(),
        };
        assert_wire(
            &JoinPage::Join {
                workspace: workspace.clone(),
                help_contact: None,
            },
            json!({"kind":"join","workspace":{"name":"Harbor","logoUrl":null,"description":"Crew"},"helpContact":null}),
        );
        assert_wire(
            &JoinPage::InviteInvalid {
                workspace,
                help_contact: Some(SignInHelpContact {
                    name: "Ada".into(),
                    email_address: "ada@example.com".into(),
                }),
                refusal: InviteRefusal::Exhausted,
                reason: "All its uses have been taken.".into(),
            },
            json!({"kind":"inviteInvalid","workspace":{"name":"Harbor","logoUrl":null,"description":"Crew"},"helpContact":{"name":"Ada","emailAddress":"ada@example.com"},"refusal":"exhausted","reason":"All its uses have been taken."}),
        );
    }
}
