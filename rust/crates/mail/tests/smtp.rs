//! A real SMTP peer in the worker's configured port range. No live mail provider or seed is used.
use campfire_mail::{
    config::Smtp,
    outbound::{self, User},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};

async fn listener() -> TcpListener {
    let range = std::env::var("MAIL_TEST_PORT_RANGE").unwrap_or_else(|_| "40000-40049".into());
    let (first, last) = range.split_once('-').expect("MAIL_TEST_PORT_RANGE=start-end");
    let first: u16 = first.parse().unwrap();
    let last: u16 = last.parse().unwrap();
    for port in first..=last {
        if let Ok(socket) = TcpListener::bind(("127.0.0.1", port)).await {
            return socket;
        }
    }
    panic!("no free SMTP test port in {range}");
}
async fn peer(
    listener: TcpListener,
    attempts: usize,
    transient_first: bool,
    auth: Option<&str>,
) -> Vec<Vec<u8>> {
    let mut delivered = Vec::new();
    for attempt in 0..attempts {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = BufReader::new(stream);
        socket
            .get_mut()
            .write_all(b"220 ws10 SMTP\r\n")
            .await
            .unwrap();
        let mut line = String::new();
        socket.read_line(&mut line).await.unwrap();
        assert_eq!(line, "EHLO localhost.localdomain\r\n");
        socket
            .get_mut()
            .write_all(b"250-ws10\r\n250-AUTH PLAIN LOGIN CRAM-MD5\r\n250 8BITMIME\r\n")
            .await
            .unwrap();
        if let Some(auth) = auth {
            use base64::Engine as _;
            let encode = |s| base64::engine::general_purpose::STANDARD.encode(s);
            line.clear();
            socket.read_line(&mut line).await.unwrap();
            match auth {
                "plain" => assert_eq!(
                    line,
                    format!(
                        "AUTH PLAIN {}\r\n",
                        encode("\0fixture-user\0fixture-password")
                    )
                ),
                "login" => {
                    assert_eq!(line, "AUTH LOGIN\r\n");
                    socket
                        .get_mut()
                        .write_all(b"334 VXNlcm5hbWU6\r\n")
                        .await
                        .unwrap();
                    line.clear();
                    socket.read_line(&mut line).await.unwrap();
                    assert_eq!(line, format!("{}\r\n", encode("fixture-user")));
                    socket
                        .get_mut()
                        .write_all(b"334 UGFzc3dvcmQ6\r\n")
                        .await
                        .unwrap();
                    line.clear();
                    socket.read_line(&mut line).await.unwrap();
                    assert_eq!(line, format!("{}\r\n", encode("fixture-password")));
                }
                "cram_md5" => {
                    assert_eq!(line, "AUTH CRAM-MD5\r\n");
                    socket
                        .get_mut()
                        .write_all(format!("334 {}\r\n", encode("relay challenge")).as_bytes())
                        .await
                        .unwrap();
                    line.clear();
                    socket.read_line(&mut line).await.unwrap();
                    let reference: serde_json::Value =
                        serde_json::from_str(include_str!("../../../vectors/mail/reference.json"))
                            .unwrap();
                    assert_eq!(
                        line,
                        format!("{}\r\n", reference["cram"][1]["response"].as_str().unwrap())
                    );
                }
                _ => unreachable!(),
            }
            socket
                .get_mut()
                .write_all(b"235 Authentication accepted\r\n")
                .await
                .unwrap();
        }
        line.clear();
        socket.read_line(&mut line).await.unwrap();
        assert_eq!(line, "MAIL FROM:<noreply@smartdata.net>\r\n");
        if transient_first && attempt == 0 {
            socket
                .get_mut()
                .write_all(b"451 Try later\r\n")
                .await
                .unwrap();
            continue;
        }
        socket
            .get_mut()
            .write_all(b"250 Sender accepted\r\n")
            .await
            .unwrap();
        line.clear();
        socket.read_line(&mut line).await.unwrap();
        assert_eq!(line, "RCPT TO:<person@example.com>\r\n");
        socket
            .get_mut()
            .write_all(b"250 Recipient accepted\r\n")
            .await
            .unwrap();
        line.clear();
        socket.read_line(&mut line).await.unwrap();
        assert_eq!(line, "DATA\r\n");
        socket
            .get_mut()
            .write_all(b"354 Send mail\r\n")
            .await
            .unwrap();
        let mut raw = Vec::new();
        loop {
            line.clear();
            socket.read_line(&mut line).await.unwrap();
            if line == ".\r\n" {
                break;
            }
            raw.extend_from_slice(line.as_bytes());
        }
        socket
            .get_mut()
            .write_all(b"250 Accepted\r\n")
            .await
            .unwrap();
        delivered.push(raw);
        // Lettre sends QUIT with pooling disabled; closing after accepting is also valid.
    }
    delivered
}
fn settings(port: u16) -> Smtp {
    Smtp {
        address: "127.0.0.1".into(),
        port,
        enable_starttls: false,
        ..Default::default()
    }
}
#[tokio::test]
async fn smtp_delivers_raw_multipart_over_a_real_socket() {
    let listener = listener().await;
    let smtp = settings(listener.local_addr().unwrap().port());
    let peer = tokio::spawn(peer(listener, 1, false, None));
    let m = outbound::lockout_notice(&User {
        name: "Person".into(),
        email: "person@example.com".into(),
    });
    outbound::deliver(&smtp, &m, "2026-09-29T12:30:00Z".parse().unwrap())
        .await
        .unwrap();
    let mails = peer.await.unwrap();
    assert_eq!(mails.len(), 1);
    let parsed = mailparse::parse_mail(&mails[0]).unwrap();
    assert_eq!(parsed.subparts.len(), 2);
    assert_eq!(
        parsed.subparts[0].get_body().unwrap().replace("\r\n", "\n"),
        m.text
    );
    assert_eq!(
        parsed.subparts[1].get_body().unwrap().replace("\r\n", "\n"),
        m.html
    );
}
#[derive(Clone)]
struct Context {
    settings: Smtp,
    count: Arc<AtomicUsize>,
}
#[tokio::test]
async fn durable_delivery_retries_a_real_smtp_transient_failure() {
    use campfire_jobs::{JobRequest, Outcome, QueueConfig, Registry, RunnerConfig};
    use campfire_mail::jobs::{DeliveryJob, Notification};
    let listener = listener().await;
    let settings = settings(listener.local_addr().unwrap().port());
    let peer = tokio::spawn(peer(listener, 2, true, None));
    let mut registry = Registry::<Context>::new();
    registry.register::<DeliveryJob, _, _>(|context, _, _| async move {
        context.count.fetch_add(1, Ordering::SeqCst);
        let message = outbound::lockout_notice(&User {
            name: "Person".into(),
            email: "person@example.com".into(),
        });
        outbound::deliver(
            &context.settings,
            &message,
            "2026-09-29T12:30:00Z".parse().unwrap(),
        )
        .await?;
        Ok(Outcome::Done)
    });
    let dir = tempfile::tempdir().unwrap();
    let db = campfire_db::Database::open(
        campfire_db::Config::new(dir.path().join("jobs.sqlite3")),
        campfire_db::Env::default(),
    )
    .unwrap();
    let mut config = RunnerConfig::new(vec![QueueConfig::new("default", 1)]);
    config.poll = std::time::Duration::from_millis(10);
    let queue = campfire_jobs::JobQueue::new(&registry, &config).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let runner = campfire_jobs::start(
        db.clone(),
        queue.clone(),
        registry,
        Context {
            settings,
            count: count.clone(),
        },
        config,
    );
    queue
        .perform_later(
            &db,
            JobRequest::new(&DeliveryJob {
                notification: Notification::Lockout { user_id: 1 },
            }),
        )
        .await
        .unwrap();
    let delivered = tokio::time::timeout(std::time::Duration::from_secs(10), peer)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(delivered.len(), 1);
    assert_eq!(count.load(Ordering::SeqCst), 2);
    runner.shutdown(std::time::Duration::from_secs(1)).await;
    assert!(
        db.read(|c| campfire_jobs::inspect::with_status(c, campfire_jobs::READY))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        db.read(|c| campfire_jobs::inspect::with_status(c, campfire_jobs::FAILED))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn starttls_is_required_without_sending_mail_or_credentials() {
    let listener = listener().await;
    let mut smtp = settings(listener.local_addr().unwrap().port());
    smtp.enable_starttls = true;
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        stream
            .get_mut()
            .write_all(b"220 ws10 SMTP\r\n")
            .await
            .unwrap();
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert!(line.starts_with("EHLO "));
        stream
            .get_mut()
            .write_all(b"250 ws10 no TLS\r\n")
            .await
            .unwrap();
        line.clear();
        let _ = stream.read_line(&mut line).await;
        assert!(!line.starts_with("MAIL "));
        assert!(!line.starts_with("AUTH "));
    });
    let m = outbound::lockout_notice(&User {
        name: "Person".into(),
        email: "person@example.com".into(),
    });
    assert!(
        outbound::deliver(&smtp, &m, "2026-09-29T12:30:00Z".parse().unwrap())
            .await
            .is_err()
    );
    tokio::time::timeout(std::time::Duration::from_secs(10), task)
        .await
        .unwrap()
        .unwrap();
}
#[test]
fn cram_md5_matches_net_smtp_reference() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../../../vectors/mail/reference.json")).unwrap();
    for case in reference["cram"].as_array().unwrap() {
        assert_eq!(
            outbound::cram_md5_response(
                case["user"].as_str().unwrap(),
                case["password"].as_str().unwrap(),
                case["challenge"].as_str().unwrap().as_bytes()
            ),
            case["response"].as_str().unwrap()
        );
    }
}

#[tokio::test]
async fn smtp_authentication_modes_match_the_reference_on_real_sockets() {
    for mechanism in ["plain", "login", "cram_md5"] {
        let listener = listener().await;
        let mut smtp = settings(listener.local_addr().unwrap().port());
        smtp.user_name = Some("fixture-user".into());
        smtp.password = Some("fixture-password".into());
        smtp.authentication = Some(mechanism.into());
        let server = tokio::spawn(async move { peer(listener, 1, false, Some(mechanism)).await });
        let m = outbound::lockout_notice(&User {
            name: "Person".into(),
            email: "person@example.com".into(),
        });
        outbound::deliver(&smtp, &m, "2026-09-29T12:30:00Z".parse().unwrap())
            .await
            .unwrap();
        assert_eq!(server.await.unwrap().len(), 1);
    }
}
