// Viewer facts for the Google profile panels; domain policy stays outside the view layer.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Account {
    pub email: String,
    pub connected: bool,
    pub calendar: bool,
    pub drive: bool,
}
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct CalendarData {
    pub configured: bool,
    pub account: Option<Account>,
}
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct SignInData {
    pub configured: bool,
    pub email: Option<String>,
}
