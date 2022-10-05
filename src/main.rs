use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use clap::Parser;
use dns_lookup::lookup_addr;
use futures::stream;
use futures::StreamExt;

use crate::args::{expand_hosts, expand_port_list};

mod args;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Host IP or CIDR range to scan, e.g. 192.168.1.1/24
    hosts: String,
    /// Ports to scan
    #[arg(default_value = "20-23,25,80,110,143,194,443,465,587,993")]
    ports: String,
    /// Connection timeout in ms
    #[arg(short, default_value_t = 1000)]
    timeout_ms: u64,
    /// Show ports also for range scan
    #[arg(short, long, default_value_t = false)]
    show_ports: bool,
    /// Omit host name resolution
    #[arg(id = "No DNS resolution", short = 'n', default_value_t = false)]
    no_resolve_hostname: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum PortState {
    Open,
    Closed,
    Timeout,
}

#[tokio::main()]
async fn main() {
    let cli: Cli = Cli::parse();

    let hosts = expand_hosts(&cli.hosts).expect("No valid host specification");
    let ports = expand_port_list(&cli.ports);
    let timeout = cli.timeout_ms;

    let show_ports = hosts.clone().count() == 1 || cli.show_ports;

    let scan_result: HashMap<Ipv4Addr, _> = stream::iter(hosts)
        .map(|host| get_port_states(host, ports.clone(), timeout))
        .buffer_unordered(20)
        .collect()
        .await;

    let online_scan_results = scan_result.iter().filter_map(|(host, port_states)| {
        let (open_count, closed_count, timeout_count) = port_statistics_from(port_states);
        if open_count > 0 || closed_count > 0 {
            Some((host, port_states, (open_count, closed_count, timeout_count)))
        } else {
            None
        }
    });

    let _ = online_scan_results.map(|(host, port_states, (open_count, closed_count, timeout_count))| {
        let host = if cli.no_resolve_hostname {
            host.to_string()
        } else {
            format!("{host} [{}]", lookup_addr(&IpAddr::from(*host)).unwrap_or_else(|_| { String::new() }))
        };

        println!("{} (open: {}, closed: {}, timeout: {})", host, open_count, closed_count, timeout_count);
        if show_ports {
            for (port, port_state) in port_states {
                println!("    {} : {:?}", port, port_state);
            }
        }
    }).collect::<Vec<_>>();
}

fn port_statistics_from(port_states: &HashMap<u16, PortState>) -> (u16, u16, u16) {
    port_states.iter().map(|(_, state)| {
        match state {
            PortState::Open => (1, 0, 0),
            PortState::Closed => (0, 1, 0),
            PortState::Timeout => (0, 0, 1)
        }
    }).reduce(|(open_acc, closed_acc, timeout_acc), (open, closed, timeout)| {
        (open_acc + open, closed_acc + closed, timeout_acc + timeout)
    }).unwrap()
}

async fn get_port_states(host: Ipv4Addr, ports: Vec<u16>, timeout: u64) -> (Ipv4Addr, HashMap<u16, PortState>) {
    let port_states = stream::iter(ports).map(|port| async move {
        let address = format!("{}:{}", host, port);
        let port_state = match tokio::time::timeout(Duration::from_millis(timeout),
                                                    tokio::net::TcpStream::connect(&address)).await {
            Ok(Ok(_)) => PortState::Open,
            Ok(Err(_)) => PortState::Closed,
            Err(_) => PortState::Timeout,
        };
        (port, port_state)
    }).buffer_unordered(20).collect().await;
    (host, port_states)
}