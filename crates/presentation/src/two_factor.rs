

#[derive(Clone, Default)]
pub struct ProfileData {
    pub confirmed_at: Option<jiff::Timestamp>,
    pub devices: Vec<Device>,
    pub google: bool,
}
#[derive(Clone)]
pub struct Device {
    pub id: i64,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub last_used_at: Option<jiff::Timestamp>,
}
/// The remembered device row's bold line, shared with the account JSON twin.
pub fn device_description(device: &Device) -> String {
    device
        .user_agent
        .as_deref()
        .filter(|s| !s.chars().all(char::is_whitespace))
        .unwrap_or("Unknown browser")
        .into()
}
