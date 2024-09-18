use serde::{Deserialize, Serialize};

use reqwest::Client;

use axum::http::StatusCode;

#[derive(Serialize, Deserialize)]
pub(crate) enum WebhookRequest {
    #[serde(rename = "infoHash")]
    InfoHash(String),
    #[serde(rename = "path")]
    Path(String),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct AnnounceRequest {
    pub name: String,
    pub guid: String,
    pub link: String,
    pub tracker: String,
}

pub(crate) async fn cross_seed_announce(
    cross_seed_url: String,
    cross_seed_api_key: String,
    announce: &AnnounceRequest,
) -> anyhow::Result<StatusCode> {
    let client = Client::new();
    let response = client
        .post(format!("{cross_seed_url}/api/announce"))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("X-Api-Key", cross_seed_api_key)
        .json(announce)
        .send()
        .await?;

    Ok(response.status())
}

pub(crate) async fn cross_seed_webhook(
    cross_seed_url: &str,
    cross_seed_api_key: &str,
    webhook: WebhookRequest,
) -> anyhow::Result<StatusCode> {
    let client = Client::new();
    let response = client
        .post(format!("{cross_seed_url}/api/webhook"))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("X-Api-Key", cross_seed_api_key)
        .json(&webhook)
        .send()
        .await?;

    Ok(response.status())
}
