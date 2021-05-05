use crate::PortState::{Closed, Open};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Cli {
    host: String,
    ports: String,
    #[structopt(short, long, default_value = "500")]
    timeout_ms: u64,
}

/**
Parse IP addresses with and without subnet masks
Examples:
 192.168.1.1
 192.168.1.1/24
 */
fn expand_hosts(host_spec: &str) -> Option<Vec<Ipv4Addr>> {
    address_and_netmask_from_str(host_spec)
        .and_then(|(addr, mask)| Some(expand_hosts_with_netmask(addr, mask)))
}

fn address_and_netmask_from_str(host_spec: &str) -> Option<(Ipv4Addr, u32)> {
    let parts = host_spec.split('/').collect::<Vec<_>>();

    match parts.len() {
        1 => Some((Ipv4Addr::from_str(parts[0]).unwrap(), 32_u32)),
        2 => Some((
            Ipv4Addr::from_str(parts[0]).unwrap(),
            u32::from_str(parts[1]).unwrap(),
        )),
        _ => None,
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

fn main() {
    let args = Cli::from_args();

    let hosts = expand_hosts(&args.host).expect("No valid host specification");
    let ports = expand_port_list(&args.ports);
    let timeout = std::time::Duration::from_millis(args.timeout_ms);

    let scan = hosts
        .iter()
        .map(|h| (h.clone(), ports.clone()))
        .map(|(h, ports)| {
            (
                h,
                ports
                    .iter()
                    .map(|p| SocketAddr::from(SocketAddrV4::new(h, *p)))
                    .map(|a| (a.port(), test_port(&a, timeout)))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    println!("{:?}", scan);
}

#[derive(Debug)]
enum PortState {
    Open,
    Closed,
}

fn test_port(addr: &SocketAddr, timeout: Duration) -> PortState {
    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_) => Open,
        Err(_) => Closed,
    }
}
