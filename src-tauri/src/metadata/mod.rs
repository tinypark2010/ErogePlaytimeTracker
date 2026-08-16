mod erogamescape;
use anyhow::Result;
use async_trait::async_trait;
pub use erogamescape::ErogameScapeProvider;
use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
pub struct GameMetadata {
    pub erogamescape_id: i64,
    pub title: String,
    pub brand: Option<String>,
    pub release_date: Option<String>,
    pub thumbnail_url: Option<String>,
    pub source_url: String,
}
#[async_trait]
pub trait GameMetadataProvider: Send + Sync {
    async fn fetch_game(&self, input: &str) -> Result<GameMetadata>;
}
