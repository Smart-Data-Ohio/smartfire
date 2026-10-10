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
    Ready { reauthentication: SudoState },
    Confirmed { retry: Option<SudoRetry> },
    Navigate { location: String },
    Error { message: String },
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
    pub csrf_token: String,
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
            },
            json!({"kind":"ready","reauthentication":{"methods":["password","totp","google"],"retry":{"method":"PATCH","path":"/api/v1/admin/custom_styles","returnTo":"/app/admin"}}}),
        );
        assert_wire(
            &SudoResponse::Confirmed { retry: Some(retry) },
            json!({"kind":"confirmed","retry":{"method":"PATCH","path":"/api/v1/admin/custom_styles","returnTo":"/app/admin"}}),
        );
        assert_wire(
            &SudoResponse::Confirmed { retry: None },
            json!({"kind":"confirmed","retry":null}),
        );
        assert_wire(
            &SudoResponse::Navigate {
                location: "https://accounts.google.com/authorize".into(),
            },
            json!({"kind":"navigate","location":"https://accounts.google.com/authorize"}),
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
                    name: None,
                    logo_url: None,
                    description: String::new(),
                },
                sign_in_methods: SignInMethods {
                    password: true,
                    google: false,
                },
                first_run_pending: true,
                csrf_token: "masked".into(),
            },
            json!({"kind":"signedOut","workspace":{"name":null,"logoUrl":null,"description":""},"signInMethods":{"password":true,"google":false},"firstRunPending":true,"csrfToken":"masked"}),
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
        assert_wire(&GoogleSignInStart {}, json!({}));
        assert_wire(&TransferSignIn {}, json!({}));
    }
}
