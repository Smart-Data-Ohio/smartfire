//! `RestrictedHTTP::PrivateNetworkGuard` (reference/lib/restricted_http/private_network_guard.rb)
//! and the Surfguard policy pinned by our fork (revision 910be917fd0a).
//!
//! A host goes in and a public address to pin comes out. Numeric hosts never reach DNS; names
//! go unchanged to the resolver. Every answer is classified and the blocked ones dropped; IPv4
//! answers come before IPv6 ones, in resolver order within each family.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::Resolver;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GuardError {
    /// `RestrictedHTTP::Violation`: the host only resolves to blocked addresses (or is malformed).
    #[error("Attempt to access private IP via {0}")]
    Violation(String),
    /// `Surfguard::Unresolvable`: the lookup failed or came back empty.
    #[error("Host could not be resolved")]
    Unresolvable,
}

/// `RestrictedHTTP::PrivateNetworkGuard.resolve(hostname)`: the first public address.
pub async fn resolve(resolver: &dyn Resolver, host: &str) -> Result<IpAddr, GuardError> {
    resolve_public_ips(resolver, host).await?.into_iter().next().ok_or_else(|| GuardError::Violation(host.to_string()))
}

/// `Surfguard.resolve_public_ips(host)`. Malformed input comes back empty rather than raising.
pub async fn resolve_public_ips(resolver: &dyn Resolver, host: &str) -> Result<Vec<IpAddr>, GuardError> {
    // The fork's shim tries AI_NUMERICHOST before Surfguard, including bracketed
    // IPv4 and legacy inet_aton spellings. Names are handed to Resolv unchanged.
    if host.is_empty() {
        return Err(GuardError::Violation(host.to_string()));
    }
    let bare = host.strip_prefix('[').unwrap_or(host);
    let bare = bare.strip_suffix(']').unwrap_or(bare);
    let addresses = match getaddrinfo_numeric(bare).or_else(|| ip_literal(host)) {
        Some(ip) => vec![ip],
        None => resolver.lookup(host).await.map_err(|_| GuardError::Unresolvable)?,
    };
    if addresses.is_empty() {
        return Err(GuardError::Unresolvable);
    }
    let (v4, v6): (Vec<IpAddr>, Vec<IpAddr>) = addresses.into_iter().filter(|ip| !blocked_address(*ip)).partition(IpAddr::is_ipv4);
    Ok(v4.into_iter().chain(v6).collect())
}

/// `IPAddr.new(text)` for a single address: dotted-quad IPv4, or IPv6 with optional brackets.
fn ip_literal(text: &str) -> Option<IpAddr> {
    let inner = text.strip_prefix('[').and_then(|t| t.strip_suffix(']'));
    if let Some(inner) = inner {
        return inner.parse::<Ipv6Addr>().ok().map(IpAddr::V6);
    }
    if text.contains(':') {
        return text.parse::<Ipv6Addr>().ok().map(IpAddr::V6);
    }
    // IPAddr only takes four decimal octets without leading zeros ("01" is rejected)
    let octets: Vec<&str> = text.split('.').collect();
    if octets.len() == 4
        && octets.iter().all(|o| !o.is_empty() && o.bytes().all(|b| b.is_ascii_digit()) && (o.len() == 1 || !o.starts_with('0')))
    {
        return text.parse::<Ipv4Addr>().ok().map(IpAddr::V4);
    }
    None
}

/// glibc's `getaddrinfo(..., AI_NUMERICHOST)`: `inet_aton` forms for IPv4, `inet_pton` for IPv6.
fn getaddrinfo_numeric(host: &str) -> Option<IpAddr> {
    if host.contains(':') {
        return host.parse::<Ipv6Addr>().ok().map(IpAddr::V6);
    }
    inet_aton(host).map(IpAddr::V4)
}

