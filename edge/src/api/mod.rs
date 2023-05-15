use std::sync::Arc;

use actix_web::{
    error::ErrorForbidden,
    get,
    web::{Data, Json},
    Responder, Result,
};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde_json::json;
use tokio::sync::Mutex;

use crate::{listener::ListenerMessage, state::State};

pub mod edge;

#[get("/api/v1/health")]
pub async fn health() -> Result<impl Responder> {
    Ok(Json(json!({"status": "ok"})))
}

#[get("/api/v1/check_authorization")]
pub async fn check_authorization(
    auth: BearerAuth,
    data: Data<Arc<Mutex<State>>>,
) -> Result<impl Responder> {
    let state = data.lock().await;

    if state.secrets.contains_key(auth.token()) {
        Ok(Json(json!({"status": "ok"})))
    } else {
        Err(ErrorForbidden(Json(json!({"status": "forbidden"}))))
    }
}

#[get("/api/v1/connect")]
pub async fn connect(auth: BearerAuth, data: Data<Arc<Mutex<State>>>) -> Result<impl Responder> {
    let state = data.lock().await;

    if state.secrets.contains_key(auth.token()) {
        Ok(Json(json!({"status": "ok", "worker": state.worker_port})))
    } else {
        Err(ErrorForbidden(Json(json!({"status": "forbidden"}))))
    }
}

#[get("/api/v1/goodbye")]
pub async fn goodbye(auth: BearerAuth, data: Data<Arc<Mutex<State>>>) -> Result<impl Responder> {
    let mut state = data.lock().await;

    let listener_tx = state.listener_tx.clone();

    let secret = state
        .secrets
        .get_mut(auth.token())
        .ok_or_else(|| ErrorForbidden(Json(json!({"status": "forbidden"}))))?;

    for tunnel in secret.active_tunnels.drain(..) {
        if let Err(e) = listener_tx.send(ListenerMessage::Stop { port: tunnel.port }) {
            eprintln!("failed to send stop message: {}", e);
        }
    }

    for worker in secret.workers.drain(..) {
        if worker.close_tx.send(()).is_err() {
            eprintln!("failed to send close message: {}", worker.client_addr);
        }
    }

    Ok(Json(json!({"status": "ok"})))
}
