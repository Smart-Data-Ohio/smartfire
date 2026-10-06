//! The integrations' test helpers: the shared network fakes ([`crate::net::test_support`]), and
//! the listener Fizzy's HTTP cases inherit.

use tokio::net::TcpListener;

pub use crate::net::test_support::*;

pub fn ws15e_http_case_listener() -> TcpListener {
    use std::os::fd::AsFd;
    let listener =
        std::net::TcpListener::from(std::io::stdin().as_fd().try_clone_to_owned().unwrap());
    assert_eq!(
        crate::integrations::fizzy::client::api_base_url(),
        format!("http://{}", listener.local_addr().unwrap())
    );
    listener.set_nonblocking(true).unwrap();
    TcpListener::from_std(listener).unwrap()
}