/// glibc `__inet_aton_exact`: 1-4 parts in decimal, octal (leading 0) or hex (0x); the last
/// part fills the remaining bytes.
fn inet_aton(text: &str) -> Option<Ipv4Addr> {
    let parts: Vec<&str> = text.split('.').collect();
    if parts.is_empty() || parts.len() > 4 {
        return None;
    }
    let mut values = Vec::with_capacity(parts.len());
    for part in &parts {
        let (digits, radix) = if let Some(hex) = part.strip_prefix("0x").or_else(|| part.strip_prefix("0X")) {
            (hex, 16)
        } else if part.len() > 1 && part.starts_with('0') {
            (&part[1..], 8)
        } else {
            (*part, 10)
        };
        if part.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
            return None;
        }
        let value = if digits.is_empty() { 0 } else { u64::from_str_radix(digits, radix).ok()? };
        if value > u32::MAX as u64 {
            return None;
        }
        values.push(value as u32);
    }
    let (last, leading) = values.split_last()?;
    if leading.iter().any(|v| *v > 0xff) {
        return None;
    }
    let remaining_bits = 32 - 8 * leading.len() as u32;
    if remaining_bits < 32 && *last >= 1 << remaining_bits {
        return None;
    }
    let mut address = *last;
    for (i, v) in leading.iter().enumerate() {
        address |= v << (24 - 8 * i as u32);
    }
    Some(Ipv4Addr::from(address))
}

// --- Classification (Surfguard.blocked_address?, default policy) ---------------------------------

type V4Range = (u32, u8);
type V6Range = (u128, u8);

const fn v4(a: u8, b: u8, c: u8, d: u8, prefix: u8) -> V4Range {
    (u32::from_be_bytes([a, b, c, d]), prefix)
}

const fn v6(segments: [u16; 8], prefix: u8) -> V6Range {
    let mut value: u128 = 0;
    let mut i = 0;
    while i < 8 {
        value = (value << 16) | segments[i] as u128;
        i += 1;
    }
    (value, prefix)
}

/// `Surfguard::DISALLOWED_IPV4`
const DISALLOWED_IPV4: &[V4Range] = &[
    v4(0, 0, 0, 0, 8),
    v4(10, 0, 0, 0, 8),
    v4(100, 64, 0, 0, 10),
    v4(127, 0, 0, 0, 8),
    v4(169, 254, 0, 0, 16),
    v4(172, 16, 0, 0, 12),
    v4(192, 0, 0, 0, 24),
    v4(192, 0, 2, 0, 24),
    v4(192, 88, 99, 0, 24),
    v4(192, 168, 0, 0, 16),
    v4(198, 18, 0, 0, 15),
    v4(198, 51, 100, 0, 24),
    v4(203, 0, 113, 0, 24),
    v4(224, 0, 0, 0, 4),
    v4(240, 0, 0, 0, 4),
];

/// `Surfguard::DISALLOWED_IPV6`
const DISALLOWED_IPV6: &[V6Range] = &[
    v6([0, 0, 0, 0, 0, 0, 0, 0], 128),
    v6([0x100, 0, 0, 0, 0, 0, 0, 0], 64),
    v6([0x2001, 0, 0, 0, 0, 0, 0, 0], 32),
    v6([0x2001, 2, 0, 0, 0, 0, 0, 0], 48),
    v6([0x2001, 0xdb8, 0, 0, 0, 0, 0, 0], 32),
    v6([0x2002, 0, 0, 0, 0, 0, 0, 0], 16),
    v6([0xfec0, 0, 0, 0, 0, 0, 0, 0], 10),
    v6([0xff00, 0, 0, 0, 0, 0, 0, 0], 8),
];

