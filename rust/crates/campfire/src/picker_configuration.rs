//! Public browser configuration adapter from WS13 abf4ecde's runtime chrome seam.
//! No account credentials, tokens or client secrets are read here.
pub(crate) fn picker(mut get: impl FnMut(&str) -> Option<String>) -> Option<campfire_views::layouts::GooglePicker> {
    let mut present = |name| get(name).filter(|s| !campfire_richtext::ruby::is_blank(s));
    Some(campfire_views::layouts::GooglePicker {
        client_id: present("GOOGLE_CLIENT_ID")?,
        api_key: present("GOOGLE_PICKER_API_KEY")?,
        project_number: present("GOOGLE_CLOUD_PROJECT_NUMBER")?,
    })
}
