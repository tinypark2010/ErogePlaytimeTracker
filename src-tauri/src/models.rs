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
    pub play_status: String,
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
    pub playtime_seconds: i64,
    pub background_seconds: i64,
    pub running_seconds: Option<i64>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct StatisticsPeriodInput {
    pub kind: String,
    pub year: Option<i32>,
    pub month: Option<u32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct StatisticsPeriod {
    pub kind: String,
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub start_date: String,
    pub end_date: String,
    pub generated_at: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct StatisticsSessionHighlight {
    pub session_id: i64,
    pub game_id: i64,
    pub title: String,
    pub launched_at: String,
    pub exited_at: Option<String>,
    pub playtime_seconds: i64,
    pub needs_review: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct StatisticsDayHighlight {
    pub date: String,
    pub playtime_seconds: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct StatisticsSummary {
    pub total_playtime_seconds: i64,
    pub active_day_count: i64,
    pub average_per_day_seconds: i64,
    pub game_count: i64,
    pub session_count: i64,
    pub average_session_seconds: i64,
    pub longest_session: Option<StatisticsSessionHighlight>,
    pub busiest_day: Option<StatisticsDayHighlight>,
    pub needs_review_session_count: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct StatisticsDayGame {
    pub game_id: i64,
    pub title: String,
    pub thumbnail_path: Option<String>,
    pub playtime_seconds: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct StatisticsDay {
    pub date: String,
    pub playtime_seconds: i64,
    pub games: Vec<StatisticsDayGame>,
}
#[derive(Debug, Clone, Serialize)]
pub struct StatisticsGame {
    pub game_id: i64,
    pub title: String,
    pub brand: Option<String>,
    pub thumbnail_path: Option<String>,
    pub playtime_seconds: i64,
    pub session_count: i64,
    pub active_day_count: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct StatisticsReport {
    pub period: StatisticsPeriod,
    pub summary: StatisticsSummary,
    pub days: Vec<StatisticsDay>,
    pub games: Vec<StatisticsGame>,
    pub available_years: Vec<i32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct BackgroundInterval {
    pub id: i64,
    pub play_session_id: i64,
    pub started_at: String,
    pub ended_at: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct GameTimestamp {
    pub id: i64,
    pub game_id: i64,
    pub name: String,
    pub marked_at: String,
    pub playtime_seconds: i64,
    pub since_previous_seconds: i64,
}
#[derive(Debug, Clone, Deserialize)]
pub struct CreateGameInput {
    pub title: String,
    pub brand: Option<String>,
    pub release_date: Option<String>,
    pub thumbnail_path: Option<String>,
    pub erogamescape_id: Option<i64>,
    pub source_url: Option<String>,
    #[serde(default)]
    pub executable_paths: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct MetadataPreview {
    #[serde(flatten)]
    pub metadata: crate::metadata::GameMetadata,
    pub thumbnail_path: Option<String>,
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
    pub auto_check_updates: bool,
    pub skipped_update_version: Option<String>,
    pub close_to_tray: bool,
    pub theme: String,
    pub screenshot_hotkey: String,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autostart: false,
            auto_check_updates: true,
            skipped_update_version: None,
            close_to_tray: true,
            theme: "dark".into(),
            screenshot_hotkey: String::new(),
        }
    }
}
#[cfg(test)]
mod settings_tests {
    use super::AppSettings;

    #[test]
    fn older_settings_enable_update_checks_and_have_no_skipped_version() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"autostart":false,"close_to_tray":true,"theme":"dark","screenshot_hotkey":""}"#,
        )
        .unwrap();

        assert!(settings.auto_check_updates);
        assert_eq!(settings.skipped_update_version, None);
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct GameScreenshot {
    pub id: i64,
    pub game_id: i64,
    pub play_session_id: Option<i64>,
    pub path: String,
    pub captured_at: String,
    pub width: i64,
    pub height: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct TrackingGameStatus {
    pub game_id: i64,
    pub title: String,
    pub session_id: i64,
    pub phase: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct TrackingStatus {
    pub games: Vec<TrackingGameStatus>,
}
