use std::sync::Arc;

use actix_web::{
    delete,
    error::{ErrorBadRequest, ErrorForbidden, ErrorTooManyRequests},
    post,
    web::{Data, Form, Json},
    Responder, Result,
};
use actix_web_httpauth::extractors::bearer::BearerAuth;

use serde::Deserialize;
use serde_json::json;
use tokio::sync::{oneshot, Mutex};

use crate::{
    listener::{
        proxy::{Mode, Protocol},
        ListenerMessage,
    },
    state::{State, Tunnel},
};

#[derive(Deserialize)]
pub struct CreateRequestData {
    name: String,
    target: String,
    protocol: Protocol,
    mode: Mode,
}

#[derive(Deserialize)]
pub struct DeleteRequestData {
    target: String,
}

#[post("/api/v1/edge")]
pub async fn create_edge(
    auth: BearerAuth,
    data: Data<Arc<Mutex<State>>>,
    form: Form<CreateRequestData>,
) -> Result<impl Responder> {
    let mut state = data.lock().await;

    let listener_tx = state.listener_tx.clone();

    let secret = state
        .secrets
        .get_mut(auth.token())
        .ok_or_else(|| ErrorForbidden(Json(json!({"status": "forbidden"}))))?;

    if secret.max_tunnels <= secret.active_tunnels.len() {
        return Err(ErrorTooManyRequests(Json(
            json!({"status": "too many tunnels"}),
        )));
    }

    let (tx, rx) = oneshot::channel();

    if listener_tx
        .send(ListenerMessage::Listen {
            reply: tx,
            tunnel: form.target.clone(),
            protocol: form.protocol.clone(),
            mode: form.mode.clone(),
            name: form.name.clone(),
            secret: auth.token().to_string(),
        })
        .is_err()
    {
        return Err(ErrorBadRequest(Json(
            json!({"status": "failed to request tunnel creation"}),
        )));
    }

    match rx.await {
        Ok(Some(port)) => {
            let tunnel = Tunnel {
                name: form.name.clone(),
                target: form.target.clone(),
                port,
            };

            secret.active_tunnels.push(tunnel);

            Ok(Json(json!({"status": "ok", "port": port})))
        }

        Ok(None) => Err(ErrorBadRequest(Json(
            json!({"status": "failed to reserve port"}),
        ))),

        Err(_) => Err(ErrorBadRequest(Json(
            json!({"status": "failed to create tunnel"}),
        ))),
    }
}

#[delete("/api/v1/edge")]
pub async fn delete_edge(
    auth: BearerAuth,
    data: Data<Arc<Mutex<State>>>,
    form: Form<DeleteRequestData>,
) -> Result<impl Responder> {
    let mut state = data.lock().await;

    let listener_tx = state.listener_tx.clone();

    let secret = state
        .secrets
        .get_mut(auth.token())
        .ok_or_else(|| ErrorForbidden(Json(json!({"status": "forbidden"}))))?;

    let tunnel = secret
        .active_tunnels
        .iter()
        .find(|t| t.target == form.target)
        .ok_or_else(|| ErrorBadRequest(Json(json!({"status": "no such tunnel"}))))?;

    if let Err(e) = listener_tx.send(ListenerMessage::Stop { port: tunnel.port }) {
        eprintln!("failed to send stop message: {}", e);
    }

    secret.active_tunnels.retain(|t| t.target != form.target);

    Ok(Json(json!({"status": "ok"})))
}

#[delete("/api/v1/edge/all")]
pub async fn delete_edges(
    auth: BearerAuth,
    data: Data<Arc<Mutex<State>>>,
) -> Result<impl Responder> {
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

    Ok(Json(json!({"status": "ok"})))
}
