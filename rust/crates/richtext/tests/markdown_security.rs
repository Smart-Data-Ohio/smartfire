//! Security assertions independent of the Ruby oracle and sanitizer implementation.
use campfire_richtext::dom::Dom;
use campfire_richtext::markdown::{self, Icon, IconCatalog, IconResolver, MentionResolver, RoomMember};
use campfire_richtext::{Error, MentionUser};
use std::collections::HashMap;

fn icons() -> IconCatalog {
    IconCatalog {
        brands: HashMap::from([(
            "brand".into(),
            Icon::Brand { name: "brand".into(), title: "Safe & brand".into(), url: Some("/assets/brand-abc.svg".into()) },
        )]),
        custom: HashMap::from([("acme".into(), Icon::Custom { name: "acme".into(), title: "Custom".into(), url: "/icons/acme".into() })]),
    }
}
fn no_mentions(_: &str) -> Option<MentionUser> {
    None
}

fn violations(html: &str) -> Vec<String> {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).unwrap();
    let mut bad = Vec::new();
    for node in dom.descendants(root) {
        let Some(tag) = dom.local_name(node) else {
            continue;
        };
        if !dom.is_html_element(node)
            || ["script", "style", "iframe", "object", "embed", "svg", "math", "form", "button", "base", "link", "meta"].contains(&tag)
        {
            bad.push(format!("tag {tag}"));
        }
        for (key, value) in dom.attrs(node) {
            if key.starts_with("on") || ["style", "srcdoc", "id", "name"].contains(&key.as_str()) {
                bad.push(format!("attribute {key}"));
            }
            if ["href", "src", "url"].contains(&key.as_str()) {
                // HTML parsing has decoded entities; browsers ignore ASCII controls in URLs.
                let normalized =
                    value.chars().filter(|c| !c.is_ascii_whitespace() && !c.is_ascii_control()).collect::<String>().to_ascii_lowercase();
                if normalized.starts_with("javascript:") || normalized.starts_with("vbscript:") || normalized.starts_with("data:text/html")
                {
                    bad.push(format!("URL {value}"));
                }
            }
            if key == "data-turbo-frame" && value != "_top" {
                bad.push(format!("frame {value}"));
            }
            if key == "data-turbo-prefetch" && value != "false" {
                bad.push(format!("prefetch {value}"));
            }
            if tag == "img" && key == "src" && !["/assets/brand-abc.svg", "/icons/acme", "/users/1/avatar?v=1"].contains(&value.as_str()) {
                bad.push(format!("unexpected image source {value}"));
            }
        }
    }
    bad
}

#[test]
fn xss_properties_hold_for_adversarial_markdown_and_stored_html() {
    let urls = [
        "javascript:alert(1)",
        "JAVASCRIPT:alert(1)",
        "jav&#x09;ascript:alert(1)",
        "&#106;avascript:alert(1)",
        "java\nscript:alert(1)",
        "java\rscript:alert(1)",
        "vbscript:msgbox(1)",
        "data:text/html,<script>alert(1)</script>",
    ];
    let tags = ["a", "p", "span", "div", "svg", "math", "script", "style", "img", "iframe", "table", "input"];
    let icons = icons();
    let mut count = 0;
    for url in urls {
        for tag in tags {
            let attacks = [
                format!(
                    "<{tag} href=\"{url}\" src=\"{url}\" onclick=\"alert(1)\" onerror=\"alert(1)\" style=\"position:fixed\"><svg><a xlink:href=\"{url}\">bad</a></svg></{tag}>"
                ),
                format!("[bad]({url})\n\n![bad]({url})\n\n```html\n<img src=x onerror=alert(1)>\n```"),
                format!(
                    "<a title='x><img src=x onerror=alert(1)>' href=\"{url}\">link</a><img src=\"https://evil.test/track\"><img src=\"{url}\" alt=\":removed:\">"
                ),
                format!(
                    "<math><mtext><table><mglyph><style><!--</style><img title=\"--><img src=x onerror=alert(1)>\"></table></mtext></math><{tag} href=\"{url}\">x"
                ),
            ];
            for attack in attacks {
                let rendered = markdown::render(&attack, &no_mentions, &icons).unwrap();
                assert!(violations(&rendered).is_empty(), "render: {attack:?}: {:?}", violations(&rendered));
                let safe = markdown::sanitize_presentation(&attack, &icons, None).unwrap();
                assert!(violations(&safe).is_empty(), "presentation: {attack:?}: {:?}", violations(&safe));
                let mut dom = Dom::new();
                let root = dom.parse_fragment(&rendered).unwrap();
                for n in dom.descendants(root) {
                    if dom.local_name(n) == Some("input") {
                        assert_eq!(dom.attr(n, "type"), Some("checkbox"));
                        assert_eq!(dom.attr(n, "disabled"), Some("disabled"));
                    }
                }
                count += 2;
            }
        }
    }
    println!("Independent XSS properties: {count} rendered outputs, 0 violations");
}

#[test]
fn property_detector_rejects_planted_defects() {
    for html in [
        "<script>x</script>",
        "<svg><a>x</a></svg>",
        "<img src=x onerror=alert(1)>",
        "<a href='jav&#9;ascript:x'>x</a>",
        "<img src='https://evil.test/tracker'>",
        "<p style='position:fixed'>x</p>",
        "<a data-turbo-prefetch=true>x</a>",
    ] {
        assert!(!violations(html).is_empty(), "missed {html}");
    }
}

