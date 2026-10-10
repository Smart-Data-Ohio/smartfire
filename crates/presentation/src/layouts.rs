pub use crate::context::{NotificationSounds, UserPreferences};

#[derive(Clone, Debug)]
pub struct GooglePicker {
    pub client_id: String,
    pub api_key: String,
    pub project_number: String,
}
