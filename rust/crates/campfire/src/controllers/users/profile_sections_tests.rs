//! Complete owner-input fragments, with real Rails configuration/account states.
use crate::controllers::presenters::test_support::*;
use askama::Template;
use campfire_views::users;
async fn calendar_case(name: &str) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_profile_sections.json"
    ))
    .unwrap();
    let case = vectors["google_calendar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap();
    let data = serde_json::from_value(case["input"].clone()).unwrap();
    let actual = super::people_tests::render(&app, |_| {
        users::GoogleCalendar {
            sections: users::ProfileSections {
                google: data,
                ..Default::default()
            },
        }
        .render()
        .unwrap()
    });
    assert_eq!(
        actual,
        case["html"].as_str().unwrap(),
        "{name}: complete Google Calendar fragment"
    );
}
macro_rules! cases {
    ($($name:ident),* $(,)?) => { $(#[tokio::test] async fn $name() { calendar_case(stringify!($name)).await; })* };
}
cases!(
    unconfigured,
    missing,
    calendar_only,
    calendar_drive,
    drive_only,
    rejected_drive,
    rejected_calendar
);
