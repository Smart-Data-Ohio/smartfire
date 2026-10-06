// Appended only to a generated test-source copy by browser_host.py. No
// tracked crate or release binary is changed. Use the existing test boundary.
#[tokio::test]
#[ignore = "utility: tools-only browser host"]
async fn ws8bm_browser_host_without_jobs() {
    assert_eq!(std::env::var("WS8BM_BROWSER_HOST").as_deref(), Ok("1"));
    let booted = boot_with_services(
        Config::from_env().unwrap(),
        campfire_kit::clock::from_env().unwrap(),
        crate::net::Network::system(),
        crate::jobs::periodic::Intervals { periodic: None, huddle: None },
    ).await.unwrap();
    let app = TestApp { booted, _dir: tempfile::tempdir().unwrap() }
        .without_job_runner().await;
    println!("WS8bm browser host: TestApp::without_job_runner; real router and durable enqueue");
    campfire_kit::front::serve(
        campfire_kit::front::FrontConfig::from_env(),
        app.booted.router.clone(), campfire_kit::server::shutdown_signal(),
    ).await.unwrap();
}
