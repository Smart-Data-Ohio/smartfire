//! Wire shapes of the S7 people types, including explicit nulls.

use serde_json::{Value, json};

use crate::tests::{assert_wire, user};
use crate::*;

fn user_json() -> Value {
    json!({
        "id": 7, "name": "Ada Lovelace", "role": "administrator", "status": "active",
        "bio": null, "avatarUrl": "/users/7/avatar?v=1700000000", "hasAvatar": true,
        "customStatus": {"emoji": "🌴", "text": "On a beach", "expiresAt": null},
        "avatarIcon": null, "agent": null, "createdAt": "2026-09-26T12:26:46.848Z",
        "updatedAt": "2026-10-06T09:15:00.123456Z"
    })
}

#[test]
fn directory_and_rows_round_trip() {
    let person = DirectoryPerson {
        user_id: 7,
        online: true,
        starred: true,
        agent: false,
    };
    let wire = json!({"userId": 7, "online": true, "starred": true, "agent": false});
    assert_wire(&person, wire.clone());
    assert_wire(
        &PeopleDirectory {
            people: vec![person],
            users: vec![user()],
        },
        json!({"people": [wire], "users": [user_json()]}),
    );
    assert_wire(
        &PeopleDirectory {
            people: vec![],
            users: vec![],
        },
        json!({"people": [], "users": []}),
    );
}

#[test]
fn profile_and_status_round_trip() {
    for presence in [
        Presence::Online,
        Presence::Idle,
        Presence::Offline,
        Presence::Dnd,
    ] {
        let wire = match presence {
            Presence::Online => "online",
            Presence::Idle => "idle",
            Presence::Offline => "offline",
            Presence::Dnd => "dnd",
        };
        assert_wire(
            &PersonStatus {
                presence,
                status_text: None,
            },
            json!({"presence": wire, "statusText": null}),
        );
    }
    let status = PersonStatus {
        presence: Presence::Dnd,
        status_text: Some("Writing".into()),
    };
    let wire = json!({"presence": "dnd", "statusText": "Writing"});
    assert_wire(&status, wire.clone());
    for allowed in [false, true] {
        assert_wire(
            &PersonProfile {
                user: user(),
                status: Some(status.clone()),
                dnd_allowed: Some(allowed),
                email_address: Some("ada@example.com".into()),
                transfer_url: Some("https://chat.example/session/transfers/token".into()),
                transfer_qr_svg: Some("<svg>transfer</svg>".into()),
                can_ban: true,
                can_manage_bot: false,
            },
            json!({
                "user": user_json(), "status": wire, "dndAllowed": allowed,
                "emailAddress": "ada@example.com",
                "transferUrl": "https://chat.example/session/transfers/token",
                "transferQrSvg": "<svg>transfer</svg>", "canBan": true, "canManageBot": false
            }),
        );
    }
    assert_wire(
        &PersonProfile {
            user: user(),
            status: None,
            dnd_allowed: None,
            email_address: None,
            transfer_url: None,
            transfer_qr_svg: None,
            can_ban: false,
            can_manage_bot: false,
        },
        json!({"user": user_json(), "status": null, "dndAllowed": null,
        "emailAddress": null, "transferUrl": null, "transferQrSvg": null, "canBan": false,
        "canManageBot": false}),
    );
}
