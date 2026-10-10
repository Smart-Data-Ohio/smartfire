//! Wire shapes of the S7 settings types.

use std::collections::BTreeMap;

use serde_json::json;

use crate::tests::assert_wire;
use crate::*;

fn settings() -> Settings {
    Settings {
        profile: ProfileSettings {
            user_id: 7,
            name: "Ada Lovelace".into(),
            email_address: Some("ada@example.com".into()),
            bio: None,
            avatar_url: "/users/7/avatar?v=1700000000".into(),
            avatar_attached: true,
            has_password: true,
            github_login: Some("ada".into()),
            github_verified: false,
            bot: false,
        },
        appearance: AppearanceSettings {
            theme: Theme::Dark,
            text_size: TextSize::Default,
            time_zone: Some("Europe/London".into()),
            time_zones: vec![TimeZoneChoice { label: "(GMT+00:00) London".into(), value: "Europe/London".into() }],
        },
        notifications: NotificationSettings {
            default_notification_level: NotificationLevel::Everything,
            room_notification_levels: Default::default(),
            room_mute_until: Default::default(),
            dnd_enabled: false,
            quiet_hours_enabled: true,
            quiet_hours_start: Some("22:00".into()),
            quiet_hours_end: Some("07:30".into()),
            meeting_dnd_enabled: false,
            ooo_notify_enabled: true,
            allowed_people: vec![DndAllowedPerson { user_id: 8, name: "Grace Hopper".into() }],
            keyword_alerts: vec!["deploy freeze".into()],
            inbox: vec![InboxSwitch {
                key: "github_review_requests".into(),
                label: "GitHub review requests".into(),
                description: "When someone asks you to review a pull request.".into(),
                enabled: true,
            }],
        },
        status: StatusSettings {
            presence_setting: PresenceSetting::Auto,
            custom_status_emoji: Some("🌴".into()),
            custom_status_text: Some("On a beach".into()),
            custom_status_expires_at: None,
            meeting_status_enabled: true,
            ooo_calendar_enabled: false,
            ooo_until: Some("2026-10-09T17:00:00.000Z".into()),
            ooo_manual: true,
            ooo_note: Some("Back Friday".into()),
            calendar_error: None,
        },
        calls: CallSettings { voice_mode: VoiceMode::PushToTalk, push_to_talk_key: Some("`".into()) },
        integrations: IntegrationSettings {
            google: GoogleIntegration {
                sign_in_configured: true,
                identity_email: None,
                calendar_configured: true,
                connected: true,
                calendar: true,
                drive: false,
                email: Some("ada@example.com".into()),
            },
            github: Connection::Connected { name: "ada".into(), workspace: None, app_token: false },
            github_app_configured: false,
            fizzy: Connection::Rejected { reason: Some("The token was revoked.".into()) },
            manage_path: "/users/me/profile".into(),
            slack_import_path: "/slack/imports".into(),
        },
    }
}

#[test]
fn settings_wire() {
    assert_wire(
        &settings(),
        json!({
            "profile": {
                "userId": 7,
                "name": "Ada Lovelace",
                "emailAddress": "ada@example.com",
                "bio": null,
                "avatarUrl": "/users/7/avatar?v=1700000000",
                "avatarAttached": true,
                "hasPassword": true,
                "githubLogin": "ada",
                "githubVerified": false,
                "bot": false
            },
            "appearance": {
                "theme": "dark",
                "textSize": "default",
                "timeZone": "Europe/London",
                "timeZones": [{ "label": "(GMT+00:00) London", "value": "Europe/London" }]
            },
            "notifications": {
                "defaultNotificationLevel": "everything",
                "roomNotificationLevels": {},
                "roomMuteUntil": {},
                "dndEnabled": false,
                "quietHoursEnabled": true,
                "quietHoursStart": "22:00",
                "quietHoursEnd": "07:30",
                "meetingDndEnabled": false,
                "oooNotifyEnabled": true,
                "allowedPeople": [{ "userId": 8, "name": "Grace Hopper" }],
                "keywordAlerts": ["deploy freeze"],
                "inbox": [{
                    "key": "github_review_requests",
                    "label": "GitHub review requests",
                    "description": "When someone asks you to review a pull request.",
                    "enabled": true
                }]
            },
            "status": {
                "presenceSetting": "auto",
                "customStatusEmoji": "🌴",
                "customStatusText": "On a beach",
                "customStatusExpiresAt": null,
                "meetingStatusEnabled": true,
                "oooCalendarEnabled": false,
                "oooUntil": "2026-10-09T17:00:00.000Z",
                "oooManual": true,
                "oooNote": "Back Friday",
                "calendarError": null
            },
            "calls": { "voiceMode": "push_to_talk", "pushToTalkKey": "`" },
            "integrations": {
                "google": {
                    "signInConfigured": true,
                    "identityEmail": null,
                    "calendarConfigured": true,
                    "connected": true,
                    "calendar": true,
                    "drive": false,
                    "email": "ada@example.com"
                },
                "github": { "state": "connected", "name": "ada", "workspace": null, "appToken": false },
                "githubAppConfigured": false,
                "fizzy": { "state": "rejected", "reason": "The token was revoked." },
                "managePath": "/users/me/profile",
                "slackImportPath": "/slack/imports"
            }
        }),
    );
    assert_wire(&Connection::Missing, json!({ "state": "missing" }));
}

