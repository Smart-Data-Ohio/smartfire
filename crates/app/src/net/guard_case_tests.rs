//! Named cases from test/lib/restricted_http/private_network_guard_test.rb.
use super::*;
use crate::net::{BoxFuture, Resolver};
use std::{io, net::IpAddr};
struct Dns(Vec<IpAddr>);
impl Resolver for Dns {
    fn lookup<'a>(&'a self, _host: &'a str) -> BoxFuture<'a, io::Result<Vec<IpAddr>>> {
        Box::pin(async { Ok(self.0.clone()) })
    }
}
async fn compare(index: usize) {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_private_guard_cases.json"
    ))
    .unwrap();
    let case = &v["cases"][index];
    if case["kind"] == "private_ip" {
        for row in case["results"].as_array().unwrap() {
            let address = row["address"].as_str().unwrap();
            assert_eq!(
                address
                    .parse::<IpAddr>()
                    .map(blocked_at_reference_pin)
                    .unwrap_or(true),
                row["private"].as_bool().unwrap(),
                "{address}"
            );
        }
    } else {
        for row in case["results"].as_array().unwrap() {
            let resolver = Dns(row["answers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().parse().unwrap())
                .collect());
            let actual = match resolve_webhook(&resolver, row["host"].as_str().unwrap()).await {
                Ok(ip) => serde_json::json!({"address":ip.to_string()}),
                Err(GuardError::Violation(_)) => {
                    serde_json::json!({"error":"RestrictedHTTP::Violation"})
                }
                Err(GuardError::Unresolvable) => {
                    serde_json::json!({"error":"Surfguard::Unresolvable"})
                }
            };
            let expected = if row.get("address").is_some() {
                serde_json::json!({"address":row["address"]})
            } else {
                serde_json::json!({"error":row["error"]})
            };
            assert_eq!(actual, expected);
        }
    }
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_this_network_rfc1700() {
    compare(0).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_loopback_addresses() {
    compare(1).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_rfc1918_private_addresses() {
    compare(2).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_link_local_addresses() {
    compare(3).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_false_for_public_addresses() {
    compare(4).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_ipv4_mapped_ipv6_addresses_with_private_ips()
 {
    compare(5).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_ipv4_mapped_ipv6_addresses_with_link_local_ips()
 {
    compare(6).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_ipv4_mapped_ipv6_addresses_even_with_public_ips()
 {
    compare(7).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_ipv4_compatible_ipv6_addresses_with_private_ips()
 {
    compare(8).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_ipv4_compatible_ipv6_addresses_with_link_local_ips()
 {
    compare(9).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_ipv4_compatible_ipv6_addresses_even_with_public_ips()
 {
    compare(10).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_carrier_grade_nat_addresses_rfc6598() {
    compare(11).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_nat64_addresses_embedding_a_private_ipv4()
 {
    compare(12).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_the_whole_local_use_nat64_block_rfc8215()
 {
    compare(13).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_siit_ipv4_translated_addresses_rfc2765()
 {
    compare(14).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_false_for_nat64_addresses_embedding_a_public_ipv4()
 {
    compare(15).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_6to4_and_teredo_transition_addresses()
{
    compare(16).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_ipv6_loopback_ula_and_link_local() {
    compare(17).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_ipv6_multicast_documentation_and_benchmarking_ranges()
 {
    compare(18).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_false_for_public_ipv6_addresses() {
    compare(19).await;
}

#[tokio::test]
async fn ws11_private_guard_case_private_ip_returns_true_for_invalid_addresses() {
    compare(20).await;
}

#[tokio::test]
async fn ws11_private_guard_case_resolve_raises_violation_for_private_hostname() {
    compare(21).await;
}

#[tokio::test]
async fn ws11_private_guard_case_resolve_returns_ip_for_public_hostname() {
    compare(22).await;
}

#[tokio::test]
async fn ws11_private_guard_case_resolve_refuses_hostnames_answering_only_blocked_ipv6_addresses() {
    compare(23).await;
}

#[tokio::test]
async fn ws11_private_guard_case_resolve_refuses_numeric_ipv4_literals_however_they_are_spelled() {
    compare(24).await;
}

#[tokio::test]
async fn ws11_private_guard_case_resolve_returns_the_normalized_address_for_public_numeric_literals()
 {
    compare(25).await;
}

#[tokio::test]
async fn ws11_private_guard_case_resolve_refuses_bracketed_ipv6_literals_pointing_at_blocked_addresses()
 {
    compare(26).await;
}

#[tokio::test]
async fn ws11_private_guard_case_resolve_raises_unresolvable_not_violation_when_the_host_resolves_to_nothing()
 {
    compare(27).await;
}
