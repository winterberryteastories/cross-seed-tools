use std::collections::HashSet;
use std::env;
use std::sync::{Arc, RwLock};

use env_logger::Env;
use log::{info, warn};

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::IntoResponse,
    routing::post,
    Router,
};

mod data_types;

mod announce;
mod inject_seedbox_torrents;
mod xseed;

mod cross_seed;

use crate::announce::announce;
use crate::inject_seedbox_torrents::{
    inject_seedbox_torrents_radarr, inject_seedbox_torrents_sonarr,
};
use crate::xseed::{xseed_radarr, xseed_sonarr};

#[derive(Clone, Default)]
pub struct AppState {
    api_key: String,

    cross_seed_seedbox_url: Option<String>,
    cross_seed_seedbox_api_key: Option<String>,

    cross_seed_local_url: Option<String>,
    cross_seed_local_api_key: Option<String>,

    xseed_torrent_clients: Option<Vec<String>>,
    xseed_usenet_clients: Option<Vec<String>>,

    xseed_unique_ids: HashSet<String>,

    qbittorrent_local_host: Option<String>,
    qbittorrent_local_user: Option<String>,
    qbittorrent_local_password: Option<String>,

    qbittorrent_seedbox_host: Option<String>,
    qbittorrent_seedbox_user: Option<String>,
    qbittorrent_seedbox_password: Option<String>,

    qbittorrent_local_dir: Option<String>,
    qbittorrent_seedbox_name: Option<String>,
}

// Middleware for authentication
async fn auth_middleware(
    State(state): State<Arc<RwLock<AppState>>>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let api_key = request
        .headers()
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok());

    if api_key == Some(&state.read().unwrap().api_key) {
        Ok(next.run(request).await)
    } else {
        warn!("UNAUTHORIZED request: {request:?}");
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn create_config_state() -> anyhow::Result<Arc<RwLock<AppState>>> {
    let xseed_torrent_clients = match env::var("XSEED_TORRENT_CLIENTS") {
        Ok(torrent_clients) => Some(
            torrent_clients
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        ),
        _ => None,
    };
    let xseed_usenet_clients = match env::var("XSEED_USENET_CLIENTS") {
        Ok(usenet_clients) => Some(
            usenet_clients
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        ),
        _ => None,
    };

    let state = Arc::new(RwLock::new(AppState {
        api_key: env::var("API_KEY")?,

        cross_seed_seedbox_url: env::var("CROSS_SEED_SEEDBOX_URL").ok(),
        cross_seed_seedbox_api_key: env::var("CROSS_SEED_SEEDBOX_API_KEY").ok(),

        cross_seed_local_url: env::var("CROSS_SEED_LOCAL_URL").ok(),
        cross_seed_local_api_key: env::var("CROSS_SEED_LOCAL_API_KEY").ok(),

        xseed_torrent_clients,
        xseed_usenet_clients,

        xseed_unique_ids: HashSet::new(),

        qbittorrent_local_host: env::var("QBITTORRENT_LOCAL_HOST").ok(),
        qbittorrent_local_user: env::var("QBITTORRENT_LOCAL_USER").ok(),
        qbittorrent_local_password: env::var("QBITTORRENT_LOCAL_PASSWORD").ok(),

        qbittorrent_seedbox_name: env::var("QBITTORRENT_SEEDBOX_NAME").ok(),
        qbittorrent_seedbox_user: env::var("QBITTORRENT_SEEDBOX_USER").ok(),
        qbittorrent_seedbox_password: env::var("QBITTORRENT_SEEDBOX_PASSWORD").ok(),

        qbittorrent_local_dir: env::var("QBITTORRENT_LOCAL_DIR").ok(),
        qbittorrent_seedbox_host: env::var("QBITTORRENT_SEEDBOX_HOST").ok(),
    }));
    Ok(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    dotenvy::dotenv()?;

    let host = &env::var("HOST")?;
    let state = create_config_state()?;

    let router = Router::new()
        .route("/announce", post(announce))
        .route("/xseed-sonarr", post(xseed_sonarr))
        .route("/xseed-radarr", post(xseed_radarr))
        .route(
            "/inject-seedbox-torrents-sonarr",
            post(inject_seedbox_torrents_sonarr),
        )
        .route(
            "/inject-seedbox-torrents-radarr",
            post(inject_seedbox_torrents_radarr),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(host).await.unwrap();

    info!("Run server on {host}...");
    axum::serve(listener, router).await.unwrap();

    Ok(())
}
