//! `app/models/public_policy.rb`: installation identity without workspace state.

use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Debug)]
pub struct PublicPolicy {
    pub operator_name: Option<String>,
    pub contact_email: Option<String>,
    pub effective_date: String,
}

impl PublicPolicy {
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        static EMAIL: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"\A[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)+\z")
                .unwrap()
        });
        static DATE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\A[\p{Alphabetic}\p{Decimal_Number} ,.\-]{1,40}\z").unwrap());
        let value = |name| {
            get(name)
                .unwrap_or_default()
                .trim_matches(|c: char| c == '\0' || c.is_ascii_whitespace())
                .to_string()
        };
        let operator = value("LEGAL_OPERATOR_NAME");
        let email = value("LEGAL_CONTACT_EMAIL");
        let date = value("LEGAL_EFFECTIVE_DATE");
        Self {
            operator_name: (!operator.chars().all(char::is_whitespace)).then_some(operator),
            contact_email: EMAIL.is_match(&email).then_some(email),
            effective_date: if DATE.is_match(&date) {
                date
            } else {
                "September 18, 2026".into()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_matches_rails_environment_vectors() {
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("../../../vectors/users_public.json")).unwrap();
        for vector in vectors["policy"].as_array().unwrap() {
            let policy =
                PublicPolicy::from_lookup(|_| vector["input"].as_str().map(str::to_string));
            assert_eq!(
                serde_json::to_value(&policy.operator_name).unwrap(),
                vector["operator_name"]
            );
            assert_eq!(
                serde_json::to_value(&policy.contact_email).unwrap(),
                vector["contact_email"]
            );
            assert_eq!(policy.effective_date, vector["effective_date"]);
        }
    }
}
