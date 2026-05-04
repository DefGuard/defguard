use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::Url;

use ipnetwork::IpNetwork;
use serde::Serialize;

/// Strip any `http://` or `https://` scheme prefix a user may have accidentally
/// included in a hostname/IP field that expects a bare host, not a URL.
#[must_use]
pub fn strip_scheme(s: &str) -> &str {
    s.strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(s)
}

/// Validates that `s` is a bare hostname or IP address with no embedded port,
/// path, query string, or fragment. Intended for the `ip_or_domain` fields of
/// gateway and proxy setup requests, where the port is supplied separately.
pub fn validate_host_only(s: &str) -> Result<(), String> {
    let test_url = format!("http://{s}/");
    let url = Url::parse(&test_url)
        .map_err(|_| format!("'{s}' is not a valid hostname or IP address"))?;
    if url.host_str().is_none() {
        return Err(format!("'{s}' is not a valid hostname or IP address"));
    }
    if url.port().is_some() {
        return Err(format!(
            "'{s}' must not include a port; provide the port in the grpc_port field"
        ));
    }
    if url.path() != "/" {
        return Err(format!("'{s}' must not include a path component"));
    }
    if url.query().is_some() {
        return Err(format!("'{s}' must not include a query string"));
    }
    if url.fragment().is_some() {
        return Err(format!("'{s}' must not include a fragment"));
    }
    Ok(())
}

/// Parse a string with comma-separated IP addresses.
/// Invalid addresses will be silently ignored.
#[must_use]
pub fn parse_address_list(ips: &str) -> Vec<IpNetwork> {
    ips.split(',')
        .filter_map(|ip| ip.trim().parse().ok())
        .collect()
}

