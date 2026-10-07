//! Rails form and visible-behavior contracts that permit presentation changes.
#![allow(dead_code)]
use campfire_richtext::dom::{Dom, NodeId};

#[derive(Debug, PartialEq, Eq)]
pub struct Form {
    action: String,
    /// The `method` attribute the browser submits with (`get` or `post`).
    html_method: String,
    /// The method Rails routes on: `_method` when present, else the attribute.
    method: String,
    enctype: String,
    controls: Vec<Control>,
}

#[derive(Debug, PartialEq, Eq)]
struct Control {
    element: String,
    name: Option<String>,
    kind: String,
    values: Vec<String>,
    attributes: Vec<Option<String>>,
}

pub fn forms(html: &str) -> Vec<Form> {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).unwrap();
    let nodes = dom.descendants(root);
    nodes
        .iter()
        .copied()
        .filter(|id| dom.name(*id) == "form")
        .map(|form| {
            let controls = nodes
                .iter()
                .copied()
                .filter(|id| {
                    matches!(
                        dom.name(*id).as_ref(),
                        "input" | "select" | "textarea" | "button"
                    ) && owner(&dom, *id, &nodes) == Some(form)
                })
                .map(|id| control(&dom, id))
                .collect::<Vec<_>>();
            let method = controls
                .iter()
                .find(|c| c.name.as_deref() == Some("_method"))
                .and_then(|c| c.values.first())
                .map(String::as_str)
                .unwrap_or_else(|| dom.attr(form, "method").unwrap_or("get"))
                .to_ascii_lowercase();
            Form {
                action: dom.attr(form, "action").unwrap_or("").into(),
                html_method: dom.attr(form, "method").unwrap_or("get").to_ascii_lowercase(),
                method,
                enctype: dom
                    .attr(form, "enctype")
                    .unwrap_or("application/x-www-form-urlencoded")
                    .to_ascii_lowercase(),
                controls,
            }
        })
        .collect()
}

fn owner(dom: &Dom, id: NodeId, nodes: &[NodeId]) -> Option<NodeId> {
    if let Some(name) = dom.attr(id, "form") {
        return nodes
            .iter()
            .copied()
            .find(|node| dom.name(*node) == "form" && dom.attr(*node, "id") == Some(name));
    }
    dom.ancestors(id)
        .into_iter()
        .find(|node| dom.name(*node) == "form")
}

fn control(dom: &Dom, id: NodeId) -> Control {
    let element = dom.name(id).into_owned();
    let name = dom.attr(id, "name").map(str::to_owned);
    let kind = match element.as_str() {
        "input" => dom.attr(id, "type").unwrap_or("text").to_ascii_lowercase(),
        "button" => dom
            .attr(id, "type")
            .unwrap_or("submit")
            .to_ascii_lowercase(),
        "select" if dom.has_attr(id, "multiple") => "select-multiple".into(),
        "select" => "select-one".into(),
        _ => element.clone(),
    };
    let values = if name.as_deref() == Some("authenticity_token") {
        // Any token stands for any other, but an empty or missing one never matches a real one.
        match dom.attr(id, "value") {
            Some(value) if !value.is_empty() => vec!["<authenticity_token>".into()],
            _ => vec!["<missing authenticity_token>".into()],
        }
    } else if element == "textarea" {
        vec![dom.text_content(id)]
    } else if element == "select" {
        let options = dom
            .descendants(id)
            .into_iter()
            .filter(|n| dom.name(*n) == "option")
            .collect::<Vec<_>>();
        let mut selected = options
            .iter()
            .copied()
            .filter(|n| dom.has_attr(*n, "selected"))
            .collect::<Vec<_>>();
        if selected.is_empty() && !dom.has_attr(id, "multiple") {
            selected.extend(options.first().copied());
        }
        selected
            .into_iter()
            .map(|n| {
                dom.attr(n, "value")
                    .map(str::to_owned)
                    .unwrap_or_else(|| dom.text_content(n))
            })
            .collect()
    } else {
        vec![
            dom.attr(id, "value")
                .unwrap_or(if matches!(kind.as_str(), "checkbox" | "radio") {
                    "on"
                } else {
                    ""
                })
                .into(),
        ]
    };
    let attributes = [
        "required",
        "autocomplete",
        "maxlength",
        "pattern",
        "inputmode",
        "accept",
        "checked",
        "multiple",
        "disabled",
    ]
    .into_iter()
    .map(|attr| {
        if matches!(attr, "required" | "checked" | "multiple" | "disabled") {
            dom.has_attr(id, attr).then(String::new)
        } else {
            dom.attr(id, attr).map(str::to_owned)
        }
    })
    .collect();
    Control {
        element,
        name,
        kind,
        values,
        attributes,
    }
}

/// The head metadata a page's behaviour depends on: the CSRF pair (any token stands for any
/// other; an empty one stands for none), the signed-in user and Turbo's reload and cache directives.
const HEAD_META: [&str; 5] = [
    "csrf-param",
    "csrf-token",
    "current-user-id",
    "turbo-visit-control",
    "turbo-cache-control",
];

