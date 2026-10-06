use campfire_views::{AccountSummary, ViewContext};
pub fn context<'a>(
    asset: &'a dyn Fn(&str) -> String,
    signer: &'a dyn Fn(&[&str]) -> String,
) -> ViewContext<'a> {
    ViewContext {
        current_user: None,
        account: AccountSummary {
            name: "Smartfire".into(),
            logo_url: String::new(),
            has_logo: false,
        },
        flash_notice: None,
        flash_alert: None,
        platform: Default::default(),
        vapid_public_key: None,
        asset_path: asset,
        importmap_tags: "",
        stylesheet_tags: "",
        custom_styles: None,
        cable_url: "/cable".into(),
        base_url: "http://campfire.test".into(),
        request_url: "http://campfire.test/".into(),
        referrer: None,
        last_room_visited_id: None,
        app_version: "parity".into(),
        signed_stream_name: signer,
        time_zone: campfire_views::time::Zone::utc(),
        chrome: Default::default(),
    }
}