const NAT64_WELL_KNOWN: V6Range = v6([0x64, 0xff9b, 0, 0, 0, 0, 0, 0], 96);
const NAT64_LOCAL_USE: V6Range = v6([0x64, 0xff9b, 1, 0, 0, 0, 0, 0], 48);
const IPV4_MAPPED: V6Range = v6([0, 0, 0, 0, 0, 0xffff, 0, 0], 96);
const IPV4_TRANSLATABLE: V6Range = v6([0, 0, 0, 0, 0xffff, 0, 0, 0], 96);
const IPV4_COMPATIBLE: V6Range = v6([0, 0, 0, 0, 0, 0, 0, 0], 96);
const UNIQUE_LOCAL: V6Range = v6([0xfc00, 0, 0, 0, 0, 0, 0, 0], 7);
const LINK_LOCAL_V6: V6Range = v6([0xfe80, 0, 0, 0, 0, 0, 0, 0], 10);

fn in_v4(ip: u32, (network, prefix): V4Range) -> bool {
    prefix == 0 || (ip ^ network) >> (32 - prefix as u32) == 0
}

fn in_v6(ip: u128, (network, prefix): V6Range) -> bool {
    prefix == 0 || (ip ^ network) >> (128 - prefix as u32) == 0
}

/// `Surfguard.blocked_address?(ip)`
pub fn blocked_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => disallowed_ipv4(u32::from(ip)),
        IpAddr::V6(ip) => {
            let ip = u128::from(ip);
            if in_v6(ip, IPV4_MAPPED) || in_v6(ip, IPV4_COMPATIBLE) || in_v6(ip, NAT64_LOCAL_USE) {
                true
            } else if in_v6(ip, NAT64_WELL_KNOWN) || in_v6(ip, IPV4_TRANSLATABLE) {
                disallowed_ipv4(ip as u32)
            } else {
                disallowed_ipv6(ip)
            }
        }
    }
}

/// `disallowed_ipv4?`: `private?`, `loopback?` and `link_local?` are all inside the list.
fn disallowed_ipv4(ip: u32) -> bool {
    DISALLOWED_IPV4.iter().any(|range| in_v4(ip, *range))
}

