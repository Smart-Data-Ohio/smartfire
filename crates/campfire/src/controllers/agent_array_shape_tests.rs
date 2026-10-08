//! Independent array lookup acceptance receipts, written before assertions.
use crate::controllers::presenters::test_support::{Req, TestApp};
use base64::{Engine, engine::general_purpose::STANDARD};
use campfire_kit::Method;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const HEADERS: [&str; 6] = [
    "content-type",
    "cache-control",
    "pragma",
    "retry-after",
    "location",
    "content-length",
];
fn rows(conn: &campfire_db::Connection, table: &str) -> campfire_db::Result<Vec<Value>> {
    let mut q = conn.prepare(&format!("SELECT * FROM \"{table}\" ORDER BY id"))?;
    let columns = q
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let values = q
        .query_map([], |r| {
            let mut object = serde_json::Map::new();
            for (i, name) in columns.iter().enumerate() {
                // Port-only counter for the SPA activity badge that Rails doesn't have.
                if table == "users" && name == "activity_revision" {
                    continue;
                }
                use rusqlite::types::ValueRef;
                object.insert(
                    name.clone(),
                    match r.get_ref(i)? {
                        ValueRef::Null => Value::Null,
                        ValueRef::Integer(v) => json!(v),
                        ValueRef::Real(v) => json!(v),
                        ValueRef::Text(v) => json!(std::str::from_utf8(v).unwrap()),
                        ValueRef::Blob(v) => json!({"blob_base64":STANDARD.encode(v)}),
                    },
                );
            }
            Ok(json!(object))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(values)
}
async fn state(app: &TestApp, tables: &[String]) -> Value {
    let tables = tables.to_vec();
    app.db().read(move|conn|{let mut state=serde_json::Map::new();let mut raw=serde_json::Map::new();let mut json_columns=serde_json::Map::new();for table in tables{let values=rows(conn,&table)?;let columns=conn.prepare(&format!("PRAGMA table_info(\"{table}\")"))?.query_map([],|r|Ok((r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?.into_iter().filter(|(_,t)|t.eq_ignore_ascii_case("json")).map(|(n,_)|n).collect::<Vec<_>>();let mut parsed=values.clone();for row in &mut parsed{for column in &columns{if let Some(text)=row[column].as_str(){row[column]=serde_json::from_str(text).unwrap();}}}state.insert(table.clone(),json!(parsed));raw.insert(table.clone(),json!(values));json_columns.insert(table,json!(columns));}let jobs=rows(conn,"background_jobs")?;let logical_jobs=jobs.iter().map(|row|json!({"class":row["job_class"],"args":serde_json::from_str::<Value>(row["arguments"].as_str().unwrap()).unwrap()})).collect::<Vec<_>>();Ok(json!({"state":state,"raw_rows":raw,"raw_job_rows":jobs,"jobs":logical_jobs,"json_columns":json_columns,"read_connection_autocommit":conn.is_autocommit()}))}).await.unwrap()
}
fn request(case: &Value, manifest: &Value) -> Req {
    let mut req = Req::new(
        Method::from_bytes(case["method"].as_str().unwrap().as_bytes()).unwrap(),
        case["path"].as_str().unwrap(),
    )
    .header("host", "campfire.test");
    for (name, value) in manifest["request_headers"].as_object().unwrap() {
        req = req.header(name, value.as_str().unwrap());
    }
    req = req.header(
        "authorization",
        &format!("Bearer {}", manifest["secret"].as_str().unwrap()),
    );
    if let Some(body) = case["body"].as_str() {
        req = req.body(body.as_bytes().to_vec());
    }
    req
}
async fn run(case: &Value, manifest: &Value, fixture: &str, tables: &[String]) -> Value {
    let app = TestApp::boot_frozen()
        .await
        .expect("fresh default seed required")
        .without_job_runner()
        .await;
    let setup = format!(
        "{fixture}\n{}\nDELETE FROM background_jobs;",
        case["setup_sql"].as_str().unwrap()
    );
    app.db()
        .write(move |tx| {
            tx.conn().execute_batch(&setup)?;
            Ok(())
        })
        .await
        .unwrap();
    let mut browser = app.anonymous();
    let mut warmups = Vec::new();
    for _ in 0..case["warmups"].as_u64().unwrap_or(0) {
        let reply = browser.send(request(case, manifest)).await;
        warmups.push(json!({"status":reply.status.as_u16(),"body":reply.text(),"body_base64":STANDARD.encode(&reply.body)}));
    }
    let before = state(&app, tables).await;
    let mut steps = Vec::new();
    for attempt in 1..=case["repeat"].as_u64().unwrap() {
        let writer = case["surface"] == "react";
        let queries = if writer {
            app.db().capture_queries()
        } else {
            app.db().capture_read_queries()
        };
        let reply = browser.send(request(case, manifest)).await;
        app.db().stop_capturing_queries();
        let queries = queries.lock().unwrap().clone();
        let selects = queries
            .iter()
            .filter(|q| q.trim_start().to_ascii_uppercase().starts_with("SELECT"))
            .cloned()
            .collect::<Vec<_>>();
        let after = state(&app, tables).await;
        let headers = HEADERS
            .into_iter()
            .map(|h| (h.into(), json!(reply.header(h))))
            .collect::<serde_json::Map<_, _>>();
        let all_headers = reply
            .headers
            .keys()
            .map(|name| {
                (
                    name.as_str().to_string(),
                    json!(
                        reply
                            .headers
                            .get_all(name)
                            .iter()
                            .map(|v| v.to_str().unwrap().to_string())
                            .collect::<Vec<_>>()
                    ),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let changed = tables
            .iter()
            .filter(|t| after["state"][t.as_str()] != before["state"][t.as_str()])
            .cloned()
            .collect::<Vec<_>>();
        steps.push(json!({"attempt":attempt,"status":reply.status.as_u16(),"response_body":reply.text(),"response_body_base64":STANDARD.encode(&reply.body),"response_json":serde_json::from_slice::<Value>(&reply.body).ok(),"response_headers":headers,"all_headers":all_headers,"state":after["state"],"raw_rows":after["raw_rows"],"raw_job_rows":after["raw_job_rows"],"jobs":after["jobs"],"changed_tables":changed,"selects":selects.len(),"select_sql":selects,"query_boundary":if writer{"reader+writer"}else{"reader"},"read_connection_autocommit":after["read_connection_autocommit"]}));
    }
    println!(
        "WS11 next3 array case={} statuses={} SELECTs={}",
        case["name"].as_str().unwrap(),
        steps
            .iter()
            .map(|s| s["status"].to_string())
            .collect::<Vec<_>>()
            .join("/"),
        steps
            .iter()
            .map(|s| s["selects"].to_string())
            .collect::<Vec<_>>()
            .join("/")
    );
    json!({"name":case["name"],"surface":case["surface"],"count_label":case["count_label"],"candidate_count":case["candidate_count"],"warmups":warmups,"before":before["state"],"raw_before":before["raw_rows"],"json_columns":before["json_columns"],"steps":steps})
}
fn diff(a: &Value, e: &Value, path: &str, out: &mut Vec<String>) {
    if a == e {
        return;
    }
    match (a, e) {
        (Value::Object(a), Value::Object(e)) => {
            for key in a.keys().chain(e.keys()).collect::<BTreeSet<_>>() {
                match (a.get(key), e.get(key)) {
                    (Some(a), Some(e)) => diff(a, e, &format!("{path}.{key}"), out),
                    (a, e) => out.push(format!(
                        "{path}.{key}: actual presence={} expected presence={}",
                        a.is_some(),
                        e.is_some()
                    )),
                }
            }
        }
        (Value::Array(a), Value::Array(e)) => {
            if a.len() != e.len() {
                out.push(format!(
                    "{path}.length: actual={} expected={}",
                    a.len(),
                    e.len()
                ));
            }
            for i in 0..a.len().max(e.len()) {
                diff(
                    a.get(i).unwrap_or(&Value::Null),
                    e.get(i).unwrap_or(&Value::Null),
                    &format!("{path}[{i}]"),
                    out,
                );
            }
        }
        _ => out.push(format!("{path}: actual={a} expected={e}")),
    }
}
#[tokio::test]
async fn ws11_next3_array_shapes_match_rails_full_state() {
    let gold: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_array_shapes.json")).unwrap();
    let manifest = &gold["manifest"];
    let fixture = gold["fixture"].as_str().unwrap();
    let tables = manifest["projection_tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    let mut differences = Vec::new();
    let mut counts = BTreeMap::<String, Vec<(u64, u64)>>::new();
    for case in manifest["cases"].as_array().unwrap() {
        let actual = run(case, manifest, fixture, &tables).await;
        let name = case["name"].as_str().unwrap();
        let want = gold["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == case["name"])
            .unwrap();
        let before = &gold["baselines"][want["before"].as_u64().unwrap() as usize];
        diff(
            &actual["before"],
            before,
            &format!("{name}.before"),
            &mut differences,
        );
        diff(
            &actual["json_columns"],
            &gold["json_columns"],
            &format!("{name}.json_columns"),
            &mut differences,
        );
        for (i, step) in actual["steps"].as_array().unwrap().iter().enumerate() {
            let expected = &want["steps"][i];
            for field in [
                "attempt",
                "status",
                "response_body",
                "response_headers",
                "jobs",
                "changed_tables",
            ] {
                diff(
                    &step[field],
                    &expected[field],
                    &format!("{name}.steps[{i}].{field}"),
                    &mut differences,
                );
            }
            let mut state = before.clone();
            for (table, rows) in expected["changed_rows"].as_object().unwrap() {
                state[table] = rows.clone();
            }
            diff(
                &step["state"],
                &state,
                &format!("{name}.steps[{i}].state"),
                &mut differences,
            );
            assert_eq!(step["read_connection_autocommit"], true);
            assert_eq!(step["raw_job_rows"].as_array().unwrap().len(), 0);
        }
        if let Some(label) = case["count_label"].as_str() {
            counts.entry(label.into()).or_default().push((
                case["candidate_count"].as_u64().unwrap(),
                actual["steps"][0]["selects"].as_u64().unwrap(),
            ));
        }
    }
    for (label, values) in counts {
        println!("WS11 next3 array counts: {label}: {values:?}");
        assert_eq!(values.len(), 2);
        assert_eq!(values[0].1, values[1].1, "{label}: query growth");
    }
    for difference in differences.iter().take(100) {
        println!("WS11 shape difference: {difference}");
    }
    println!(
        "WS11 next3 array shapes: {} cases; {} strict differences",
        manifest["cases"].as_array().unwrap().len(),
        differences.len()
    );
    assert!(
        differences.is_empty(),
        "shared Rails array coercion/state matrix"
    );
}
