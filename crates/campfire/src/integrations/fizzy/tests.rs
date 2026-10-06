use super::*;
use crate::{controllers::presenters::test_support::*, integrations::test_support::*};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::{Value, json};
use std::sync::Arc;
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/ws15e_fizzy.json")).unwrap()
}
async fn fake(
    routes: Vec<Route>,
) -> (
    FakeServer,
    crate::net::Network,
    Arc<FakeResolver>,
    Arc<MappingDialer>,
) {
    let server = FakeServer::start_ws15e(routes).await;
    let resolver = Arc::new(FakeResolver::new([("app.fizzy.do", vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: ["93.184.216.34".parse().unwrap()].into(),
        to: server.addr,
        dialed: Default::default(),
    });
    let net = network(resolver.clone(), dialer.clone());
    (server, net, resolver, dialer)
}
async fn invoke(
    client: &client::Client,
    method: &str,
    args: &[Value],
    kwargs: &Value,
) -> Result<Value, client::Error> {
    let s = |i: usize| ruby_string(&args[i]);
    match method {
        "identity" => client.identity().await,
        "boards" => client.boards(&s(0)).await,
        "board" => client.board(&s(0), &s(1)).await,
        "columns" => client.columns(&s(0), &s(1)).await,
        "card" => client.card(&s(0), &s(1)).await,
        "search" => client.search(&s(0), &s(1)).await,
        "create_card" => {
            client
                .create_card(
                    &s(0),
                    &s(1),
                    kwargs["title"].clone(),
                    kwargs.get("description").cloned(),
                )
                .await
        }
        "create_comment" => {
            client
                .create_comment(&s(0), &s(1), kwargs["body"].clone())
                .await
        }
        "move_to_column" => client
            .move_to_column(&s(0), &s(1), kwargs["column_id"].clone())
            .await
            .map(Value::Bool),
        "close_card" => client.close_card(&s(0), &s(1)).await.map(Value::Bool),
        "reopen_card" => client.reopen_card(&s(0), &s(1)).await.map(Value::Bool),
        _ => panic!("unexpected operation"),
    }
}
#[tokio::test]
async fn ws15e_fizzy_client_requests_match_rails_and_pin_once() {
    for row in vectors()["requests"].as_array().unwrap() {
        let expected = &row["request"];
        let (server, net, resolver, dialer) = fake(vec![
            Route::new(
                expected["method"].as_str().unwrap(),
                "app.fizzy.do",
                expected["path"].as_str().unwrap(),
                200,
            )
            .body("{}"),
        ])
        .await;
        let token = "fixture-fizzy-token";
        let client = client::Client::new(net, token.into(), "http://app.fizzy.do/base/is/ignored");
        assert_eq!(
            invoke(
                &client,
                row["method"].as_str().unwrap(),
                row["args"].as_array().unwrap(),
                &row["kwargs"]
            )
            .await
            .unwrap_or_else(|error| panic!(
                "operation {}; received {:?}; {error}",
                row["method"],
                server
                    .received
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|r| &r.target)
                    .collect::<Vec<_>>()
            )),
            row["value"]
        );
        assert_eq!(resolver.lookups(), ["app.fizzy.do"]);
        assert_eq!(
            dialer.dialed.lock().unwrap()[0].ip().to_string(),
            "93.184.216.34"
        );
        let requests = server.received.lock().unwrap();
        let actual = &requests[0];
        assert_eq!(actual.method, expected["method"].as_str().unwrap());
        assert_eq!(actual.target, expected["path"].as_str().unwrap());
        assert_eq!(
            actual.header("authorization"),
            Some(format!("Bearer {token}").as_str())
        );
        assert_eq!(actual.header("accept"), Some("application/json"));
        assert_eq!(actual.header("content-type"), Some("application/json"));
        assert_eq!(actual.header("user-agent"), Some("Smartfire-Fizzy"));
        match expected["body"].as_str() {
            Some(body) => assert_eq!(actual.body, body.as_bytes()),
            None => assert!(actual.body.is_empty()),
        }
        assert_eq!(
            client::TIMEOUTS.open.as_secs(),
            row["options"]["open_timeout"].as_u64().unwrap()
        );
        assert_eq!(
            client::TIMEOUTS.read.as_secs(),
            row["options"]["read_timeout"].as_u64().unwrap()
        );
        assert_eq!(
            client::TIMEOUTS.write.as_secs(),
            row["options"]["write_timeout"].as_u64().unwrap()
        );
    }
}
#[tokio::test]
async fn ws15e_fizzy_path_traversal_ids_fail_before_any_dns() {
    let (server, net, resolver, _) = fake(vec![]).await;
    let client = client::Client::new(net, "fixture-private-token".into(), client::DEFAULT_BASE);
    for invalid in [
        "",
        "../acc",
        "acc/other",
        "acc?query",
        "acc#fragment",
        "acc%2Fother",
        "acc\n",
        "é",
    ] {
        for method in [
            "boards",
            "board",
            "columns",
            "card",
            "search",
            "create_card",
            "create_comment",
            "move_to_column",
            "close_card",
            "reopen_card",
        ] {
            let row = json!([invalid, "12"]);
            let error = invoke(&client, method, row.as_array().unwrap(), &json!({}))
                .await
                .unwrap_err();
            assert_eq!(error.message, "Invalid Fizzy id");
        }
        assert!(client.board("acc", invalid).await.is_err());
        assert!(client.columns("acc", invalid).await.is_err());
        assert!(
            client
                .create_card("acc", invalid, Value::Null, None)
                .await
                .is_err()
        );
    }
    for number in ["", "-1", "1.2", "12/x", "12?x", "12\n", "１２"] {
        let error = client.card("acc", number).await.unwrap_err();
        assert_eq!(error.message, "Invalid Fizzy card number");
    }
    assert!(resolver.lookups().is_empty());
    assert!(server.received.lock().unwrap().is_empty());
}
#[tokio::test]
async fn ws15e_fizzy_response_status_and_redaction_match_rails() {
    for row in vectors()["responses"].as_array().unwrap() {
        let status = row["status"].as_u64().unwrap() as u16;
        let (server, net, _, _) = fake(vec![
            Route::new("GET", "app.fizzy.do", "/acc/cards/12.json", status)
                .body(row["body"].as_str().unwrap()),
        ])
        .await;
        let client =
            client::Client::new(net, "fixture-private-token".into(), "http://app.fizzy.do");
        let result = client.card("acc", "12").await;
        match row.get("error") {
            Some(kind) => {
                let error = result.unwrap_err();
                assert_eq!(
                    format!("{:?}", error.kind).replace("Other", "Error"),
                    kind.as_str().unwrap(),
                    "{row}"
                );
                assert_eq!(error.message, row["message"], "{row}");
                assert!(!error.message.contains("fixture-private-token"));
            }
            None => assert_eq!(result.unwrap(), row["value"]),
        }
        assert_eq!(server.received.lock().unwrap().len(), 1);
    }
}
#[test]
fn ws15e_fizzy_url_extraction_matches_rails() {
    assert_eq!(client::TIMEOUTS.open.as_secs(), 10);
    for row in vectors()["urls"].as_array().unwrap() {
        assert_eq!(
            serde_json::to_value(
                urls::extract(row["text"].as_str().unwrap(), row["base"].as_str().unwrap())
                    .unwrap()
            )
            .unwrap(),
            row["refs"]
        );
    }
    let text = (1..=6)
        .map(|n| format!("https://app.fizzy.do/acc/cards/{n} "))
        .collect::<String>();
    assert_eq!(urls::extract(&text, client::DEFAULT_BASE).unwrap().len(), 4);
}
#[tokio::test]
async fn ws15e_fizzy_account_encryption_validation_and_unusable_tokens() {
    let mut app = TestApp::boot().await.expect("build the pinned parity seed");
    app.booted
        .jobs
        .stop(std::time::Duration::from_secs(1))
        .await;
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(move |tx| {
            accounts::Account::disconnect(tx, DAVID)?;
            let input = accounts::Input {
                user_id: DAVID,
                account_id: "acc",
                account_name: Some("Account"),
                fizzy_user_id: Some("person"),
                fizzy_user_name: Some("Person"),
                token: "fixture-private-token",
            };
            let account = accounts::Account::create(tx, &crypto, &input)?;
            assert_eq!(
                account.usable_token(tx, &crypto)?.as_deref(),
                Some(input.token)
            );
            let cipher: String = tx.conn().query_row(
                "SELECT access_token FROM fizzy_connected_accounts WHERE id=?",
                [account.id],
                |r| r.get(0),
            )?;
            assert!(!cipher.contains(input.token));
            assert!(accounts::Account::create(tx, &crypto, &input).is_err());
            let bad = accounts::Input {
                account_id: "\u{a0}",
                ..input
            };
            assert!(accounts::Account::relink(tx, &crypto, &bad).is_err());
            let missing = accounts::Input {
                user_id: 0,
                account_id: "acc",
                ..bad
            };
            assert!(accounts::Account::create(tx, &crypto, &missing).is_err());
            account.mark_disconnected(tx, "Disconnected")?;
            assert!(
                accounts::Account::for_user(tx.conn(), DAVID)?
                    .unwrap()
                    .usable_token(tx, &crypto)?
                    .is_none()
            );
            let relinked = accounts::Account::relink(
                tx,
                &crypto,
                &accounts::Input {
                    user_id: DAVID,
                    ..missing
                },
            )?;
            assert_eq!(relinked.id, account.id);
            assert!(relinked.connected());
            tx.conn().execute(
                "UPDATE fizzy_connected_accounts SET access_token='not ciphertext' WHERE id=?",
                [account.id],
            )?;
            assert!(
                accounts::Account::for_user(tx.conn(), DAVID)?
                    .unwrap()
                    .usable_token(tx, &crypto)?
                    .is_none()
            );
            assert_eq!(
                accounts::Account::for_user(tx.conn(), DAVID)?
                    .unwrap()
                    .disconnected_reason
                    .as_deref(),
                Some(accounts::UNREADABLE_TOKEN_REASON)
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[test]
fn ws15e_fizzy_reads_rails_encrypted_token_and_sweep_constants() {
    let v = vectors();
    let crypto = ArEncryption::new(&rails_compat::Secrets::new(
        v["account"]["secret_key_base"].as_str().unwrap(),
    ));
    assert_eq!(
        crypto
            .decrypt(v["account"]["ciphertext"].as_str().unwrap())
            .unwrap(),
        v["account"]["token"].as_str().unwrap()
    );
    assert_eq!(
        crate::integrations::action_claims::SWEEP_INTERVAL.as_secs(),
        v["sweep"]["interval"].as_u64().unwrap()
    );
    assert_eq!(
        crate::integrations::action_claims::STUCK_CLAIM_AFTER.as_secs(),
        v["sweep"]["age"].as_i64().unwrap()
    );
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.scratch/fizzy_rust_token.json");
    std::fs::write(output,serde_json::to_vec(&json!({"ciphertext":crypto.encrypt(v["account"]["token"].as_str().unwrap()),"token":v["account"]["token"]})).unwrap()).unwrap();
}

#[tokio::test]
async fn ws15e_fizzy_real_read_timeout_retries_once_and_redacts_credentials() {
    let mut route = Route::new("GET", "app.fizzy.do", "/my/identity.json", 200).body("{}");
    route.delay = std::time::Duration::from_secs(30);
    let (server, net, resolver, _) = fake(vec![route]).await;
    let client = client::Client::new(
        net,
        "fixture-secret-timeout-token".into(),
        "http://app.fizzy.do",
    );
    let before = tokio::time::Instant::now();
    let error = client.identity().await.unwrap_err();
    assert_eq!(error.message, "Could not reach Fizzy (Read Timeout)");
    assert!(before.elapsed() >= std::time::Duration::from_millis(19500));
    assert!(before.elapsed() < std::time::Duration::from_secs(25));
    assert!(!error.message.contains("fixture-secret-timeout-token"));
    assert_eq!(server.received.lock().unwrap().len(), 2);
    assert_eq!(resolver.lookups().len(), 2);
}

#[tokio::test]
async fn ws15e_fizzy_transport_retries_match_real_pinned_rails() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let v: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/ws15e_fizzy_retries.json"
    ))
    .unwrap();
    for row in v["cases"].as_array().unwrap() {
        let listener = ws15e_listener().await;
        let addr = listener.local_addr().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let task = tokio::spawn(async move {
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                let mut reader = BufReader::new(socket);
                let mut line = String::new();
                reader.read_line(&mut line).await.unwrap();
                let method = line.split_whitespace().next().unwrap().to_string();
                loop {
                    line.clear();
                    reader.read_line(&mut line).await.unwrap();
                    if line == "\r\n" {
                        break;
                    }
                }
                let attempt = observed.fetch_add(1, Ordering::SeqCst) + 1;
                if attempt > 1 {
                    reader
                        .get_mut()
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                        )
                        .await
                        .unwrap();
                } else {
                    // Drain POST's body so closing is a clean EOF, not ECONNRESET.
                    if method == "POST" {
                        use tokio::io::AsyncReadExt;
                        let mut body = [0; 30];
                        reader.read_exact(&mut body).await.unwrap();
                    }
                }
            }
        });
        let resolver = Arc::new(FakeResolver::new([("app.fizzy.do", vec!["93.184.216.34"])]));
        let dialer = Arc::new(MappingDialer {
            public: ["93.184.216.34".parse().unwrap()].into(),
            to: addr,
            dialed: Default::default(),
        });
        let client = client::Client::new(
            network(resolver.clone(), dialer.clone()),
            "fixture-fizzy-retry".into(),
            "http://app.fizzy.do",
        );
        let result = match row["method"].as_str().unwrap() {
            "GET" => client.identity().await,
            "DELETE" => client.reopen_card("acc", "12").await.map(Value::Bool),
            "POST" => {
                client
                    .create_comment("acc", "12", Value::String("Comment".into()))
                    .await
            }
            _ => unreachable!(),
        };
        if let Some(error) = row.get("error") {
            assert_eq!(result.unwrap_err().message, error.as_str().unwrap());
        } else {
            assert_eq!(result.unwrap(), row["value"]);
        }
        assert_eq!(
            count.load(Ordering::SeqCst),
            row["requests"].as_u64().unwrap() as usize
        );
        assert_eq!(resolver.lookups().len(), count.load(Ordering::SeqCst));
        task.abort();
    }
}

