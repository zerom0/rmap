# rmap
Simple nmap implementation in rust

Use it to scan for open ports on the given hosts.

# Usage
```sh
rmap <hosts> [<ports>] [--timeout-ms <timeout-ms>]
```

Parameters:
- hosts: CIDR notation, like `192.168.1.1/24`
- ports: Comma separated values, like `80,443`
- timeout-ms: Timeout if a port is closed and silent
