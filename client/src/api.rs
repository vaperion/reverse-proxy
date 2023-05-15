use std::collections::HashMap;

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

use crate::config::{Configuration, Tunnel};

#[allow(dead_code)]
#[derive(Deserialize)]
struct ConnectResponse {
    status: String,
    worker: u16,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct StatusResponse {
    status: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct EdgeResponse {
    status: String,
    port: u16,
}

pub async fn check_authorization(cfg: &Configuration) -> Result<bool> {
    let url = format!("{}/api/v1/check_authorization", cfg.edge);

    let response = Client::new()
        .get(&url)
        .bearer_auth(&cfg.secret_key)
        .send()
        .await?;

    Ok(response.status().is_success())
}

pub async fn connect(cfg: &Configuration) -> Result<u16> {
    let url = format!("{}/api/v1/connect", cfg.edge);

    let response = Client::new()
        .get(&url)
        .bearer_auth(&cfg.secret_key)
        .send()
        .await?;

    let response: ConnectResponse = response.json().await?;
    Ok(response.worker)
}

pub async fn goodbye(cfg: &Configuration) -> Result<String> {
    let url = format!("{}/api/v1/goodbye", cfg.edge);

    let response = Client::new()
        .get(&url)
        .bearer_auth(&cfg.secret_key)
        .send()
        .await?;

    let response: StatusResponse = response.json().await?;
    Ok(response.status)
}

pub async fn create_edge(
    cfg: &Configuration,
    name: String,
    tunnel: &Tunnel,
) -> Result<(String, String)> {
    let url = format!("{}/api/v1/edge", cfg.edge);

    let mut params = HashMap::new();
    params.insert("name", name);
    params.insert("target", tunnel.target.clone());
    params.insert("protocol", format!("{:?}", tunnel.protocol));
    params.insert("mode", format!("{:?}", tunnel.mode));

    let response = Client::new()
        .post(&url)
        .bearer_auth(&cfg.secret_key)
        .form(&params)
        .send()
        .await?;

    let response: EdgeResponse = response.json().await?;
    Ok((
        response.status,
        format!("{}:{}", cfg.edge_ip, response.port),
    ))
}

pub async fn delete_edge(cfg: &Configuration, tunnel: &Tunnel) -> Result<String> {
    let url = format!("{}/api/v1/edge", cfg.edge);

    let mut params = HashMap::new();
    params.insert("target", tunnel.target.clone());

    let response = Client::new()
        .delete(&url)
        .bearer_auth(&cfg.secret_key)
        .form(&params)
        .send()
        .await?;

    let response: StatusResponse = response.json().await?;
    Ok(response.status)
}

pub async fn delete_edges(cfg: &Configuration) -> Result<String> {
    let url = format!("{}/api/v1/edge/all", cfg.edge);

    let response = Client::new()
        .delete(&url)
        .bearer_auth(&cfg.secret_key)
        .send()
        .await?;

    let response: StatusResponse = response.json().await?;
    Ok(response.status)
}
