use ipnetwork::IpNetwork;

use crate::{csv::AsCsv, db::models::device::WireguardNetworkDevice};

use super::db::{Id, models::WireguardNetwork};

/// Create a WireGuard INI-format config string for a device in a network.
#[must_use]
pub fn create_wireguard_config(
    network: &WireguardNetwork<Id>,
    wireguard_network_device: &WireguardNetworkDevice,
    allowed_ips: &[IpNetwork],
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

    format!(
        "[Interface]\n\
        PrivateKey = YOUR_PRIVATE_KEY\n\
        Address = {}\n\
        {dns}\n\
        \n\
        [Peer]\n\
        PublicKey = {}\n\
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
