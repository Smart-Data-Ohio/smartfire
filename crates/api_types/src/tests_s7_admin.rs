//! Wire shapes of the S7 admin types.

use serde_json::json;

use crate::tests::assert_wire;
use crate::*;

fn person() -> Person {
    Person {
        id: 8,
        name: "Grace Hopper".into(),
        avatar_url: "/users/8/avatar?v=1700000000".into(),
        role: PersonRole::Administrator,
        banned: false,
        you: false,
        two_factor_enabled: true,
        email_address: Some("grace@example.com".into()),
        google_identity_email: None,
        offer_google_email_link: true,
    }
}

fn person_json() -> serde_json::Value {
    json!({
        "id": 8,
        "name": "Grace Hopper",
        "avatarUrl": "/users/8/avatar?v=1700000000",
        "role": "administrator",
        "banned": false,
        "you": false,
        "twoFactorEnabled": true,
        "emailAddress": "grace@example.com",
        "googleIdentityEmail": null,
        "offerGoogleEmailLink": true
    })
}

#[test]
fn the_workspace_and_its_writes() {
    assert_wire(
        &Workspace {
            name: "Smart Data".into(),
            logo_url: "/account/logo?v=1700000000".into(),
            logo_still_url: None,
            banner_url: None,
            banner_still_url: None,
            logo_attached: false,
            join_url: "https://chat.example/join/abc-123".into(),
            can_administer: true,
            restrict_room_creation_to_administrators: false,
            upload_limit_bytes: 104_857_600,
            version: "2.0.0".into(),
        },
        json!({
            "name": "Smart Data",
            "logoUrl": "/account/logo?v=1700000000",
            "logoStillUrl": null,
            "bannerUrl": null,
            "bannerStillUrl": null,
            "logoAttached": false,
            "joinUrl": "https://chat.example/join/abc-123",
            "canAdminister": true,
            "restrictRoomCreationToAdministrators": false,
            "uploadLimitBytes": 104857600,
            "version": "2.0.0"
        }),
    );
    assert_wire(
        &UpdateWorkspace {
            name: None,
            restrict_room_creation_to_administrators: Some(true),
            upload_limit_bytes: None,
        },
        json!({ "name": null, "restrictRoomCreationToAdministrators": true, "uploadLimitBytes": null }),
    );
    assert_wire(
        &UpdateLogo {
            signed_id: "blob-1".into(),
        },
        json!({ "signedId": "blob-1" }),
    );
    assert_wire(
        &UpdateBanner {
            signed_id: "blob-2".into(),
        },
        json!({ "signedId": "blob-2" }),
    );
}

#[test]
fn workspace_branding_sync_wire_shape() {
    let branding = WorkspaceBranding {
        name: "Smart Data".into(),
        logo_url: Some("/account/logo?v=1&animated=1".into()),
        logo_still_url: Some("/account/logo?v=1&still=1".into()),
        banner_url: Some("/account/banner?v=2".into()),
        banner_still_url: Some("/account/banner?v=2&still=1".into()),
    };
    assert_wire(
        &SyncPayload::WorkspaceUpdated(branding),
        json!({"type": "workspace.updated", "data": {
            "name": "Smart Data", "logoUrl": "/account/logo?v=1&animated=1", "logoStillUrl": "/account/logo?v=1&still=1",
            "bannerUrl": "/account/banner?v=2", "bannerStillUrl": "/account/banner?v=2&still=1"
        }}),
    );
}

#[test]
fn workspace_styles_sync_wire_shape() {
    for css in [None, Some(":root { --accent: red; }".into())] {
        assert_wire(
            &SyncPayload::WorkspaceStylesUpdated(CustomStyles { css: css.clone() }),
            json!({"type": "workspace.styles.updated", "data": {"css": css}}),
        );
    }
}

#[test]
fn people_and_the_changes_to_them() {
    assert_wire(
        &PeoplePage {
            people: vec![person()],
            next_page: Some("2".into()),
        },
        json!({ "people": [person_json()], "nextPage": "2" }),
    );
    assert_wire(
        &UpdatePerson {
            role: PersonRole::Member,
        },
        json!({ "role": "member" }),
    );
    assert_wire(
        &PersonChange {
            person: person(),
            notice: Some(
                "Two-step sign-in reset for Grace Hopper. They will set it up again at next sign-in."
                    .into(),
            ),
        },
        json!({
            "person": person_json(),
            "notice": "Two-step sign-in reset for Grace Hopper. They will set it up again at next sign-in."
        }),
    );
    assert_wire(&PersonRemoved { id: 8 }, json!({ "id": 8 }));
}

