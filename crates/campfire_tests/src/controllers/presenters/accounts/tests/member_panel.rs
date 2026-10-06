use super::*;

#[tokio::test]
async fn rooms_mount_the_member_panel_with_the_selection_form_and_toggle() {
    let test = boot_seed("default")
        .await
        .expect("build default parity seed");
    let mut browser = test.browser("198.51.100.114");
    browser.sign_in(&test.label("emails.david")).await;
    for label in ["rooms.pets", "rooms.watercooler", "rooms.david_and_kevin"] {
        let id = test.label(label);
        let page = browser.get(&format!("/rooms/{id}")).await;
        assert_eq!(page.status, StatusCode::OK, "{label}");
        let html = page.text();
        assert_eq!(html.matches("id=\"channel-members\"").count(), 1);
        assert!(html.contains(&format!("data-members-url=\"/rooms/{id}/members.json\"")));
        assert!(html.contains("data-member-panel-target=\"toggle\""));
        assert!(html.contains("sidebar room-workspace"));
        assert!(html.contains("data-card-url-template=\"/users/USER_ID/card\""));
        assert!(html.contains("data-star-url-template=\"/users/USER_ID/star\""));
        page.assert_form("/rooms/directs");
        let frame = browser
            .request(
                Method::GET,
                &format!("/rooms/{id}"),
                &[("turbo-frame", "room")],
                None,
            )
            .await;
        assert_eq!(frame.status, StatusCode::OK);
        assert!(
            !frame.text().contains("id=\"channel-members\""),
            "a Turbo frame must not duplicate the layout panel"
        );
    }
}
