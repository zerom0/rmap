use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
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
    InvalidNetworkSpecification,
}

/**
Parse IP addresses with and without subnet masks
Examples:
 192.168.1.1
 192.168.1.1/24
 */
fn expand_hosts(host_spec: &str) -> Result<Vec<Ipv4Addr>, NetworkParseError> {
    address_and_netmask_from_str(host_spec)
        .map(|(addr, mask)| expand_hosts_with_netmask(addr, mask))
}

fn address_and_netmask_from_str(host_spec: &str) -> Result<(Ipv4Addr, u32), NetworkParseError> {
    if host_spec.is_empty() {
        return Err(NetworkParseError::MissingAddress);
    }

    let parts = host_spec.split('/').collect::<Vec<_>>();

    let part_count = parts.len();

    match part_count {
        1 => {}
        2 => {}
        _ => return Err(NetworkParseError::InvalidNetworkSpecification),
    }

    let addr = Ipv4Addr::from_str(parts[0]).map_err(|_err| NetworkParseError::BadIpAddress);

    let mask = if part_count == 2 {
        parts[1]
            .parse::<u32>()
            .map_err(|_err| NetworkParseError::BadNetmask)
            .and_then(|mask| {
                if mask > 32 {
                    Err(NetworkParseError::BadNetmask)
                } else {
                    Ok(mask)
                }
            })
    } else {
        Ok(32_u32)
    };

    Ok((addr?, mask?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address() {
        assert_eq!(
            address_and_netmask_from_str("192.168.1.1").unwrap(),
            (Ipv4Addr::new(192, 168, 1, 1), 32)
        );
    }

    #[test]
    fn test_parse_address_and_netmask() {
        assert_eq!(
            address_and_netmask_from_str("192.168.1.1/24").unwrap(),
            (Ipv4Addr::new(192, 168, 1, 1), 24)
        );
    }

    #[test]
    #[should_panic(expected = "Intended: MissingAddress")]
    fn test_parse_missing_address() {
        address_and_netmask_from_str("").expect("Intended");
    }

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

fn expand_port_range(x: &str) -> Vec<u16> {
    let parts: Vec<u16> = x.split('-').map(|s| u16::from_str(s).unwrap()).collect();
    match parts.len() {
        1 => parts,
        2 => (parts[0]..=parts[1]).collect::<Vec<_>>(),
        _ => vec![],
    }
}

/**
Parse comma separated ports and port ranges.
Examples:
  22,80,110-120
 */
fn expand_port_list(port_spec: &str) -> Vec<u16> {
    port_spec
        .split(',')
        .flat_map(|p| expand_port_range(p))
        .collect()
}

#[derive(Debug)]
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
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    println!("{:?}", scan);
}