#[test]
fn settings_writes_wire() {
    assert_wire(
        &UpdateProfile { name: Some("Ada".into()), current_password: Some("secret".into()), ..UpdateProfile::default() },
        json!({
            "name": "Ada",
            "emailAddress": null,
            "currentPassword": "secret",
            "password": null,
            "bio": null,
            "githubLogin": null
        }),
    );
    assert_wire(&UpdateAvatar { signed_id: "eyJf--1".into() }, json!({ "signedId": "eyJf--1" }));
    assert_wire(
        &UpdateAppearance { theme: Some(Theme::System), text_size: None, time_zone: Some(String::new()) },
        json!({ "theme": "system", "textSize": null, "timeZone": "" }),
    );
    assert_wire(
        &UpdateNotifications {
            dnd_enabled: Some(true),
            keyword_alerts: Some(vec!["prod".into()]),
            inbox: Some(BTreeMap::from([("mentions".into(), false)])),
            ..UpdateNotifications::default()
        },
        json!({
            "defaultNotificationLevel": null,
            "roomNotification": null,
            "roomMute": null,
            "dndEnabled": true,
            "quietHoursEnabled": null,
            "quietHoursStart": null,
            "quietHoursEnd": null,
            "meetingDndEnabled": null,
            "oooNotifyEnabled": null,
            "keywordAlerts": ["prod"],
            "inbox": { "mentions": false }
        }),
    );
    assert_wire(
        &UpdateStatus {
            presence_setting: Some(PresenceSetting::Dnd),
            custom_status_expires_in: Some(StatusExpiry::Minutes30),
            ooo_preset: Some(OooPreset::Custom),
            ooo_until_custom: Some("2026-10-09T17:00".into()),
            ..UpdateStatus::default()
        },
        json!({
            "presenceSetting": "dnd",
            "customStatusEmoji": null,
            "customStatusText": null,
            "customStatusExpiresIn": "minutes_30",
            "clearCustomStatus": null,
            "meetingStatusEnabled": null,
            "oooCalendarEnabled": null,
            "oooPreset": "custom",
            "oooUntilCustom": "2026-10-09T17:00",
            "oooNote": null,
            "clearOoo": null
        }),
    );
    for (expiry, wire) in [
        (StatusExpiry::Hour1, "hour_1"),
        (StatusExpiry::Hours4, "hours_4"),
        (StatusExpiry::Today, "today"),
        (StatusExpiry::Week, "week"),
        (StatusExpiry::Never, "never"),
    ] {
        assert_wire(&expiry, json!(wire));
    }
    assert_wire(
        &UpdateCalls { voice_mode: Some(VoiceMode::VoiceActivity), push_to_talk_key: None },
        json!({ "voiceMode": "voice_activity", "pushToTalkKey": null }),
    );
}

#[test]
fn sessions_and_push_subscriptions_wire() {
    assert_wire(
        &SessionList {
            sessions: vec![SessionInfo {
                id: 3,
                current: true,
                description: "Firefox on macOS".into(),
                ip_address: Some("203.0.113.9".into()),
                last_active_at: "2026-10-06T10:00:00.000Z".into(),
                created_at: "2026-10-01T09:00:00.000Z".into(),
            }],
            notice: Some("Signed out 1 other session.".into()),
        },
        json!({
            "sessions": [{
                "id": 3,
                "current": true,
                "description": "Firefox on macOS",
                "ipAddress": "203.0.113.9",
                "lastActiveAt": "2026-10-06T10:00:00.000Z",
                "createdAt": "2026-10-01T09:00:00.000Z"
            }],
            "notice": "Signed out 1 other session."
        }),
    );
    assert_wire(
        &PushSubscriptionList {
            push_subscriptions: vec![PushSubscriptionInfo {
                id: 4,
                endpoint: "https://push.example/abc".into(),
                browser: "Chrome".into(),
                version: "141".into(),
                platform: "Android".into(),
            }],
        },
        json!({
            "pushSubscriptions": [{
                "id": 4,
                "endpoint": "https://push.example/abc",
                "browser": "Chrome",
                "version": "141",
                "platform": "Android"
            }]
        }),
    );
}

