use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use anyhow::{bail, Result};
use log::{error, info};
use tokio::{
    io::{copy, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::oneshot::Receiver,
};

use crate::config::Configuration;

pub async fn start_workers(cfg: &Configuration, port: u16, close: Receiver<()>) -> Result<()> {
    let closed = AtomicBool::new(false);
    let closed = Arc::new(closed);

    {
        let closed = closed.clone();
        tokio::spawn(async move {
            close.await.unwrap();
            closed.store(true, Ordering::Relaxed);
        });
    }

    let worker_id = AtomicUsize::new(0);
    let worker_id = Arc::new(worker_id);

    for _ in 0..cfg.idle_workers {
        let ip = cfg.edge_ip.clone();
        let secret = cfg.secret_key.clone();

        let closed = closed.clone();
        let worker_id = worker_id.clone();
        tokio::spawn(async move {
            loop {
                if closed.load(Ordering::Relaxed) {
                    break;
                }

                let id = worker_id.fetch_add(1, Ordering::Relaxed);

                info!("starting worker #{id}...");
                match run_worker(id, ip.clone(), port, secret.clone()).await {
                    Ok(_) => {}
                    Err(e) => {
                        if !closed.load(Ordering::Relaxed) {
                            error!("worker {id} failed: {e}");
                        }
                    }
                }
            }
        });
    }

    Ok(())
}

async fn run_worker(id: usize, ip: String, port: u16, secret: String) -> Result<()> {
    let mut stream = TcpStream::connect((ip, port)).await?;

    // Send authorization
    stream.write_all(secret.as_bytes()).await?;
    stream.write_all(b"\n").await?;

    // Wait for signal
    let mut b = [0; 1];
    stream.read_exact(&mut b).await?;
    let expecting_length = b[0];

    let mut b = vec![0; expecting_length as usize];
    stream.read_exact(&mut b).await?;

    // First byte should always be \x01
    if b[0] != 0x01 {
        bail!("invalid signal received");
    }

    // Last byte should always be \x02
    if b[expecting_length as usize - 1] != 0x02 {
        bail!("invalid signal received");
    }

    // After that comes the length of the target
    let target_length =
        u64::from_be_bytes([b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8]]) as usize;

    // And the actual target
    let target = String::from_utf8(b[9..9 + target_length].to_vec())?;

    info!("worker #{id} is being used to proxy to {target}");

    // We create a stream
    let server = TcpStream::connect(target).await?;

    // We proxy
    let (mut server_reader, mut server_writer) = server.into_split();
    let (mut client_reader, mut client_writer) = stream.into_split();

    tokio::spawn(async move {
        if let Err(e) = copy(&mut server_reader, &mut client_writer).await {
            error!("failed to copy from server to client: {}", e);
        }
    });

    tokio::spawn(async move {
        if let Err(e) = copy(&mut client_reader, &mut server_writer).await {
            error!("failed to copy from client to server: {}", e);
        }
    });

    Ok(())
}
