//! Pinned Rails request/flash/audit/identity observations for complete Google controller cases.
use super::*;
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
        let audits=c.prepare("SELECT action,actor_id,target_id,target_type,target_label,details FROM audit_logs ORDER BY id")?.query_map([],|r|Ok(json!({"action":r.get::<_,String>(0)?,"actor_id":r.get::<_,Option<i64>>(1)?,"target_id":r.get::<_,Option<i64>>(2)?,"target_type":r.get::<_,Option<String>>(3)?,"target_label":r.get::<_,Option<String>>(4)?,"details":r.get::<_,Value>(5)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let user=campfire_db::User::find(c,KEVIN)?;
        let device_count:i64=c.query_row("SELECT count(*) FROM two_factor_remembered_devices WHERE user_id=?",[DAVID],|r|r.get(0))?;
        let backup_count:i64=c.query_row("SELECT count(*) FROM two_factor_backup_codes b JOIN two_factor_credentials c ON c.id=b.two_factor_credential_id WHERE c.user_id=? AND b.used_at IS NULL",[DAVID],|r|r.get(0))?;
        let credential_enabled=campfire_db::TwoFactorCredential::for_user(c,DAVID)?.is_some_and(|c|c.enabled());
        Ok(json!({"identities":identities,"audits":audits,"email":user.email_address,"password_preserved":user.password_digest==password,"device_count":device_count,"backup_count":backup_count,"credential_enabled":credential_enabled}))
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
        let memberships=c.prepare("SELECT room_id FROM memberships WHERE user_id=? ORDER BY room_id")?.query_map([id],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let actual=c.prepare("SELECT id,room_id,creator_id FROM messages WHERE creator_id=? ORDER BY id")?.query_map([KEVIN],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?.collect::<rusqlite::Result<Vec<(i64,i64,i64)>>>()?;
        Ok(json!({"user":{"id":id,"name":user.name,"email_address":user.email_address,"role":user.role.name(),"status":user.status.name()},"password_present":user.password_digest.is_some(),"memberships":memberships,"history_preserved":actual==history}))
    }).await.unwrap()
}
#[tokio::test]
async fn google_controller_cases_match_complete_pinned_rails_observations() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_controller_cases.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
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
        a.booted.app.google.install(SignIn::with_client(
            config(if scenario == "provision_org" {
                &["example.org"]
            } else if matches!(scenario, "unconfigured" | "unconfigured_password") {
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
                    if scenario=="bot" {tx.conn().execute("UPDATE users SET role=2 WHERE id=?",[KEVIN])?;}
                    if matches!(scenario,"self_changed"|"admin_allowed") {tx.conn().execute("UPDATE users SET google_email_link_allowed=0,email_self_changed_at=? WHERE id=?",rusqlite::params![tx.now(),KEVIN])?;}
                    if scenario=="linking_disabled" {tx.conn().execute("UPDATE users SET google_email_link_allowed=0 WHERE id=?",[KEVIN])?;}
                    if scenario=="admin_allowed" {campfire_db::User::find(tx.conn(),KEVIN)?.update(tx,campfire_db::UserChanges{allow_google_email_link:true,..Default::default()})?;}
                    if matches!(scenario,"immutable_email"|"retained_deactivated"|"retained_banned"|"different_subject") {
                        campfire_db::models::google_identity::GoogleIdentity::link_to_user(tx,json!({"sub":if scenario=="different_subject"{"controller-old"}else{"controller-member"},"email":"legacy@smartdata.net","hd":"smartdata.net"}).as_object().unwrap(),KEVIN)?;
                    }
                    if matches!(scenario,"deactivated"|"retained_deactivated") {tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[KEVIN])?;}
                    if matches!(scenario,"banned"|"retained_banned") {tx.conn().execute("UPDATE users SET status=2 WHERE id=?",[KEVIN])?;}
                    tx.conn().execute_batch("UPDATE sqlite_sequence SET seq=9000000000 WHERE name='users'")?;
                }
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
        let initial_page = b
            .get(if purpose == "sign_in" {
                "/session/new"
            } else {
                "/users/me/profile"
            })
            .await;
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
            "sudo" => "/sudo/google",
            "link" => "/user/profile/google_sign_in_link",
            _ => "/session/google",
        };
        let start_reply = b.write(Req::new(Method::POST, path)).await;
        let mut started = json!({"status":start_reply.status.as_u16()});
        if purpose == "sign_in" {
            let html = initial_page.text();
            let domains = regex::Regex::new("Google sign-in for (.*?) accounts").unwrap();
            started["page"] = json!({"status":initial_page.status.as_u16(), "google_mark":html.contains("Sign in with Google"), "domain_sentence":domains.captures(&html).map(|c|c[1].to_owned())});
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
                    "/session/new"
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
                    "provision" => "new-member@smartdata.net",
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
            answer(&r, claims);
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
            let cb = b.get(&callback_path).await;
            let mut observed = observation(&a, &b, &cb, &before, &r, 0, &password).await;
            if lifecycle {
                observed["lifecycle"] = lifecycle_observation(&a, &history).await;
                if matches!(
                    scenario,
                    "legacy_password" | "provision" | "provision_secondary" | "external_password"
                ) {
                    b.get(cb.location().unwrap()).await;
                    b.write(Req::new(Method::DELETE, "/session")).await;
                    b.get("/session/new").await;
                    let call_before = r.calls.lock().unwrap().len();
                    let response = b
                        .write(Req::new(Method::POST, "/session").form(&[
                            (
                                "email_address",
                                if scenario == "legacy_password" {
                                    "legacy@smartdata.net"
                                } else {
                                    email
                                },
                            ),
                            ("password", "secret123456"),
                        ]))
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
                let mut requests = match scenario {
                    "consume_device" => vec![Req::new(
                        Method::DELETE,
                        &format!("/two_factor_remembered_devices/{}", device.unwrap()),
                    )],
                    "consume_all_devices" => {
                        vec![Req::new(Method::DELETE, "/two_factor_remembered_devices")]
                    }
                    "consume_disable" => vec![Req::new(Method::DELETE, "/two_factor_setup")],
                    "consume_backup" => vec![
                        Req::new(Method::POST, "/two_factor_backup_codes"),
                        Req::new(Method::POST, "/two_factor_backup_codes"),
                    ],
                    "wrong_credential_preserves" => vec![
                        Req::new(Method::POST, "/two_factor_backup_codes")
                            .form(&[("reauth", "wrong-credential")]),
                        Req::new(Method::POST, "/two_factor_backup_codes"),
                    ],
                    _ => vec![Req::new(Method::POST, "/two_factor_backup_codes")],
                };
                let mut protected = Vec::new();
                for request in requests.drain(..) {
                    let call_before = r.calls.lock().unwrap().len();
                    let response = b.write(request).await;
                    protected.push(
                        observation(&a, &b, &response, &before, &r, call_before, &password).await,
                    );
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
            if scenario == "unconfigured_password" {
                b.get("/session/new").await;
                let response = b
                    .write(Req::new(Method::POST, "/session").form(&[
                        ("email_address", "kevin@37signals.com"),
                        ("password", "secret123456"),
                    ]))
                    .await;
                observed["password_login"] =
                    observation(&a, &b, &response, &before, &r, 0, &password).await;
            }
            observed
        };
        assert_eq!(started, row["start"], "{}: authorize", row["spec"]);
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
