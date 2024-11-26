use std::{array::TryFromSliceError, net::IpAddr};

use base64::{prelude::BASE64_STANDARD, DecodeError, Engine};
use ipnetwork::{IpNetwork, IpNetworkError};
use thiserror::Error;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::{
    db::{
        models::wireguard::{
            WireguardNetworkError, DEFAULT_DISCONNECT_THRESHOLD, DEFAULT_KEEPALIVE_INTERVAL,
        },
        Device, WireguardNetwork,
    },
    KEY_LENGTH,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportedDevice {
    pub user_id: Option<i64>,
    pub name: String,
    pub wireguard_pubkey: String,
    pub wireguard_ip: IpAddr,
}

#[derive(Debug, Error)]
pub(crate) enum WireguardConfigParseError {
    #[error(transparent)]
    ParseError(#[from] ini::ParseError),
    #[error("Config section not found: {0}")]
    SectionNotFound(&'static str),
    #[error("Config key not found: {0}")]
    KeyNotFound(&'static str),
    #[error("Invalid IP error")]
    InvalidIp(#[from] IpNetworkError),
    #[error("Invalid peer IP: {0}")]
    InvalidPeerIp(IpAddr),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Invalid port: {0}")]
    InvalidPort(String),
    #[error("Missing interface network address")]
    MissingAddress,
    #[error("WireGuard network error")]
    NetworkError(#[from] WireguardNetworkError),
}

impl From<TryFromSliceError> for WireguardConfigParseError {
    fn from(e: TryFromSliceError) -> Self {
        WireguardConfigParseError::InvalidKey(format!("{e}"))
    }
}

impl From<DecodeError> for WireguardConfigParseError {
    fn from(e: DecodeError) -> Self {
        WireguardConfigParseError::InvalidKey(format!("{e}"))
    }
}

pub(crate) fn parse_wireguard_config(
    config: &str,
) -> Result<(WireguardNetwork, Vec<ImportedDevice>), WireguardConfigParseError> {
    let config = ini::Ini::load_from_str(config)?;
    // Parse WireGuardNetwork
    let interface_section = config
        .section(Some("Interface"))
        .ok_or_else(|| WireguardConfigParseError::SectionNotFound("Interface"))?;
    let prvkey = interface_section
        .get("PrivateKey")
        .ok_or_else(|| WireguardConfigParseError::KeyNotFound("PrivateKey"))?;
    let prvkey_bytes: [u8; KEY_LENGTH] = BASE64_STANDARD
        .decode(prvkey.as_bytes())?
        .try_into()
        .map_err(|_| WireguardConfigParseError::InvalidKey(prvkey.to_string()))?;
    let pubkey =
        BASE64_STANDARD.encode(PublicKey::from(&StaticSecret::from(prvkey_bytes)).to_bytes());
    let address = interface_section
        .get("Address")
        .ok_or_else(|| WireguardConfigParseError::KeyNotFound("Address"))?;
    let port = interface_section
        .get("ListenPort")
        .ok_or_else(|| WireguardConfigParseError::KeyNotFound("ListenPort"))?;
    let port = port
        .parse()
        .map_err(|_| WireguardConfigParseError::InvalidPort(port.to_string()))?;
    let dns = interface_section.get("DNS").map(ToString::to_string);
    let mut addresses: Vec<IpNetwork> = Vec::new();
    for addr in address.split(',') {
        match addr.parse() {
            Ok(ip) => addresses.push(ip),
            Err(err) => return Err(WireguardConfigParseError::InvalidIp(err)),
        }
    }
    // Require at least one IP address.
    let Some(network_address) = addresses.first() else {
        return Err(WireguardConfigParseError::MissingAddress);
    };
    let allowed_ips = IpNetwork::new(network_address.network(), network_address.prefix())?;
    let network_address = *network_address;
    let mut network = WireguardNetwork::new(
        pubkey.clone(),
        addresses,
        port,
        String::new(),
        dns,
        vec![allowed_ips],
        false,
        DEFAULT_KEEPALIVE_INTERVAL,
        DEFAULT_DISCONNECT_THRESHOLD,
    )?;
    network.pubkey = pubkey;
    network.prvkey = prvkey.to_string();

    // Parse Devices
    let peer_sections = config.section_all(Some("Peer"));

    let mut devices = Vec::new();
    for peer in peer_sections {
        let ip = peer
            .get("AllowedIPs")
            .ok_or_else(|| WireguardConfigParseError::KeyNotFound("AllowedIPs"))?;
        let ip_network: IpNetwork = ip.parse()?;
        let ip = ip_network.ip();

        // check if assigned IP collides with gateway IP
        let net_ip = network_address.ip();
        let net_network = network_address.network();
        let net_broadcast = network_address.broadcast();
        if ip == net_ip || ip == net_network || ip == net_broadcast {
            return Err(WireguardConfigParseError::InvalidPeerIp(ip));
        }

        let pubkey = peer
            .get("PublicKey")
            .ok_or_else(|| WireguardConfigParseError::KeyNotFound("PublicKey"))?;
        Device::validate_pubkey(pubkey).map_err(WireguardConfigParseError::InvalidKey)?;

        // check if device pubkey collides with network pubkey
        if pubkey == network.pubkey {
            return Err(WireguardConfigParseError::InvalidKey(format!(
                "Device pubkey is the same as network pubkey {pubkey}"
            )));
        }

        devices.push(ImportedDevice {
            user_id: None,
            name: pubkey.to_string(),
            wireguard_pubkey: pubkey.to_string(),
            wireguard_ip: ip,
        });
    }

    Ok((network, devices))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::NoId;

    #[test]
    fn test_parse_config() {
        let config = "
            [Interface]
            PrivateKey = GAA2X3DW0WakGVx+DsGjhDpTgg50s1MlmrLf24Psrlg=
            Address = 10.0.0.1/24
            ListenPort = 55055
            DNS = 10.0.0.2

            [Peer]
            PublicKey = 2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE=
            AllowedIPs = 10.0.0.10/24
            PersistentKeepalive = 300

            [Peer]
            PublicKey = OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0=
            AllowedIPs = 10.0.0.11/24
            PersistentKeepalive = 300
        ";
        let (network, devices) = parse_wireguard_config(config).unwrap();
        assert_eq!(
            network.prvkey,
            "GAA2X3DW0WakGVx+DsGjhDpTgg50s1MlmrLf24Psrlg="
        );
        assert_eq!(network.id, NoId);
        assert_eq!(network.name, "Y5ewP5RXstQd71gkmS/M0xL8wi0yVbbVY/ocLM4cQ1Y=");
        assert_eq!(network.address, vec!["10.0.0.1/24".parse().unwrap()]);
        assert_eq!(network.port, 55055);
        assert_eq!(
            network.pubkey,
            "Y5ewP5RXstQd71gkmS/M0xL8wi0yVbbVY/ocLM4cQ1Y="
        );
        assert_eq!(
            network.prvkey,
            "GAA2X3DW0WakGVx+DsGjhDpTgg50s1MlmrLf24Psrlg="
        );
        assert_eq!(network.endpoint, "");
        assert_eq!(network.dns, Some("10.0.0.2".to_string()));
        assert_eq!(network.allowed_ips, vec!["10.0.0.0/24".parse().unwrap()]);
        assert_eq!(network.connected_at, None);

        assert_eq!(devices.len(), 2);

        let device1 = &devices[0];
        assert_eq!(
            device1.wireguard_pubkey,
            "2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE="
        );
        assert_eq!(device1.wireguard_ip.to_string(), "10.0.0.10");

        let device2 = &devices[1];
        assert_eq!(
            device2.wireguard_pubkey,
            "OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0="
        );
        assert_eq!(device2.wireguard_ip.to_string(), "10.0.0.11");
    }
}
