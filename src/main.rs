use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::ops::RangeInclusive;
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Cli {
    host: String,
    #[structopt(default_value = "20-23,25,80,110,143,194,443,465,587,993")]
    ports: String,
    #[structopt(short, long, default_value = "1000")]
    timeout_ms: u64,
}

#[derive(Debug)]
enum NetworkParseError {
    MissingAddress,
    BadIpAddress,
    BadNetmask,
}

/**
Parse IP addresses with and without subnet masks
Examples:
 192.168.1.1
 192.168.1.1/24
 */
fn expand_hosts(host_spec: &str) -> Result<Vec<Ipv4Addr>, NetworkParseError> {
    let (addr, mask) = address_and_netmask_from_str(host_spec)?;
    Ok(expand_hosts_with_netmask(addr, mask))
}

fn address_and_netmask_from_str(host_spec: &str) -> Result<(Ipv4Addr, u32), NetworkParseError> {
    if host_spec.is_empty() {
        return Err(NetworkParseError::MissingAddress);
    }

    let (ip, mask) = match host_spec.split_once("/") {
        // Just an IP address
        None => { (host_spec, "32") }
        // CIDR notation IP/mask
        Some((ip, mask)) => { (ip, mask) }
    };

    Ok((
        Ipv4Addr::from_str(ip).map_err(|_err| NetworkParseError::BadIpAddress)?,
        mask.parse::<u32>().map_err(|_err| NetworkParseError::BadNetmask)
            .and_then(|mask| {
                match mask {
                    0 ..= 32 => { Ok(mask) }
                    _ => { Err(NetworkParseError::BadNetmask) }
                }
            })?
    ))
}

fn expand_hosts_with_netmask(addr: Ipv4Addr, mask: u32) -> Vec<Ipv4Addr> {
    if mask == 32 {
        vec![addr]
    } else {
        let ignore_mask = 2_u32.pow(32 - mask) - 1;
        let netmask = !ignore_mask;
        (1..=ignore_mask)
            .map(|i| Ipv4Addr::from((u32::from(addr) & netmask) + i))
            .collect()
    }
}

#[derive(Debug, Clone)]
enum PortRangeParseError {
    InvalidPortNumber,
}

fn expand_port_range(x: &str) -> Result<RangeInclusive<u16>, PortRangeParseError> {
    let (from, to) = match x.split_once('-') {
        None => { (x.parse::<u16>().map_err(|_err| PortRangeParseError::InvalidPortNumber)?,
                   x.parse::<u16>().map_err(|_err| PortRangeParseError::InvalidPortNumber)?) }
        Some( (x, y) ) => {
            (x.parse::<u16>().map_err(|_err| PortRangeParseError::InvalidPortNumber)?,
             y.parse::<u16>().map_err(|_err| PortRangeParseError::InvalidPortNumber)?) } };

    Ok(from..=to)
}

/**
Parse comma separated ports and port ranges.
Examples:
  22,80,110-120
 */
fn expand_port_list(port_spec: &str) -> Vec<u16> {
    port_spec
        .split(',')
        .flat_map(|p| expand_port_range(p).unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

#[derive(Debug, PartialEq, Eq)]
enum PortState {
    Open,
    Closed,
}

fn test_port(addr: &SocketAddr, timeout: Duration) -> PortState {
    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => PortState::Open,
        Err(_) => PortState::Closed,
    }
}

fn main() {
    let args = Cli::from_args();

    let hosts = expand_hosts(&args.host).expect("No valid host specification");
    let ports = expand_port_list(&args.ports);
    let timeout = std::time::Duration::from_millis(args.timeout_ms);

    let scan = hosts
        .iter()
        .map(|h| {
            (
                h,
                ports
                    .iter()
                    .map(|p| SocketAddr::from(SocketAddrV4::new(*h, *p)))
                    .map(|a| (a.port(), test_port(&a, timeout)))
                    .filter(|(port, state)| *state == PortState::Open)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    println!("{:?}", scan);
}
