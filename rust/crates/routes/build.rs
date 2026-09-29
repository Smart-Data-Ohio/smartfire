//! Writes the route table and the `*_path` helpers from `routes.json`, which
//! `reference-tools/routes/routes.rb` dumps from the reference's `config/routes.rb`.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize)]
struct Routes {
    table: Vec<TableRoute>,
    named_routes: Vec<NamedRoute>,
}

#[derive(Deserialize)]
struct TableRoute {
    verb: String,
    spec: String,
    endpoint: String,
    name: Option<String>,
    defaults: Vec<(String, String)>,
    action: String,
}

#[derive(Deserialize)]
struct NamedRoute {
    name: String,
    verb: String,
    spec: String,
    endpoint: String,
    arguments: Vec<String>,
    defaults: Vec<(String, String)>,
}

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source = crate_dir.join("routes.json");
    println!("cargo:rerun-if-changed={}", source.display());
    let routes: Routes = serde_json::from_str(&deserializable(&fs::read_to_string(&source).unwrap()))
        .expect("routes.json");

    let mut code = String::new();
    code.push_str("/// Every route, in `config/routes.rb` (`bin/rails routes`) order.\n");
    code.push_str("pub static TABLE: &[TableRoute] = &[\n");
    for route in &routes.table {
        writeln!(
            code,
            "    TableRoute {{ verb: {:?}, spec: {:?}, endpoint: {:?}, name: {:?}, defaults: &{:?}, action: ActionStatus::{} }},",
            route.verb,
            route.spec,
            route.endpoint,
            route.name,
            route.defaults,
            match route.action.as_str() {
                "defined" => "Defined",
                "implicit" => "Implicit",
                "action_not_found" => "ActionNotFound",
                "missing_controller" => "MissingController",
                other => panic!("unknown action status {other}"),
            }
        )
        .unwrap();
    }
    code.push_str("];\n\n");

    for route in &routes.named_routes {
        let constant = route.name.to_uppercase();
        let default_format = route.defaults.iter().find(|(k, _)| k == "format").map(|(_, v)| v.as_str());
        writeln!(
            code,
            "/// `{name}_path`: {verb} `{spec}` ({endpoint}).\npub static {constant}: NamedRoute = NamedRoute {{ name: {name:?}, spec: {spec:?}, segments: &[{segments}], default_format: {default_format:?} }};",
            name = route.name,
            verb = route.verb,
            spec = route.spec,
            endpoint = route.endpoint,
            segments = segments(&route.spec, &route.defaults).join(", "),
        )
        .unwrap();
        let params = route
            .arguments
            .iter()
            .map(|arg| format!("{arg}: impl Display"))
            .collect::<Vec<_>>()
            .join(", ");
        let args = route.arguments.iter().map(|arg| format!("&{arg}")).collect::<Vec<_>>().join(", ");
        writeln!(
            code,
            "/// `{name}_path`: {verb} `{spec}` ({endpoint}).\npub fn {name}({params}) -> String {{ {constant}.path(&[{args}]) }}\n",
            name = route.name,
            verb = route.verb,
            spec = route.spec,
            endpoint = route.endpoint,
        )
        .unwrap();
    }

    code.push_str("/// Every named route, sorted by name.\npub static NAMED_ROUTES: &[&NamedRoute] = &[\n");
    for route in &routes.named_routes {
        writeln!(code, "    &{},", route.name.to_uppercase()).unwrap();
    }
    code.push_str("];\n");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("generated.rs");
    fs::write(out, code).unwrap();
}

/// A Journey spec as segments: literals, `:param`s and `*glob`s, with `(.:format)` left to the
/// runtime. A param with a default (`scope defaults: { user_id: "me" }`) becomes its default.
fn segments(spec: &str, defaults: &[(String, String)]) -> Vec<String> {
    let spec = spec.strip_suffix("(.:format)").unwrap_or(spec);
    assert!(!spec.contains('('), "optional segments other than (.:format) aren't supported: {spec}");
    let mut segments = Vec::new();
    let mut literal = String::new();
    let mut chars = spec.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ':' || c == '*' {
            let mut name = String::new();
            while let Some(&n) = chars.peek().filter(|n| n.is_ascii_alphanumeric() || **n == '_') {
                name.push(n);
                chars.next();
            }
            if let Some((_, value)) = defaults.iter().find(|(k, _)| *k == name) {
                literal.push_str(value);
                continue;
            }
            if !literal.is_empty() {
                segments.push(format!("Segment::Literal({:?})", std::mem::take(&mut literal)));
            }
            segments.push(if c == ':' { "Segment::Param".to_string() } else { "Segment::Glob".to_string() });
        } else {
            literal.push(c);
        }
    }
    if !literal.is_empty() {
        segments.push(format!("Segment::Literal({literal:?})"));
    }
    segments
}

/// The JSON's `defaults` objects as `[[key, value], ...]`, which serde reads as pairs in order.
fn deserializable(json: &str) -> String {
    let mut value: serde_json::Value = serde_json::from_str(json).unwrap();
    fn pairs(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map.iter_mut() {
                    if key == "defaults" {
                        if let serde_json::Value::Object(defaults) = child {
                            *child = serde_json::Value::Array(
                                defaults
                                    .iter()
                                    .map(|(k, v)| serde_json::json!([k, v]))
                                    .collect(),
                            );
                        }
                    } else {
                        pairs(child);
                    }
                }
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(pairs),
            _ => {}
        }
    }
    pairs(&mut value);
    value.to_string()
}