/// Parse a string with comma-separated IP network addresses.
/// Host bits will be stripped.
/// Invalid addresses will be silently ignored.
#[must_use]
pub fn parse_network_address_list(ips: &str) -> Vec<IpNetwork> {
    ips.split(',')
        .filter_map(|ip| ip.trim().parse().ok())
        .filter_map(|ip: IpNetwork| {
            let network_address = ip.network();
            let network_mask = ip.mask();
            IpNetwork::with_netmask(network_address, network_mask).ok()
        })
        .collect()
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SplitIp {
    network_part: String,
    modifiable_part: String,
    network_prefix: String,
    ip: String,
}

/// Splits the IP address (IPv4 or IPv6) into three parts: network part, modifiable part and prefix
/// The network part is the part that can't be changed by the user.
/// This is to display an IP address in the UI like this: 192.168.(1.1)/16, where the part in the parenthesis can be changed by the user.
/// The algorithm works as follows:
/// 1. Get the network address, last address and IP address segments, e.g. 192.1.1.1 would be [192, 1, 1, 1]
/// 2. Iterate over the segments and compare the last address and network segments, as long as the current segments are equal, append the segment to the network part.
///    If they are not equal, we found the first modifiable segment (one of the segments of an address that may change between hosts in the same network),
///    append the rest of the segments to the modifiable part.
/// 3. Join the segments with the delimiter and return the network part, modifiable part and the network prefix
#[must_use]
pub fn split_ip(ip: &IpAddr, network: &IpNetwork) -> SplitIp {
    let network_addr = network.network();
    let network_prefix = network.prefix();

    let ip_segments = match ip {
        IpAddr::V4(ip) => ip.octets().iter().map(|x| u16::from(*x)).collect(),
        IpAddr::V6(ip) => ip.segments().to_vec(),
    };

    let last_addr_segments = match network {
        IpNetwork::V4(net) => {
            let last_ip = u32::from(net.ip()) | (!u32::from(net.mask()));
            let last_ip: Ipv4Addr = last_ip.into();
            last_ip.octets().iter().map(|x| u16::from(*x)).collect()
        }
        IpNetwork::V6(net) => {
            let last_ip = u128::from(net.ip()) | (!u128::from(net.mask()));
            let last_ip: Ipv6Addr = last_ip.into();
            last_ip.segments().to_vec()
        }
    };

    let network_segments = match network_addr {
        IpAddr::V4(ip) => ip.octets().iter().map(|x| u16::from(*x)).collect(),
        IpAddr::V6(ip) => ip.segments().to_vec(),
    };

    let mut network_part = String::new();
    let mut modifiable_part = String::new();
    let delimiter = if ip.is_ipv4() { "." } else { ":" };
    let formatter = |x: &u16| {
        if ip.is_ipv4() {
            x.to_string()
        } else {
            format!("{x:04x}")
        }
    };

    for (i, ((last_addr_segment, network_segment), ip_segment)) in last_addr_segments
        .iter()
        .zip(network_segments.iter())
        .zip(ip_segments.iter())
        .enumerate()
    {
        if last_addr_segment != network_segment {
            let parts = ip_segments.split_at(i).1;
            let joined = parts
                .iter()
                .map(formatter)
                .collect::<Vec<String>>()
                .join(delimiter);
            modifiable_part.push_str(&joined);
            break;
        }
        let formatted = formatter(ip_segment);
        network_part.push_str(&formatted);
        network_part.push_str(delimiter);
    }

    SplitIp {
        ip: ip.to_string(),
        network_part,
        modifiable_part,
        network_prefix: network_prefix.to_string(),
    }
}

#[cfg(test)]
mod test {
    use super::{split_ip, validate_host_only};
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_ip_splitter() {
        let net = split_ip(
            &IpAddr::from_str("192.168.3.1").unwrap(),
            &IpNetwork::from_str("192.168.3.1/30").unwrap(),
        );

        assert_eq!(net.network_part, "192.168.3.");
        assert_eq!(net.modifiable_part, "1");
        assert_eq!(net.network_prefix, "30");

        let net = split_ip(
            &IpAddr::from_str("192.168.5.7").unwrap(),
            &IpNetwork::from_str("192.168.3.1/24").unwrap(),
        );

        assert_eq!(net.network_part, "192.168.5.");
        assert_eq!(net.modifiable_part, "7");
        assert_eq!(net.network_prefix, "24");

        let net = split_ip(
            &IpAddr::from_str("2001:0db8:85a3::8a2e:0370:7334").unwrap(),
            &IpNetwork::from_str("2001:0db8:85a3::8a2e:0370:7334/64").unwrap(),
        );

        assert_eq!(net.network_part, "2001:0db8:85a3:0000:");
        assert_eq!(net.modifiable_part, "0000:8a2e:0370:7334");
        assert_eq!(net.network_prefix, "64");

        let net = split_ip(
            &IpAddr::from_str("2001:0db8::0010:8a2e:0370:aaaa").unwrap(),
            &IpNetwork::from_str("2001:db8::10:8a2e:370:aaa8/125").unwrap(),
        );

        assert_eq!(net.network_part, "2001:0db8:0000:0000:0010:8a2e:0370:");
        assert_eq!(net.modifiable_part, "aaaa");
        assert_eq!(net.network_prefix, "125");
    }

    #[test]
    fn test_validate_host_only_accepts_bare_ipv4() {
        assert!(validate_host_only("192.168.1.1").is_ok());
        assert!(validate_host_only("10.0.0.1").is_ok());
    }

    #[test]
    fn test_validate_host_only_accepts_bare_ipv6() {
        assert!(validate_host_only("[::1]").is_ok());
        assert!(validate_host_only("[2001:db8::1]").is_ok());
    }

    #[test]
    fn test_validate_host_only_accepts_bare_hostname() {
        assert!(validate_host_only("gateway.example.com").is_ok());
        assert!(validate_host_only("my-gateway").is_ok());
        assert!(validate_host_only("edge01.vpn.internal").is_ok());
    }

    #[test]
    fn test_validate_host_only_rejects_embedded_port() {
        let err = validate_host_only("192.168.1.1:4444").unwrap_err();
        assert!(err.contains("must not include a port"), "got: {err}");

        let err = validate_host_only("gateway.example.com:8443").unwrap_err();
        assert!(err.contains("must not include a port"), "got: {err}");
    }

    #[test]
    fn test_validate_host_only_rejects_path() {
        let err = validate_host_only("192.168.1.1/testpath").unwrap_err();
        assert!(err.contains("must not include a path"), "got: {err}");
    }

    #[test]
    fn test_validate_host_only_rejects_query_string() {
        // A bare query string without a path separator is not reachable via URL
        // parsing in practice, but a port+path+query is.
        let err = validate_host_only("192.168.1.1:4444/path?a=b").unwrap_err();
        assert!(err.contains("must not"), "got: {err}");
    }

    #[test]
    fn dg26_11_test_validate_host_only_rejects_fragment() {
        // Fragment is the key part of the DG26-11 PoC: it absorbs the grpc_port.
        let err = validate_host_only("192.168.1.1/path?a=b#").unwrap_err();
        assert!(err.contains("must not"), "got: {err}");
    }

    #[test]
    fn dg26_11_test_validate_host_only_rejects_poc_input() {
        // Exact input from the DG26-11 audit report (after URL-decode of %23 -> #).
        let err = validate_host_only("46.101.217.165:4444/testpath?a=b#").unwrap_err();
        assert!(err.contains("must not"), "got: {err}");
    }
}
