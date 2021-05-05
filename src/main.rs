use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::str::FromStr;
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
fn parse_hosts(src: &str) -> Vec<Ipv4Addr> {
    let parts = src.split('/').collect::<Vec<_>>();

    let res = match parts.len() {
        1 => Some((Ipv4Addr::from_str(parts[0]).unwrap(), 32_u32)),
        2 => Some((
            Ipv4Addr::from_str(parts[0]).unwrap(),
            u32::from_str(parts[1]).unwrap(),
        )),
        _ => None,
    };

    match res {
        Some((addr, mask)) => {
            if mask == 32 {
                vec![addr]
            } else {
                let ignore_mask = 2_u32.pow(32 - mask) - 1;
                (1..=ignore_mask)
                    .collect::<Vec<_>>()
                    .iter()
                    .map(|i| Ipv4Addr::from((u32::from(addr) & !ignore_mask) + i))
                    .collect()
            }
        }
        None => vec![],
    }
}

fn expand(x: &str) -> Vec<String> {
    let parts = x.split('-').collect::<Vec<_>>();
    match parts.len() {
        1 => vec![parts[0].to_string()],
        2 => (u16::from_str(parts[0]).unwrap()..=u16::from_str(parts[1]).unwrap())
            .collect::<Vec<_>>()
            .iter()
            .map(|&i| i.to_string())
            .collect::<Vec<_>>(),
        _ => vec![],
    }
}

/**
Parse comma separated ports and port ranges.
Examples:
  22,80,110-120
 */
fn parse_ports(src: &str) -> Vec<u16> {
    src.split(',')
        .collect::<Vec<_>>()
        .iter()
        .flat_map(|x| expand(x))
        .map(|x| u16::from_str(x.as_str()).unwrap())
        .collect()
}

fn main() {
    let args = Cli::from_args();

    let hosts = parse_hosts(&args.host);
    let ports = parse_ports(&args.ports);

    for host in hosts {
        print!("\n{}: ", host);
        for port in &ports {
            match TcpStream::connect_timeout(
                &SocketAddr::from(SocketAddrV4::new(host, *port)),
                std::time::Duration::from_millis(args.timeout_ms),
            ) {
                Ok(_) => print!("{} ", port),
                Err(_) => {
                    //print!("{} ", port)
                }
            }
        }
    }
    println!()
}