#[test]
fn shortcode_sources_are_replaced_and_unknown_icons_become_text() {
    let icons = icons();
    let safe = markdown::sanitize_presentation("<img src='https://evil.test/tracker' alt=':brand:'><img src='javascript:x' alt=':acme:'><img alt=':removed:' src='/assets/stale.svg'><img src='/users/1/avatar?v=1'><img src='/users/1/avatar/no'>", &icons, None).unwrap();
    assert!(violations(&safe).is_empty(), "{safe}");
    assert!(safe.contains("/assets/brand-abc.svg") && safe.contains("/icons/acme"));
    assert!(safe.contains(":removed:"));
    assert!(!safe.contains("evil.test") && !safe.contains("/avatar/no"));
}

#[test]
fn avatars_and_links_cannot_use_a_foreign_origin_or_browser_control_characters() {
    for src in [
        "//evil.test/users/1/avatar",
        "/\\evil.test/users/1/avatar",
        "https://evil.test/users/1/avatar",
        "data:/users/1/avatar",
        "/users/1/avatar/other",
        "/users/1/avatar\n",
        "https://assets.example.test.evil.test/users/1/avatar",
    ] {
        assert!(!markdown::avatar_src(src, Some("assets.example.test")), "{src:?}");
    }
    assert!(markdown::avatar_src("https://assets12.example.test/users/1/avatar", Some("//assets%d.example.test/")));
    for href in
        ["//evil.test", "/\\evil.test", "/\t/evil.test", "/\r/evil.test", "/\n/evil.test", "/rails/active_storage/blobs/x", " /rooms/1"]
    {
        assert!(!markdown::in_app_href(href), "{href:?}");
    }
}

#[test]
fn source_limit_counts_characters_and_never_truncates() {
    let icons = icons();
    let limits: serde_json::Value = serde_json::from_str(include_str!("markdown/limits.json")).unwrap();
    for case in limits.as_array().unwrap() {
        let source = case["character"].as_str().unwrap().repeat(case["count"].as_u64().unwrap() as usize);
        assert_eq!(
            matches!(markdown::render(&source, &no_mentions, &icons), Err(Error::SourceTooLong)),
            case["too_long"].as_bool().unwrap()
        );
    }
    for unit in ["x", "😀"] {
        let source = unit.repeat(50_000);
        assert!(markdown::render(&source, &no_mentions, &icons).unwrap().contains(&source));
        assert_eq!(markdown::render(&format!("{source}{unit}"), &no_mentions, &icons), Err(Error::SourceTooLong));
    }
}

#[test]
fn classes_checkboxes_code_and_shortcodes_obey_the_write_allowlist() {
    let icons = icons();
    let html = markdown::render("- [x] done\n- [ ] next\n\n```c++\n@[David] :brand: <img onerror=alert(1)>\n```\n\n```ruby{bad}\nx\n```\n\n:brand: :smile: [link :brand:](/rooms/1)", &no_mentions, &icons).unwrap();
    let mut dom = Dom::new();
    let root = dom.parse_fragment(&html).unwrap();
    for n in dom.descendants(root) {
        if dom.local_name(n) == Some("input") {
            assert_eq!(dom.attr(n, "disabled"), Some("disabled"));
        }
        if let Some(class) = dom.attr(n, "class") {
            assert!(["language-c++", "icon icon--brand"].contains(&class), "{class}");
        }
    }
    assert!(html.contains("<code class=\"language-c++\">@[David] :brand: &lt;img onerror=alert(1)&gt;"));
    assert!(html.contains(">link :brand:</a>"));
    assert!(html.contains("😄"));
}

#[test]
fn ambiguous_inactive_and_nonmembers_do_not_resolve() {
    let user = |id, name: &str| MentionUser {
        id,
        name: name.into(),
        title: name.into(),
        attachable_sgid: format!("sgid-{id}"),
        user_path: format!("/users/{id}"),
        avatar_path: "/users/1/avatar?v=1".into(),
    };
    let members = [
        RoomMember { user: user(1, "Unique"), active: true },
        RoomMember { user: user(2, "Duplicate"), active: true },
        RoomMember { user: user(3, "Duplicate"), active: true },
        RoomMember { user: user(4, "Inactive"), active: false },
    ];
    let resolve = |name: &str| members.as_slice().unique_active_member(name);
    let source =
        "@[Unique] @[Duplicate] @[Inactive] @[Nonmember] `@[Unique]` [@[Unique]](https://example.com/@[Unique] \"@[Unique]\") \\@[Unique]";
    let html = markdown::render(source, &resolve, &icons()).unwrap();
    assert_eq!(html.matches("<action-text-attachment ").count(), 1, "{html}");
    assert!(html.contains("@[Duplicate] @[Inactive] @[Nonmember]"));
    assert!(html.contains("href=\"https://example.com/@[Unique]\" title=\"@[Unique]\""));
    assert!(!html.contains("SMARTFIREMENTION"));
}

#[test]
fn catalog_precedence_and_missing_brand_assets_match_icons() {
    let mut icons = icons();
    icons.brands.insert("smile".into(), Icon::Brand { name: "brand".into(), title: "Brand wins".into(), url: None });
    icons.custom.insert("smile".into(), Icon::Custom { name: "smile".into(), title: "Custom".into(), url: "/icons/smile".into() });
    assert!(matches!(icons.find(" SMILE "), Some(Icon::Brand { .. })));
    assert_eq!(markdown::render(":smile:", &no_mentions, &icons).unwrap(), "<p>:smile:</p>\n");
    assert_eq!(markdown::mention_token(""), None);
    assert_eq!(markdown::mention_token("x\ny"), None);
}
