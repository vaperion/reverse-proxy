use std::{collections::HashMap, sync::Arc, time::Duration};

use actix_web::{http::KeepAlive, web, App, HttpServer};
use anyhow::Result;
use log::info;
use tokio::sync::{mpsc::unbounded_channel, Mutex};

use crate::{
    api,
    config::Configuration,
    listener::{self, worker},
    state::{Secret, State},
};

pub async fn serve(cfg: &'static mut Configuration) -> Result<()> {
    info!("starting listener...");

    let (tx, rx) = unbounded_channel();

    info!("booting with {} secrets...", cfg.secrets.len());

    let state = State {
        cfg,
        worker_port: None,
        listener_tx: tx,
        secrets: HashMap::new(),
    };

    let state = Arc::new(Mutex::new(state));

    {
        let state = state.clone();
        tokio::spawn(async move {
            listener::start_listener(rx, state).await.unwrap();
        });
    }

    let worker_port = worker::start_worker_server(state.clone()).await?;

    {
        let mut state = state.lock().await;

        state.worker_port = Some(worker_port);

        cfg.secrets.iter().for_each(|(_, secret)| {
            state.secrets.insert(
                secret.key.clone(),
                Secret {
                    secret: secret.key.clone(),
                    max_tunnels: secret.max_tunnels,
                    active_tunnels: vec![],
                    workers: vec![],
                },
            );
        });
    }

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(api::health)
            .service(api::check_authorization)
            .service(api::connect)
            .service(api::goodbye)
            .service(api::edge::create_edge)
            .service(api::edge::delete_edge)
            .service(api::edge::delete_edges)
    })
    .workers(4)
    .keep_alive(KeepAlive::Timeout(Duration::from_secs(900)))
    .bind(("0.0.0.0", cfg.port))?
    .run()
    .await?;

    info!("shutting down...");

    Ok(())
}
