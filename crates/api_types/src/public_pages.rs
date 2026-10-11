//! About, Privacy and Terms for the SPA: what the retained public pages show, for anyone.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PublicPageName {
    About,
    Privacy,
    Terms,
}

/// `GET /api/v1/public_pages/{about,privacy,terms}`, signed in or out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PublicPage {
    pub page: PublicPageName,
    /// The retained page's `<title>`.
    pub title: String,
    /// The retained page's meta description.
    pub description: String,
    pub policy: PublicPagePolicy,
    /// The retained page's article, rendered by the same template; its links name the SPA's pages.
    pub html: String,
}

/// The installation's `LEGAL_*` settings as the pages read them: who runs the workspace, where to
/// write, and when the policies took effect. An absent or invalid name or address is `null`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PublicPagePolicy {
    pub operator_name: Option<String>,
    pub contact_email: Option<String>,
    pub effective_date: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::assert_wire;
    use serde_json::json;

    #[test]
    fn public_page_round_trips() {
        assert_wire(
            &PublicPage {
                page: PublicPageName::Privacy,
                title: "Smartfire | Privacy Policy".into(),
                description: "Policy.".into(),
                policy: PublicPagePolicy {
                    operator_name: Some("Acme".into()),
                    contact_email: None,
                    effective_date: "September 18, 2026".into(),
                },
                html: "<h1>Privacy Policy</h1>".into(),
            },
            json!({"page":"privacy","title":"Smartfire | Privacy Policy","description":"Policy.","policy":{"operatorName":"Acme","contactEmail":null,"effectiveDate":"September 18, 2026"},"html":"<h1>Privacy Policy</h1>"}),
        );
    }
}
