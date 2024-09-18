use std::sync::{Arc, RwLock};

use log::{error, info};

use serde::{Deserialize, Serialize};

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use anyhow::{anyhow, Context};

use crate::cross_seed::{cross_seed_announce, AnnounceRequest};
use crate::AppState;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Announce {
    name: String,
    guid: String,
    link: String,
    tracker: String,
}

impl From<Announce> for AnnounceRequest {
    fn from(announce: Announce) -> Self {
        AnnounceRequest {
            name: announce.name,
            guid: announce.guid,
            link: announce.link,
            tracker: announce.tracker,
        }
    }
}

async fn do_announce(state: Arc<RwLock<AppState>>, announce: Announce) -> anyhow::Result<bool> {
    let name = announce.name.clone();

    info!("[/announce] Release {name} checking...");

    let (
        cross_seed_seedbox_url,
        cross_seed_seedbox_api_key,
        cross_seed_local_url,
        cross_seed_local_api_key,
    ) = {
        let read_guard = state
            .read()
            .map_err(|_| anyhow!("Could not read from state."))?;

        let cross_seed_seedbox_url = read_guard.cross_seed_seedbox_url.clone();
        let cross_seed_seedbox_api_key = read_guard.cross_seed_seedbox_api_key.clone();
        let cross_seed_local_url = read_guard.cross_seed_local_url.clone();
        let cross_seed_local_api_key = read_guard.cross_seed_local_api_key.clone();

        (
            cross_seed_seedbox_url,
            cross_seed_seedbox_api_key,
            cross_seed_local_url,
            cross_seed_local_api_key,
        )
    };

    let mut any_success = false;

    if let Some(cross_seed_url) = cross_seed_seedbox_url {
        let cross_seed_api_key =
            cross_seed_seedbox_api_key.context("No API key for cross-seed seedbox found.")?;

        if let Ok(status_code) = cross_seed_announce(
            cross_seed_url,
            cross_seed_api_key,
            &(announce.clone().into()),
        )
        .await
        {
            if status_code.as_u16() == 200 {
                info!("[/announce] Release {name} accepted by cross-seed-seedbox.");
                any_success = true;
            }
        } else {
            info!("[/announce] Error returned from cross-seed seedbox API");
        }
    }

    if let Some(cross_seed_url) = cross_seed_local_url {
        let cross_seed_api_key =
            cross_seed_local_api_key.context("No API key for cross-seed local found.")?;

        if let Ok(status_code) = cross_seed_announce(
            cross_seed_url,
            cross_seed_api_key,
            &(announce.clone().into()),
        )
        .await
        {
            if status_code.as_u16() == 200 {
                info!("[/announce] Release {name} accepted by cross-seed-seedbox.");
                any_success = true;
            }
        } else {
            info!("[/announce] Error returned from cross-seed seedbox API");
        }
    }

    Ok(any_success)
}

pub(crate) async fn announce(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(payload): Json<Announce>,
) -> Result<impl IntoResponse, StatusCode> {
    match do_announce(state, payload).await {
        Ok(success) => {
            if success {
                Ok(StatusCode::OK)
            } else {
                error!("Failed to handle request correctly.");
                Err(StatusCode::BAD_REQUEST)
            }
        }
        Err(err) => {
            error!("Error occured: {err}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
