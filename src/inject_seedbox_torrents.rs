use std::path::Path;
use std::sync::{Arc, RwLock};

use log::{error, info, trace};

use anyhow::{anyhow, Context};

use tokio::time::{sleep, Duration};

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use qbit_rs::{
    model::{AddTorrentArg, Credential, TorrentFile, TorrentSource},
    Qbit,
};

use crate::cross_seed::{cross_seed_webhook, WebhookRequest};
use crate::AppState;

use crate::data_types::radarr::RadarrConnectWebhook;
use crate::data_types::sonarr::SonarrConnectWebhook;

enum ArrConnectWebhook {
    Sonarr(SonarrConnectWebhook),
    Radarr(RadarrConnectWebhook),
}

fn get_category(sourcepath: &Path, local_qbit_dir: &Path) -> String {
    sourcepath
        .strip_prefix(local_qbit_dir)
        .expect("Invalid path")
        .iter()
        .next()
        .expect("Empty relative path")
        .to_str()
        .expect("Invalid path")
        .to_string()
}

async fn inject_seedbox_torrents(
    request: ArrConnectWebhook,
    state: Arc<RwLock<AppState>>,
) -> anyhow::Result<()> {
    let event_type = match &request {
        ArrConnectWebhook::Sonarr(request) => &request.event_type,
        ArrConnectWebhook::Radarr(request) => &request.event_type,
    };

    if event_type == "Test" {
        info!("[/inject-seedbox-torrents] Test event detected.");
        return Ok(());
    }

    let client_id = match &request {
        ArrConnectWebhook::Sonarr(request) => request.download_client.clone(),
        ArrConnectWebhook::Radarr(request) => request.download_client.clone(),
    }
    .context("Request does not include a download_id.")?;
    trace!("[/inject-seedbox-torrents] client_id: {client_id}");

    let qbittorrent_seedbox_name = {
        let read_guard = state
            .read()
            .map_err(|_| anyhow!("Could not read from state."))?;

        read_guard
            .qbittorrent_seedbox_name
            .clone()
            .context("QBITORRENT_SEEDBOX_NAME is not set.")?
    };

    if client_id != qbittorrent_seedbox_name {
        trace!("[/inject-seedbox-torrents] Download using {client_id}, which is not the seedbox qbittorrent");
        return Ok(());
    }

    let download_id = match &request {
        ArrConnectWebhook::Sonarr(request) => request.download_id.clone(),
        ArrConnectWebhook::Radarr(request) => request.download_id.clone(),
    }
    .context("Request does not include a download_client.")?;
    trace!("[/inject-seedbox-torrents] download_id: {download_id}");
    let source_path = match &request {
        ArrConnectWebhook::Sonarr(request) => request.source_path.clone(),
        ArrConnectWebhook::Radarr(request) => request
            .movie_file
            .as_ref()
            .map(|movie_file| movie_file.source_path.clone()),
    }
    .context("Couldn't set source_path based on the request.")?;
    trace!("[/inject-seedbox-torrents] source_path: {source_path}");

    let (
        qbittorrent_local_dir,
        qbittorrent_seedbox_host,
        qbittorrent_seedbox_user,
        qbittorrent_seedbox_password,
        qbittorrent_local_host,
        qbittorrent_local_user,
        qbittorrent_local_password,
        cross_seed_url,
        cross_seed_api_key,
    ) = {
        let read_guard = state
            .read()
            .map_err(|_| anyhow!("Could not read from state."))?;

        let qbittorrent_local_dir = read_guard
            .qbittorrent_local_dir
            .clone()
            .context("QBITORRENT_LOCAL_DIR is not set.")?;
        let qbittorrent_seedbox_host = read_guard
            .qbittorrent_seedbox_host
            .clone()
            .context("QBITORRENT_SEEDBOX_HOST is not set.")?;
        let qbittorrent_seedbox_user = read_guard
            .qbittorrent_seedbox_user
            .clone()
            .context("QBITORRENT_SEEDBOX_USER is not set.")?;
        let qbittorrent_seedbox_password = read_guard
            .qbittorrent_seedbox_password
            .clone()
            .context("QBITORRENT_SEEDBOX_PASSWORD is not set.")?;

        let qbittorrent_local_host = read_guard
            .qbittorrent_local_host
            .clone()
            .context("QBITORRENT_LOCAL_HOST is not set.")?;
        let qbittorrent_local_user = read_guard
            .qbittorrent_local_user
            .clone()
            .context("QBITORRENT_LOCAL_USER is not set.")?;
        let qbittorrent_local_password = read_guard
            .qbittorrent_local_password
            .clone()
            .context("QBITORRENT_LOCAL_PASSWORD is not set.")?;

        let cross_seed_url = read_guard.cross_seed_local_url.clone();
        let cross_seed_api_key = read_guard.cross_seed_local_api_key.clone();

        (
            qbittorrent_local_dir,
            qbittorrent_seedbox_host,
            qbittorrent_seedbox_user,
            qbittorrent_seedbox_password,
            qbittorrent_local_host,
            qbittorrent_local_user,
            qbittorrent_local_password,
            cross_seed_url,
            cross_seed_api_key,
        )
    };
    let local_qbit_dir = Path::new(&qbittorrent_local_dir);
    let source_path = Path::new(&source_path);
    trace!(
        "[/inject-seedbox-torrents] source_path: {}",
        source_path.to_str().unwrap()
    );
    trace!(
        "[/inject-seedbox-torrents] local_qbit_dir: {}",
        local_qbit_dir.to_str().unwrap()
    );
    let category = get_category(source_path, local_qbit_dir);
    trace!("[/inject-seedbox-torrents] category: {category}");

    let qbittorrent_seedbox_host = reqwest::Url::parse(&qbittorrent_seedbox_host)?;
    let qbit_creds = Credential::new(qbittorrent_seedbox_user, qbittorrent_seedbox_password);
    let qbit_seedbox = Qbit::new(qbittorrent_seedbox_host, qbit_creds);

    info!("[/inject-seedbox-torrents] start with exporting...");
    let torrent = qbit_seedbox.export_torrent(&download_id).await?;
    info!("[/inject-seedbox-torrents] exported torrent from qbittorrent-seedbox");

    let local_qbit_host = reqwest::Url::parse(&qbittorrent_local_host)?;
    let qbit_creds = Credential::new(qbittorrent_local_user, qbittorrent_local_password);
    let qbit_local = Qbit::new(local_qbit_host, qbit_creds);

    let torrent_file = TorrentFile {
        filename: download_id.clone(),
        data: torrent.into(),
    };
    let add_torrent_arg = AddTorrentArg {
        source: TorrentSource::TorrentFiles {
            torrents: vec![torrent_file],
        },
        category: Some(category),
        auto_torrent_management: Some(true),
        ..Default::default()
    };
    trace!(
        "[/inject-seedbox-torrents] add_torrent_arg: {:?}",
        add_torrent_arg
    );
    qbit_local.add_torrent(&add_torrent_arg).await?;
    info!("[/inject-seedbox-torrents] inserted torrent into qbittorrent-local");

    if let Some(cross_seed_url) = cross_seed_url {
        let cross_seed_api_key = cross_seed_api_key.context("No API key for cross-seed found.")?;

        sleep(Duration::from_secs(60)).await;

        let webhook = WebhookRequest::InfoHash(download_id);
        let resp = cross_seed_webhook(&cross_seed_url, &cross_seed_api_key, webhook).await?;
        if resp == StatusCode::from_u16(204).unwrap() {
            info!("[/inject-seedbox-torrents] Succesfully called cross-seed.");
        } else {
            info!("[/inject-seedbox-torrents] Calling cross-seed failed!");
        }
    }
    Ok(())
}

pub(crate) async fn inject_seedbox_torrents_radarr(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<RadarrConnectWebhook>,
) -> Result<impl IntoResponse, StatusCode> {
    trace!("[/inject-seedbox-torrents] payload: {payload:?}");
    match inject_seedbox_torrents(ArrConnectWebhook::Radarr(payload), state).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(err) => {
            error!("Error occured: {err}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub(crate) async fn inject_seedbox_torrents_sonarr(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<SonarrConnectWebhook>,
) -> Result<impl IntoResponse, StatusCode> {
    trace!("[/inject-seedbox-torrents] payload: {payload:?}");
    match inject_seedbox_torrents(ArrConnectWebhook::Sonarr(payload), state).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(err) => {
            error!("Error occured: {err}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
