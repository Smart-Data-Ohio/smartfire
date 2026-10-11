// Viewer facts for the Google profile panels; domain policy stays outside the view layer.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Account {
    pub email: String,
    pub connected: bool,
    pub calendar: bool,
    pub drive: bool,
}
