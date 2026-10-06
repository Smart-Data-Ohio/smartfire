//! Behavioral receipts, separate from the screenshot-state inventory.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The 284 view files of the Rails app, which is gone: `rails_templates`' keys are their frozen list.
const RAILS_TEMPLATES: usize = 284;

fn resolve(path: &str) -> PathBuf {
    let relative = path
        .strip_prefix("rust/")
        .unwrap_or_else(|| panic!("{path} is outside rust/"));
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(relative)
}

fn files(directory: &Path) -> BTreeSet<String> {
    fn visit(root: &Path, directory: &Path, output: &mut BTreeSet<String>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, output);
            } else {
                output.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut output = BTreeSet::new();
    visit(directory, directory, &mut output);
    output
}

fn map() -> Value {
    serde_json::from_slice(&std::fs::read(resolve("rust/parity/template-coverage.json")).unwrap())
        .unwrap()
}

fn validate(map: &Value) -> Result<(), String> {
    let fresh: Value = serde_json::from_slice(
        &std::fs::read(resolve("rust/vectors/template_coverage_http.json")).unwrap(),
    )
    .unwrap();
    let pin = std::fs::read_to_string(resolve("rust/parity/reference.sha")).unwrap();
    if map["reference_sha"] != pin.trim() || fresh["reference"] != pin.trim() {
        return Err("template map and fresh controller oracle must identify the parity pin".into());
    }
    for (key, directory) in [
        ("templates", Some("rust/crates/views/templates")),
        ("rails_templates", None),
    ] {
        let entries = map[key]
            .as_object()
            .ok_or_else(|| format!("missing {key} map"))?;
        let actual: BTreeSet<_> = entries.keys().cloned().collect();
        let expected = match directory {
            Some(directory) => files(&resolve(directory)),
            None if actual.len() == RAILS_TEMPLATES => actual.clone(),
            None => return Err(format!("{key}: {} entries, not the frozen {RAILS_TEMPLATES}", actual.len())),
        };
        let missing: Vec<_> = expected.difference(&actual).cloned().collect();
        let stale: Vec<_> = actual.difference(&expected).cloned().collect();
        if !missing.is_empty() || !stale.is_empty() {
            return Err(format!(
                "{key}: {} missing (first 20: {:?}); stale {stale:?}",
                missing.len(),
                &missing[..missing.len().min(20)]
            ));
        }
        for (name, entry) in entries {
            if entry["branch"].as_str().is_none_or(str::is_empty) {
                return Err(format!("{name}: missing branch/reachability evidence"));
            }
            match entry["status"].as_str() {
                Some("covered") => {
                    let evidence = entry["evidence"]
                        .as_array()
                        .filter(|ids| !ids.is_empty())
                        .ok_or_else(|| format!("{name}: missing behavioral receipt"))?;
                    for id in evidence {
                        if map["evidence"]
                            .get(id.as_str().unwrap_or_default())
                            .is_none()
                        {
                            return Err(format!("{name}: unknown receipt {id}"));
                        }
                    }
                }
                Some("not_reachable") => {
                    let sources = entry["sources"]
                        .as_array()
                        .filter(|paths| !paths.is_empty())
                        .ok_or_else(|| format!("{name}: missing unreachable-source evidence"))?;
                    // Sources outside rust/ were the Rails app's, removed with it.
                    for source in sources {
                        let source_path = source.as_str().unwrap_or_default();
                        if source_path.starts_with("rust/") && !resolve(source_path).is_file() {
                            return Err(format!("{name}: missing source {source}"));
                        }
                    }
                }
                _ => return Err(format!("{name}: no covered or not_reachable disposition")),
            }
            if entry["status"] == "covered" {
                if let Some(witness) = entry.get("witness") {
                    let oracle = witness["oracle"].as_str().ok_or("witness missing oracle")?;
                    let pointer = witness["pointer"]
                        .as_str()
                        .ok_or("witness missing pointer")?;
                    let needle = witness["needle"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .ok_or("witness missing needle")?;
                    let text = std::fs::read_to_string(resolve(oracle))
                        .map_err(|error| format!("{name}: {error}"))?;
                    let body = if oracle.ends_with(".json") {
                        let value: Value = serde_json::from_str(&text)
                            .map_err(|error| format!("{name}: {error}"))?;
                        value.pointer(pointer).and_then(Value::as_str).ok_or_else(|| format!("{name}: witness pointer {pointer} does not identify an output string"))?.to_owned()
                    } else {
                        text
                    };
                    if !body.contains(needle) {
                        return Err(format!(
                            "{name}: rendered branch witness absent from {oracle}{pointer}"
                        ));
                    }
                    let recorded = entry["evidence"].as_array().unwrap().iter().any(|id| {
                        map["evidence"][id.as_str().unwrap()]["oracles"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|path| path == oracle)
                    });
                    if !recorded {
                        return Err(format!(
                            "{name}: witness is outside the recorded test's oracles"
                        ));
                    }
                    if key == "rails_templates"
                        && oracle == "rust/vectors/template_coverage_http.json"
                    {
                        let case_pointer = pointer.strip_suffix("/body").ok_or_else(|| {
                            format!("{name}: fresh controller witness must identify response body")
                        })?;
                        let traces = fresh
                            .pointer(case_pointer)
                            .and_then(|case| case["templates"].as_array())
                            .ok_or_else(|| format!("{name}: missing Rails render trace"))?;
                        if !traces.iter().any(|template| template == name) {
                            return Err(format!(
                                "{name}: controller witness did not render this Rails declaration"
                            ));
                        }
                    }
                } else if entry["proof"].as_str().is_none_or(str::is_empty) {
                    return Err(format!("{name}: no output witness or composition proof"));
                }
            }
        }
    }
    let logical: BTreeSet<_> = map["rails_templates"]
        .as_object()
        .unwrap()
        .keys()
        .map(|name| {
            let name = name
                .strip_suffix(".erb")
                .or_else(|| name.strip_suffix(".jbuilder"))
                .unwrap_or(name);
            name.strip_suffix(".html").unwrap_or(name).to_owned()
        })
        .collect();
    for name in map["cutover_gap"]["templates"]
        .as_array()
        .ok_or("missing cutover gap snapshot")?
    {
        if !logical.contains(name.as_str().ok_or("invalid cutover template name")?) {
            return Err(format!(
                "cutover template {name} is missing its disposition"
            ));
        }
    }
    for (id, receipt) in map["evidence"].as_object().ok_or("missing evidence map")? {
        let source = resolve(
            receipt["test_file"]
                .as_str()
                .ok_or_else(|| format!("{id}: no test file"))?,
        );
        let contents = std::fs::read_to_string(source).map_err(|error| format!("{id}: {error}"))?;
        let name = receipt["test"]
            .as_str()
            .ok_or_else(|| format!("{id}: no test name"))?;
        let declaration = regex::Regex::new(&format!(
            r"#\[(?:tokio::)?test[^\]]*\]\s*(?:#\[[^\]]*\]\s*)*(?:async\s+)?fn\s+{}\s*\(",
            regex::escape(name)
        ))
        .unwrap();
        let macro_case = receipt["test_macro"].as_str().is_some_and(|macro_name| {
            let invocation = regex::Regex::new(&format!(
                r"{}!\([\s\S]*?\b{}\b",
                regex::escape(macro_name),
                regex::escape(name)
            ))
            .unwrap();
            let definition = format!("macro_rules! {macro_name}");
            contents.contains(&definition)
                && contents.contains("#[tokio::test]")
                && invocation.is_match(&contents)
        });
        if !declaration.is_match(&contents) && !macro_case {
            return Err(format!("{id}: missing test declaration {name}"));
        }
        let oracles = receipt["oracles"]
            .as_array()
            .filter(|paths| !paths.is_empty())
            .ok_or_else(|| format!("{id}: missing Rails oracle"))?;
        for oracle in oracles {
            let file = resolve(oracle.as_str().unwrap_or_default());
            if !file.is_file() || file.metadata().unwrap().len() == 0 {
                return Err(format!("{id}: missing or empty Rails oracle {oracle}"));
            }
        }
    }
    Ok(())
}

#[test]
fn every_template_has_a_behavioral_receipt_or_reachability_disposition() {
    let map = map();
    validate(&map).unwrap();
    println!(
        "Template coverage: {} Rust files, {} Rails declarations, {} behavioral receipts",
        map["templates"].as_object().unwrap().len(),
        map["rails_templates"].as_object().unwrap().len(),
        map["evidence"].as_object().unwrap().len()
    );
}

#[test]
fn coverage_guard_rejects_missing_templates_and_unproven_receipts() {
    let mut missing = map();
    let name = missing["templates"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    missing["templates"].as_object_mut().unwrap().remove(&name);
    assert!(validate(&missing).unwrap_err().contains(&name));

    let mut unproven = map();
    let entry = unproven["templates"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .find(|entry| entry["status"] == "covered")
        .unwrap();
    entry["evidence"] = serde_json::json!([]);
    assert!(
        validate(&unproven)
            .unwrap_err()
            .contains("missing behavioral receipt")
    );

    let mut absent_branch = map();
    let witness = absent_branch["templates"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .find_map(|entry| entry.get_mut("witness"))
        .unwrap();
    witness["needle"] = serde_json::json!("this branch is not present in the Rails output");
    assert!(
        validate(&absent_branch)
            .unwrap_err()
            .contains("rendered branch witness absent")
    );

    let mut missing_test = map();
    missing_test["evidence"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()["test"] = serde_json::json!("this_test_does_not_exist");
    assert!(
        validate(&missing_test)
            .unwrap_err()
            .contains("missing test declaration")
    );

    let mut wrong_controller_branch = map();
    wrong_controller_branch["rails_templates"]["users/show.html.erb"]["witness"] = serde_json::json!({
        "oracle": "rust/vectors/template_coverage_http.json",
        "pointer": "/cases/11/body",
        "needle": "Failed to load message content"
    });
    assert!(
        validate(&wrong_controller_branch)
            .unwrap_err()
            .contains("controller witness did not render this Rails declaration")
    );
}