#[test]
fn custom_styles_and_icons() {
    assert_wire(&CustomStyles { css: None }, json!({ "css": null }));
    assert_wire(
        &WorkspaceIconList {
            icons: vec![WorkspaceIcon {
                id: 3,
                name: "acme".into(),
                title: "Acme Corp".into(),
                creator_name: "Ada Lovelace".into(),
                image_url: "/icons/acme".into(),
            }],
        },
        json!({
            "icons": [{
                "id": 3,
                "name": "acme",
                "title": "Acme Corp",
                "creatorName": "Ada Lovelace",
                "imageUrl": "/icons/acme"
            }]
        }),
    );
    assert_wire(
        &CreateIcon {
            name: "acme".into(),
            title: "Acme Corp".into(),
            signed_id: None,
        },
        json!({ "name": "acme", "title": "Acme Corp", "signedId": null }),
    );
}

#[test]
fn an_audit_log_page() {
    assert_wire(
        &AuditLogPage {
            filters: AuditLogFilters {
                actor: Some("ada".into()),
                action: None,
                target_type: Some("User".into()),
                from: Some("2026-10-01".into()),
                to: None,
            },
            entries: vec![AuditLogEntry {
                id: 41,
                created_at: "2026-10-06T10:00:00.000Z".into(),
                action: "user.role_change".into(),
                actor: Some("Ada Lovelace".into()),
                target: Some("Grace Hopper".into()),
                target_type: Some("User".into()),
                changes: "role: member → administrator".into(),
                ip_address: None,
            }],
            next_page: None,
            actions: vec!["user.role_change".into()],
            target_types: vec!["User".into()],
            export_url: "/account/audit_log.csv?actor=ada".into(),
            export_truncated: false,
            export_limit: 5000,
            time_zone: "America/New_York".into(),
        },
        json!({
            "filters": {
                "actor": "ada",
                "action": null,
                "targetType": "User",
                "from": "2026-10-01",
                "to": null
            },
            "entries": [{
                "id": 41,
                "createdAt": "2026-10-06T10:00:00.000Z",
                "action": "user.role_change",
                "actor": "Ada Lovelace",
                "target": "Grace Hopper",
                "targetType": "User",
                "changes": "role: member → administrator",
                "ipAddress": null
            }],
            "nextPage": null,
            "actions": ["user.role_change"],
            "targetTypes": ["User"],
            "exportUrl": "/account/audit_log.csv?actor=ada",
            "exportTruncated": false,
            "exportLimit": 5000,
            "timeZone": "America/New_York"
        }),
    );
}

#[test]
fn integrations_health() {
    let issue = || HealthIssue {
        subject: "ada".into(),
        detail: "token revoked".into(),
    };
    let issue_json = json!({ "subject": "ada", "detail": "token revoked" });
    assert_wire(
        &IntegrationsHealth {
            github: GithubHealth {
                workspace_token: true,
                app_configured: false,
                webhook_secret: true,
                connected: 3,
                app_tokens: 1,
                deliveries_24h: 12,
                disconnected: vec![issue()],
                last_errors: vec![],
                fetch_errors: vec![],
            },
            google: GoogleHealth {
                configured: true,
                connected: 2,
                push_enabled: false,
                push_channels: 0,
                disconnected: vec![],
                entry_errors: vec![],
                expiring: vec![PushChannelExpiry {
                    user_id: 7,
                    expires_at: None,
                    error: None,
                }],
            },
            fizzy: FizzyHealth {
                configured: false,
                note: Some("No Fizzy integration is configured in this workspace.".into()),
            },
            agent_delivery: DeliveryHealth {
                pending: 0,
                failed_24h: 1,
                recent_errors: vec![issue()],
            },
            email: EmailHealth {
                enabled: true,
                rooms_with_addresses: 1,
            },
        },
        json!({
            "github": {
                "workspaceToken": true,
                "appConfigured": false,
                "webhookSecret": true,
                "connected": 3,
                "appTokens": 1,
                "deliveries24h": 12,
                "disconnected": [issue_json],
                "lastErrors": [],
                "fetchErrors": []
            },
            "google": {
                "configured": true,
                "connected": 2,
                "pushEnabled": false,
                "pushChannels": 0,
                "disconnected": [],
                "entryErrors": [],
                "expiring": [{ "userId": 7, "expiresAt": null, "error": null }]
            },
            "fizzy": {
                "configured": false,
                "note": "No Fizzy integration is configured in this workspace."
            },
            "agentDelivery": { "pending": 0, "failed24h": 1, "recentErrors": [issue_json] },
            "email": { "enabled": true, "roomsWithAddresses": 1 }
        }),
    );
}
