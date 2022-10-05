use std::net::Ipv4Addr;
use std::ops::RangeInclusive;
use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetworkParseError {
    #[error("MissingAddress")]
    MissingAddress,
    #[error("BadIpAddress")]
    BadIpAddress,
    #[error("BadNetmask")]
    BadNetmask,
    #[error("InvalidPortNumber")]
    InvalidPortNumber,
}

/**
Parse IP addresses with and without subnet masks
Examples:
 192.168.1.1
 192.168.1.1/24
 */
pub fn expand_hosts(host_spec: &str) -> Result<HostIpRange, NetworkParseError> {
    let (addr, mask) = address_and_netmask_from_str(host_spec)?;
    Ok(expand_hosts_with_netmask(addr, mask))
}


fn address_and_netmask_from_str(host_spec: &str) -> Result<(Ipv4Addr, u32), NetworkParseError> {
    if host_spec.is_empty() {
        return Err(NetworkParseError::MissingAddress);
    }

    let (ip, mask) = match host_spec.split_once('/') {
        // Just an IP address
        None => (host_spec, "32"),
        // CIDR notation IP/mask
        Some((ip, mask)) => (ip, mask),
    };

    Ok((
        Ipv4Addr::from_str(ip).map_err(|_err| NetworkParseError::BadIpAddress)?,
        mask.parse::<u32>()
            .map_err(|_err| NetworkParseError::BadNetmask)
            .and_then(|mask| match mask {
                0..=32 => Ok(mask),
                _ => Err(NetworkParseError::BadNetmask),
            })?,
    ))
}

#[derive(Clone)]
pub struct HostIpRange {
    next: u32,
    last: u32,
}

impl Iterator for HostIpRange {
    type Item = Ipv4Addr;
    fn next(&mut self) -> Option<Ipv4Addr> {
        if self.next <= self.last {
            self.next += 1;
            Some(Ipv4Addr::from(self.next - 1))
        } else {
            None
        }
    }
}

fn expand_hosts_with_netmask(addr: Ipv4Addr, mask: u32) -> HostIpRange {
    if mask == 32 {
        HostIpRange {
            next: u32::from(addr),
            last: u32::from(addr),
        }
    } else {
        let ignore_mask = 2_u32.pow(32 - mask) - 1;
        let netmask = !ignore_mask;

        HostIpRange {
            next: (u32::from(addr) & netmask),
            last: (u32::from(addr) & netmask) + ignore_mask,
        }
    }
}

fn expand_port_range(x: &str) -> Result<RangeInclusive<u16>, NetworkParseError> {
    let (from, to) = match x.split_once('-') {
        None => (
            x.parse::<u16>()
                .map_err(|_err| NetworkParseError::InvalidPortNumber)?,
            x.parse::<u16>()
                .map_err(|_err| NetworkParseError::InvalidPortNumber)?,
        ),
        Some(("", "")) => (1, 65535),
        Some((x, y)) => (
            x.parse::<u16>()
                .map_err(|_err| NetworkParseError::InvalidPortNumber)?,
            y.parse::<u16>()
                .map_err(|_err| NetworkParseError::InvalidPortNumber)?,
        ),
    };

    Ok(from..=to)
}

/**
Parse comma separated ports and port ranges.
Examples:
  22,80,110-120
 */
pub fn expand_port_list(port_spec: &str) -> Vec<u16> {
    port_spec
        .split(',')
        .flat_map(|p| expand_port_range(p).unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_hosts_with_netmask() {
        let mut hosts = expand_hosts_with_netmask(Ipv4Addr::new(192, 168, 1, 1), 32);
        assert_eq!(hosts.next().unwrap(), Ipv4Addr::new(192, 168, 1, 1));
        assert!(hosts.next().is_none());

        let mut hosts = expand_hosts_with_netmask(Ipv4Addr::new(192, 168, 1, 1), 31);
        assert_eq!(hosts.next().unwrap(), Ipv4Addr::new(192, 168, 1, 0));
        assert_eq!(hosts.next().unwrap(), Ipv4Addr::new(192, 168, 1, 1));
        assert!(hosts.next().is_none());

        let mut hosts = expand_hosts_with_netmask(Ipv4Addr::new(192, 168, 1, 2), 31);
        assert_eq!(hosts.next().unwrap(), Ipv4Addr::new(192, 168, 1, 2));
        assert_eq!(hosts.next().unwrap(), Ipv4Addr::new(192, 168, 1, 3));
        assert!(hosts.next().is_none());

        let mut hosts = expand_hosts_with_netmask(Ipv4Addr::new(192, 168, 1, 1), 30);
        assert_eq!(hosts.next().unwrap(), Ipv4Addr::new(192, 168, 1, 0));
        assert_eq!(hosts.next().unwrap(), Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(hosts.next().unwrap(), Ipv4Addr::new(192, 168, 1, 2));
        assert_eq!(hosts.next().unwrap(), Ipv4Addr::new(192, 168, 1, 3));
        assert!(hosts.next().is_none());

        let hosts = expand_hosts_with_netmask(Ipv4Addr::new(192, 168, 1, 1), 24);
        assert_eq!(hosts.count(), 256);

        let hosts = expand_hosts_with_netmask(Ipv4Addr::new(192, 168, 1, 1), 8);
        assert_eq!(hosts.count(), 256 * 256 * 256);
    }

    #[test]
    fn test_parse_address_without_netmask_succeeds() {
        assert_eq!(
            address_and_netmask_from_str("192.168.1.1").unwrap(),
            (Ipv4Addr::new(192, 168, 1, 1), 32)
        );
    }

    #[test]
    fn test_parse_address_with_netmask_succeeds() {
        assert_eq!(
            address_and_netmask_from_str("192.168.1.1/24").unwrap(),
            (Ipv4Addr::new(192, 168, 1, 1), 24)
        );
    }

    #[test]
    #[should_panic(expected = "Intended: MissingAddress")]
    fn test_parse_missing_address_fails() {
        address_and_netmask_from_str("").expect("Intended");
    }

    #[test]
    fn test_expand_port_range_succeeds() {
        assert_eq!(expand_port_range("5-9").unwrap(), 5..=9);
    }

    #[test]
    fn test_expand_full_range_succeeds() {
        assert_eq!(expand_port_range("-").unwrap(), 1..=65535);
    }

    #[test]
    fn test_expand_single_port_succeeds() {
        assert_eq!(expand_port_range("23").unwrap(), 23..=23);
    }

    #[test]
    #[should_panic]
    fn test_expand_missing_port_fails() {
        expand_port_range("").expect("Intended");
    }

    #[test]
    #[should_panic]
    fn test_expand_invalid_port_range_characters_fails() {
        expand_port_range("5..9").expect("Intended");
    }

    #[test]
    #[should_panic]
    fn test_expand_half_port_range_fails() {
        expand_port_range("5-").expect("Intended");
    }

    #[test]
    fn test_expand_port_list_with_range_succeeds() {
        assert_eq!(expand_port_list("1-5"), [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_expand_port_list_with_enumeration_succeeds() {
        assert_eq!(expand_port_list("1,2,3,4,5"), [1, 2, 3, 4, 5]);
    }
}
