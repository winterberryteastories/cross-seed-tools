use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct RadarrMovieFile {
    pub path: String,
    #[serde(rename = "sourcePath")]
    pub source_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct RadarrConnectWebhook {
    #[serde(rename = "downloadClient")]
    pub download_client: Option<String>,
    #[serde(rename = "downloadId")]
    pub download_id: Option<String>,
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "movieFile")]
    pub movie_file: Option<RadarrMovieFile>,
    #[serde(rename = "sourcePath")]
    pub source_path: Option<String>,
}
