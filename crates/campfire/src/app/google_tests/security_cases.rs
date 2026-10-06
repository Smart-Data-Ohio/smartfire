//! Committed status changes on a separate SQLite connection; no timing sleeps or mocked linker.
use super::*;
use campfire_kit::FrozenClock;
#[tokio::test]
async fn google_status_races_and_link_csrf_match_pinned_rails_requests() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_auth_security.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let at = jiff::Timestamp::from_second(oracle["now"].as_i64().unwrap()).unwrap();
        let a = TestApp::boot_with_clock(Arc::new(FrozenClock::new(at)))
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let r = Arc::new(Recorded {
            response: Mutex::new(Err(())),
            calls: Mutex::new(vec![]),
            certs: Mutex::new(None),
        });
        a.booted
            .app
            .google
            .install(SignIn::with_client(config(&["smartdata.net"]), r.clone()));
        let linked = row["spec"]["linked"] == true;
        a.db().write(move |tx| {
            tx.conn().execute_batch("DELETE FROM google_identities")?;
            tx.conn().execute("DELETE FROM sessions WHERE user_id=?",[KEVIN])?;
            tx.conn().execute("DELETE FROM two_factor_credentials WHERE user_id=?",[KEVIN])?;
            tx.conn().execute("UPDATE users SET email_address='race@smartdata.net',google_email_link_allowed=1 WHERE id=?",[KEVIN])?;
            if linked {campfire_db::models::google_identity::GoogleIdentity::link_to_user(tx,json!({"sub":"race-member","email":"race@smartdata.net","hd":"smartdata.net"}).as_object().unwrap(),KEVIN)?;}
            Ok(())
        }).await.unwrap();
        let action = row["spec"]["action"].as_str().map(str::to_owned);
        let actual = if let Some(action) = action {
            let mut b = a.anonymous();
            b.get("/session/new").await;
            let q = start(&mut b, "/session/google").await;
            let path = a.db().path().to_owned();
            let env = a.db().env().clone();
            let fence = Arc::new(Mutex::new(None));
            let observed = fence.clone();
            let intervention = action.clone();
            a.db().write(move |tx| {
                tx.conn().create_scalar_function("google_status_intervention",0,rusqlite::functions::FunctionFlags::SQLITE_UTF8,move |_| {
                    let path=path.clone();let env=env.clone();let action=intervention.clone();
                    let result=std::thread::spawn(move || {
                        let conn=rusqlite::Connection::open(path).unwrap();conn.busy_timeout(std::time::Duration::ZERO).unwrap();
                        campfire_db::database::run_write(&conn,&env,|tx| {
                            let mut user=campfire_db::User::find(tx.conn(),KEVIN)?;
                            if action=="deactivate" {user.deactivate(tx)} else {user.ban(tx)}
                        })
                    }).join().unwrap();
                    let outcome=match result {Ok(())=>"completed",Err(campfire_db::Error::Sqlite(rusqlite::Error::SqliteFailure(error,_))) if error.code==rusqlite::ErrorCode::DatabaseBusy=>"retry",other=>panic!("unexpected status intervention: {other:?}")};
                    assert!(observed.lock().unwrap().replace(outcome).is_none(),"fence must run exactly once");
                    Ok(0i64)
                })?;
                tx.conn().execute_batch(&format!("CREATE TRIGGER google_status_fence BEFORE INSERT ON sessions WHEN NEW.user_id={KEVIN} BEGIN SELECT google_status_intervention(); END;"))?;
                Ok(())
            }).await.unwrap();
            answer(&r, claims(&a, &q, "race-member", "race@smartdata.net"));
            let callback_path = format!(
                "/session/google/callback?state={}&code=recorded-code",
                crate::controllers::presenters::test_support::encode(&q["state"])
            );
            let reply = b
                .send(Req::new(Method::GET, &callback_path).header("x-forwarded-for", "8.8.8.8"))
                .await;
            let outcome = fence.lock().unwrap().unwrap_or_else(|| {
                panic!(
                    "callback never reached session insert: {:?} {:?}; audits: {:?}",
                    reply.status,
                    reply.location(),
                    r.calls.lock().unwrap()
                )
            });
            if outcome == "retry" {
                a.db()
                    .write(move |tx| {
                        let mut user = campfire_db::User::find(tx.conn(), KEVIN)?;
                        if action == "deactivate" {
                            user.deactivate(tx)
                        } else {
                            user.ban(tx)
                        }
                    })
                    .await
                    .unwrap();
            }
            let root = b.get("/").await;
            let (status, sessions) = a
                .db()
                .read(|conn| {
                    Ok((
                        campfire_db::User::find(conn, KEVIN)?.status.name(),
                        conn.query_row(
                            "SELECT count(*) FROM sessions WHERE user_id=?",
                            [KEVIN],
                            |r| r.get::<_, i64>(0),
                        )?,
                    ))
                })
                .await
                .unwrap();
            json!({"status":status,"sessions":sessions,"root_status":root.status.as_u16(),"root_location":root.location(),"contention":outcome})
        } else {
            let kind = row["spec"]["kind"].as_str().unwrap();
            let mut b = if kind == "anonymous" {
                a.anonymous()
            } else {
                a.sign_in(KEVIN).await
            };
            b.get("/session/new").await;
            let request = Req::new(Method::POST, "/user/profile/google_sign_in_link");
            let reply = match kind {
                "anonymous" => b.write(request).await,
                "csrf_invalid" => b.send(request.header("X-CSRF-Token", "invalid")).await,
                _ => b.send(request).await,
            };
            let identities = a
                .db()
                .read(|c| {
                    Ok(
                        c.query_row("SELECT count(*) FROM google_identities", [], |r| {
                            r.get::<_, i64>(0)
                        })?,
                    )
                })
                .await
                .unwrap();
            json!({"status":reply.status.as_u16(),"location":reply.location(),"body":(reply.status==StatusCode::UNPROCESSABLE_ENTITY).then(||reply.text()),"identities":identities})
        };
        assert_eq!(actual, row["result"], "pinned Rails {:?}", row["spec"]);
        if row["spec"]["kind"].is_string() {
            assert!(r.calls.lock().unwrap().is_empty());
        }
    }
    println!("Pinned Rails Google status races: 4 exercised; link/CSRF: 3 exercised; 0 skipped");
}
