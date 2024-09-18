use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct SonarrRelease {
    pub indexer: String,
    #[serde(rename = "releaseTitle")]
    pub release_title: String,
    #[serde(rename = "releaseType")]
    pub release_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct SonarrEpisodeFile {
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct SonarrConnectWebhook {
    #[serde(rename = "destinationPath")]
    pub destination_path: Option<String>,
    #[serde(rename = "instanceName")]
    pub instance_name: Option<String>,
    #[serde(rename = "downloadClient")]
    pub download_client: Option<String>,
    #[serde(rename = "downloadId")]
    pub download_id: Option<String>,
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "episodeFiles")]
    pub episode_files: Option<Vec<SonarrEpisodeFile>>,
    pub release: Option<SonarrRelease>,
    #[serde(rename = "sourcePath")]
    pub source_path: Option<String>,
}