fn disallowed_ipv6(ip: u128) -> bool {
    in_v6(ip, UNIQUE_LOCAL) || ip == 1 || in_v6(ip, LINK_LOCAL_V6) || DISALLOWED_IPV6.iter().any(|range| in_v6(ip, *range))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::test_support::FakeResolver;

    fn blocked(ip: &str) -> bool {
        blocked_address(ip.parse().unwrap())
    }

    #[test]
    fn classifies_addresses_like_surfguard() {
        for ip in [
            "0.0.0.0",
            "10.1.2.3",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.0.0",
            "172.31.255.255",
            "192.0.0.8",
            "192.0.2.1",
            "192.88.99.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "240.0.0.1",
            "255.255.255.255",
            "::",
            "::1",
            "::ffff:192.168.1.1",
            "::ffff:8.8.8.8",
            "::8.8.8.8",
            "64:ff9b::a00:1",
            "64:ff9b:1::1",
            "::ffff:0:a00:1",
            "fc00::1",
            "fd00::1",
            "fe80::1",
            "fec0::1",
            "ff02::1",
            "2001::1",
            "2001:db8::1",
            "2002::1",
            "100::1",
            "2001:2::1",
        ] {
            assert!(blocked(ip), "{ip} should be blocked");
        }
        for ip in [
            "168.63.129.16",
            "3fff::1",
            "5f00::1",
            "4000::1",
            "2001:10::1",
            "8.8.8.8",
            "1.1.1.1",
            "93.184.216.34",
            "142.250.185.206",
            "172.32.0.1",
            "100.128.0.1",
            "192.0.1.1",
            "2606:2800:220:1:248:1893:25c8:1946",
            "2a00:1450:4001:82a::200e",
            "2001:3::1",
            "2001:4:112::1",
            "64:ff9b::808:808",
            "::ffff:0:808:808",
            "2c0f:ffff::1",
        ] {
            assert!(!blocked(ip), "{ip} should be public");
        }
    }

    #[test]
    fn inet_aton_forms() {
        assert_eq!(inet_aton("127.1"), Some(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(inet_aton("0x7f.1"), Some(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(inet_aton("2130706433"), Some(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(inet_aton("0177.0.0.01"), Some(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(inet_aton("10.0.258"), Some(Ipv4Addr::new(10, 0, 1, 2)));
        assert_eq!(inet_aton("09.1.1.1"), None);
        assert_eq!(inet_aton("256.1.1.1"), None);
        assert_eq!(inet_aton("1.2.3.4."), None);
        assert_eq!(inet_aton("www.example.com"), None);
    }

    #[tokio::test]
    async fn resolves_hosts_like_the_private_network_guard() {
        let resolver = FakeResolver::new([
            ("www.example.com", vec!["93.184.216.34"]),
            ("mixed.example", vec!["10.0.0.1", "2606:2800:220:1:248:1893:25c8:1946", "::1", "93.184.216.39"]),
            ("private.example", vec!["192.168.1.10"]),
        ]);
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();
        assert_eq!(resolve(&resolver, "www.example.com").await, Ok(ip("93.184.216.34")));
        assert_eq!(
            resolve_public_ips(&resolver, "mixed.example").await,
            Ok(vec![ip("93.184.216.39"), ip("2606:2800:220:1:248:1893:25c8:1946")])
        );
        assert_eq!(resolve(&resolver, "private.example").await, Err(GuardError::Violation("private.example".into())));
        assert_eq!(resolve(&resolver, "nowhere.example").await, Err(GuardError::Unresolvable));
        assert_eq!(resolve(&resolver, "8.8.8.8").await, Ok(ip("8.8.8.8")));
        assert_eq!(resolve(&resolver, "[2606:2800:220:1:248:1893:25c8:1946]").await, Ok(ip("2606:2800:220:1:248:1893:25c8:1946")));
        for host in ["127.0.0.1", "0x7f.1", "2130706433", "[::1]", "::1", "[fd00::1]"] {
            assert!(matches!(resolve(&resolver, host).await, Err(GuardError::Violation(_))), "{host:?}");
        }
        assert_eq!(resolver.lookups(), vec!["www.example.com", "mixed.example", "private.example", "nowhere.example"]);
    }
}

#[cfg(test)]
mod ws15e_tests {
    use super::*;
    use crate::integrations::test_support::FakeResolver;
    use serde_json::{Value, json};

    #[tokio::test]
    async fn ws15e_matches_our_rails_guard_corpus() {
        let vectors: Value = serde_json::from_str(include_str!("../../../../../vectors/ws15e_embeds.json")).unwrap();
        let mut failures = Vec::new();
        for case in vectors["addresses"].as_array().unwrap() {
            let address = case["address"].as_str().unwrap();
            let actual = address.parse().map(blocked_address).unwrap_or(true);
            if actual != case["blocked"].as_bool().unwrap() {
                failures.push(format!("address {address}: {actual}"));
            }
        }
        for case in vectors["hosts"].as_array().unwrap() {
            let host = case["host"].as_str().unwrap();
            let answers = case["answers"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
            let resolver = FakeResolver::new([(host, answers)]);
            let actual = match resolve(&resolver, host).await {
                Ok(ip) => json!({"ip": ip.to_string()}),
                Err(GuardError::Violation(_)) => json!({"error": "violation"}),
                Err(GuardError::Unresolvable) => json!({"error": "unresolvable"}),
            };
            if actual != case["result"] || json!(resolver.lookups()) != case["lookups"] {
                failures.push(format!(
                    "host {host:?}: {actual}, lookups {:?}; expected {} {}",
                    resolver.lookups(),
                    case["result"],
                    case["lookups"]
                ));
            }
        }
        assert!(failures.is_empty(), "{} guard differences:\n{}", failures.len(), failures.join("\n"));
    }
}
