//! All eight pinned Fizzy::CardUrlTest behaviors. The configured base is explicit: no ENV race.
use super::*;
use crate::integrations::{
    fizzy::client::DEFAULT_BASE, link_embed::url_classifier::fizzy_card_url,
};
fn refs(text: &str) -> Vec<Reference> {
    extract(text, DEFAULT_BASE).unwrap()
}
fn pair(account: &str, number: i64) -> Reference {
    Reference {
        account_id: account.into(),
        number,
    }
}
#[test]
fn ws15e_rails_fizzy_url_canonical() {
    assert_eq!(
        refs("see https://app.fizzy.do/897362094/cards/579 please"),
        [pair("897362094", 579)]
    );
}
#[test]
fn ws15e_rails_fizzy_url_suffixes() {
    for url in [
        "https://app.fizzy.do/897362094/cards/579/comments/03comment1",
        "https://app.fizzy.do/897362094/cards/579?x=1#frag",
    ] {
        assert_eq!(refs(url), [pair("897362094", 579)]);
    }
}
#[test]
fn ws15e_rails_fizzy_url_order_dedup() {
    assert_eq!(
        refs(
            "https://app.fizzy.do/897362094/cards/1 and https://app.fizzy.do/6264925/cards/2\nagain: https://app.fizzy.do/897362094/cards/1"
        ),
        [pair("897362094", 1), pair("6264925", 2)]
    );
}
#[test]
fn ws15e_rails_fizzy_url_non_cards() {
    for text in [
        "https://app.fizzy.do/897362094/boards/03board1",
        "https://app.fizzy.do/897362094/cards",
        "https://example.com/897362094/cards/123",
        "http://app.fizzy.do/897362094/cards/123",
        "just some text",
        "",
    ] {
        assert!(refs(text).is_empty(), "{text}");
    }
}
#[test]
fn ws15e_rails_fizzy_url_four_card_cap() {
    let text = (1..=6)
        .map(|n| format!("https://app.fizzy.do/897362094/cards/{n}"))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        refs(&text).iter().map(|r| r.number).collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
}
#[test]
fn ws15e_rails_fizzy_url_non_code_text() {
    // The very same scanner invoked by Fizzy's real WS8 reference hook.
    let html = r#"<p>see <code>https://app.fizzy.do/1/cards/1</code> and https://app.fizzy.do/1/cards/2</p>
        <pre><code class="language-text">https://app.fizzy.do/1/cards/3</code></pre>
        <p><a href="https://app.fizzy.do/1/cards/4">the card</a></p>"#;
    let text = crate::integrations::linkedin::non_code_text(html).unwrap();
    assert!(!text.contains("cards/1"));
    assert!(!text.contains("cards/3"));
    assert!(text.contains("cards/2"));
    assert!(text.contains("cards/4"));
    assert_eq!(refs(&text), [pair("1", 2), pair("1", 4)]);
}
#[test]
fn ws15e_rails_fizzy_url_predicate() {
    assert!(fizzy_card_url(
        "https://app.fizzy.do/897362094/cards/579",
        DEFAULT_BASE
    ));
    assert!(!fizzy_card_url(
        "https://app.fizzy.do/897362094/boards/1",
        DEFAULT_BASE
    ));
    assert!(!fizzy_card_url("https://example.com/x", DEFAULT_BASE));
}
#[test]
fn ws15e_rails_fizzy_url_configured_host() {
    let base = "https://fizzy.example.com";
    let custom = "https://fizzy.example.com/897362094/cards/579";
    let canonical = "https://app.fizzy.do/897362094/cards/579";
    assert_eq!(
        extract(&format!("see {custom} please"), base).unwrap(),
        [pair("897362094", 579)]
    );
    assert!(
        extract(&format!("see {canonical} please"), base)
            .unwrap()
            .is_empty()
    );
    assert!(fizzy_card_url(custom, base));
    assert!(!fizzy_card_url(canonical, base));
}
