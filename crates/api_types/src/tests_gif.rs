use crate::tests::assert_wire;
use crate::*;
use serde_json::json;

#[test]
fn gif_discovery_wire_contract() {
    assert_wire(&GifConfig { enabled: false }, json!({"enabled":false}));
    assert_wire(&GifSuggestions { suggestions: vec!["hello there".into()] }, json!({"suggestions":["hello there"]}));
    assert_wire(&GifList { items: vec![GifItem {
        slug: "hello".into(), klipy_id: "9007199254740993".into(), title: "Hello".into(),
        attribution: "Powered by KLIPY".into(), preview_url: "/api/v1/gifs/media?u=x".into(),
        still_url: None, width: 220, height: 180,
    }], page: 1, has_next: false }, json!({"items":[{"slug":"hello","klipyId":"9007199254740993","title":"Hello","attribution":"Powered by KLIPY","previewUrl":"/api/v1/gifs/media?u=x","stillUrl":null,"width":220,"height":180}],"page":1,"hasNext":false}));
    assert_wire(&GifCategoryList { categories: vec![GifCategory { name:"Hello".into(), query:"hello".into(), preview_url:None }] }, json!({"categories":[{"name":"Hello","query":"hello","previewUrl":null}]}));
    let input: CreateMessage = serde_json::from_value(json!({"clientMessageId":"gif-1","markdownSource":"","replyToMessageId":null,"replyNotifyAuthor":null,"attachmentSignedId":null,"gifSlug":"hello"})).unwrap();
    assert_eq!(input.gif_slug.as_deref(), Some("hello"));
    assert_wire(&input, json!({"clientMessageId":"gif-1","markdownSource":"","replyToMessageId":null,"replyNotifyAuthor":null,"attachmentSignedId":null,"gifSlug":"hello"}));
}
