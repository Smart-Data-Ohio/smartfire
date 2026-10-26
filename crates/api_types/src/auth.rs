//! Authentication contracts. Retained forms use the same operations and cookies.
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

/// `POST /api/v1/first_run`: the first administrator, as the retained `/first_run` form's `user`
/// fields. Sent as the JSON body, or with an avatar as `multipart/form-data`: this JSON in a
/// `submission` field and the picture in an `avatar` file field. Answers an [`AuthResponse`]:
/// `signedIn` when set up, `navigate` home once the workspace already exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FirstRunSubmission {
    /// Required: the retained form fails without one (after committing the account).
    pub name: String,
    /// Omitted (or null) leaves the administrator without an email address, as a retained
    /// submission without the field does; an empty string is kept as one.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email_address: Option<String>,
    /// Omitted, null or empty leaves the administrator without a password, as the retained
    /// form does.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// `GET /api/v1/first_run`: whether the workspace can still be set up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum FirstRunState {
    /// No workspace yet: the form is open, and writes carry this token.
    Pending { csrf_token: String },
    /// Already set up: where the retained page redirects (home).
    Navigate { location: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TwoFactorSetupSubmission {
    pub code: String,
}

/// Provisioning material is private to the signed-in browser's pending setup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TwoFactorSetupState {
    pub secret: String,
    pub manual_key: String,
    pub otpauth_uri: String,
    pub qr_svg: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum TwoFactorSetupResponse {
    Ready {
        setup: TwoFactorSetupState,
    },
    /// Returned by successful confirmation only. Plaintext codes are never persisted.
    RecoveryCodes {
        codes: Vec<String>,
        signed_out: usize,
        continue_url: String,
    },
    Navigate {
        location: String,
    },
    Error {
        message: String,
        setup: TwoFactorSetupState,
    },
}

/// An enrollment gate carries a next step, never provisioning material or a pending identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum TwoFactorRequirement {
    Setup { location: String },
    Challenge { location: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SignOut {
    pub push_subscription_endpoint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum SudoSubmission {
    Password { password: String },
    Totp { code: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SudoMethod {
    Password,
    Totp,
    Google,
}

/// `POST /api/v1/sudo/google`: the SPA's id for this Google confirmation. The continuation names it
/// once Google confirms, so the page replays only the writes it kept under that id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SudoGoogleStart {
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<String>,
}

/// The caller retains the write body; it is never echoed or stored in the cookie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SudoRetry {
    pub method: String,
    pub path: String,
    pub return_to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SudoState {
    pub methods: Vec<SudoMethod>,
    pub retry: Option<SudoRetry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum SudoResponse {
    Ready {
        reauthentication: SudoState,
        /// The signed-in session, so the SPA can tell a Google hand-off from another session's.
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session: Option<i64>,
    },
    Confirmed {
        /// The request the gate holds for this session: `GET /api/v1/sudo` only peeks at it; a
        /// confirmation (`POST /api/v1/sudo`) or the continuation hands it over and lets it go.
        retry: Option<SudoRetry>,
        /// The Google attempt this session confirmed, when the continuation asked for it by id.
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attempt: Option<String>,
        /// The signed-in session, so the SPA can tell a Google hand-off from another session's.
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session: Option<i64>,
    },
    Navigate {
        location: String,
        /// The signed-in session, so the SPA can tell a Google hand-off from another session's.
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session: Option<i64>,
    },
    Error {
        message: String,
    },
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
    /// The notice or alert a redirect left for this page load (a refused Google sign-in, say), as
    /// the retained auth layout shows it above the card. Read once: the next load has none.
    pub flash: Option<FlashMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FlashMessage {
    pub kind: FlashMessageKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FlashMessageKind {
    Notice,
    Alert,
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
    pub invite_splash_url: Option<String>,
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
    fn setup_contracts_round_trip() {
        let setup = TwoFactorSetupState {
            secret: "ABCD1234".into(),
            manual_key: "ABCD 1234".into(),
            otpauth_uri: "otpauth://totp/Smartfire:ada?secret=ABCD1234".into(),
            qr_svg: "<svg/>".into(),
        };
        let wire = json!({"secret":"ABCD1234","manualKey":"ABCD 1234","otpauthUri":"otpauth://totp/Smartfire:ada?secret=ABCD1234","qrSvg":"<svg/>"});
        assert_wire(
            &TwoFactorSetupSubmission {
                code: "123 456".into(),
            },
            json!({"code":"123 456"}),
        );
        assert_wire(
            &TwoFactorSetupResponse::Ready {
                setup: setup.clone(),
            },
            json!({"kind":"ready","setup":wire}),
        );
        assert_wire(
            &TwoFactorSetupResponse::Error {
                message: "That code didn't work.".into(),
                setup,
            },
            json!({"kind":"error","message":"That code didn't work.","setup":wire}),
        );
        assert_wire(
            &TwoFactorSetupResponse::RecoveryCodes {
                codes: vec!["abcd-1234-efgh".into()],
                signed_out: 2,
                continue_url: "/app/".into(),
            },
            json!({"kind":"recoveryCodes","codes":["abcd-1234-efgh"],"signedOut":2,"continueUrl":"/app/"}),
        );
        assert_wire(
            &TwoFactorSetupResponse::Navigate {
                location: "/session/new".into(),
            },
            json!({"kind":"navigate","location":"/session/new"}),
        );
        assert_wire(
            &TwoFactorRequirement::Setup {
                location: "/two_factor_setup".into(),
            },
            json!({"kind":"setup","location":"/two_factor_setup"}),
        );
        assert_wire(
            &TwoFactorRequirement::Challenge {
                location: "/two_factor_challenge".into(),
            },
            json!({"kind":"challenge","location":"/two_factor_challenge"}),
        );
    }

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
    fn sudo_requests_and_actions_round_trip() {
        assert_wire(
            &SudoSubmission::Password {
                password: "secret".into(),
            },
            json!({"kind":"password","password":"secret"}),
        );
        assert_wire(
            &SudoSubmission::Totp {
                code: "123456".into(),
            },
            json!({"kind":"totp","code":"123456"}),
        );
        let retry = SudoRetry {
            method: "PATCH".into(),
            path: "/api/v1/admin/custom_styles".into(),
            return_to: "/app/admin".into(),
        };
        assert_wire(
            &SudoResponse::Ready {
                reauthentication: SudoState {
                    methods: vec![SudoMethod::Password, SudoMethod::Totp, SudoMethod::Google],
                    retry: Some(retry.clone()),
                },
                session: Some(12),
            },
            json!({"kind":"ready","reauthentication":{"methods":["password","totp","google"],"retry":{"method":"PATCH","path":"/api/v1/admin/custom_styles","returnTo":"/app/admin"}},"session":12}),
        );
        assert_wire(
            &SudoResponse::Confirmed {
                retry: Some(retry.clone()),
                attempt: None,
                session: None,
            },
            json!({"kind":"confirmed","retry":{"method":"PATCH","path":"/api/v1/admin/custom_styles","returnTo":"/app/admin"}}),
        );
        assert_wire(
            &SudoResponse::Confirmed {
                retry: Some(retry),
                attempt: Some("attempt-0000000000000001".into()),
                session: Some(12),
            },
            json!({"kind":"confirmed","retry":{"method":"PATCH","path":"/api/v1/admin/custom_styles","returnTo":"/app/admin"},"attempt":"attempt-0000000000000001","session":12}),
        );
        assert_wire(
            &SudoGoogleStart {
                attempt: Some("attempt-0000000000000001".into()),
            },
            json!({"attempt":"attempt-0000000000000001"}),
        );
        assert_wire(
            &SudoResponse::Confirmed {
                retry: None,
                attempt: None,
                session: None,
            },
            json!({"kind":"confirmed","retry":null}),
        );
        assert_wire(
            &SudoResponse::Navigate {
                location: "https://accounts.google.com/authorize".into(),
                session: Some(12),
            },
            json!({"kind":"navigate","location":"https://accounts.google.com/authorize","session":12}),
        );
        assert_wire(
            &SudoResponse::Error {
                message: "Confirmation failed. Try again.".into(),
            },
            json!({"kind":"error","message":"Confirmation failed. Try again."}),
        );
    }

    #[test]
    fn public_boot_and_auth_requests_round_trip() {
        assert_wire(
            &SignedOutBoot {
                kind: SignedOut::SignedOut,
                workspace: SignInWorkspace {
                    invite_splash_url: None,
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
                flash: Some(FlashMessage {
                    kind: FlashMessageKind::Alert,
                    message: "Google sign-in failed.".into(),
                }),
            },
            json!({"kind":"signedOut","workspace":{"name":null,"inviteSplashUrl":null,"logoUrl":null,"description":""},"signInMethods":{"password":true,"google":true,"googleDomains":["example.com"]},"firstRunPending":true,"helpContact":{"name":"Ada","emailAddress":"ada@example.com"},"version":"1.2.3","csrfToken":"masked","flash":{"kind":"alert","message":"Google sign-in failed."}}),
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
        assert_wire(
            &FirstRunSubmission {
                name: "Ada".into(),
                email_address: Some("ada@example.com".into()),
                password: Some(String::new()),
            },
            json!({"name":"Ada","emailAddress":"ada@example.com","password":""}),
        );
        assert_wire(
            &FirstRunSubmission {
                name: "Ada".into(),
                email_address: None,
                password: None,
            },
            json!({"name":"Ada"}),
        );
        assert_eq!(
            serde_json::from_value::<FirstRunSubmission>(
                json!({"name":"Ada","emailAddress":null,"password":null})
            )
            .unwrap(),
            FirstRunSubmission {
                name: "Ada".into(),
                email_address: None,
                password: None,
            }
        );
        assert_wire(
            &FirstRunState::Pending {
                csrf_token: "masked".into(),
            },
            json!({"kind":"pending","csrfToken":"masked"}),
        );
        assert_wire(
            &FirstRunState::Navigate {
                location: "/".into(),
            },
            json!({"kind":"navigate","location":"/"}),
        );
        assert_wire(&GoogleSignInStart {}, json!({}));
        assert_wire(&TransferSignIn {}, json!({}));
    }

    #[test]
    fn join_pages_round_trip() {
        let workspace = SignInWorkspace {
            invite_splash_url: None,
            name: Some("Harbor".into()),
            logo_url: None,
            description: "Crew".into(),
        };
        assert_wire(
            &JoinPage::Join {
                workspace: workspace.clone(),
                help_contact: None,
            },
            json!({"kind":"join","workspace":{"name":"Harbor","inviteSplashUrl":null,"logoUrl":null,"description":"Crew"},"helpContact":null}),
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
            json!({"kind":"inviteInvalid","workspace":{"name":"Harbor","inviteSplashUrl":null,"logoUrl":null,"description":"Crew"},"helpContact":{"name":"Ada","emailAddress":"ada@example.com"},"refusal":"exhausted","reason":"All its uses have been taken."}),
        );
    }
}
