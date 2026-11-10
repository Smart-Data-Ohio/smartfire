//! Pinned Rails request/flash/audit/identity observations for complete Google controller cases.
use super::*;
use crate::controllers::presenters::test_support::{
    Reply, spa_auth_locations, with_fixed_render_secrets,
};
use campfire_kit::{Crypto, FrozenClock, RailsCrypto};
use std::time::Duration;
fn session(a: &TestApp, b: &Browser<'_>) -> Value {
    let raw = b.cookie_header().split(';').find_map(|s| {
        s.trim()
            .strip_prefix("_campfire_session=")
            .map(str::to_owned)
    });
    raw.and_then(|raw| {
        RailsCrypto::new(a.booted.app.secrets.clone()).decrypt_cookie(
            "_campfire_session",
            &rails_compat::cookies::unescape(&raw),
            a.booted.app.clock.now(),
        )
    })
    .unwrap_or(json!({}))
}
/// SPA cutover: what the retained sign-in page showed, read from the SPA sign-in page's signed-out
/// boot instead: its status, whether it offers Google, and the domains it names (as the Rails
/// page's sentence, `to_sentence`).
fn sign_in_page(page: &Reply) -> Value {
    let boot = crate::controllers::spa::signed_out_contracts_tests::inline_boot(page);
    let methods = &boot["signInMethods"];
    let domains = methods["googleDomains"]
        .as_array()
        .unwrap()
        .iter()
        .map(|domain| format!("@{}", domain.as_str().unwrap()))
        .collect::<Vec<_>>();
    let sentence = match domains.as_slice() {
        _ if methods["google"] != true => None,
        [] => None,
        [one] => Some(one.clone()),
        [first, second] => Some(format!("{first} and {second}")),
        [rest @ .., last] => Some(format!("{}, and {last}", rest.join(", "))),
    };
    json!({"status":page.status.as_u16(),"google_mark":methods["google"],"domain_sentence":sentence})
}
/// SPA cutover: the SPA signs in with `POST /api/v1/session` and goes where the JSON reply says
/// (the challenge page when a second factor is due); Rails' form post redirected there. This
/// records that navigation as the redirect it replaced.
async fn spa_password_sign_in(b: &mut Browser<'_>, email: &str, password: &str) -> Reply {
    let mut reply = b.password_sign_in(email, password).await;
    if reply.status == StatusCode::OK {
        let body = reply.json();
        let location = match body["kind"].as_str() {
            Some("signedIn") => body["location"].as_str().unwrap().to_owned(),
            Some("secondFactorRequired") => "http://campfire.test/app/two_factor/challenge".into(),
            _ => panic!("sign-in reply: {body}"),
        };
        reply.status = StatusCode::FOUND;
        reply.headers.insert("location", location.parse().unwrap());
    }
    reply
}
/// SPA cutover: a JSON write's feedback (its error message, or the notice of a change) in the
/// observation's `flash`, where Rails' form post left it in the flash for the page it redirected to.
fn spa_json_feedback(observed: &mut Value, reply: &Reply) {
    let body: Value = serde_json::from_slice(&reply.body).unwrap_or(Value::Null);
    // The settings API nests its refusal under `error`; the sudo API's is a top-level `message`.
    let refusal = body["error"]["message"]
        .as_str()
        .or(body["message"].as_str());
    if let Some(alert) = refusal.filter(|_| reply.status == StatusCode::UNPROCESSABLE_ENTITY) {
        observed["flash"] = json!({"alert":alert});
    } else if let Some(notice) = body["notice"].as_str() {
        observed["flash"] = json!({"notice":notice});
    }
}
/// SPA cutover: the pinned Rails redirect of a form post that the SPA now sends as JSON, as that
/// JSON reply: a refusal is a 422 and a change a 200 (both stay on the SPA page, so no location),
/// and a signed-out request a 401.
fn spa_json_write(expected: &mut Value) {
    if expected["status"] != 302 {
        return;
    }
    expected["status"] = if expected["flash"].get("alert").is_some() {
        json!(422)
    } else if expected["flash"].get("notice").is_some() {
        json!(200)
    } else {
        json!(401)
    };
    expected["location"] = Value::Null;
}
async fn observation(
    a: &TestApp,
    b: &Browser<'_>,
    reply: &crate::controllers::presenters::test_support::Reply,
    before: &[i64],
    r: &Recorded,
    call_before: usize,
    password: &Option<String>,
) -> Value {
    let cookie = session(a, b);
    let markers = ["two_factor_reauthenticated_at", "sudo_verified_at"]
        .into_iter()
        .filter_map(|key| cookie.get(key).map(|v| (key.into(), v.clone())))
        .collect::<serde_json::Map<_, _>>();
    let password = password.clone();
    let mut state=a.db().read(move |c| {
        let identities=c.prepare("SELECT user_id,subject,email,domain FROM google_identities ORDER BY user_id")?.query_map([],|r|Ok(json!({"user_id":r.get::<_,i64>(0)?,"subject":r.get::<_,String>(1)?,"email":r.get::<_,String>(2)?,"domain":r.get::<_,String>(3)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let calendar_users=c.prepare("SELECT user_id FROM google_accounts ORDER BY user_id")?.query_map([],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let audits=c.prepare("SELECT action,actor_id,target_id,target_type,target_label,details FROM audit_logs ORDER BY id")?.query_map([],|r|Ok(json!({"action":r.get::<_,String>(0)?,"actor_id":r.get::<_,Option<i64>>(1)?,"target_id":r.get::<_,Option<i64>>(2)?,"target_type":r.get::<_,Option<String>>(3)?,"target_label":r.get::<_,Option<String>>(4)?,"details":r.get::<_,Value>(5)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let user=campfire_db::User::find(c,KEVIN)?;
        let device_count:i64=c.query_row("SELECT count(*) FROM two_factor_remembered_devices WHERE user_id=?",[DAVID],|r|r.get(0))?;
        let backup_count:i64=c.query_row("SELECT count(*) FROM two_factor_backup_codes b JOIN two_factor_credentials c ON c.id=b.two_factor_credential_id WHERE c.user_id=? AND b.used_at IS NULL",[DAVID],|r|r.get(0))?;
        let credential_enabled=campfire_db::TwoFactorCredential::for_user(c,DAVID)?.is_some_and(|c|c.enabled());
        Ok(json!({"calendar_users":calendar_users,"identities":identities,"audits":audits,"email":user.email_address,"password_preserved":user.password_digest==password,"device_count":device_count,"backup_count":backup_count,"credential_enabled":credential_enabled}))
    }).await.unwrap();
    let counts = a
        .db()
        .read(|c| {
            ["users", "sessions", "google_identities", "google_accounts"]
                .into_iter()
                .map(|t| {
                    Ok(c.query_row(&format!("SELECT count(*) FROM {t}"), [], |r| {
                        r.get::<_, i64>(0)
                    })?)
                })
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .unwrap();
    state["status"] = json!(reply.status.as_u16());
    state["location"] = json!(reply.location());
    state["markers"] = json!(markers);
    state["flow_present"] = json!(
        cookie
            .get("google_sign_in_request")
            .is_some_and(|v| v.is_object())
    );
    state["flash"] = if reply.status.is_redirection() {
        cookie
            .get("flash")
            .and_then(|f| f.get("flashes"))
            .cloned()
            .unwrap_or(json!({}))
    } else {
        json!({})
    };
    state["delta"] = json!(
        counts
            .iter()
            .zip(before)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>()
    );
    state["calls"] = json!(
        r.calls.lock().unwrap()[call_before..]
            .iter()
            .map(|(_, target, _)| if target.ends_with("/token") {
                "token"
            } else {
                "keys"
            })
            .collect::<Vec<_>>()
    );
    state
}
async fn lifecycle_observation(a: &TestApp, history: &[(i64, i64, i64)]) -> Value {
    let history = history.to_vec();
    a.db().read(move |c| {
        use rusqlite::OptionalExtension;
        let id=c.query_row("SELECT user_id FROM google_identities WHERE subject='controller-member'",[],|r|r.get::<_,i64>(0)).optional()?.or(c.query_row("SELECT id FROM users WHERE email_address IN ('new-member@smartdata.net','new-member@cnbssoftware.com')",[],|r|r.get::<_,i64>(0)).optional()?).unwrap_or(KEVIN);
        let user=campfire_db::User::find(c,id)?;
        let markers:(bool,bool)=c.query_row("SELECT email_self_changed_at IS NOT NULL,google_email_link_allowed FROM users WHERE id=?",[id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        let memberships=c.prepare("SELECT room_id FROM memberships WHERE user_id=? ORDER BY room_id")?.query_map([id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let actual=c.prepare("SELECT id,room_id,creator_id FROM messages WHERE creator_id=? ORDER BY id")?.query_map([KEVIN],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?.collect::<rusqlite::Result<Vec<(i64,i64,i64)>>>()?;
        Ok(json!({"user":{"id":id,"name":user.name,"email_address":user.email_address,"role":user.role.name(),"status":user.status.name()},"password_present":user.password_digest.is_some(),"email_self_changed":markers.0,"google_email_link_allowed":markers.1,"memberships":memberships,"history_preserved":actual==history}))
    }).await.unwrap()
}
#[tokio::test]
async fn google_controller_cases_match_complete_pinned_rails_observations() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_controller_cases.json"
    ))
    .unwrap();
    run_cases(oracle).await;
}
#[tokio::test]
async fn google_rejected_identity_preserves_saved_destination_for_password_fallback() {
    let mut oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_controller_cases.json"
    ))
    .unwrap();
    oracle["rows"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["spec"]["scenario"] == "admin_link_required_return_to");
    assert_eq!(oracle["rows"].as_array().unwrap().len(), 1);
    run_cases(oracle).await;
}
#[tokio::test]
async fn google_unconfigured_callback_matches_pinned_rails_empty_not_found() {
    let mut oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_controller_cases.json"
    ))
    .unwrap();
    oracle["rows"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["spec"]["scenario"] == "unconfigured_callback");
    assert_eq!(oracle["rows"].as_array().unwrap().len(), 1);
    run_cases(oracle).await;
}
#[tokio::test]
async fn google_callback_rotation_and_predecessor_match_pinned_rails() {
    let mut oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_controller_cases.json"
    ))
    .unwrap();
    oracle["rows"].as_array_mut().unwrap().retain(|row| {
        matches!(
            row["spec"]["scenario"].as_str(),
            Some("rotation" | "predecessor")
        )
    });
    assert_eq!(oracle["rows"].as_array().unwrap().len(), 2);
    run_cases(oracle).await;
}
#[tokio::test]
async fn google_only_sudo_gated_requests_match_pinned_rails_prompt_confirmation_and_replay() {
    let mut oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_controller_cases.json"
    ))
    .unwrap();
    oracle["rows"]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["spec"]["continuation"] == true);
    assert_eq!(oracle["rows"].as_array().unwrap().len(), 4);
    run_cases(oracle).await;
}
async fn run_cases(oracle: Value) {
    assert_eq!(
        json!(
            crate::security::parameter_filter().filter(&json!({"code":"private-code-fixture"}))["code"]
        ),
        oracle["filtered_code"]
    );
    for original in oracle["rows"].as_array().unwrap() {
        let mut row = original.clone();
        let purpose = row["spec"]["purpose"].as_str().unwrap().to_owned();
        if row["result"]["location"] == "http://campfire.test/"
            && purpose == "sign_in"
            && row["result"]["delta"][1].as_i64().unwrap_or(0) > 0
        {
            row["result"]["location"] = json!("http://campfire.test/app/");
        } else if row["result"]["location"] == "http://campfire.test/users/me/profile" {
            row["result"]["location"] = json!(if purpose == "reauth" {
                "http://campfire.test/app/settings/security"
            } else {
                "http://campfire.test/app/settings/integrations"
            });
        }
        if row["spec"]["continuation"] == true {
            row["result"].as_object_mut().unwrap().remove("same_member");
            // SPA cutover: the retained sudo prompt page (`start.prompt`) is the SPA's; its JSON
            // is checked below.
            row["start"].as_object_mut().unwrap().remove("prompt");
        }
        if row["result"]["password_login"]["location"] == "http://campfire.test/account/edit" {
            row["result"]["password_login"]["location"] = json!("http://campfire.test/app/admin");
        }
        // SPA cutover: the retained sign-in, two-step and sudo pages the rows name are the SPA's.
        spa_auth_locations(&mut row);
        if let Some(protected) = row["result"]
            .get_mut("protected")
            .and_then(Value::as_array_mut)
        {
            // SPA cutover: the profile's two-step changes are the SPA settings' JSON writes.
            protected.iter_mut().for_each(spa_json_write);
        }
        if purpose == "sudo" {
            // SPA cutover: the SPA's sudo page starts Google with JSON. One Google can't serve is
            // refused in place (422) rather than redirected to the retained page with the alert;
            // a callback that finds its flow returns to the SPA's continuation with the feedback
            // the retained page showed (or, for an expired flow, that it expired) where Rails
            // replayed the held form, went home or back to the retained sudo page.
            let scenario = row["spec"]["scenario"].as_str().unwrap().to_owned();
            if row["start"].get("bound_entropy").is_none() {
                row["start"]["status"] = json!(422);
                spa_json_write(&mut row["result"]);
            } else if !matches!(scenario.as_str(), "signed_out" | "other_member") {
                let result = row["result"].as_object_mut().unwrap();
                result.remove("body");
                result.insert("status".into(), json!(302));
                result.insert(
                    "location".into(),
                    json!("http://campfire.test/app/sudo/continue"),
                );
                if scenario == "expired_flow" {
                    result.insert(
                        "flash".into(),
                        json!({"alert":"Confirmation expired. Try again."}),
                    );
                }
            }
        }
        // SPA cutover: the sign-in page a refused callback lands on is the SPA's, checked below
        // by its boot's alert; the pinned retained page (`follow`) is not served any more.
        row["result"].as_object_mut().unwrap().remove("follow");
        let now = jiff::Timestamp::from_second(oracle["now"].as_i64().unwrap()).unwrap();
        let clock = Arc::new(FrozenClock::new(now));
        let mut a = TestApp::boot_with_clock(clock.clone()).await.unwrap();
        a.booted.jobs.stop(Duration::from_secs(5)).await;
        let r = Arc::new(Recorded {
            response: Mutex::new(Err(())),
            calls: Mutex::new(vec![]),
            certs: Mutex::new(None),
        });
        let purpose = row["spec"]["purpose"].as_str().unwrap();
        let scenario = row["spec"]["scenario"].as_str().unwrap();
        let lifecycle = row["spec"]["lifecycle"].as_bool().unwrap_or(false);
        let continuation = row["spec"]["continuation"].as_bool().unwrap_or(false);
        a.booted.app.google.install(SignIn::with_client(
            config(if scenario == "provision_org" {
                &["example.org"]
            } else if matches!(
                scenario,
                "unconfigured" | "unconfigured_password" | "unconfigured_callback"
            ) {
                &[]
            } else {
                &["smartdata.net", "cnbssoftware.com"]
            }),
            r.clone(),
        ));
        let actor = row["actor_id"].as_i64().unwrap();
        let jz = row["jz_id"].as_i64().unwrap();
        let identity = if matches!(purpose, "reauth" | "sudo") && scenario != "unlinked" {
            Some((actor, "controller-member", "david@smartdata.net"))
        } else if scenario == "already_linked" {
            Some((actor, "controller-old", "kevin@smartdata.net"))
        } else if scenario == "subject_taken" {
            Some((jz, "controller-member", "jz@smartdata.net"))
        } else {
            None
        };
        let owned_scenario = scenario.to_owned();
        let password = a
            .db()
            .write(move |tx| {
                tx.conn()
                    .execute_batch("DELETE FROM google_identities;DELETE FROM audit_logs;")?;
                if lifecycle {
                    let scenario = owned_scenario.as_str();
                    tx.conn().execute("UPDATE users SET email_address='legacy@smartdata.net',role=1,google_email_link_allowed=1,email_self_changed_at=NULL WHERE id=?",[KEVIN])?;
                    if scenario=="external_password" {tx.conn().execute("UPDATE users SET email_address='legacy@external.test' WHERE id=?",[KEVIN])?;}
                    if scenario=="predecessor" {tx.conn().execute("UPDATE users SET status=1,email_address='legacy-deactivated-fixture@smartdata.net' WHERE id=?",[KEVIN])?;}
                    if scenario=="bot" {tx.conn().execute("UPDATE users SET role=2 WHERE id=?",[KEVIN])?;}
                    if matches!(scenario,"self_changed"|"admin_allowed") {tx.conn().execute("UPDATE users SET google_email_link_allowed=0,email_self_changed_at=? WHERE id=?",rusqlite::params![tx.now(),KEVIN])?;}
                    if matches!(scenario,"linking_disabled"|"admin_link_required_return_to") {tx.conn().execute("UPDATE users SET google_email_link_allowed=0 WHERE id=?",[KEVIN])?;}
                    if scenario=="admin_link_required_return_to" {tx.conn().execute("UPDATE two_factor_credentials SET confirmed_at=NULL WHERE user_id=?",[KEVIN])?;}
                    if scenario=="admin_allowed" {campfire_db::User::find(tx.conn(),KEVIN)?.update(tx,campfire_db::UserChanges{allow_google_email_link:true,..Default::default()})?;}
                    if matches!(scenario,"immutable_email"|"retained_deactivated"|"retained_banned"|"different_subject") {
                        campfire_db::models::google_identity::GoogleIdentity::link_to_user(tx,json!({"sub":if scenario=="different_subject"{"controller-old"}else{"controller-member"},"email":"legacy@smartdata.net","hd":"smartdata.net"}).as_object().unwrap(),KEVIN)?;
                    }
                    if matches!(scenario,"deactivated"|"retained_deactivated") {tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[KEVIN])?;}
                    if matches!(scenario,"banned"|"retained_banned") {tx.conn().execute("UPDATE users SET status=2 WHERE id=?",[KEVIN])?;}
                    tx.conn().execute_batch("UPDATE sqlite_sequence SET seq=9000000000 WHERE name='users'")?;
                }
                if continuation { tx.conn().execute("UPDATE users SET password_digest=NULL WHERE id=?", [actor])?; }
                if let Some((id, subject, email)) = identity {
                    campfire_db::models::google_identity::GoogleIdentity::link_to_user(
                        tx,
                        json!({"sub":subject,"email":email,"hd":"smartdata.net"})
                            .as_object()
                            .unwrap(),
                        id,
                    )?;
                }
                Ok(campfire_db::User::find(tx.conn(), KEVIN)?.password_digest)
            })
            .await
            .unwrap();
        let history = a
            .db()
            .read(|c| {
                Ok(c.prepare(
                    "SELECT id,room_id,creator_id FROM messages WHERE creator_id=? ORDER BY id",
                )?
                .query_map([KEVIN], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<rusqlite::Result<Vec<(i64, i64, i64)>>>()?)
            })
            .await
            .unwrap();
        if lifecycle {
            assert!(!history.is_empty(), "missing authored history fixture");
        }
        let mut b = if purpose == "sign_in" || scenario == "anonymous" {
            a.anonymous()
        } else {
            a.sign_in(actor).await
        };
        if scenario == "join_signup" {
            let code = a
                .db()
                .read(|conn| {
                    Ok(
                        conn.query_row("SELECT join_code FROM accounts LIMIT 1", [], |row| {
                            row.get::<_, String>(0)
                        })?,
                    )
                })
                .await
                .unwrap();
            // SPA cutover: the SPA's join page signs up through the JSON API.
            let joined = b
                .write(
                    Req::new(Method::POST, &format!("/api/v1/join/{code}")).json_body(&json!({
                        "name": "Pre-claimer",
                        "emailAddress": "newhire@smartdata.net",
                        "password": "secret123456",
                    })),
                )
                .await;
            assert!(joined.status.is_success(), "{}", joined.text());
            b.write(Req::new(Method::DELETE, "/session")).await;
        }
        if scenario == "calendar_only" {
            crate::app::google_api_tests::grant(
                &a,
                DAVID,
                campfire_db::Timestamp::from_jiff(now).since(jiff::SignedDuration::from_hours(1)),
                false,
            )
            .await;
        }
        if scenario == "admin_link_required_return_to" {
            b.get("/account/edit").await;
        }
        let initial_page = if purpose == "sign_in" {
            // SPA cutover: the retained sign-in page goes to the SPA's.
            assert_eq!(
                b.get("/session/new").await.location(),
                Some("http://campfire.test/app/session/new")
            );
            b.get("/app/session/new").await
        } else {
            b.get("/users/me/profile").await
        };
        let device = if matches!(scenario, "consume_device" | "consume_all_devices") {
            Some(
                a.db()
                    .write(move |tx| {
                        Ok(campfire_db::TwoFactorRememberedDevice::create_for(
                            tx,
                            actor,
                            Some("Controller fixture"),
                            Some("127.0.0.1"),
                        )?
                        .0
                        .id)
                    })
                    .await
                    .unwrap(),
            )
        } else {
            None
        };
        let before = a
            .db()
            .read(|c| {
                ["users", "sessions", "google_identities", "google_accounts"]
                    .into_iter()
                    .map(|t| {
                        Ok(c.query_row(&format!("SELECT count(*) FROM {t}"), [], |r| {
                            r.get::<_, i64>(0)
                        })?)
                    })
                    .collect::<campfire_db::Result<Vec<_>>>()
            })
            .await
            .unwrap();
        let path = match purpose {
            "reauth" => "/two_factor_reauthentication",
            // SPA cutover: the SPA's sudo page starts Google confirmation with JSON.
            "sudo" => "/api/v1/sudo/google",
            "link" => "/user/profile/google_sign_in_link",
            _ => "/session/google",
        };
        let mut gate = None;
        if continuation {
            let reply = b.write(Req::new(Method::DELETE, "/fizzy/connection")).await;
            gate = Some(json!({"status":reply.status.as_u16(),"location":reply.location()}));
            // The SPA's sudo page offers Google and no password, as the pinned Rails page did.
            assert_eq!(
                b.get("/sudo/new").await.location(),
                Some("http://campfire.test/app/sudo/new")
            );
            let ready = b
                .send(Req::new(Method::GET, "/api/v1/sudo").header("accept", "application/json"))
                .await;
            assert_eq!(ready.status, StatusCode::OK, "{}", ready.text());
            let methods = ready.json()["reauthentication"]["methods"].clone();
            assert!(
                methods.as_array().unwrap().contains(&json!("google")),
                "{methods}"
            );
            assert!(
                !methods.as_array().unwrap().contains(&json!("password")),
                "{methods}"
            );
        }
        let mut start_reply = if purpose == "sudo" {
            b.write(Req::new(Method::POST, path).header("accept", "application/json"))
                .await
        } else {
            b.write(Req::new(Method::POST, path)).await
        };
        if purpose == "sudo" && start_reply.status == StatusCode::OK {
            // SPA cutover: the SPA navigates to the authorization URL its JSON names, where the
            // retained form post was redirected there.
            let location = start_reply.json()["location"].as_str().unwrap().to_owned();
            start_reply.status = StatusCode::FOUND;
            start_reply
                .headers
                .insert("location", location.parse().unwrap());
        }
        let mut started = json!({"status":start_reply.status.as_u16()});
        if scenario == "unconfigured_callback" {
            started["body"] = json!(start_reply.text());
        }
        if let Some(gate) = gate {
            started["gate"] = gate;
        }
        if purpose == "sign_in" {
            started["page"] = sign_in_page(&initial_page);
        }
        let q = start_reply
            .location()
            .and_then(|l| url::Url::parse(l).ok())
            .map(|u| {
                u.query_pairs()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect::<std::collections::BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        let actual = if q.contains_key("state") {
            let flow = session(&a, &b)["google_sign_in_request"].clone();
            use sha2::Digest;
            started["bound_entropy"] = json!({
                "nonce": q["nonce"] == flow["nonce"].as_str().unwrap() && q["nonce"].len() == 32,
                "state": rails_compat::app_verifier(&a.booted.app.secrets, "google_sign_in_state").verify(&q["state"], None, a.booted.app.clock.now()).ok() == Some(flow["state"].clone()),
                "pkce": URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(flow["verifier"].as_str().unwrap().as_bytes())) == q["code_challenge"],
            });
            started["authorize"] = json!(
                q.iter()
                    .filter(|(k, _)| matches!(
                        k.as_str(),
                        "scope" | "code_challenge_method" | "prompt" | "max_age" | "redirect_uri"
                    ))
                    .collect::<std::collections::BTreeMap<_, _>>()
            );
            if matches!(scenario, "signed_out" | "other_member") {
                b.write(Req::new(Method::DELETE, "/session")).await;
                if scenario == "other_member" {
                    b = a.sign_in(jz).await;
                }
                b.get(if scenario == "other_member" {
                    "/users/me/profile"
                } else {
                    "/app/session/new"
                })
                .await;
            }
            if scenario == "expired_flow" {
                clock.advance(jiff::SignedDuration::from_secs(601));
            }
            if scenario == "policy_changed" {
                a.booted.app.google.install(SignIn::with_client(
                    config(&["cnbssoftware.com"]),
                    r.clone(),
                ));
            }
            let email = if lifecycle {
                match scenario {
                    "provision" | "provision_self_change" => "new-member@smartdata.net",
                    "join_signup" => "newhire@smartdata.net",
                    "calendar_only" => "david@smartdata.net",
                    "provision_secondary" | "provision_secondary_hosted" => {
                        "new-member@cnbssoftware.com"
                    }
                    "provision_org" => "new-member@example.org",
                    "external_password" => "legacy@external.test",
                    "immutable_email" => "changed@smartdata.net",
                    _ => "LEGACY@smartdata.net",
                }
            } else if purpose == "link" {
                "kevin.w@smartdata.net"
            } else {
                "david@smartdata.net"
            };
            let mut claims = claims(
                &a,
                &q,
                if scenario == "wrong_subject" {
                    "controller-attacker"
                } else {
                    "controller-member"
                },
                email,
            );
            claims["name"] = json!("Fixture member");
            if scenario == "provision_secondary" {
                claims["hd"] = json!("cnbssoftware.com");
            }
            if scenario == "provision_org" {
                claims["hd"] = json!("example.org");
            }
            if scenario == "external_password" {
                claims["hd"] = json!("external.test");
            }
            claims["exp"] = json!(now.as_second() + 3600);
            claims["auth_time"] = json!(
                now.as_second()
                    + match scenario {
                        "stale" => -360,
                        "before_auth_boundary" => -331,
                        "at_auth_boundary" => -330,
                        "after_auth_boundary" => -329,
                        _ => 0,
                    }
            );
            if scenario == "missing_auth" {
                claims.as_object_mut().unwrap().remove("auth_time");
            }
            if scenario == "wrong_nonce" {
                claims["nonce"] = json!("forged");
            }
            if scenario == "wrong_domain" {
                claims["hd"] = json!("wrong.test");
            }
            *r.response.lock().unwrap() = Ok((200, serde_json::to_vec(&json!({"access_token":"signin-access-token","refresh_token":"signin-refresh-token","id_token":token(claims.clone())})).unwrap()));
            if scenario == "rotation" {
                a.booted
                    .app
                    .google
                    .sign_in()
                    .verify(
                        &token(claims.clone()),
                        &q["nonce"],
                        "sign_in",
                        now.as_second(),
                    )
                    .await
                    .unwrap();
                *r.certs.lock().unwrap() = Some(Ok((
                    200,
                    serde_json::to_vec(&oracle["rotated_jwks"]).unwrap(),
                )));
                *r.response.lock().unwrap() = Ok((200, serde_json::to_vec(&json!({"access_token":"signin-access-token","refresh_token":"signin-refresh-token","id_token":token_with_key(claims, "rotated", include_bytes!("../../integrations/google/rotated-signing.der"))})).unwrap()));
            }
            if scenario == "token_shape" {
                *r.response.lock().unwrap() =
                    Ok((200, serde_json::to_vec(&row["spec"]["payload"]).unwrap()));
            }
            if scenario == "key_shape" {
                *r.certs.lock().unwrap() = Some(Ok((
                    200,
                    serde_json::to_vec(&row["spec"]["payload"]).unwrap(),
                )));
            }
            if scenario == "unavailable" {
                *r.response.lock().unwrap() = Err(());
            }
            let callback_path = format!(
                "/session/google/callback?state={}&code=fixture-code{}",
                crate::controllers::presenters::test_support::encode(
                    if scenario == "forged_state" {
                        "forged"
                    } else if scenario == "missing_state" {
                        ""
                    } else {
                        &q["state"]
                    }
                ),
                if scenario == "cancelled" {
                    "&error=access_denied"
                } else {
                    ""
                }
            );
            let mut cb = with_fixed_render_secrets(b.get(&callback_path)).await;
            if scenario == "provision_self_change" {
                let id=a.db().write(|tx| {
                    let id=campfire_db::models::google_identity::GoogleIdentity::for_subject(tx.conn(),"controller-member")?.unwrap().user_id;
                    tx.conn().execute("UPDATE sessions SET two_factor_verified_at=? WHERE id=(SELECT max(id) FROM sessions WHERE user_id=?)",rusqlite::params![tx.now(),id])?;
                    Ok(id)
                }).await.unwrap();
                let response = b
                    .write(
                        Req::new(Method::PUT, "/users/me/profile")
                            .form(&[("user[email_address]", "changed@smartdata.net")]),
                    )
                    .await;
                assert_eq!(response.status, StatusCode::FOUND);
                assert!(
                    a.db()
                        .read(move |conn| Ok(conn.query_row(
                            "SELECT email_self_changed_at IS NOT NULL FROM users WHERE id=?",
                            [id],
                            |r| r.get::<_, bool>(0)
                        )?))
                        .await
                        .unwrap()
                );
                b.write(Req::new(Method::DELETE, "/session")).await;
                b.get("/app/session/new").await;
                let next = start(&mut b, "/session/google").await;
                let mut next_claims =
                    super::claims(&a, &next, "controller-member", "new-member@smartdata.net");
                next_claims["name"] = json!("Fixture member");
                answer(&r, next_claims);
                cb = callback(&mut b, &next["state"]).await;
            }
            let mut observed = observation(&a, &b, &cb, &before, &r, 0, &password).await;
            if matches!(scenario, "token_shape" | "key_shape") {
                let reply = b.get(cb.location().unwrap()).await;
                assert_eq!(reply.status, StatusCode::OK, "{scenario}");
                assert_eq!(
                    crate::controllers::spa::signed_out_contracts_tests::inline_boot(&reply)["flash"],
                    json!({"kind":"alert","message":row["result"]["flash"]["alert"]}),
                    "{scenario}"
                );
            }
            if continuation {
                // SPA cutover: the SPA's continuation names the held write for its page to send
                // again, where Rails' replay page (`result.body`) re-posted it.
                let resumed = b
                    .send(
                        Req::new(Method::GET, "/api/v1/sudo/continue")
                            .header("accept", "application/json"),
                    )
                    .await;
                if original["result"]["status"] == 200 {
                    assert_eq!(resumed.status, StatusCode::OK, "{}", resumed.text());
                    assert_eq!(
                        resumed.json()["retry"],
                        json!({"method":"DELETE","path":"/fizzy/connection","returnTo":"/app/"})
                    );
                } else {
                    assert_eq!(resumed.status, StatusCode::FORBIDDEN, "{}", resumed.text());
                }
                let calls = r.calls.lock().unwrap().len();
                let reply = b.write(Req::new(Method::DELETE, "/fizzy/connection")).await;
                observed["protected"] =
                    observation(&a, &b, &reply, &before, &r, calls, &password).await;
                let reply = b.get("/api/v1/settings").await;
                assert_eq!(reply.status, StatusCode::OK);
                let settings: Value = serde_json::from_str(&reply.text()).unwrap();
                assert_eq!(settings["profile"]["emailAddress"], "david@37signals.com");
            }
            if lifecycle {
                observed["lifecycle"] = lifecycle_observation(&a, &history).await;
                if scenario == "admin_link_required_return_to" {
                    observed["return_to"] =
                        session(&a, &b)["return_to_after_authenticating"].clone();
                    b.get(cb.location().unwrap()).await;
                    let calls = r.calls.lock().unwrap().len();
                    let reply =
                        spa_password_sign_in(&mut b, "legacy@smartdata.net", "secret123456").await;
                    observed["password_login"] =
                        observation(&a, &b, &reply, &before, &r, calls, &password).await;
                    observed["password_login"]["return_to"] =
                        session(&a, &b)["return_to_after_authenticating"].clone();
                }
                if matches!(
                    scenario,
                    "legacy_password" | "provision" | "provision_secondary" | "external_password"
                ) {
                    b.get(cb.location().unwrap()).await;
                    b.write(Req::new(Method::DELETE, "/session")).await;
                    b.get("/app/session/new").await;
                    let call_before = r.calls.lock().unwrap().len();
                    let response = spa_password_sign_in(
                        &mut b,
                        if scenario == "legacy_password" {
                            "legacy@smartdata.net"
                        } else {
                            email
                        },
                        "secret123456",
                    )
                    .await;
                    let mut password_login =
                        observation(&a, &b, &response, &before, &r, call_before, &password).await;
                    password_login["lifecycle"] = lifecycle_observation(&a, &history).await;
                    observed["password_login"] = password_login;
                }
            }
            if purpose == "reauth" {
                // Follow the callback exactly as the real profile confirmation UI does.
                b.get(cb.location().unwrap()).await;
                clock.advance(jiff::SignedDuration::from_secs(match scenario {
                    "before_reauth_expiry" => 599,
                    "at_reauth_expiry" => 600,
                    "after_reauth_expiry" => 601,
                    _ => 0,
                }));
                // SPA cutover: the SPA's security settings make these changes through the JSON
                // API, which the retained profile's forms no longer reach.
                let settings = |method: Method, path: &str, reauth: &str| {
                    Req::new(method, &format!("/api/v1/settings/two_factor{path}"))
                        .json_body(&json!({ "reauth": reauth }))
                };
                let mut requests = match scenario {
                    "consume_device" => vec![settings(
                        Method::DELETE,
                        &format!("/devices/{}", device.unwrap()),
                        "",
                    )],
                    "consume_all_devices" => vec![settings(Method::DELETE, "/devices", "")],
                    "consume_disable" => vec![settings(Method::DELETE, "", "")],
                    "consume_backup" => vec![
                        settings(Method::POST, "/backup_codes", ""),
                        settings(Method::POST, "/backup_codes", ""),
                    ],
                    "wrong_credential_preserves" => vec![
                        settings(Method::POST, "/backup_codes", "wrong-credential"),
                        settings(Method::POST, "/backup_codes", ""),
                    ],
                    _ => vec![settings(Method::POST, "/backup_codes", "")],
                };
                let mut protected = Vec::new();
                for request in requests.drain(..) {
                    let call_before = r.calls.lock().unwrap().len();
                    let response = b.write(request).await;
                    let mut observed =
                        observation(&a, &b, &response, &before, &r, call_before, &password).await;
                    spa_json_feedback(&mut observed, &response);
                    protected.push(observed);
                }
                observed["protected"] = json!(protected);
            }
            if scenario == "replay" {
                let calls = r.calls.lock().unwrap().len();
                let replay = b.get(&callback_path).await;
                observed["replay"] =
                    observation(&a, &b, &replay, &before, &r, calls, &password).await;
            }
            observed
        } else {
            let mut observed = observation(&a, &b, &start_reply, &before, &r, 0, &password).await;
            spa_json_feedback(&mut observed, &start_reply);
            if scenario == "unconfigured_callback" {
                let reply = b
                    .get("/session/google/callback?state=fixture-state&code=fixture-code")
                    .await;
                observed = observation(&a, &b, &reply, &before, &r, 0, &password).await;
                observed["body"] = json!(reply.text());
                observed["content_type"] =
                    json!(reply.content_type().map(|v| v.split(';').next().unwrap()));
                assert_eq!(
                    reply.body,
                    row["result"]["body"].as_str().unwrap().as_bytes(),
                    "unconfigured callback must match the complete Rails body"
                );
            }
            if scenario == "unconfigured_password" {
                b.get("/app/session/new").await;
                let response =
                    spa_password_sign_in(&mut b, "kevin@37signals.com", "secret123456").await;
                observed["password_login"] =
                    observation(&a, &b, &response, &before, &r, 0, &password).await;
            }
            observed
        };
        let token_columns = a
            .db()
            .read(|c| {
                Ok(c.prepare("PRAGMA table_info(google_identities)")?
                    .query_map([], |r| r.get::<_, String>(1))?
                    .collect::<rusqlite::Result<Vec<_>>>()?
                    .into_iter()
                    .filter(|name| matches!(name.as_str(), "access_token" | "refresh_token"))
                    .collect::<Vec<_>>())
            })
            .await
            .unwrap();
        assert_eq!(json!(token_columns), oracle["identity_token_columns"]);
        assert_eq!(started, row["start"], "{}: authorize", row["spec"]);
        if actual != row["result"] {
            let path =
                std::env::temp_dir().join(format!("google-contract-{}.json", uuid::Uuid::new_v4()));
            std::fs::write(
                path,
                json!({"spec":row["spec"],"actual":actual,"expected":row["result"]}).to_string(),
            )
            .unwrap();
        }
        assert_eq!(
            actual, row["result"],
            "{}: complete request state",
            row["spec"]
        );
    }
    println!(
        "Pinned Rails Google controller observations: {} exercised; 0 skipped",
        oracle["rows"].as_array().unwrap().len()
    );
}
