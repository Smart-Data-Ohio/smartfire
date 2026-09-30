use super::*;

fn vectors() -> Value {
    let helpers: Value = serde_json::from_str(&fixture("helpers.json")).unwrap();
    helpers["review"].clone()
}

#[test]
fn review_frame_layout() {
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let ctx = context(None, &asset, &signer, "");
    for (name, head) in [
        ("frame_authenticated", ""),
        ("frame_head", "<meta name=\"extra\" content=\"yes\">"),
    ] {
        let frame = layouts::FrameLayout {
            ctx: &ctx,
            head: h::raw(head),
            content: h::raw("Body"),
        };
        assert!(compare(
            name,
            &render(&frame),
            &fixture(&format!("layouts/{name}.html"))
        ));
    }
}

#[test]
fn review_involvement_cycle() {
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let ctx = context(None, &asset, &signer, "");
    let mut failed = Vec::new();
    for row in vectors()["involvements"].as_array().unwrap() {
        let direct = row["direct"].as_bool().unwrap();
        let level = row["level"].as_str().unwrap();
        let room = h::InvolvementRoom {
            id: 1,
            param_key: row["param_key"].as_str().unwrap(),
            direct,
        };
        let html = h::request_forgery::rendering_with(
            h::request_forgery::RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: None,
            },
            || h::button_to_change_involvement(&ctx, room, level).0,
        );
        if !compare(
            &format!("involvement-{direct}-{level}"),
            &html,
            row["html"].as_str().unwrap(),
        ) || h::next_involvement(direct, level) != row["next"].as_str().unwrap()
            || h::humanize_involvement(level) != row["description"].as_str().unwrap()
            || h::short_involvement_label(level) != row["label"].as_str().unwrap()
            || !h::involvement_levels(direct).contains(&level)
        {
            failed.push(format!("{direct}:{level}"));
        }
    }
    assert!(failed.is_empty(), "notification differences: {failed:?}");
}

#[test]
fn review_sidebar_event_binding() {
    let v = vectors();
    for (key, src, body) in [
        ("sidebar_empty", None, ""),
        (
            "sidebar_src",
            Some("/users/me/sidebar?x=1&y=2"),
            "<b>Body</b>",
        ),
    ] {
        assert!(compare(
            key,
            &h::sidebar_turbo_frame_tag(src, body).0,
            v[key].as_str().unwrap()
        ));
    }
}

#[test]
fn review_direct_button_label() {
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let ctx = context(None, &asset, &signer, "");
    for row in vectors()["direct_buttons"].as_array().unwrap() {
        let html = h::request_forgery::rendering_with(
            h::request_forgery::RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: None,
            },
            || h::button_to_direct_room_with(&ctx, 2, row["name"].as_str().unwrap()).0,
        );
        assert!(compare(
            "direct-button",
            &html,
            row["html"].as_str().unwrap()
        ));
    }
}

#[test]
fn review_cached_message_is_session_neutral() {
    use campfire_views::{fragment_cache, messages};
    let v = vectors();
    for html in v["message_html"].as_object().unwrap().values() {
        assert!(
            !html.as_str().unwrap().contains("authenticity_token"),
            "Rails reference must be token-free"
        );
    }
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let plain: messages::MessageView = serde_json::from_value(v["message"].clone()).unwrap();
    let mut boosted = plain.clone();
    boosted.boosts.push(messages::BoostView {
        reaction: None,
        id: 9,
        updated_at: plain.updated_at,
        message_id: plain.id,
        content: "Great".into(),
        all_emoji: false,
        booster: plain.creator.clone(),
    });
    for message in [plain, boosted] {
        let cache = fragment_cache::FragmentCache::new(1 << 20);
        let mut outputs = Vec::new();
        for (user, token) in [("David", "DAVID-SESSION"), ("JZ", "JZ-SESSION")] {
            let ctx = context(Some(user), &asset, &signer, "");
            let output = h::request_forgery::rendering_with(
                h::request_forgery::RequestSecrets {
                    tokens: Box::new(UserTokens(token)),
                    csp_nonce: Some(token.into()),
                },
                || {
                    fragment_cache::with(&cache, || {
                        let html = messages::message(&ctx, &message);
                        let stored = messages::cached_message_fragment(
                            message.id,
                            message.updated_at,
                            &ctx.base_url,
                        )
                        .unwrap();
                        assert!(
                            !stored.contains("authenticity_token")
                                && !h::request_forgery::has_token_slots(&stored),
                            "cached message stores a form token or slot"
                        );
                        assert!(
                            !stored.contains("SESSION"),
                            "cached message stores a session value"
                        );
                        let item = messages::MessageItem::Fragment {
                            client_message_id: message.client_message_id.clone(),
                            room_id: message.room_id,
                            html: stored,
                        };
                        assert_eq!(messages::cached_message_item(&ctx, &item).0, html);
                        html
                    })
                },
            );
            eprintln!(
                "{user}: {} cached message token fields",
                output.matches("name=\"authenticity_token\"").count()
            );
            assert!(!output.contains("authenticity_token") && !output.contains("SESSION"));
            outputs.push(output);
        }
        assert_eq!(
            outputs[0], outputs[1],
            "same cached message must be session independent"
        );
        let ctx = context(None, &asset, &signer, "");
        assert_eq!(
            fragment_cache::with(&cache, || messages::message(&ctx, &message)),
            outputs[0],
            "broadcast"
        );
        assert!(
            !messages::message(&ctx, &message).contains("authenticity_token"),
            "uncached broadcast"
        );
    }
}

