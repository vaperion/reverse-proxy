use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use log::{error, info};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    select,
    sync::{oneshot::channel, Mutex},
};

use crate::state::{State, Worker};

pub async fn start_worker_server(state: Arc<Mutex<State>>) -> Result<u16> {
    info!("starting worker server...");

    match TcpListener::bind("0.0.0.0:0").await {
        Ok(listener) => {
            let port = listener.local_addr().unwrap().port();

            tokio::spawn(async move {
                loop {
                    match listener.accept().await {
                        Ok((socket, _)) => {
                            match handle_worker_tcp_stream(socket, state.clone()).await {
                                Ok(_) => {}
                                Err(e) => {
                                    error!("failed to handle worker connection: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            error!("failed to accept worker connection: {e}");
                        }
                    }
                }
            });

            Ok(port)
        }
        Err(e) => {
            bail!("failed to bind to port: {e}")
        }
    }
}

async fn handle_worker_tcp_stream(stream: TcpStream, state: Arc<Mutex<State>>) -> Result<()> {
    let client_addr = stream.peer_addr().unwrap().to_string();
    let (read, mut write) = stream.into_split();

    let mut reader = BufReader::new(read);

    let mut secret = String::new();
    reader.read_line(&mut secret).await?;

    secret.pop();
    let secret = secret.trim().to_string();

    let handoff_rx;
    let socket_tx;
    let close_rx;
    {
        let mut state = state.lock().await;

        let secret = state
            .secrets
            .get_mut(&secret)
            .ok_or(anyhow!("invalid secret: \"{secret}\""))?;

        let (tx, rx) = channel();
        let (c_tx, c_rx) = channel();
        let (s_tx, s_rx) = channel();

        let worker = Worker {
            client_addr,
            stream_rx: s_rx,
            handoff_tx: tx,
            close_tx: c_tx,
        };
        secret.workers.push(worker);

        handoff_rx = rx;
        socket_tx = s_tx;
        close_rx = c_rx;
    }

    tokio::spawn(async move {
        select! {
            _ = close_rx => {}

            result = handoff_rx => {
                if let Ok(target) = result {
                    let packet_body = {
                        let mut buf = vec![];

                        buf.extend_from_slice(b"\x01");
                        buf.extend_from_slice(&target.len().to_be_bytes());
                        buf.extend_from_slice(target.as_bytes());
                        buf.extend_from_slice(b"\x02");

                        buf.insert(0, buf.len() as u8);
                        buf
                    };
                    write.write_all(&packet_body).await.unwrap();

                    socket_tx.send((reader.into_inner(), write)).unwrap();
                }
            }
        }
    });

    Ok(())
}
