//! `ApplicationPlatform` (reference/app/models/application_platform.rb, over platform_agent 1.0.1)
//! and the `allow_browser` check from reference/app/controllers/concerns/allow_browser.rb.

use super::user_agent::{self, Agent, Raised, Rb, Version};

/// `ApplicationPlatform.new(request.user_agent)`. Predicates marked "raises" in Ruby (a nil
/// `user_agent.browser`, which Rails turns into a 500) answer false here.
#[derive(Debug, Clone)]
pub struct ApplicationPlatform {
    user_agent_string: String,
    user_agent: Agent,
}

impl ApplicationPlatform {
    pub fn new(user_agent: Option<&str>) -> Self {
        // `match?` works on `user_agent_string.to_s`, and UserAgent.parse treats nil like "".
        let user_agent_string = user_agent.unwrap_or("").to_string();
        let user_agent = user_agent::parse(&user_agent_string);
        Self {
            user_agent_string,
            user_agent,
        }
    }

    fn matches(&self, needle: &str) -> bool {
        self.user_agent_string.contains(needle)
    }

    fn browser_matches(&self, needles: &[&str]) -> Rb<bool> {
        let browser = self.user_agent.try_browser()?.ok_or(Raised)?;
        Ok(needles.iter().any(|needle| browser.contains(needle)))
    }

    pub fn ios(&self) -> bool {
        self.matches("iPhone") || self.matches("iPad")
    }

    pub fn android(&self) -> bool {
        self.matches("Android")
    }

    pub fn mac(&self) -> bool {
        self.matches("Macintosh")
    }

    pub fn chrome(&self) -> bool {
        self.try_chrome().unwrap_or(false)
    }

    pub fn firefox(&self) -> bool {
        self.try_firefox().unwrap_or(false)
    }

    pub fn safari(&self) -> bool {
        self.try_safari().unwrap_or(false)
    }

    pub fn edge(&self) -> bool {
        self.try_edge().unwrap_or(false)
    }

    fn try_chrome(&self) -> Rb<bool> {
        self.browser_matches(&["Chrome"])
    }

    fn try_firefox(&self) -> Rb<bool> {
        self.browser_matches(&["Firefox", "FxiOS"])
    }

    fn try_safari(&self) -> Rb<bool> {
        self.browser_matches(&["Safari"])
    }

    fn try_edge(&self) -> Rb<bool> {
        self.browser_matches(&["Edg"])
    }

    /// Apple Messages link previews claim to be both the Facebook and Twitter bots.
    pub fn apple_messages(&self) -> bool {
        let lowercased = self.user_agent_string.to_lowercase();
        lowercased.contains("facebookexternalhit") && lowercased.contains("twitterbot")
    }

    pub fn mobile(&self) -> bool {
        self.ios() || self.android()
    }

    pub fn desktop(&self) -> bool {
        !self.mobile()
    }

    pub fn windows(&self) -> bool {
        self.try_windows().unwrap_or(false)
    }

    fn try_windows(&self) -> Rb<bool> {
        Ok(self.try_operating_system()?.as_deref() == Some("Windows"))
    }

    /// `operating_system`: nil when the gem's `os` is nil.
    pub fn operating_system(&self) -> Option<String> {
        self.try_operating_system().ok().flatten()
    }

    fn try_operating_system(&self) -> Rb<Option<String>> {
        let platform = self.user_agent.try_platform()?.unwrap_or_default();
        let named = [
            ("Android", "Android"),
            ("iPad", "iPad"),
            ("iPhone", "iPhone"),
            ("Macintosh", "macOS"),
            ("Windows", "Windows"),
            ("CrOS", "ChromeOS"),
        ]
        .into_iter()
        .find(|(needle, _)| platform.contains(needle));

        Ok(match named {
            Some((_, name)) => Some(name.to_string()),
            None => self.user_agent.try_os()?.map(|os| {
                if os.contains("Linux") {
                    "Linux".into()
                } else {
                    os
                }
            }),
        })
    }

    /// `browser` (delegated to the useragent gem); nil is "".
    pub fn browser(&self) -> String {
        self.user_agent.browser()
    }