pub fn head(html: &str) -> Vec<(String, String)> {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).unwrap();
    dom.descendants(root)
        .into_iter()
        .filter(|id| dom.name(*id) == "meta")
        .filter_map(|id| {
            let name = dom.attr(id, "name")?;
            HEAD_META.contains(&name).then(|| {
                let content = dom.attr(id, "content").unwrap_or_default();
                let content = match (name, content.is_empty()) {
                    ("csrf-token", false) => "<csrf-token>".to_owned(),
                    _ => content.to_owned(),
                };
                (name.to_owned(), content)
            })
        })
        .collect()
}

pub fn assert_head(name: &str, actual: &str, rails: &str) {
    assert_eq!(head(actual), head(rails), "{name}: Rails head metadata");
}

/// The forms, and for complete documents the head metadata too.
pub fn assert_forms(name: &str, actual: &str, rails: &str) {
    if rails.contains("<head") {
        assert_head(name, actual, rails);
    }
    assert_eq!(forms(actual), forms(rails), "{name}: Rails form contract");
}

/// Whether a response is one of the pages restyled in the SPA's look (the auth and public
/// layouts). Their Rails vectors pin the forms, not the markup; every other page stays exact.
pub fn reskinned(html: &str) -> bool {
    html.contains("<body class=\"auth\">") || html.contains("<body class=\"public-page")
}

/// The page's head metadata and its own forms (action, method, fields and values), comparable
/// across presentations.
pub fn contract(html: &str) -> String {
    format!("{:?} {:?}", head(html), forms(page_content(html)))
}

/// A restyled page matches its Rails vector when its forms do; any other page byte for byte.
pub fn same_page(actual: &str, rails: &str) -> bool {
    if reskinned(actual) {
        contract(actual) == contract(rails)
    } else {
        actual == rails
    }
}

/// Full-page vectors contain unrelated workspace chrome. Compare the page's own region.
pub fn page_content(html: &str) -> &str {
    let Some(start) = html.find("<main ") else {
        return html;
    };
    let content = &html[start..];
    let content = &content[content.find('>').unwrap() + 1..];
    let end = content.find("<footer").or_else(|| content.find("</main>"));
    &content[..end.unwrap_or(content.len())]
}

pub fn text(html: &str) -> String {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).unwrap();
    let mut text = String::new();
    for id in dom.descendants(root) {
        if dom
            .ancestors(id)
            .into_iter()
            .any(|n| matches!(dom.name(n).as_ref(), "script" | "style" | "svg"))
        {
            continue;
        }
        if let Some(value) = dom.text(id) {
            text.push_str(value);
            text.push(' ');
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn assert_text(html: &str, expected: &str) {
    let actual = text(html);
    let expected = expected.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        actual.contains(&expected),
        "missing visible text {expected:?}: {actual}"
    );
}

/// Public navigation and policy links retain their destinations and browsing/security behavior.
pub fn links(html: &str) -> Vec<(String, Option<String>, Option<String>)> {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).unwrap();
    let mut links = dom
        .descendants(root)
        .into_iter()
        .filter(|id| dom.name(*id) == "a")
        .map(|id| {
            (
                dom.attr(id, "href").unwrap_or("").into(),
                dom.attr(id, "target").map(str::to_owned),
                dom.attr(id, "rel").map(str::to_owned),
            )
        })
        .collect::<Vec<_>>();
    links.sort();
    links
}

pub fn assert_public_page(name: &str, actual: &str, rails: &str) {
    assert_forms(name, actual, rails);
    assert_eq!(links(actual), links(rails), "{name}: public links");
    assert_eq!(
        text(page_content(actual)),
        text(page_content(rails)),
        "{name}: public policy text and escaped values"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_contracts_ignore_presentation_but_detect_submission_and_constraint_changes() {
        let rails = r#"<form action="/user?a=1&amp;b=2" method="post" enctype="multipart/form-data"><input name="_method" type="hidden" value="patch"><input type="hidden" name="authenticity_token" value="old"><input name="name" required autocomplete="name" maxlength="72" pattern=".*" inputmode="text"><input type="file" name="avatar" accept="image/*" multiple><input type="checkbox" name="remember" value="1" checked><textarea name="note">A &amp; B</textarea><select name="choice"><option value="a">A</option><option selected value="b">B</option></select><button name="confirm">Confirm</button></form>"#;
        let styled = rails
            .replace("value=\"old\"", "value=\"new\"")
            .replace("<form ", "<form class=\"auth\" data-turbo=\"false\" ");
        assert_forms("presentation", &styled, rails);
        for mutation in [
            rails.replace("patch", "delete"),
            rails.replace("maxlength=\"72\"", "maxlength=\"73\""),
            rails.replace(" required", ""),
            rails.replace(" checked", ""),
            rails.replace("selected value=\"b\"", "selected value=\"c\""),
            rails.replace("A &amp; B", "different"),
            rails.replace("name=\"confirm\"", "name=\"cancel\""),
        ] {
            assert_ne!(forms(&mutation), forms(rails));
        }
        assert_eq!(forms(rails)[0].action, "/user?a=1&b=2");
        assert_eq!(forms(rails)[0].method, "patch");
        assert_eq!(forms(rails)[0].controls[6].values, ["b"]);
    }

    #[test]
    fn form_contracts_include_external_controls_and_normalize_boolean_attributes() {
        assert_forms(
            "owner",
            r#"<form id="f" action="/" method="post"><input name="x" required></form><button form="f" name="go">Go</button>"#,
            r#"<form id="f" action="/" method="POST"><input type="text" name="x" required="required"><button type="submit" name="go">Go</button></form>"#,
        );
    }
}
