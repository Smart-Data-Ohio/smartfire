// Appended to the generated test host only. Api::new/install_api is the same
// existing Client seam used by google_api_tests. Real picker/controller writes
// remain untouched; only Google's external list/file exchange goes to the fake.
struct Ws8bmDriveClient(u16);
impl crate::integrations::google::client::Client for Ws8bmDriveClient {
    fn request<'a>(&'a self, host: &'a str, method: hyper::Method, target: &'a str,
        headers: Vec<(String, String)>, body: Vec<u8>) -> crate::net::BoxFuture<'a,
        Result<(u16, Vec<u8>), crate::integrations::google::client::Unavailable>> {
        Box::pin(async move {
            use crate::net::{Network, http::{self, Body, Endpoint, Request, Timeouts}};
            assert_eq!(host, "www.googleapis.com", "only pinned Drive requests are stubbed");
            assert!(target.starts_with("/drive/v3/files"));
            let endpoint = Endpoint { https: false, host: "127.0.0.1".into(), port: self.0, pinned_ip: None };
            let mut request = Request::net_http(method, target.into(), Some(host.into()), headers);
            request.body = body;
            let response = http::exchange(&Network::system(), &endpoint, request, &Timeouts {
                open: std::time::Duration::from_secs(10), read: std::time::Duration::from_secs(10),
                write: std::time::Duration::from_secs(10),
            }).await?;
            let status = response.status;
            let Body::Complete(body) = response.read_body(usize::MAX).await? else { unreachable!() };
            Ok((status, body))
        })
    }
}
fn ws8bm_install_drive_client(app: &TestApp) {
    if let Ok(port) = std::env::var("WS8BM_DRIVE_PORT") {
        app.booted.app.google.install_api(crate::integrations::google::api::Api::new(
            crate::integrations::google::api::Config::from_env(),
            std::sync::Arc::new(Ws8bmDriveClient(port.parse().unwrap())),
        ));
        println!("WS8bm Drive boundary: existing Google Client; local pinned list/file HTTP");
    }
}
