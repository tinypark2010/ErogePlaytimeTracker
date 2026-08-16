use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct GameSummary {
    pub id: i64,
    pub title: String,
    pub brand: Option<String>,
    pub release_date: Option<String>,
    pub thumbnail_path: Option<String>,
    pub created_at: String,
    pub total_playtime_seconds: i64,
    pub total_running_seconds: i64,
    pub last_played: Option<String>,
    pub session_count: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct Executable {
    pub id: i64,
    pub game_id: i64,
    pub path: String,
    pub file_name: String,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct GameDetail {
    #[serde(flatten)]
    pub summary: GameSummary,
    pub erogamescape_id: Option<i64>,
    pub source_url: Option<String>,
    pub executables: Vec<Executable>,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlaySession {
    pub id: i64,
    pub game_id: i64,
    pub launched_at: String,
    pub exited_at: Option<String>,
    pub needs_review: bool,
    pub foreground_seconds: i64,
    pub running_seconds: Option<i64>,
}
#[derive(Debug, Clone, Serialize)]
pub struct FocusInterval {
    pub id: i64,
    pub play_session_id: i64,
    pub started_at: String,
    pub ended_at: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct CreateGameInput {
    pub title: String,
    pub brand: Option<String>,
    pub release_date: Option<String>,
    pub thumbnail_url: Option<String>,
    pub erogamescape_id: Option<i64>,
    pub source_url: Option<String>,
    #[serde(default)]
    pub executable_paths: Vec<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateGameInput {
    pub title: String,
    pub brand: Option<String>,
    pub release_date: Option<String>,
    pub source_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub autostart: bool,
    pub reconciliation_seconds: u64,
    pub close_to_tray: bool,
    pub theme: String,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autostart: false,
            reconciliation_seconds: 3,
            close_to_tray: true,
            theme: "dark".into(),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct RunningGameStatus {
    pub game_id: i64,
    pub title: String,
    pub session_id: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct TrackingStatus {
    pub running_games: Vec<RunningGameStatus>,
    pub foreground_game_id: Option<i64>,
}
