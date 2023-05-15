use byteorder::{BigEndian, ByteOrder};
use rand::random;
use serde::Deserialize;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::{anyhow, bail, Result};
use log::{error, info};
use tokio::{
    io::{copy, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    select,
    sync::{oneshot::Receiver, Mutex},
};

use crate::state::State;

#[derive(Deserialize, Debug, Clone)]
pub enum Protocol {
    Tcp,
    HAProxyV1,
    HAProxyV2,
}

#[derive(Deserialize, Debug, Clone)]
pub enum Mode {
    Reverse,
    HolePunch,
}

pub async fn start_proxy(
    target: String,
    closer: Receiver<()>,
    protocol: Protocol,
    mode: Mode,
    name: String,
    secret: String,
    state: Arc<Mutex<State>>,
) -> Option<u16> {
    info!("creating proxy for tunnel {name} (to={target}, proto={protocol:?}, mode={mode:?})");

    match TcpListener::bind("0.0.0.0:0").await {
        Ok(listener) => {
            let port = listener.local_addr().unwrap().port();

            tokio::spawn(async move {
                select! {
                    _ = async {
                        loop {
                            match listener.accept().await {
                                Ok((socket, _)) => {
                                    match handle_tcp_stream(socket, target.clone(), protocol.clone(), mode.clone(), secret.clone(), state.clone()).await {
                                        Ok(_) => {}
                                        Err(e) => {
                                            error!("failed to handle connection: {e}");
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("failed to accept connection: {e}");
                                }
                            }
                        }
                    } => {}

                    _ = closer => {}
                }
            });

            Some(port)
        }
        Err(e) => {
            error!("failed to bind to port: {e}");
            None
        }
    }
}

async fn handle_tcp_stream(
    stream: TcpStream,
    target: String,
    protocol: Protocol,
    mode: Mode,
    secret: String,
    state: Arc<Mutex<State>>,
) -> Result<()> {
    match mode {
        Mode::Reverse => {
            let target_stream = TcpStream::connect(target.clone()).await?;
            merge_coupled_streams(stream, target_stream, protocol, mode, target).await
        }

        Mode::HolePunch => {
            let mut state = state.lock().await;

            let secret = state
                .secrets
                .get_mut(secret.as_str())
                .ok_or_else(|| anyhow!("no client found for secret \"{secret}\""))?;

            let worker = {
                let index = random::<usize>() % secret.workers.len();
                secret.workers.remove(index)
            };

            worker.handoff_tx.send(target.clone()).unwrap();
            let (server_read, server_write) = worker.stream_rx.await.unwrap();
            let (client_read, client_write) = stream.into_split();

            merge_streams(
                client_read,
                client_write,
                server_read,
                server_write,
                protocol,
                mode,
                target,
            )
            .await
        }

        #[allow(unreachable_patterns)]
        _ => bail!("mode not implemented"),
    }
}

async fn merge_coupled_streams(
    client: TcpStream,
    server: TcpStream,
    protocol: Protocol,
    mode: Mode,
    target: String,
) -> Result<()> {
    let (client_read, client_write) = client.into_split();
    let (server_read, server_write) = server.into_split();

    merge_streams(
        client_read,
        client_write,
        server_read,
        server_write,
        protocol,
        mode,
        target,
    )
    .await
}

async fn merge_streams(
    mut client_read: OwnedReadHalf,
    mut client_write: OwnedWriteHalf,
    mut server_read: OwnedReadHalf,
    mut server_write: OwnedWriteHalf,
    protocol: Protocol,
    mode: Mode,
    target: String,
) -> Result<()> {
    match protocol {
        Protocol::HAProxyV1 => {
            if let Err(e) =
                send_haproxy_v1_header(&mut client_write, &mut server_write, mode, target).await
            {
                error!("failed to send HAProxy v1 header: {e}");
            }
        }

        Protocol::HAProxyV2 => {
            if let Err(e) =
                send_haproxy_v2_header(&mut client_write, &mut server_write, mode, target).await
            {
                error!("failed to send HAProxy v2 header: {e}");
            }
        }

        _ => {}
    }

    tokio::spawn(async move {
        if let Err(e) = copy(&mut server_read, &mut client_write).await {
            error!("failed to copy from target to stream: {e}");
        }
    });

    tokio::spawn(async move {
        if let Err(e) = copy(&mut client_read, &mut server_write).await {
            error!("failed to copy from stream to target: {e}");
        }
    });

    Ok(())
}

/// The v1 header is a human-readable, text-based format.
/// It starts with the string "PROXY" followed by the protocol (TCP4, TCP6, or UNKNOWN),
/// source IP, destination IP, source port, and destination port.
/// Fields are separated by spaces, and the header ends with a CRLF sequence ("\r\n").
/// PROXY TCP4 192.168.0.1 192.168.0.2 12345 80\r\n
async fn send_haproxy_v1_header(
    client_write: &mut OwnedWriteHalf,
    server_write: &mut OwnedWriteHalf,
    mode: Mode,
    target: String,
) -> Result<()> {
    let dst_addr = match mode {
        Mode::Reverse => server_write.peer_addr()?,
        Mode::HolePunch => {
            let target = target.parse::<SocketAddr>()?;
            SocketAddr::new(target.ip(), target.port())
        }
    };

    let proxy_header = {
        let mut buf = vec![];
        let src_ip = client_write.peer_addr()?.ip();
        let dst_ip = dst_addr.ip();
        let src_port = client_write.peer_addr()?.port();
        let dst_port = dst_addr.port();

        buf.extend_from_slice(b"PROXY TCP4 ");

        buf.extend_from_slice(src_ip.to_string().as_bytes());
        buf.extend_from_slice(b" ");

        buf.extend_from_slice(dst_ip.to_string().as_bytes());
        buf.extend_from_slice(b" ");

        buf.extend_from_slice(src_port.to_string().as_bytes());
        buf.extend_from_slice(b" ");

        buf.extend_from_slice(dst_port.to_string().as_bytes());
        buf.extend_from_slice(b"\r\n");

        buf
    };

    server_write.write_all(&proxy_header).await?;

    Ok(())
}

/// The v2 header is a binary format and starts with a 12-byte fixed header,
/// followed by optional TLV (Type-Length-Value) records.
/// The fixed header consists of a 16-bit signature (0x0D0A0D0A),
/// 8-bit version and command, 8-bit protocol and address family, and 16-bit length.
/// 0D0A0D0A  21 11 00 0C  C0A80001  C0A80002  3039 0050
async fn send_haproxy_v2_header(
    client_write: &mut OwnedWriteHalf,
    server_write: &mut OwnedWriteHalf,
    mode: Mode,
    target: String,
) -> Result<()> {
    let dst_addr = match mode {
        Mode::Reverse => server_write.peer_addr()?,
        Mode::HolePunch => {
            let target = target.parse::<SocketAddr>()?;
            SocketAddr::new(target.ip(), target.port())
        }
    };

    let proxy_header = {
        let mut buf = vec![0; 20];
        let signature = b"\x0D\x0A\x0D\x0A";
        let version_and_command = 0x21;
        let protocol_and_address_family = 0x11;
        let length = 0x000C;
        let src_ip = ip_to_ipv4(client_write.peer_addr()?.ip())?;
        let dst_ip = ip_to_ipv4(dst_addr.ip())?;
        let src_port = client_write.peer_addr()?.port();
        let dst_port = dst_addr.port();

        buf[..4].copy_from_slice(signature);
        buf[4] = version_and_command;
        buf[5] = protocol_and_address_family;
        BigEndian::write_u16(&mut buf[6..8], length);
        buf[8..12].copy_from_slice(&src_ip.octets());
        buf[12..16].copy_from_slice(&dst_ip.octets());
        BigEndian::write_u16(&mut buf[16..18], src_port);
        BigEndian::write_u16(&mut buf[18..20], dst_port);

        buf
    };

    server_write.write_all(&proxy_header).await?;

    Ok(())
}

fn ip_to_ipv4(addr: IpAddr) -> Result<Ipv4Addr> {
    if let IpAddr::V4(ip) = addr {
        Ok(ip)
    } else {
        bail!("only IPv4 is supported");
    }
}
