//! Individual AuditLogsControllerTest assertions through the real router/domain.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::models::audit_log::{AuditLog, Context, NewAuditLog};
use campfire_db::{Room, User};
use campfire_richtext::dom::Dom;
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/audit_originals.json"
    ))
    .unwrap()
}
fn observation(reply: &Reply, name: &str) -> Value {
    if name == "formula" {
        // This pinned fixture has no commas in any field preceding target.
        let quoted = reply.text().lines().skip(1).any(|line| {
            line.split(',')
                .nth(4)
                .is_some_and(|value| value.starts_with("'="))
        });
        return json!({"status":reply.status.as_u16(),"quoted_formula_target":quoted});
    }
    if matches!(name, "csv" | "request_formula") {
        return json!({"status":reply.status.as_u16(),"media_type":reply.content_type().unwrap().split(';').next().unwrap(),"body":reply.text()});
    }
    if reply.status != StatusCode::OK {
        return json!({"status":reply.status.as_u16(),"location":reply.location()});
    }
    let body = reply.text();
    let mut dom = Dom::new();
    let root = dom.parse_fragment(&body).unwrap();
    let nodes = dom.descendants(root);
    let tbody = nodes
        .iter()
        .copied()
        .filter(|n| dom.name(*n) == "tbody")
        .flat_map(|n| dom.descendants(n))
        .collect::<Vec<_>>();
    json!({"status":reply.status.as_u16(),"location":reply.location(),
        "h1":nodes.iter().filter(|n|dom.name(**n)=="h1").map(|n|dom.text_content(*n)).collect::<Vec<_>>(),
        "codes":tbody.iter().filter(|n|dom.name(**n)=="td").flat_map(|n|dom.descendants(*n)).filter(|n|dom.name(*n)=="code").map(|n|dom.text_content(n)).collect::<Vec<_>>(),
        "cells":tbody.iter().filter(|n|dom.name(**n)=="td").map(|n|dom.text_content(*n)).collect::<Vec<_>>(),
        "rows":tbody.iter().filter(|n|dom.name(**n)=="tr").count(),
        "exports":nodes.iter().filter(|n|dom.name(**n)=="a" && dom.text_content(**n)=="Export CSV").map(|n|json!([dom.attr(*n,"href"),dom.text_content(*n)])).collect::<Vec<_>>(),
        "older":body.contains("Older entries"),"newest":body.contains("Newest entries")})
}
async fn run(name: &'static str) {
    let app = TestApp::boot_frozen()
        .await
        .expect("CI seed required")
        .without_job_runner()
        .await;
    let case = oracle()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
        .clone();
    app.db()
        .write(move |tx| {
            tx.conn().execute("DELETE FROM audit_logs", [])?;
            let admin = User::find(tx.conn(), DAVID)?;
            let member = User::find(tx.conn(), KEVIN)?;
            let ban = AuditLog::record(
                tx,
                NewAuditLog {
                    action: "user.ban".into(),
                    actor: Some((&admin).into()),
                    target: Some((&member).into()),
                    changes: Some(json!({"status":["active","banned"]})),
                    ip_address: Some("203.0.113.7".into()),
                    ..Default::default()
                },
                &Context::default(),
            )?;
            AuditLog::record(
                tx,
                NewAuditLog {
                    action: "user.role.change".into(),
                    actor: Some((&admin).into()),
                    target: Some((&member).into()),
                    changes: Some(json!({"role":["member","administrator"]})),
                    ..Default::default()
                },
                &Context::default(),
            )?;
            match name {
                "actor" => {
                    let jason = User::find(tx.conn(), JASON)?;
                    AuditLog::record(
                        tx,
                        NewAuditLog {
                            action: "user.ban".into(),
                            actor: Some((&jason).into()),
                            target: Some((&member).into()),
                            ..Default::default()
                        },
                        &Context::default(),
                    )?;
                }
                "action" => {
                    let room = Room::find(tx.conn(), ALL_TALK)?;
                    AuditLog::record(
                        tx,
                        NewAuditLog {
                            action: "room.create".into(),
                            actor: Some((&admin).into()),
                            target: Some((&room).into()),
                            ..Default::default()
                        },
                        &Context::default(),
                    )?;
                }
                "dates" => {
                    tx.conn().execute(
                        "UPDATE audit_logs SET created_at=? WHERE id=?",
                        rusqlite::params![
                            tx.now().ago(jiff::SignedDuration::from_hours(240)),
                            ban.id
                        ],
                    )?;
                }
                "paging" => {
                    for i in 0..60 {
                        AuditLog::record(
                            tx,
                            NewAuditLog {
                                action: "user.ban".into(),
                                actor: Some((&admin).into()),
                                target: Some((&member).into()),
                                changes: Some(json!({"n":i})),
                                ip_address: Some(format!("10.0.0.{}", i % 250 + 1)),
                                ..Default::default()
                            },
                            &Context::default(),
                        )?;
                    }
                }
                "formula" => {
                    tx.conn().execute(
                        "UPDATE users SET name=? WHERE id=?",
                        rusqlite::params!["=cmd|'/c calc'!A0", KEVIN],
                    )?;
                    let member = User::find(tx.conn(), KEVIN)?;
                    AuditLog::record(
                        tx,
                        NewAuditLog {
                            action: "user.ban".into(),
                            actor: Some((&admin).into()),
                            target: Some((&member).into()),
                            ..Default::default()
                        },
                        &Context::default(),
                    )?;
                }
                "request_formula" => {
                    AuditLog::record(
                        tx,
                        NewAuditLog {
                            action: "session.sign_in.failure".into(),
                            actor_label: Some("mallory@evil.example".into()),
                            ip_address: Some("198.51.100.9".into()),
                            user_agent: Some("=cmd|'/c calc'!A0".into()),
                            ..Default::default()
                        },
                        &Context::default(),
                    )?;
                }
                _ => {}
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut browser = if name.starts_with("visitor") {
        app.anonymous()
    } else if name.starts_with("member") {
        app.sign_in(KEVIN).await
    } else {
        app.david()
    };
    let csv = matches!(name, "csv" | "formula" | "request_formula");
    if csv {
        let sudo = browser
            .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
            .await;
        assert_eq!(sudo.status, StatusCode::FOUND);
    }
    for step in case["responses"].as_array().unwrap() {
        let reply = browser.send(Req::new(Method::GET, step["path"].as_str().unwrap()).header("x-requested-with", "XMLHttpRequest")).await;
        assert_eq!(
            observation(&reply, name),
            step["response"],
            "{name}: {}",
            step["path"]
        );
    }
}
macro_rules! cases {($($name:ident => $case:literal),+ $(,)?) => {$ (
    #[tokio::test] async fn $name() {run($case).await;}
)+};}
cases!(
    original_members_forbidden => "member",
    original_visitors_redirect_to_sign_in => "visitor",
    original_visitors_cannot_export => "visitor_csv",
    original_members_cannot_export => "member_csv",
    original_filtered_csv_headers_labels_changes_and_ip => "csv",
    original_formula_target_is_quoted => "formula",
    original_formula_request_column_is_quoted => "request_formula",
);