#[test]
fn integration_changes_wire() {
    assert_wire(
        &IntegrationToken { access_token: "personal-token".into() },
        json!({ "accessToken": "personal-token" }),
    );
    assert_wire(
        &IntegrationChange {
            integrations: settings().integrations,
            notice: "GitHub connected as ada.".into(),
        },
        json!({
            "integrations": {
                "google": {
                    "signInConfigured": true,
                    "identityEmail": null,
                    "calendarConfigured": true,
                    "connected": true,
                    "calendar": true,
                    "drive": false,
                    "email": "ada@example.com"
                },
                "github": { "state": "connected", "name": "ada", "workspace": null, "appToken": false },
                "githubAppConfigured": false,
                "fizzy": { "state": "rejected", "reason": "The token was revoked." },
                "managePath": "/users/me/profile",
                "slackImportPath": "/slack/imports"
            },
            "notice": "GitHub connected as ada."
        }),
    );
}

#[test]
fn account_settings_and_two_factor_wire() {
    let room = RoomMembershipRow {
        room_id: 12, name: "Everyone".into(), involvement: Some(Involvement::Everything), direct: false,
    };
    let room_wire = json!({"roomId": 12, "name": "Everyone", "involvement": "everything", "direct": false});
    assert_wire(&room, room_wire.clone());
    assert_wire(&RoomMembershipRow {
        room_id: 14, name: "Old room".into(), involvement: None, direct: false,
    }, json!({"roomId": 14, "name": "Old room", "involvement": null, "direct": false}));
    let device = RememberedDevice {
        id: 6, description: "Unknown browser".into(), ip_address: None, last_used_at: None,
    };
    assert_wire(&device, json!({"id": 6, "description": "Unknown browser", "ipAddress": null, "lastUsedAt": null}));
    let two_factor = TwoFactorSettings {
        confirmed_at: Some("2026-10-07T10:00:00.000Z".into()), google: true,
        has_password: false, devices: vec![RememberedDevice {
            id: 7, description: "Firefox".into(), ip_address: Some("203.0.113.9".into()),
            last_used_at: Some("2026-10-07T11:00:00.000Z".into()),
        }],
    };
    let panel_wire = json!({
        "confirmedAt": "2026-10-07T10:00:00.000Z", "google": true, "hasPassword": false,
        "devices": [{"id": 7, "description": "Firefox", "ipAddress": "203.0.113.9", "lastUsedAt": "2026-10-07T11:00:00.000Z"}]
    });
    assert_wire(&two_factor, panel_wire.clone());
    assert_wire(&TwoFactorSettings {
        confirmed_at: None, google: false, has_password: true, devices: vec![],
    }, json!({"confirmedAt": null, "google": false, "hasPassword": true, "devices": []}));
    assert_wire(&AccountSettings {
        shared_rooms: vec![room], direct_rooms: vec![RoomMembershipRow {
            room_id: 13, name: "Grace".into(), involvement: Some(Involvement::Mentions), direct: true,
        }], two_factor: two_factor.clone(), transfer_url: "https://chat.example/session/transfers/signed".into(),
        transfer_qr_svg: "<svg/>".into(),
    }, json!({
        "sharedRooms": [room_wire],
        "directRooms": [{"roomId": 13, "name": "Grace", "involvement": "mentions", "direct": true}],
        "twoFactor": panel_wire, "transferUrl": "https://chat.example/session/transfers/signed",
        "transferQrSvg": "<svg/>"
    }));
    assert_wire(&Reauthentication { reauth: "123456".into() }, json!({"reauth": "123456"}));
    assert_wire(&Reauthentication { reauth: String::new() }, json!({"reauth": ""}));
    assert_wire(&BackupCodes { codes: vec!["1234-5678".into()] }, json!({"codes": ["1234-5678"]}));
    assert_wire(&TwoFactorChange {
        notice: "Device forgotten. It will ask for a code at next sign-in.".into(), two_factor,
    }, json!({"notice": "Device forgotten. It will ask for a code at next sign-in.", "twoFactor": panel_wire}));
}
