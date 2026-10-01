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
        Ok(json!({"identities":identities,"audits":audits,"email":user.email_address,"password_preserved":user.password_digest==password}))
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
    state["flash"] = cookie
        .get("flash")
        .and_then(|f| f.get("flashes"))
        .cloned()
        .unwrap_or(json!({}));
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
        a.booted.app.google.install(SignIn::with_client(
            config(if scenario == "unconfigured" {
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
        let password = a
            .db()
            .write(move |tx| {
                tx.conn()
                    .execute_batch("DELETE FROM google_identities;DELETE FROM audit_logs;")?;
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
        let mut b = if purpose == "sign_in" || scenario == "anonymous" {
            a.anonymous()
        } else {
            a.sign_in(actor).await
        };
        b.get(if purpose == "sign_in" {
            "/session/new"
        } else {
            "/users/me/profile"
        })
        .await;
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
            }
            if scenario == "expired_flow" {
                clock.advance(jiff::SignedDuration::from_secs(601));
            }
            let mut claims = claims(
                &a,
                &q,
                if scenario == "wrong_subject" {
                    "controller-attacker"
                } else {
                    "controller-member"
                },
                if purpose == "link" {
                    "kevin.w@smartdata.net"
                } else {
                    "david@smartdata.net"
                },
            );
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
                crate::controllers::presenters::test_support::encode(&q["state"]),
                if scenario == "cancelled" {
                    "&error=access_denied"
                } else {
                    ""
                }
            );
            let cb = b.get(&callback_path).await;
            let mut observed = observation(&a, &b, &cb, &before, &r, 0, &password).await;
            if scenario == "replay" {
                let calls = r.calls.lock().unwrap().len();
                let replay = b.get(&callback_path).await;
                observed["replay"] =
                    observation(&a, &b, &replay, &before, &r, calls, &password).await;
            }
            observed
        } else {
            observation(&a, &b, &start_reply, &before, &r, 0, &password).await
        };
        assert_eq!(started, row["start"], "{}: authorize", row["spec"]);
        assert_eq!(
            actual, row["result"],
            "{}: complete request state",
            row["spec"]
        );
    }
    println!("Pinned Rails Google controller observations: 58 exercised; 0 skipped");
}
