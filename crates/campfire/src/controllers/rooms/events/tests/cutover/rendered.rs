//! Successful HTML controls and navigation read from the actual server response.
//! This drives HTTP contracts; it does not execute Turbo/Stimulus JavaScript.
use super::super::*;
use campfire_richtext::dom::{Dom, NodeId};

fn parse(html: &str) -> (Dom, NodeId) {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).unwrap();
    (dom, root)
}
fn text(dom: &Dom, node: NodeId) -> String {
    dom.text_content(node)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn target(dom: &Dom, node: NodeId) -> Option<String> {
    // Turbo gives submitter data-turbo-frame precedence over the form, then
    // uses the containing frame's target/id (vendored turbo.js FrameController).
    let chain = std::iter::once(node).chain(dom.ancestors(node));
    for candidate in chain {
        if let Some(id) = dom.attr(candidate, "data-turbo-frame") {
            return (id != "_top" && !id.is_empty()).then(|| id.to_owned());
        }
        if dom.local_name(candidate) == Some("turbo-frame") {
            let id = dom
                .attr(candidate, "target")
                .or_else(|| dom.attr(candidate, "id"));
            return id
                .filter(|id| *id != "_top" && !id.is_empty())
                .map(str::to_owned);
        }
    }
    None
}
fn with_target(req: Req, frame: Option<String>) -> Req {
    match frame {
        Some(id) => req.header("turbo-frame", &id),
        None => req,
    }
}
pub(super) fn link(html: &str, label: &str) -> Req {
    let (dom, root) = parse(html);
    let matches = dom
        .descendants(root)
        .into_iter()
        .filter(|&n| {
            dom.local_name(n) == Some("a")
                && (text(&dom, n) == label || dom.attr(n, "aria-label") == Some(label))
        })
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "rendered link {label}");
    let node = matches[0];
    with_target(
        Req::new(Method::GET, dom.attr(node, "href").unwrap()),
        target(&dom, node),
    )
}
pub(super) fn article(html: &str, id: &str) -> String {
    let (dom, root) = parse(html);
    let matches = dom
        .descendants(root)
        .into_iter()
        .filter(|&n| dom.attr(n, "id") == Some(id))
        .collect::<Vec<_>>();
    assert_eq!(matches.len(), 1, "rendered inbox item {id}");
    dom.to_html(matches[0])
}
pub(super) fn lazy_frame(html: &str) -> (Req, String) {
    let (dom, root) = parse(html);
    let frames = dom
        .descendants(root)
        .into_iter()
        .filter(|&n| dom.local_name(n) == Some("turbo-frame") && dom.has_attr(n, "src"))
        .collect::<Vec<_>>();
    assert_eq!(frames.len(), 1, "the selected card's rendered lazy frame");
    let node = frames[0];
    let id = dom.attr(node, "id").unwrap().to_owned();
    (
        Req::new(Method::GET, dom.attr(node, "src").unwrap()).header("turbo-frame", &id),
        id,
    )
}
pub(super) fn submit(html: &str, label: &str, changes: &[(&str, &str)]) -> Req {
    let (dom, root) = parse(html);
    let buttons = dom
        .descendants(root)
        .into_iter()
        .filter(|&n| match dom.local_name(n) {
            Some("button") => {
                dom.attr(n, "type").is_none_or(|kind| kind == "submit") && text(&dom, n) == label
            }
            Some("input") => {
                dom.attr(n, "type") == Some("submit") && dom.attr(n, "value") == Some(label)
            }
            _ => false,
        })
        .collect::<Vec<_>>();
    assert_eq!(buttons.len(), 1, "rendered submitter {label}");
    let button = buttons[0];
    let form = dom
        .ancestors(button)
        .into_iter()
        .find(|&n| dom.local_name(n) == Some("form"))
        .unwrap();
    let action = dom
        .attr(button, "formaction")
        .or_else(|| dom.attr(form, "action"))
        .unwrap();
    let method = dom
        .attr(button, "formmethod")
        .or_else(|| dom.attr(form, "method"))
        .unwrap_or("get");
    let method = Method::from_bytes(method.to_uppercase().as_bytes()).unwrap();
    let mut fields = Vec::<(String, String)>::new();
    let mut filled = Vec::new();
    for node in dom.descendants(form) {
        let Some(name) = dom.attr(node, "name") else {
            continue;
        };
        if dom.has_attr(node, "disabled")
            || dom
                .ancestors(node)
                .iter()
                .any(|&n| dom.local_name(n) == Some("fieldset") && dom.has_attr(n, "disabled"))
        {
            continue;
        }
        let changed = changes
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| *value);
        let value = match dom.local_name(node) {
            Some("input") => {
                match dom.attr(node, "type").unwrap_or("text") {
                    "submit" if node != button => continue,
                    "button" | "reset" | "file" => continue,
                    "checkbox" | "radio" if !dom.has_attr(node, "checked") => continue,
                    _ => (),
                }
                changed
                    .unwrap_or_else(|| dom.attr(node, "value").unwrap_or(""))
                    .to_owned()
            }
            Some("textarea") => changed
                .map(str::to_owned)
                .unwrap_or_else(|| dom.text_content(node)),
            Some("select") => {
                let options = dom
                    .descendants(node)
                    .into_iter()
                    .filter(|&n| {
                        dom.local_name(n) == Some("option") && !dom.has_attr(n, "disabled")
                    })
                    .collect::<Vec<_>>();
                let chosen = if let Some(value) = changed {
                    options
                        .iter()
                        .copied()
                        .find(|&n| text(&dom, n) == value || dom.attr(n, "value") == Some(value))
                } else {
                    options
                        .iter()
                        .copied()
                        .find(|&n| dom.has_attr(n, "selected"))
                        .or_else(|| options.first().copied())
                };
                let chosen = chosen.expect("the requested choice exists in the rendered select");
                dom.attr(chosen, "value")
                    .map(str::to_owned)
                    .unwrap_or_else(|| text(&dom, chosen))
            }
            Some("button") if node == button => dom.attr(node, "value").unwrap_or("").to_owned(),
            _ => continue,
        };
        if changed.is_some() {
            filled.push(name.to_owned());
        }
        fields.push((name.to_owned(), value));
    }
    for (name, _) in changes {
        assert!(
            filled.iter().any(|n| n == name),
            "filled an actual rendered control {name}"
        );
    }
    let pairs = fields
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect::<Vec<_>>();
    // Do not use Browser::write: the rendered authenticity_token must authorize
    // this submission, rather than an independently synthesized CSRF header.
    let request = if method == Method::GET {
        let query = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(pairs)
            .finish();
        Req::new(method, &format!("{action}?{query}"))
    } else {
        Req::new(method, action).form(&pairs)
    };
    with_target(request, target(&dom, button))
}
pub(super) async fn follow(client: &mut Browser<'_>, reply: &Reply) -> Reply {
    assert!(
        matches!(reply.status, StatusCode::FOUND | StatusCode::SEE_OTHER),
        "submitted form redirects: {} {}",
        reply.status,
        reply.text()
    );
    let url = url::Url::parse(reply.location().unwrap()).unwrap();
    assert_eq!(url.host_str(), Some("campfire.test"));
    let path = match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_owned(),
    };
    client.get(&path).await
}
