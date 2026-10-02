//! Slash-command readiness uses the WS13 LiveKit configuration contract.
pub(crate) fn huddles_configured(get: &impl Fn(&str) -> Option<String>) -> bool {
    crate::huddle::Config::from_lookup(get).configured()
}

#[cfg(test)]
mod tests {
    #[test]
    fn readiness_matches_pinned_rails() {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../../../vectors/messaging/slash.json")).unwrap();
        for case in data["readiness"].as_array().unwrap() {
            let get = |key: &str| case["env"][key].as_str().map(str::to_owned);
            assert_eq!(
                super::huddles_configured(&get),
                case["configured"].as_bool().unwrap(),
                "{}",
                case["env"]
            );
        }
    }
}
