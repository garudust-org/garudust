use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

use crate::error::ToolError;

/// Hostnames that are always blocked regardless of IP resolution.
static BLOCKED_HOSTS: &[&str] = &[
    "169.254.169.254", // AWS / Azure / GCP IMDS
    "metadata.google.internal",
    "metadata.google",
    "169.254.170.2", // ECS task metadata
    "fd00:ec2::254", // AWS IPv6 IMDS
];

/// Validate that a URL is safe to fetch (not SSRF-able).
///
/// Blocks: private IPs, loopback, link-local, unspecified, cloud metadata.
///
/// # DNS TOCTOU note
/// This checks the IP at call time. `reqwest` re-resolves when connecting, so a
/// DNS-rebinding attack can change the record between check and connect. Full
/// mitigation requires a custom connector — accepted limitation for now.
pub fn is_safe_url(url: &str) -> Result<(), ToolError> {
    // Parse scheme + host manually using basic string splitting (no extra dep in core).
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| ToolError::Execution("only http/https URLs are allowed".into()))?;

    // Extract host (strip path/query/fragment, handle IPv6 brackets)
    let host_port = without_scheme.split('/').next().unwrap_or(without_scheme);

    let host = if host_port.starts_with('[') {
        // IPv6 literal: [::1]:port  or  [::1]
        host_port
            .trim_start_matches('[')
            .split(']')
            .next()
            .unwrap_or("")
    } else {
        // Strip port if present
        host_port.split(':').next().unwrap_or(host_port)
    };

    if host.is_empty() {
        return Err(ToolError::Execution("URL has no host".into()));
    }

    // Check blocked hostname list
    let host_lower = host.to_lowercase();
    for blocked in BLOCKED_HOSTS {
        if host_lower == *blocked {
            return Err(ToolError::Execution(format!(
                "blocked: '{host}' is a cloud metadata endpoint"
            )));
        }
    }

    // Resolve host to IPs and check each one
    let addrs = format!("{host}:80")
        .to_socket_addrs()
        .map_err(|e| ToolError::Execution(format!("could not resolve host '{host}': {e}")))?;

    for addr in addrs {
        check_ip(addr.ip(), host)?;
    }

    Ok(())
}

/// Returns `true` if the IP is private, loopback, link-local, or unspecified
/// and must therefore be blocked to prevent SSRF.
pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            is_private_v4(v4) || v4.is_loopback() || v4.is_unspecified() || v4.is_link_local()
        }
        IpAddr::V6(v6) => is_private_v6(v6) || v6.is_loopback() || v6.is_unspecified(),
    }
}

fn check_ip(ip: IpAddr, host: &str) -> Result<(), ToolError> {
    if is_blocked_ip(ip) {
        return Err(ToolError::Execution(format!(
            "blocked: '{host}' resolves to a private/reserved address ({ip})"
        )));
    }
    Ok(())
}

fn is_private_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    // 10.0.0.0/8
    o[0] == 10
    // 172.16.0.0/12
    || (o[0] == 172 && (16..=31).contains(&o[1]))
    // 192.168.0.0/16
    || (o[0] == 192 && o[1] == 168)
    // 100.64.0.0/10 (carrier-grade NAT / RFC 6598)
    || (o[0] == 100 && (64..=127).contains(&o[1]))
}

fn is_private_v6(ip: Ipv6Addr) -> bool {
    let segs = ip.segments();
    // fc00::/7 — unique local
    (segs[0] & 0xfe00) == 0xfc00
    // fe80::/10 — link-local
    || (segs[0] & 0xffc0) == 0xfe80
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_scheme() {
        assert!(is_safe_url("ftp://example.com").is_err());
        assert!(is_safe_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn rejects_empty_host() {
        assert!(is_safe_url("http://").is_err());
    }

    #[test]
    fn rejects_loopback() {
        assert!(is_safe_url("http://127.0.0.1/anything").is_err());
        assert!(is_safe_url("http://localhost/anything").is_err());
    }

    #[test]
    fn rejects_private_ipv4_ranges() {
        assert!(is_safe_url("http://10.0.0.1").is_err());
        assert!(is_safe_url("http://172.16.0.1").is_err());
        assert!(is_safe_url("http://192.168.1.1").is_err());
        assert!(is_safe_url("http://100.64.0.1").is_err());
    }

    #[test]
    fn rejects_cloud_metadata_hosts() {
        assert!(is_safe_url("http://169.254.169.254/latest/meta-data/").is_err());
        assert!(is_safe_url("http://metadata.google.internal/computeMetadata/v1/").is_err());
    }

    #[test]
    fn is_private_v4_covers_all_ranges() {
        assert!(is_private_v4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_private_v4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_private_v4(Ipv4Addr::new(172, 31, 255, 255)));
        assert!(is_private_v4(Ipv4Addr::new(192, 168, 0, 1)));
        assert!(is_private_v4(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(!is_private_v4(Ipv4Addr::new(8, 8, 8, 8)));
        assert!(!is_private_v4(Ipv4Addr::new(172, 15, 0, 1)));
        assert!(!is_private_v4(Ipv4Addr::new(172, 32, 0, 1)));
    }

    #[test]
    fn is_private_v6_covers_ranges() {
        // fc00::/7 unique local
        assert!(is_private_v6("fc00::1".parse().unwrap()));
        assert!(is_private_v6("fd00::1".parse().unwrap()));
        // fe80::/10 link-local
        assert!(is_private_v6("fe80::1".parse().unwrap()));
        // public
        assert!(!is_private_v6("2001:db8::1".parse().unwrap()));
    }
}
