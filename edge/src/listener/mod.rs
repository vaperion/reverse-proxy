use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use log::info;
use tokio::sync::{
    mpsc::UnboundedReceiver,
    oneshot::{self, Sender},
    Mutex,
};

use crate::{listener::proxy::start_proxy, state::State};

use self::proxy::{Mode, Protocol};

pub mod proxy;
pub mod worker;

pub enum ListenerMessage {
    Listen {
        reply: Sender<Option<u16>>,
        tunnel: String,
        protocol: Protocol,
        mode: Mode,
        name: String,
        secret: String,
    },
    Stop {
        port: u16,
    },
}

pub async fn start_listener(
    mut rx: UnboundedReceiver<ListenerMessage>,
    state: Arc<Mutex<State>>,
) -> Result<()> {
    let mut port_closer_map = HashMap::new();
    loop {
        match rx.recv().await {
            Some(ListenerMessage::Listen {
                reply,
                tunnel,
                protocol,
                mode,
                name,
                secret,
            }) => {
                info!("creating listener for tunnel {name} (to={tunnel}, proto={protocol:?}, mode={mode:?})");

                let (tx, rx) = oneshot::channel();

                let result =
                    start_proxy(tunnel, rx, protocol, mode, name, secret, state.clone()).await;
                reply.send(result).unwrap();

                if let Some(port) = result {
                    port_closer_map.insert(port, tx);
                }
            }
            Some(ListenerMessage::Stop { port }) => {
                info!("stopping listener for port {port}");

                if let Some(closer) = port_closer_map.remove(&port) {
                    closer.send(()).unwrap();
                }
            }
            None => {}
        }
    }
}