#[test]
fn review_whole_message_matches_rails_for_both_viewers() {
    use campfire_views::{fragment_cache, messages};
    let v = vectors();
    let message: messages::MessageView = serde_json::from_value(v["message"].clone()).unwrap();
    assert_eq!(
        message.boosts.len(),
        1,
        "fixture 0001 includes its real boost"
    );
    assert_eq!(message.boosts[0].content, "Hello");
    let cache = fragment_cache::FragmentCache::new(1 << 20);
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let mut differences = Vec::new();
    for (name, email, token) in [
        ("David", "david@37signals.com", "DAVID-SESSION"),
        ("JZ", "jz@37signals.com", "JZ-SESSION"),
    ] {
        let ctx = context(Some(name), &asset, &signer, "");
        let html = h::request_forgery::rendering_with(
            h::request_forgery::RequestSecrets {
                tokens: Box::new(UserTokens(token)),
                csp_nonce: Some(token.into()),
            },
            || fragment_cache::with(&cache, || messages::message(&ctx, &message)),
        );
        assert!(!html.contains("authenticity_token") && !html.contains("SESSION"));
        if !compare(
            &format!("whole-message-{name}"),
            &html,
            v["message_html"][email].as_str().unwrap(),
        ) {
            differences.push(name);
        }
    }
    assert!(
        differences.is_empty(),
        "whole-message differences: {differences:?}"
    );
}

fn image_vectors() -> Value {
    serde_json::from_str(&fixture("images.json")).unwrap()
}

