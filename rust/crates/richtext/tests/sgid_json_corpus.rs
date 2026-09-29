//! Golden vectors run through the real Rails MessagesHelper, including its logging rescue.
use base64::Engine;
use campfire_richtext::{AttachableResolver, GidLookup, Presentation, RenderContext, SignedLookup, present_message};

struct LookupInputs(std::collections::BTreeMap<String, bool>);
impl AttachableResolver for LookupInputs {
    fn locate_signed(&self, _: &str) -> SignedLookup {
        SignedLookup::Invalid
    }
    fn find_gid(&self, gid: &str) -> GidLookup {
        if self.0.get(gid) == Some(&true) { GidLookup::Raises } else { GidLookup::NotFound }
    }
}

#[test]
fn malformed_sgid_helper_corpus_is_byte_identical() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("corpus/sgid-json.json")).unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert!(cases.len() >= 20000);
    // These are independently generated GlobalID.find inputs for the valid-JSON controls.
    // The resolver is a DB-free application input; it does not classify SGIDs or presentations.
    let mut inputs = std::collections::BTreeMap::new();
    for case in cases {
        let lookup = &case["gid_lookup"];
        if let Some(gid) = lookup["gid"].as_str() {
            assert_ne!(lookup["unloggable"], true);
            let bytes = base64::engine::general_purpose::STANDARD.decode(gid).unwrap();
            let gid = String::from_utf8_lossy(&bytes).into_owned();
            let raised = lookup["outcome"] == "raised";
            if let Some(prior) = inputs.insert(gid, raised) {
                assert_eq!(prior, raised);
            }
        }
    }
    let resolver = LookupInputs(inputs);
    let ctx = RenderContext { resolver: &resolver, request_host: Some("once.campfire.test".into()) };
    let mut differences = Vec::new();
    for case in cases {
        let body = format!("<action-text-attachment sgid=\"{}\"></action-text-attachment>", case["payload"].as_str().unwrap());
        let actual = present_message(&body, &ctx);
        let expected = match case["presentation"]["ok"].as_str() {
            Some(html) => Presentation::Html(html.to_owned()),
            None => Presentation::Unrenderable,
        };
        if actual != expected {
            let payload = base64::engine::general_purpose::STANDARD.decode(case["payload"].as_str().unwrap()).unwrap();
            differences.push(format!("{} {:?}: {actual:?} != {expected:?}", case["name"], String::from_utf8_lossy(&payload)));
        }
    }
    println!("Ruby SGID helper corpus: {} cases, {} differences (no normalization)", cases.len(), differences.len());
    assert!(differences.is_empty(), "{}", differences.iter().take(30).cloned().collect::<Vec<_>>().join("\n"));
}
