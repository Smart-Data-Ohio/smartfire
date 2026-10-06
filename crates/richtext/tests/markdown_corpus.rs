use campfire_richtext::{AttachableResolver, GidLookup, MentionUser, RenderContext, SignedLookup};
use campfire_richtext::{
    legacy_markdown,
    markdown::{self, Icon, IconCatalog, RoomMember},
};
use serde_json::Value;
use std::collections::HashMap;

struct Oracle {
    users: Vec<MentionUser>,
    images: HashMap<String, String>,
    avatars: HashMap<i64, String>,
    preloaded: HashMap<String, i64>,
}
impl AttachableResolver for Oracle {
    fn preloaded_user_for_sgid(&self, sgid: &str) -> Option<MentionUser> {
        self.preloaded.get(sgid).and_then(|id| self.users.iter().find(|u| u.id == *id)).cloned()
    }
    fn mention_avatar_html(&self, user: &MentionUser) -> String {
        self.avatars[&user.id].clone()
    }
    fn locate_signed(&self, sgid: &str) -> SignedLookup {
        self.users.iter().find(|u| u.attachable_sgid == sgid).cloned().map_or(SignedLookup::Invalid, SignedLookup::User)
    }
    fn embed_image_path(&self, url: &str) -> Result<String, campfire_richtext::Error> {
        Ok(self.images.get(url).unwrap().clone())
    }
    fn find_gid(&self, gid: &str) -> GidLookup {
        let id = gid.split('?').next().unwrap().rsplit('/').next().and_then(|s| s.parse::<i64>().ok());
        self.users.iter().find(|u| Some(u.id) == id && gid.contains("/User/")).cloned().map_or(GidLookup::NotFound, GidLookup::User)
    }
}
fn fixtures() -> (Value, Oracle, IconCatalog, Vec<RoomMember>) {
    let json: Value = serde_json::from_str(include_str!("markdown/expected.json")).unwrap();
    let users = json["users"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| MentionUser {
            id: u["id"].as_i64().unwrap(),
            name: u["name"].as_str().unwrap().into(),
            title: u["title"].as_str().unwrap().into(),
            attachable_sgid: u["attachable_sgid"].as_str().unwrap().into(),
            user_path: u["user_path"].as_str().unwrap().into(),
            avatar_path: u["avatar_path"].as_str().unwrap().into(),
        })
        .collect::<Vec<_>>();
    let members = users
        .iter()
        .zip(json["users"].as_array().unwrap())
        .filter(|(_, u)| u["member"] == true)
        .map(|(user, u)| RoomMember { user: user.clone(), active: u["active"] == true })
        .collect();
    let images = serde_json::from_value(json["embed_images"].clone()).unwrap();
    let preloaded = serde_json::from_value(json["preloaded_sgids"].clone()).unwrap();
    let avatars = json["users"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| (u["id"].as_i64().unwrap(), u["avatar_html"].as_str().unwrap().to_owned()))
        .collect();
    let mut icons = IconCatalog::default();
    for (alias, record) in json["icons"].as_object().unwrap() {
        let name = record["name"].as_str().unwrap().into();
        let title = record["title"].as_str().unwrap().into();
        if record["kind"] == "brand" {
            icons.brands.insert(alias.clone(), Icon::Brand { name, title, url: record["url"].as_str().map(str::to_owned) });
        } else {
            icons.custom.insert(alias.clone(), Icon::Custom { name, title, url: record["url"].as_str().unwrap().into() });
        }
    }
    (json, Oracle { users, images, avatars, preloaded }, icons, members)
}
#[test]
fn ruby_differential_corpus_is_byte_identical() {
    let (json, oracle, icons, members) = fixtures();
    for (user, reference) in oracle.users.iter().zip(json["users"].as_array().unwrap()) {
        assert_eq!(
            campfire_richtext::attachables::render_mention_in_context(user, &RenderContext { resolver: &oracle, request_host: None }),
            reference["mention_partial"].as_str().unwrap()
        );
    }
    let ctx = RenderContext { resolver: &oracle, request_host: Some("once.campfire.test".into()) };
    let mentions = |name: &str| markdown::MentionResolver::unique_active_member(members.as_slice(), name);
    let mut failures = Vec::new();
    let cases = json["cases"].as_array().unwrap();
    assert!(cases.len() >= 4200);
    for c in cases {
        let host = c["asset_host"].as_str();
        let name = c["name"].as_str().unwrap();
        let mut compare = |field: &str, result: Result<String, campfire_richtext::Error>| {
            let expected = c[field].as_str().unwrap();
            match result {
                Ok(actual) if actual == expected => {}
                result => {
                    if failures.len() < 30 {
                        eprintln!(
                            "{name} {field}:\nexpected: {:?}\nactual: {:?}",
                            expected.chars().take(400).collect::<String>(),
                            result.map(|s| s.chars().take(400).collect::<String>())
                        );
                    }
                    failures.push(format!("{name} {field}"));
                }
            }
        };
        if c["mode"] == "sanitize" {
            compare("output", markdown::sanitize_presentation(c["body"].as_str().unwrap(), &icons, host));
        } else {
            let markdown = c["mode"] == "markdown";
            let body = if markdown {
                let rendered = markdown::render(c["source"].as_str().unwrap(), &mentions, &icons);
                compare("rendered", rendered.clone());
                rendered.unwrap_or_default()
            } else {
                c["body"].as_str().unwrap().into()
            };
            compare("presentation", campfire_richtext::text_message_presentation(&body, markdown, false, &ctx, &icons, host));
            if markdown {
                compare("forwarded_presentation", campfire_richtext::text_message_presentation(&body, false, true, &ctx, &icons, host));
            }
            let loaded = campfire_richtext::Content::load(&body, &ctx).unwrap();
            compare("editable", legacy_markdown::render(&loaded.to_html(), &ctx));
            compare(
                "plain_text",
                if markdown { markdown::plain_text(&body, &ctx, &icons) } else { campfire_richtext::to_plain_text(&body, &ctx) },
            );
            if let Some(ids) = c["mentioned"].as_array() {
                let expected = ids.iter().map(|id| id.as_i64().unwrap()).collect::<Vec<_>>();
                let actual = campfire_richtext::mentioned_users(&body, &ctx).unwrap().iter().map(|u| u.id).collect::<Vec<_>>();
                if actual != expected {
                    failures.push(format!("{name} mentioned: {actual:?} != {expected:?}"));
                }
            }
        }
    }
    println!("Ruby differential corpus: {} cases, {} diffs (no normalization or allowlist)", cases.len(), failures.len());
    assert!(failures.is_empty(), "{} diffs; first failures: {:?}", failures.len(), &failures[..failures.len().min(30)]);
}
