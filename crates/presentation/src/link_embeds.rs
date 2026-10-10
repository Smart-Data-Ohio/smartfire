
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Card {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub image_url: Option<String>,
}
impl Card {
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref().filter(|s| !crate::helpers::is_blank(s))
    }
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref().filter(|s| !crate::helpers::is_blank(s))
    }
    pub fn site_name(&self) -> Option<&str> {
        self.site_name.as_deref().filter(|s| !crate::helpers::is_blank(s))
    }
    pub fn image(&self) -> Option<&str> {
        self.image_url.as_deref().filter(|s| !crate::helpers::is_blank(s))
    }
    pub fn usable(&self) -> bool {
        self.title().is_some() || self.description().is_some()
    }
}
