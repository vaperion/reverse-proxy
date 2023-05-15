use std::collections::HashMap;

use tokio::{
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::{
        mpsc::UnboundedSender,
        oneshot::{Receiver, Sender},
    },
};

use crate::{config::Configuration, listener::ListenerMessage};

pub struct State {
    pub cfg: &'static Configuration,
    pub worker_port: Option<u16>,
    pub listener_tx: UnboundedSender<ListenerMessage>,
    pub secrets: HashMap<String, Secret>,
}

pub struct Secret {
    pub secret: String,
    pub max_tunnels: usize,
    pub active_tunnels: Vec<Tunnel>,
    pub workers: Vec<Worker>,
}

pub struct Tunnel {
    pub name: String,
    pub target: String,
    pub port: u16,
}

pub struct Worker {
    pub client_addr: String,
    pub handoff_tx: Sender<String>,
    pub stream_rx: Receiver<(OwnedReadHalf, OwnedWriteHalf)>,
    pub close_tx: Sender<()>,
}
