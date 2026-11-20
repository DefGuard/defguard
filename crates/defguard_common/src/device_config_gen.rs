use ipnetwork::IpNetwork;

use super::db::{Id, models::WireguardNetwork};
use crate::{csv::AsCsv, db::models::device::WireguardNetworkDevice};

/// Create a WireGuard INI-format config string for a device in a network.
#[must_use]
pub fn create_wireguard_config(
    network: &WireguardNetwork<Id>,
    wireguard_network_device: &WireguardNetworkDevice,
    allowed_ips: &[IpNetwork],
    has_preshared_key: bool,
) -> String {
    let dns = match &network.dns {
        Some(dns) if !dns.is_empty() => format!("DNS = {dns}"),
        _ => String::new(),
    };

    let allowed_ips_line = if allowed_ips.is_empty() {
        String::new()
    } else {
        format!(
            "AllowedIPs = {}\n",
            allowed_ips
                .iter()
                .map(IpNetwork::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )
    };

    let preshared_key_line = if has_preshared_key {
        "PresharedKey = YOUR_PRESHARED_KEY\n"
    } else {
        ""
    };

    format!(
        "[Interface]\n\
        PrivateKey = YOUR_PRIVATE_KEY\n\
        Address = {}\n\
        {dns}\n\
        \n\
        [Peer]\n\
        PublicKey = {}\n\
        {preshared_key_line}\
        {allowed_ips_line}\
        Endpoint = {}:{}\n\
        PersistentKeepalive = {}",
        wireguard_network_device.wireguard_ips.as_csv(),
        network.pubkey,
        network.endpoint,
        network.port,
        network.keepalive_interval,
    )
}
