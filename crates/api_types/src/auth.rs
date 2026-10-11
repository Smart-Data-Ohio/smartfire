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
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum TwoFactorSetupResponse {
    Ready { setup: TwoFactorSetupState },
    /// Returned by successful confirmation only. Plaintext codes are never persisted.
    RecoveryCodes { codes: Vec<String>, signed_out: usize, continue_url: String },
    Navigate { location: String },
    Error { message: String, setup: TwoFactorSetupState },
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
    fn setup_contracts_round_trip() {
        let setup = TwoFactorSetupState {
            secret: "ABCD1234".into(),
            manual_key: "ABCD 1234".into(),
            otpauth_uri: "otpauth://totp/Smartfire:ada?secret=ABCD1234".into(),
            qr_svg: "<svg/>".into(),
        };
        let wire = json!({"secret":"ABCD1234","manualKey":"ABCD 1234","otpauthUri":"otpauth://totp/Smartfire:ada?secret=ABCD1234","qrSvg":"<svg/>"});
        assert_wire(&TwoFactorSetupSubmission { code: "123 456".into() }, json!({"code":"123 456"}));
        assert_wire(&TwoFactorSetupResponse::Ready { setup: setup.clone() }, json!({"kind":"ready","setup":wire}));
        assert_wire(&TwoFactorSetupResponse::Error { message: "That code didn't work.".into(), setup }, json!({"kind":"error","message":"That code didn't work.","setup":wire}));
        assert_wire(&TwoFactorSetupResponse::RecoveryCodes {
            codes: vec!["abcd-1234-efgh".into()], signed_out: 2, continue_url: "/app/".into(),
        }, json!({"kind":"recoveryCodes","codes":["abcd-1234-efgh"],"signedOut":2,"continueUrl":"/app/"}));
        assert_wire(&TwoFactorSetupResponse::Navigate { location: "/session/new".into() }, json!({"kind":"navigate","location":"/session/new"}));
        assert_wire(&TwoFactorRequirement::Setup { location: "/two_factor_setup".into() }, json!({"kind":"setup","location":"/two_factor_setup"}));
        assert_wire(&TwoFactorRequirement::Challenge { location: "/two_factor_challenge".into() }, json!({"kind":"challenge","location":"/two_factor_challenge"}));
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