fn check_image_message(name: &str) {
    use campfire_views::{fragment_cache, messages};
    let vectors = image_vectors();
    let row = vectors["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let message: messages::MessageView = serde_json::from_value(row["message"].clone()).unwrap();
    assert!(matches!(
        message.content,
        messages::MessageContent::Attachment(_)
    ));
    let cache = fragment_cache::FragmentCache::new(1 << 20);
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let mut differences = Vec::new();
    for (viewer, email, token) in [
        ("David", "david@37signals.com", "DAVID-SESSION"),
        ("JZ", "jz@37signals.com", "JZ-SESSION"),
    ] {
        let ctx = context(Some(viewer), &asset, &signer, "");
        let html = h::request_forgery::rendering_with(
            h::request_forgery::RequestSecrets {
                tokens: Box::new(UserTokens(token)),
                csp_nonce: Some(token.into()),
            },
            || fragment_cache::with(&cache, || messages::message(&ctx, &message)),
        );
        assert!(!html.contains("authenticity_token") && !html.contains("SESSION"));
        fragment_cache::with(&cache, || {
            let stored =
                messages::cached_message_fragment(message.id, message.updated_at, &ctx.base_url)
                    .unwrap();
            assert_eq!(stored.as_str(), html.as_str());
            assert!(!h::request_forgery::has_token_slots(&stored));
        });
        if !compare(
            &format!("{name}-{viewer}"),
            &html,
            row["html_by_viewer"][email].as_str().unwrap(),
        ) {
            differences.push(viewer);
        }
    }
    assert!(
        differences.is_empty(),
        "{name} message differences: {differences:?}"
    );
}

#[test]
fn image_message_matches_rails_for_both_viewers() {
    check_image_message("image");
}

#[test]
fn image_large_message_matches_rails_for_both_viewers() {
    check_image_message("image_large");
}

#[test]
fn image_filename_previews_match_rails() {
    use campfire_views::messages::{MessageContent, presentation};
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let ctx = context(None, &asset, &signer, "");
    let mut differences = Vec::new();
    for row in image_vectors()["filename_previews"].as_array().unwrap() {
        let MessageContent::Attachment(mut attachment) =
            serde_json::from_value(row["content"].clone()).unwrap()
        else {
            panic!("filename probe must use the attachment renderer");
        };
        let name = row["raw_filename"].as_str().unwrap();
        // Use the same existing Filename implementation as the production presenter, starting
        // with the raw value: sanitizing before removing the extension loses Rails semantics.
        let filename = campfire_storage::Filename::new(name);
        assert_eq!(
            filename.base(),
            attachment.filename_base,
            "filename base: {name}"
        );
        assert_eq!(
            filename.to_string(),
            attachment.filename,
            "sanitized filename: {name}"
        );
        attachment.filename_base = filename.base().to_string();
        attachment.filename = filename.to_string();
        let actual = presentation::attachment_presentation(&ctx, &attachment);
        if !compare(
            &format!("filename-preview-{}", differences.len()),
            &actual,
            row["html"].as_str().unwrap(),
        ) {
            differences.push(name.to_string());
        }
    }
    assert!(
        differences.is_empty(),
        "image filename differences: {differences:?}"
    );
}

#[test]
fn review_helper_attribute_sweep() {
    let v = vectors();
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let mut ctx = context(None, &asset, &signer, "");
    let actual = h::request_forgery::rendering_with(
        h::request_forgery::RequestSecrets {
            tokens: Box::new(Tokens),
            csp_nonce: None,
        },
        || {
            let mut out = std::collections::BTreeMap::new();
            out.insert(
                "link_room",
                h::link_to_room(
                    1,
                    h::attrs()
                        .class("btn")
                        .data("room_id", 9)
                        .data("action", "click->test#open"),
                    "<b>Room</b>",
                )
                .0,
            );
            out.insert("link_back", h::link_back_to(&ctx, "/rooms/1").0);
            out.insert(
                "clipboard",
                h::button_to_copy_to_clipboard("a <&> b", "Copy").0,
            );
            out.insert(
                "qr",
                h::link_to_zoom_qr_code("https://campfire.test/?a=1&b=2", "QR").0,
            );
            out.insert(
                "web_share",
                h::web_share_session_button("/session", "Title <&>", "Text", "Share").0,
            );
            out.insert("filter_menu", h::user_filter_menu_tag("<b>User</b>").0);
            out.insert("filter_search", h::user_filter_search_tag().0);
            out.insert("profile_submit", h::profile_form_submit_button(&ctx).0);
            out.insert(
                "turbo_frame",
                h::turbo_frame_tag(
                    "custom",
                    Some("/frame?a=1&b=2"),
                    Some("_top"),
                    h::attrs().data("action", "turbo:frame-load->test#loaded"),
                    "Body",
                )
                .0,
            );
            out.insert("turbo_reload", h::turbo_page_requires_reload_tag().0);
            out.insert("cable_meta", h::script_aware_action_cable_meta_tag(&ctx).0);
            out.insert("current_user_nil", h::current_user_meta_tags(&ctx).0);
            ctx.current_user = Some(CurrentUser {
                id: 2,
                name: "Jane <&>".into(),
                administrator: false,
                bot: false,
                avatar_url: String::new(),
                preferences: Default::default(),
            });
            out.insert("current_user", h::current_user_meta_tags(&ctx).0);
            out.insert("title_default", h::page_title_tag(None).0);
            out.insert("title_escaped", h::page_title_tag(Some("Title <&>")).0);
            out.insert("version_badge", h::version_badge(&ctx).0);
            let form = h::form_with("/x").model("room");
            out.insert(
                "textarea",
                form.wrap(&form.text_area("description", Some("\nhi"), h::attrs()).0)
                    .0,
            );
            let form = h::form_with("/x").model("room").method("get");
            out.insert(
                "search_label",
                form.wrap(
                    &(form.label("q", "Find <&>", h::attrs().class("label")).0
                        + &form
                            .search_field(
                                "q",
                                h::attrs()
                                    .value("a <&>")
                                    .data("action", "input->search#query"),
                            )
                            .0),
                )
                .0,
            );
            out.insert(
                "button_form",
                h::button_to_form(
                    "/x",
                    h::attrs().method("delete").aria("label", "Delete"),
                    h::attrs().data("action", "submit->test#send"),
                    "Delete",
                )
                .0,
            );
            out.insert(
                "tokenless_form",
                h::form_with("/x").authenticity_token(false).wrap("Body").0,
            );
            out.insert(
                "radio",
                h::radio_button_tag(
                    "mode",
                    "on",
                    true,
                    h::attrs()
                        .id("radio-id")
                        .data("action", "change->test#choose")
                        .aria("label", "On"),
                )
                .0,
            );
            out.insert(
                "datetime",
                h::datetime_local_field_tag(
                    "dt",
                    Some("2026-01-01T09:00"),
                    h::attrs()
                        .data("action", "change->test#choose")
                        .aria("label", "Date"),
                )
                .0,
            );
            out
        },
    );
    let mut failed = Vec::new();
    for (key, html) in actual {
        if !compare(key, &html, v[key].as_str().unwrap()) {
            failed.push(key.to_string());
        }
    }
    for (i, keyboard) in [false, true].into_iter().enumerate() {
        let html = h::content_tag_text(
            "span",
            h::attrs().merge(h::profile_card_trigger(2, keyboard)),
            "Jane",
        )
        .0;
        if !compare(
            "profile-trigger",
            &html,
            v["profile_trigger"][i].as_str().unwrap(),
        ) {
            failed.push("profile_trigger".into());
        }
    }
    for (i, style) in [None, Some("small")].into_iter().enumerate() {
        if !compare(
            "account-logo",
            &h::account_logo_tag(&ctx, style).0,
            v["account_logo"][i].as_str().unwrap(),
        ) {
            failed.push("account_logo".into());
        }
    }
    for (i, referrer) in [
        None,
        Some("http://campfire.test/"),
        Some("http://campfire.test/rooms/1"),
    ]
    .into_iter()
    .enumerate()
    {
        ctx.referrer = referrer.map(str::to_string);
        if !compare(
            "link-back-referrer",
            &h::link_back(&ctx).0,
            v["link_back_referrers"][i].as_str().unwrap(),
        ) {
            failed.push("link_back".into());
        }
    }
    for (i, room) in [None, Some(1)].into_iter().enumerate() {
        ctx.last_room_visited_id = room;
        if !compare(
            "link-back-visited",
            &h::link_back_to_last_room_visited(&ctx).0,
            v["link_back_visited"][i].as_str().unwrap(),
        ) {
            failed.push("link_back_visited".into());
        }
    }
    for (i, styles) in [None, Some(""), Some(".card { color: red; }")]
        .into_iter()
        .enumerate()
    {
        ctx.custom_styles = styles.map(str::to_string);
        if !compare(
            "custom-styles",
            &h::custom_styles_tag(&ctx).0,
            v["custom_styles"][i].as_str().unwrap(),
        ) {
            failed.push("custom_styles".into());
        }
    }
    for row in v["body_classes"].as_array().unwrap() {
        ctx.current_user.as_mut().unwrap().administrator = row["admin"].as_bool().unwrap();
        ctx.account.has_logo = row["logo"].as_bool().unwrap();
        if h::body_classes(&ctx, row["body"].as_str()) != row["result"].as_str().unwrap() {
            failed.push("body_classes".into());
        }
    }
    for row in v["icons"].as_array().unwrap() {
        let icon = match row["name"].as_str().unwrap() {
            "github" => Some(h::AvatarIcon::Image {
                title: "GitHub".into(),
                url: "icons/brands/github.svg".into(),
                brand: true,
            }),
            "tada" => Some(h::AvatarIcon::Emoji {
                title: "Tada".into(),
                character: "🎉".into(),
            }),
            _ => None,
        };
        let html = h::icon_avatar_tag(
            &ctx,
            icon.as_ref(),
            24,
            h::attrs()
                .class("extra")
                .attr("style", "color: red")
                .aria("label", "Override"),
        )
        .0;
        if !compare("icon-options", &html, row["html"].as_str().unwrap()) {
            failed.push(format!("icon:{}", row["name"]));
        }
    }
    for row in v["avatars"].as_array().unwrap() {
        let avatar = h::AvatarUser {
            id: row["id"].as_i64().unwrap(),
            title: row["title"].as_str().unwrap().into(),
            avatar_path: row["avatar_path"].as_str().unwrap().into(),
        };
        let icon = if row["uploaded"].as_bool().unwrap() {
            None
        } else {
            match row["icon"].as_str().unwrap() {
                "github" => Some(h::AvatarIcon::Image {
                    title: "GitHub".into(),
                    url: "icons/brands/github.svg".into(),
                    brand: true,
                }),
                "tada" => Some(h::AvatarIcon::Emoji {
                    title: "Tada".into(),
                    character: "🎉".into(),
                }),
                _ => None,
            }
        };
        let html = h::avatar_tag_with_icon(
            &ctx,
            &avatar,
            icon.as_ref(),
            h::attrs().size(32).class("extra").attr("loading", "lazy"),
        )
        .0;
        if !compare("avatar-options", &html, row["html"].as_str().unwrap()) {
            failed.push(format!("avatar:{}", row["icon"]));
        }
    }
    for row in v["settings"].as_array().unwrap() {
        let prefs = &mut ctx.current_user.as_mut().unwrap().preferences;
        prefs.theme = row["theme"].as_str().map(str::to_string);
        prefs.text_size = row["text_size"].as_str().map(str::to_string);
        let html = h::builder_tag(
            "meta",
            h::attrs()
                .name("color-scheme")
                .attr("content", h::theme_color_scheme_meta_content(&ctx)),
        )
        .0;
        if !compare("theme-meta", &html, row["html"].as_str().unwrap())
            || h::user_theme(&ctx) != row["user_theme"].as_str().unwrap()
            || h::user_text_size(&ctx) != row["user_text_size"].as_str().unwrap()
        {
            failed.push(format!("settings:{}:{}", row["theme"], row["text_size"]));
        }
    }
    for row in v["zones_meta"].as_array().unwrap() {
        let prefs = &mut ctx.current_user.as_mut().unwrap().preferences;
        prefs.time_zone = row["zone"].as_str().map(str::to_string);
        prefs.time_zone_explicit = row["explicit"].as_bool().unwrap();
        let html = h::builder_tag(
            "meta",
            h::attrs()
                .name("time-zone")
                .attr_opt("content", h::current_user_time_zone_meta_content(&ctx)),
        )
        .0;
        if !compare("time-zone-meta", &html, row["html"].as_str().unwrap()) {
            failed.push("zones_meta".into());
        }
    }
    assert!(failed.is_empty(), "helper sweep mismatches: {failed:?}");
}

#[test]
fn message_partial_branches_match_rails() {
    use campfire_views::messages;
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let ctx = context(Some("David"), &asset, &signer, "");
    let mut differences = Vec::new();
    for row in vectors()["message_states"].as_array().unwrap() {
        let message: messages::MessageView =
            serde_json::from_value(row["message"].clone()).unwrap();
        let name = row["name"].as_str().unwrap();
        let html = messages::message(&ctx, &message);
        assert!(
            !html.contains("authenticity_token"),
            "{name} must be session-neutral"
        );
        if !compare(
            &format!("message-state-{name}"),
            &html,
            row["html"].as_str().unwrap(),
        ) {
            differences.push(name.to_string());
        }
    }
    assert!(
        differences.is_empty(),
        "message branch differences: {differences:?}"
    );
}

#[test]
fn reaction_registry_and_graphemes_match_rails() {
    use campfire_views::messages::{reaction_body, reactions};
    struct StaticIcons;
    impl h::IconSource for StaticIcons {
        fn resolve_avatar_icon(&self, name: &str) -> Option<h::AvatarIcon> {
            reactions::static_icon(name)
        }
    }
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let ctx = context(None, &asset, &signer, "");
    for row in vectors()["reaction_probes"].as_array().unwrap() {
        let content = row["content"].as_str().unwrap();
        let resolved = reactions::resolve(content, &StaticIcons);
        assert_eq!(
            resolved.is_some(),
            row["reaction"].as_bool().unwrap(),
            "Boost.reaction? {content}"
        );
        if let Some(resolved) = &resolved {
            assert_eq!(
                resolved.title,
                row["title"].as_str().unwrap(),
                "reaction title: {content}"
            );
        }
        let icon = resolved.as_ref().and_then(|value| value.icon.as_ref());
        let alt = resolved
            .as_ref()
            .and_then(|value| value.icon_alt.as_deref());
        assert!(compare(
            &format!("reaction body: {content}"),
            &reaction_body(&ctx, content, icon, alt, false).0,
            row["body"].as_str().unwrap()
        ));
        assert!(compare(
            &format!("legacy body: {content}"),
            &reaction_body(&ctx, content, icon, alt, true).0,
            row["legacy_body"].as_str().unwrap()
        ));
    }
}

#[test]
fn edge_install_matches_pinned_rails() {
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_: &[&str]| String::new();
    let mut ctx = context(Some("David"), &asset, &signer, "");
    ctx.platform.chrome = false;
    ctx.platform.edge = true;
    ctx.platform.mac = false;
    ctx.platform.browser = "Edge".into();
    ctx.platform.operating_system = "Windows".into();
    let html = campfire_views::pwa::InstallInstructions { ctx: &ctx }
        .render()
        .unwrap();
    assert!(compare(
        "edge-install",
        &html,
        vectors()["edge_install"].as_str().unwrap()
    ));
}
