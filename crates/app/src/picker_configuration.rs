//! Public browser configuration adapter from WS13 abf4ecde's runtime chrome seam.
//! No account credentials, tokens or client secrets are read here.
pub(crate) fn picker(mut get: impl FnMut(&str) -> Option<String>) -> Option<campfire_presentation::layouts::GooglePicker> {
    let mut present = |name| get(name).filter(|s| !campfire_richtext::ruby::is_blank(s));
    Some(campfire_presentation::layouts::GooglePicker {
        client_id: present("GOOGLE_CLIENT_ID")?,
        api_key: present("GOOGLE_PICKER_API_KEY")?,
        project_number: present("GOOGLE_CLOUD_PROJECT_NUMBER")?,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn google_picker_public_configuration_matches_pinned_rails_without_client_secrets() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../vectors/google_picker.json")).unwrap();
        for row in oracle["rows"].as_array().unwrap() {
            let mut reads=Vec::new();
            let config=super::picker(|key| {reads.push(key.to_owned());row["values"][key].as_str().map(str::to_owned)});
            assert_eq!(config.is_some(), row["configured"].as_bool().unwrap(), "{row}");
            let public=config.map(|c|serde_json::json!({"client_id":c.client_id,"api_key":c.api_key,"project_number":c.project_number}));
            assert_eq!(serde_json::json!(public),row["public"],"{row}");
            assert!(reads.iter().all(|key|matches!(key.as_str(),"GOOGLE_CLIENT_ID"|"GOOGLE_PICKER_API_KEY"|"GOOGLE_CLOUD_PROJECT_NUMBER")));
        }
        println!("Pinned Rails public Picker configurations: {} exercised; 0 skipped; no client secrets",oracle["rows"].as_array().unwrap().len());
    }
}
