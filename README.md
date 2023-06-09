# reverse-proxy
reverse-proxy is a self-hosted, fast, and efficient replacement for ngrok, built with Rust. The project leverages the power of Tokio and Actix to ensure high performance and reliability.

## Features
- Written in Rust for optimal performance and safety
- Self-hosted and easy to deploy
- HAProxy header support for easy integration with existing services

## License
This project is licensed under the GNU GPLv3 license. See the [LICENSE](LICENSE) file for more information.

## Tunnels
reverse-proxy supports two modes of tunneling: Reverse and HolePunch. Tunneling currently only supports TCP connections. HAProxy V1 and V2 headers can be enabled for easy integration with existing services (e.g. for Minecraft servers: [Velocity](https://github.com/PaperMC/Velocity) has HAProxy support).

### Reverse
Reverse tunnels are the default mode of tunneling. In this mode, the client connects to the edge server, and the edge server forwards the connection to the target server.

### HolePunch
HolePunch tunnels are useful for environments where the edge server cannot connect to the target server (e.g. NAT, firewall, etc.). In this mode, the client connects to the edge server, and the edge server uses a connected worker (initiated by the target server) to forward the connection to the target server.

## Usage
Currently, we do not provide pre-built binaries. You will need to build the project yourself. You can do so by running `cargo build --release`. The binaries will be located in `target/release` (`edge` and `client`).

### Edge Server
The edge server is the server that clients connect to. The edge server is responsible for forwarding connections to the target server. You should run this on a server with a public IP address and support for port forwarding (e.g. a VPS).

Before running the edge server, you will need to create a configuration file (`config.toml`). An example configuration file is provided below:
```toml
port = 4120 # Port to listen on

# ./edge add-user <name> [max tunnels]
# ./edge delete-user <name or secret key>
[secrets.example]
max_tunnels = 999
key = "example123"
```

Once you have created a configuration file, you can run the edge server by running `./edge serve`.

### Client
The client should be run on the server that hosts the service you want to expose. You will need to create a configuration file before running the client (`config.toml`). An example configuration file is provided below:
```toml
secret_key = "example123"      # Secret generated by the edge
edge = "http://localhost:4120" # Edge API url, can be behind a reverse proxy
edge_ip = "127.0.0.1"          # Edge IP, used for connections
idle_workers = 5               # Number of idle workers, only used for HolePunch mode

[tunnels.example-web]
target = "localhost:8000" # Target address, can be a domain, port must be specified
protocol = "Tcp"          # Tcp, HAProxyV1, HAProxyV2
mode = "Reverse"          # Reverse, HolePunch

[tunnels.example-mc]
target = "localhost:25565"
protocol = "HAProxyV1"
mode = "HolePunch"
```

Once you have created a configuration file, you can run the client by running `./client`.