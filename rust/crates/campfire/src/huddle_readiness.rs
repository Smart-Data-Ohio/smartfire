//! Huddle.configured?: all five settings present, recognized schemes and distinct host/port.
//! This exposes readiness only; gateway/token/grant operations remain WS13's.
pub(crate) fn huddles_configured(get: &impl Fn(&str) -> Option<String>) -> bool {
    let required = [
        "LIVEKIT_URL",
        "LIVEKIT_INTERNAL_URL",
        "LIVEKIT_API_KEY",
        "LIVEKIT_API_SECRET",
        "LIVEKIT_GATEWAY_SECRET",
    ];
    if required
        .iter()
        .any(|key| get(key).is_none_or(|s| campfire_richtext::ruby::is_blank(&s)))
    {
        return false;
    }
    let endpoint = |value: String| {
        let uri: axum::http::Uri = value.parse().ok()?;
        let port = match uri.scheme_str()? {
            "http" | "ws" => 80,
            "https" | "wss" => 443,
            _ => return None,
        };
        Some((uri.host()?.to_lowercase(), uri.port_u16().unwrap_or(port)))
    };
    match (
        get("LIVEKIT_URL").and_then(endpoint),
        get("LIVEKIT_INTERNAL_URL").and_then(endpoint),
    ) {
        (Some(public), Some(internal)) => public != internal,
        _ => false,
    }
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