    #[allow(clippy::needless_update)]
    pub fn to_view(&self) -> campfire_presentation::Platform {
        campfire_presentation::Platform {
            ios: self.ios(),
            android: self.android(),
            mac: self.mac(),
            windows: self.windows(),
            chrome: self.chrome(),
            firefox: self.firefox(),
            safari: self.safari(),
            edge: self.edge(),
            mobile: self.mobile(),
            desktop: self.desktop(),
            apple_messages: self.apple_messages(),
            browser: self.browser(),
            operating_system: self.operating_system().unwrap_or_default(),
            ..Default::default()
        }
    }
}

/// `ActionController::AllowBrowser::BrowserBlocker#blocked?` with Campfire's
/// `AllowBrowser::VERSIONS = { safari: 17.2, chrome: 120, firefox: 121, opera: 104, ie: false }`.
/// Rails raises (a 500) for a versioned agent with a nil browser; that is not blocked here.
pub fn browser_blocked(user_agent: Option<&str>) -> bool {
    try_browser_blocked(user_agent).unwrap_or(false)
}

fn try_browser_blocked(user_agent: Option<&str>) -> Rb<bool> {
    let Some(user_agent) = user_agent.filter(|ua| user_agent::is_present(ua)) else {
        return Ok(false);
    };
    let agent = user_agent::parse(user_agent);
    let Some(version) = agent.try_version()?.filter(Version::is_present) else {
        return Ok(false);
    };

    let browser = agent.try_browser()?.ok_or(Raised)?.to_lowercase();
    // `nil` means the browser isn't version-guarded; `Some(None)` is `ie: false`, always blocked.
    let minimum = match browser.as_str() {
        "safari" => Some(Some("17.2")),
        "chrome" => Some(Some("120")),
        "firefox" => Some(Some("121")),
        "opera" => Some(Some("104")),
        "internet explorer" => Some(None),
        _ => None,
    };

    let Some(minimum) = minimum else {
        return Ok(false);
    };
    let below_minimum = minimum.is_none_or(|minimum| version < Version::new(minimum));
    Ok(below_minimum && !agent.is_bot())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concerns::user_agent::tests::{check, vectors};
    use serde_json::json;

    #[test]
    fn matches_application_platform_and_allow_browser() {
        let vectors = vectors();
        let mut failures = Vec::new();

        for case in vectors["user_agents"].as_array().unwrap() {
            let ua = case["ua"].as_str();
            let platform = ApplicationPlatform::new(ua);
            let expected = &case["application_platform"];
            let label = &case["ua"];
            let mut field = |name: &str, actual: Rb<serde_json::Value>| {
                check(
                    &mut failures,
                    &format!("{label} {name}"),
                    &expected[name],
                    actual,
                );
            };

            field("ios", Ok(json!(platform.ios())));
            field("android", Ok(json!(platform.android())));
            field("mac", Ok(json!(platform.mac())));
            field("chrome", platform.try_chrome().map(|v| json!(v)));
            field("firefox", platform.try_firefox().map(|v| json!(v)));
            field("safari", platform.try_safari().map(|v| json!(v)));
            field("edge", platform.try_edge().map(|v| json!(v)));
            field("apple_messages", Ok(json!(platform.apple_messages())));
            field("mobile", Ok(json!(platform.mobile())));
            field("desktop", Ok(json!(platform.desktop())));
            field("windows", platform.try_windows().map(|v| json!(v)));
            field(
                "operating_system",
                platform.try_operating_system().map(|v| json!(v)),
            );
            field(
                "browser",
                platform.user_agent.try_browser().map(|v| json!(v)),
            );

            check(
                &mut failures,
                &format!("{label} blocked"),
                &case["blocked"],
                try_browser_blocked(ua).map(|v| json!(v)),
            );
        }

        assert!(
            failures.is_empty(),
            "{} mismatches:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn view_platform_uses_empty_strings_for_nil() {
        let view = ApplicationPlatform::new(Some("curl/8.4.0")).to_view();
        assert_eq!(view.operating_system, "");
        assert_eq!(view.browser, "curl");
        assert!(view.desktop);
    }
}
