use gateway_core::{GatewayError, GatewayResult};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cidr {
    network: IpAddr,
    prefix: u8,
}

impl Cidr {
    pub fn parse(value: &str) -> GatewayResult<Self> {
        let (address, prefix) = value
            .split_once('/')
            .ok_or_else(|| GatewayError::Configuration(format!("invalid CIDR: {value}")))?;
        let network: IpAddr = address
            .parse()
            .map_err(|_| GatewayError::Configuration(format!("invalid CIDR address: {value}")))?;
        let prefix: u8 = prefix
            .parse()
            .map_err(|_| GatewayError::Configuration(format!("invalid CIDR prefix: {value}")))?;
        let valid = match network {
            IpAddr::V4(_) => prefix <= 32,
            IpAddr::V6(_) => prefix <= 128,
        };
        if !valid {
            return Err(GatewayError::Configuration(format!(
                "CIDR prefix out of range: {value}"
            )));
        }
        Ok(Self { network, prefix })
    }

    pub fn contains(&self, candidate: IpAddr) -> bool {
        match (self.network, candidate) {
            (IpAddr::V4(network), IpAddr::V4(candidate)) => {
                let bits = self.prefix;
                let mask = if bits == 0 {
                    0
                } else {
                    u32::MAX << (32 - bits)
                };
                (u32::from(network) & mask) == (u32::from(candidate) & mask)
            }
            (IpAddr::V6(network), IpAddr::V6(candidate)) => {
                let bits = self.prefix;
                let mask = if bits == 0 {
                    0
                } else {
                    u128::MAX << (128 - bits)
                };
                (u128::from(network) & mask) == (u128::from(candidate) & mask)
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AccountNetworkPolicy {
    private_allowlist: Vec<Cidr>,
}

impl AccountNetworkPolicy {
    pub fn new(private_allowlist: Vec<Cidr>) -> Self {
        Self { private_allowlist }
    }

    pub fn from_allowlist(values: &[String]) -> GatewayResult<Self> {
        let private_allowlist = values
            .iter()
            .map(|value| Cidr::parse(value))
            .collect::<GatewayResult<Vec<_>>>()?;
        Ok(Self::new(private_allowlist))
    }

    pub fn validate_resolved(&self, addresses: &[IpAddr]) -> GatewayResult<()> {
        if addresses.is_empty() {
            return Err(GatewayError::InvalidRequest(
                "mail server hostname did not resolve".into(),
            ));
        }
        for address in addresses {
            if is_forbidden_destination(*address)
                && !self
                    .private_allowlist
                    .iter()
                    .any(|cidr| cidr.contains(*address))
            {
                return Err(GatewayError::PermissionDenied(
                    "mail server destination is blocked by network policy".into(),
                ));
            }
        }
        Ok(())
    }
}

pub fn is_forbidden_destination(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => forbidden_v4(address),
        IpAddr::V6(address) => forbidden_v6(address),
    }
}

fn forbidden_v4(ip: Ipv4Addr) -> bool {
    let value = u32::from(ip);
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_unspecified()
        || in_v4(value, Ipv4Addr::new(0, 0, 0, 0), 8)
        || in_v4(value, Ipv4Addr::new(100, 64, 0, 0), 10)
        || in_v4(value, Ipv4Addr::new(198, 18, 0, 0), 15)
        || in_v4(value, Ipv4Addr::new(240, 0, 0, 0), 4)
}

fn forbidden_v6(ip: Ipv6Addr) -> bool {
    let value = u128::from(ip);
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || in_v6(value, "fe80::".parse().expect("static IPv6"), 10)
        || in_v6(value, "fc00::".parse().expect("static IPv6"), 7)
}

fn in_v4(value: u32, network: Ipv4Addr, prefix: u8) -> bool {
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    (value & mask) == (u32::from(network) & mask)
}

fn in_v6(value: u128, network: Ipv6Addr, prefix: u8) -> bool {
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    (value & mask) == (u128::from(network) & mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_and_metadata_like_destinations_fail_closed() {
        for value in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "::1",
            "fe80::1",
            "fd00::1",
        ] {
            assert!(is_forbidden_destination(value.parse().unwrap()), "{value}");
        }
        assert!(!is_forbidden_destination("8.8.8.8".parse().unwrap()));
        assert!(!is_forbidden_destination(
            "2606:4700:4700::1111".parse().unwrap()
        ));
    }

    #[test]
    fn explicit_private_allowlist_is_narrow() {
        let policy = AccountNetworkPolicy::from_allowlist(&["10.20.0.0/16".into()]).unwrap();
        assert!(policy
            .validate_resolved(&["10.20.1.2".parse().unwrap()])
            .is_ok());
        assert!(policy
            .validate_resolved(&["10.21.1.2".parse().unwrap()])
            .is_err());
        // All resolved addresses must be acceptable; one poisoned DNS answer blocks the host.
        assert!(policy
            .validate_resolved(&["8.8.8.8".parse().unwrap(), "127.0.0.1".parse().unwrap(),])
            .is_err());
    }
}