#[tokio::test]
async fn ws15e_review_fizzy_dns_is_inside_open_timeout() {
    use crate::net::{Resolver,BoxFuture};
    struct SlowDns;
    impl Resolver for SlowDns {
        fn lookup<'a>(&'a self,_host:&'a str)->BoxFuture<'a,std::io::Result<Vec<std::net::IpAddr>>> {
            Box::pin(async {tokio::time::sleep(std::time::Duration::from_millis(10500)).await;Ok(vec!["93.184.216.34".parse().unwrap()])})
        }
    }
    let server=FakeServer::start_ws15e(vec![Route::new("GET","app.fizzy.do","/my/identity.json",200).body("{}")]).await;
    let dialer=Arc::new(MappingDialer {public:["93.184.216.34".parse().unwrap()].into(),to:server.addr,dialed:Default::default()});
    let mut net=network(Arc::new(FakeResolver::new([])),dialer);
    net.resolver=Arc::new(SlowDns);
    let client=client::Client::new(net,"fixture".into(),"http://app.fizzy.do");
    let started=std::time::Instant::now();
    let result=client.identity().await;
    eprintln!("REVIEW Fizzy slow DNS: elapsed={:?} requests={} success={}",started.elapsed(),server.received().len(),result.is_ok());
    assert!(result.is_err(),"Rails Net::HTTP open_timeout bounds TCPSocket DNS resolution at 10 seconds");
    assert_eq!(result.unwrap_err().message,"Could not reach Fizzy (Open Timeout)");
    assert!(server.received().is_empty());
}
