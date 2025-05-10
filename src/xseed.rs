use std::sync::{Arc, RwLock};

use log::{error, info, trace, warn};

use anyhow::{anyhow, Context};

use tokio::time::{sleep, Duration};

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::cross_seed::{cross_seed_webhook, WebhookRequest};
use crate::discord::discord_webhook;
use crate::AppState;

use crate::data_types::radarr::RadarrConnectWebhook;
use crate::data_types::sonarr::SonarrConnectWebhook;

enum ArrConnectWebhook {
    Sonarr(SonarrConnectWebhook),
    Radarr(RadarrConnectWebhook),
}

/// Use the `ArrConnectWebhook` to extract the correct path to send to cross-seed's webhook
/// endpoint.
async fn cross_seed_webhook_data(
    cross_seed_url: &str,
    cross_seed_api_key: &str,
    request: &ArrConnectWebhook,
) -> anyhow::Result<StatusCode> {
    let path = match request {
        ArrConnectWebhook::Sonarr(request) => {
            let release = request.release.clone().context("No release found")?;
            let episode_files = request
                .episode_files
                .clone()
                .context("No episode_files found")?;
            let destination_path = request
                .destination_path
                .clone()
                .context("No destination_path found")?;
            warn!("Release type: {}", release.release_type);
            if release.release_type.to_lowercase() == "seasonpack" {
                warn!("Release is a season pack, path: {destination_path}");
                destination_path
            } else {
                episode_files
                    .first()
                    .context("Episode files are empty")?
                    .path
                    .clone()
            }
        }
        ArrConnectWebhook::Radarr(request) => {
            let movie_file = request.movie_file.clone().context("No movie_file found")?;
            movie_file.path
        }
    };

    let webhook = WebhookRequest::Path(path);

    cross_seed_webhook(cross_seed_url, cross_seed_api_key, webhook).await
}

async fn xseed(request: ArrConnectWebhook, state: Arc<RwLock<AppState>>) -> anyhow::Result<()> {
    let (cross_seed_url, cross_seed_api_key, xseed_torrent_clients, xseed_usenet_clients) = {
        let read_guard = state
            .read()
            .map_err(|_| anyhow!("Could not read from state."))?;

        let cross_seed_url = read_guard
            .cross_seed_local_url
            .clone()
            .context("CROSS_SEED_LOCAL_URL is not set")?;
        let cross_seed_api_key = read_guard
            .cross_seed_local_api_key
            .clone()
            .context("CROSS_SEED_LOCAL_API_KEY is not set")?;

        let xseed_torrent_clients = read_guard.xseed_torrent_clients.clone();
        let xseed_usenet_clients = read_guard.xseed_usenet_clients.clone();

        (
            cross_seed_url,
            cross_seed_api_key,
            xseed_torrent_clients,
            xseed_usenet_clients,
        )
    };

    let event_type = match &request {
        ArrConnectWebhook::Sonarr(request) => &request.event_type,
        ArrConnectWebhook::Radarr(request) => &request.event_type,
    };

    if event_type == "Test" {
        info!("[/xseed-*] Test event detected.");
        return Ok(());
    }

    trace!("[/seed-*] EventType: {event_type}");

    let download_id = match &request {
        ArrConnectWebhook::Sonarr(request) => request.download_id.clone(),
        ArrConnectWebhook::Radarr(request) => request.download_id.clone(),
    }
    .context("Request does not include a download_id.")?;
    trace!("[/xseed-*] download_id: {download_id}");

    let client_id = match &request {
        ArrConnectWebhook::Sonarr(request) => request.download_client.clone(),
        ArrConnectWebhook::Radarr(request) => request.download_client.clone(),
    }
    .context("Request does not include a download_client.")?;
    trace!("[/xseed-*] client_id: {client_id}");

    let unique_id = format!("{download_id}-{client_id}");
    trace!("[/xseed-*] Unique id: {unique_id}");

    if state
        .read()
        .map_err(|_| anyhow!("Could not read from state."))?
        .xseed_unique_ids
        .contains(&unique_id)
    {
        info!("[/xseed-*] Download ID [{unique_id}] already processed");
        return Ok(());
    }

    let torrent_client = xseed_torrent_clients
        .filter(|clients| clients.contains(&client_id))
        .map(|_| client_id.clone());

    let usenet_client = xseed_usenet_clients
        .filter(|clients| clients.contains(&client_id))
        .map(|_| client_id.clone());

    let resp = if let Some(torrent_client) = torrent_client {
        info!("[/xseed-*] Processing torrent client operations for {torrent_client}");

        // send cross-seed webhook request with infoHash
        let info_hash = download_id.to_string();
        let webhook = WebhookRequest::InfoHash(info_hash);
        let resp = cross_seed_webhook(&cross_seed_url, &cross_seed_api_key, webhook).await?;

        if resp == StatusCode::from_u16(204).unwrap() {
            resp
        } else {
            sleep(Duration::from_secs(15)).await;
            // send cross-seed webhook request with path
            cross_seed_webhook_data(&cross_seed_url, &cross_seed_api_key, &request).await?
        }
    } else if let Some(usenet_client) = usenet_client {
        info!("[/xseed-*] Processing usenet client operations for {usenet_client}");

        // send cross-seed webhook request with path
        cross_seed_webhook_data(&cross_seed_url, &cross_seed_api_key, &request).await?
    } else {
        info!("[/xseed-*] Unrecognized client {client_id}.");
        return Ok(());
    };

    trace!("[/xseed-*] cross-seed API response: {resp}");

    if resp == StatusCode::from_u16(204).unwrap() {
        // update xseed_unique_ids
        state
            .write()
            .map_err(|_| anyhow!("Could not write to state."))?
            .xseed_unique_ids
            .insert(unique_id);
        info!("[/xseed-*] cross-seed completed successfully.");

        let release_title = match &request {
            ArrConnectWebhook::Sonarr(request) => request.release.clone().unwrap().release_title,
            ArrConnectWebhook::Radarr(request) => request.release.clone().unwrap().release_title,
        };

        let discord_webhook_url = {
            let read_guard = state
                .read()
                .map_err(|_| anyhow!("Could not read from state."))?;

            read_guard.discord_webhook_url.clone()
        };

        if let Some(discord_webhook_url) = discord_webhook_url {
            let content = format!("[/xseed-*] cross-seed completed successfully ({release_title})");
            discord_webhook(&discord_webhook_url, &content).await?;
        }

        Ok(())
    } else {
        info!("[/xseed-*] cross-seed failed with status code: {resp}");
        Err(anyhow!("cross-seed failed with status code: {resp}"))
    }
}

pub(crate) async fn xseed_radarr(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<RadarrConnectWebhook>,
) -> Result<impl IntoResponse, StatusCode> {
    match xseed(ArrConnectWebhook::Radarr(payload), state).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(err) => {
            error!("Error occured: {err}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub(crate) async fn xseed_sonarr(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<SonarrConnectWebhook>,
) -> Result<impl IntoResponse, StatusCode> {
    match xseed(ArrConnectWebhook::Sonarr(payload), state).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(err) => {
            error!("Error occured: {err}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
