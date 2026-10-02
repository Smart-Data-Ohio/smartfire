use super::*;
use campfire_views::accounts::bot_access::*;
fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::to_owned)
}
fn time(v: &Value) -> Option<jiff::Timestamp> {
    v.as_str().map(|s| s.parse().unwrap())
}
fn errors(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().into())
        .collect()
}
#[test]
fn bot_access_pages_match_pinned_rails_bytes() {
    let data: Value =
        serde_json::from_str(include_str!("../golden/bot_access_ui/pages.json")).unwrap();
    for fact in data["direct_names"].as_array().unwrap() {
        let names = fact["names"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            campfire_views::rooms::room_display_name(
                fact["room_name"].as_str(),
                true,
                &names,
                fact["viewer_name"].as_str()
            ),
            fact["expected"].as_str().unwrap(),
            "{fact}"
        );
    }
    let env = include_str!("../../../../parity/.env.reference");
    let secrets = rails_compat::Secrets::new(
        env.lines()
            .find_map(|s| s.strip_prefix("SECRET_KEY_BASE="))
            .unwrap(),
    );
    let signer = |parts: &[&str]| rails_compat::turbo::signed_stream_name(&secrets, parts);
    let asset = |path: &str| campfire_assets::asset_path(path);
    let styles = campfire_assets::stylesheet_link_tag_all(&[("data-turbo-track", "reload")]).html;
    let mut mismatches = Vec::new();
    for (name, expected) in data["pages"].as_object().unwrap() {
        let ctx = context(
            Some(if name.ends_with("owner") {
                "Kevin"
            } else {
                "David"
            }),
            &asset,
            &signer,
            &styles,
        );
        let f = &data["facts"][name];
        let bot_id = data["bot_id"].as_i64().unwrap();
        let bot_name = data["bot_name"].as_str().unwrap().to_owned();
        let actual = if name == "credential_show" {
            render(&CredentialCreated {
                ctx: &ctx,
                bot_id,
                secret: "fixture-reveal-once-secret".into(),
            })
        } else if name.starts_with("credentials_") {
            let credentials = f["credentials"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| Credential {
                    id: c["id"].as_i64().unwrap(),
                    name: text(&c["name"]).unwrap(),
                    last_four: text(&c["last_four"]).unwrap(),
                    created_by: text(&c["created_by"]).unwrap(),
                    created_at: time(&c["created_at"]).unwrap(),
                    expires_at: time(&c["expires_at"]),
                    last_used_at: time(&c["last_used_at"]),
                    revoked: c["revoked"].as_bool().unwrap(),
                })
                .collect();
            render(&Credentials {
                ctx: &ctx,
                bot_id,
                bot_name,
                credentials,
                credential: CredentialForm {
                    name: text(&f["name"]),
                    expires_at: text(&f["expires_at"]),
                    errors: text(&f["errors"]),
                    error_fields: errors(&f["error_fields"]),
                },
                now: time(&data["now"]).unwrap(),
            })
        } else {
            let grants = f["grants"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| Grant {
                    id: g["id"].as_i64().unwrap(),
                    capability: text(&g["capability"]).unwrap(),
                    room_name: text(&g["room_name"]).unwrap(),
                    granted_by: text(&g["granted_by"]).unwrap(),
                    created_at: time(&g["created_at"]).unwrap(),
                    revoked: g["revoked"].as_bool().unwrap(),
                })
                .collect();
            let rooms = f["rooms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| (text(&r[0]).unwrap(), text(&r[1]).unwrap()))
                .collect();
            render(&Grants {
                ctx: &ctx,
                bot_id,
                bot_name,
                legacy: f["legacy"].as_bool().unwrap(),
                grants,
                grant: GrantForm {
                    capability: text(&f["capability"]),
                    room_id: text(&f["room_id"]),
                    errors: text(&f["errors"]),
                    error_fields: errors(&f["error_fields"]),
                },
                rooms,
            })
        };
        if !compare(name, &actual, expected.as_str().unwrap()) {
            mismatches.push(name);
        }
    }
    assert!(
        mismatches.is_empty(),
        "Rails bot access UI differences: {mismatches:?}"
    );
}
