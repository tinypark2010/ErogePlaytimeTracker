use crate::{
    AppState,
    metadata::{ErogameScapeProvider, GameMetadataProvider},
    models::*,
};
use tauri::State;
type Cmd<T> = Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
#[tauri::command]
pub fn list_games(
    state: State<AppState>,
    search: String,
    brand: Option<String>,
    play_status: Option<String>,
    sort: String,
    descending: bool,
) -> Cmd<Vec<GameSummary>> {
    state
        .db
        .list_games(
            &search,
            brand.as_deref(),
            play_status.as_deref(),
            &sort,
            descending,
        )
        .map_err(err)
}
#[tauri::command]
pub fn list_brands(state: State<AppState>) -> Cmd<Vec<String>> {
    state.db.list_brands().map_err(err)
}
#[tauri::command]
pub fn get_game(state: State<AppState>, id: i64) -> Cmd<GameDetail> {
    state.db.get_game(id).map_err(err)
}
#[tauri::command]
pub fn create_game(state: State<AppState>, input: CreateGameInput) -> Cmd<i64> {
    let local_thumbnail = input
        .thumbnail_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty());
    let thumb = local_thumbnail
        .map(|path| {
            crate::thumbnail::import_local(path, &state.thumbnails)
                .map(|path| path.to_string_lossy().to_string())
        })
        .transpose()
        .map_err(err)?;
    state.db.create_game(&input, thumb.as_deref()).map_err(err)
}
#[tauri::command]
pub fn update_game(state: State<AppState>, id: i64, input: UpdateGameInput) -> Cmd<()> {
    state.db.update_game(id, &input).map_err(err)
}
#[tauri::command]
pub fn update_game_thumbnail(
    state: State<AppState>,
    game_id: i64,
    thumbnail_path: Option<String>,
) -> Cmd<()> {
    let local_thumbnail = thumbnail_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty());
    let stored_thumbnail = local_thumbnail
        .map(|path| {
            crate::thumbnail::import_local(path, &state.thumbnails)
                .map(|path| path.to_string_lossy().to_string())
        })
        .transpose()
        .map_err(err)?;
    state
        .db
        .update_game_thumbnail(game_id, stored_thumbnail.as_deref())
        .map_err(err)
}
#[tauri::command]
pub fn update_game_play_status(state: State<AppState>, id: i64, status: String) -> Cmd<()> {
    state.db.update_play_status(id, &status).map_err(err)
}
#[tauri::command]
pub fn open_external_url(url: String) -> Cmd<()> {
    let parsed = url::Url::parse(&url).map_err(|_| "URLの形式が正しくありません".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("HTTPまたはHTTPSのURLのみ開けます".into());
    }
    let operation: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let target: Vec<u16> = parsed
        .as_str()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        core::PCWSTR,
    };
    // ShellExecuteW uses the user's Windows URL association, which opens HTTP(S)
    // targets in the configured default browser. The UTF-16 buffers live for the call.
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err(format!(
            "既定のブラウザを開けませんでした (ShellExecute error {})",
            result.0 as isize
        ))
    } else {
        Ok(())
    }
}
#[tauri::command]
pub fn delete_game(state: State<AppState>, id: i64) -> Cmd<()> {
    state.db.delete_game(id).map_err(err)?;
    let directory = state.screenshots.join(id.to_string());
    if directory.exists() {
        std::fs::remove_dir_all(directory).map_err(err)?;
    }
    Ok(())
}
#[tauri::command]
pub fn add_game_executable(state: State<AppState>, game_id: i64, path: String) -> Cmd<()> {
    state.db.add_executable(game_id, &path).map_err(err)
}
#[tauri::command]
pub fn remove_game_executable(state: State<AppState>, id: i64) -> Cmd<()> {
    state.db.remove_executable(id).map_err(err)
}
#[tauri::command]
pub fn launch_game(state: State<AppState>, game_id: i64) -> Cmd<()> {
    let path = state.db.launcher_path(game_id).map_err(err)?;
    let executable = std::path::Path::new(&path);
    if !executable.is_file() {
        return Err(format!("実行ファイルが見つかりません: {path}"));
    }
    let mut command = std::process::Command::new(executable);
    if let Some(directory) = executable.parent() {
        command.current_dir(directory);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("ゲームを起動できません: {e}"))
}
#[tauri::command]
pub fn list_sessions(state: State<AppState>, game_id: i64) -> Cmd<Vec<PlaySession>> {
    state.db.list_sessions(game_id).map_err(err)
}
#[tauri::command]
pub fn list_background_intervals(
    state: State<AppState>,
    session_id: i64,
) -> Cmd<Vec<BackgroundInterval>> {
    state.db.intervals(session_id).map_err(err)
}
#[tauri::command]
pub fn create_manual_session(
    state: State<AppState>,
    game_id: i64,
    start: String,
    end: String,
) -> Cmd<i64> {
    state.db.manual_session(game_id, &start, &end).map_err(err)
}
#[tauri::command]
pub fn update_session(
    state: State<AppState>,
    id: i64,
    start: String,
    end: Option<String>,
) -> Cmd<()> {
    state
        .db
        .update_session(id, &start, end.as_deref())
        .map_err(err)
}
#[tauri::command]
pub fn delete_session(state: State<AppState>, id: i64) -> Cmd<()> {
    state.db.delete_session(id).map_err(err)
}
#[tauri::command]
pub fn delete_all_sessions(state: State<AppState>, game_id: i64) -> Cmd<usize> {
    state.db.delete_game_sessions(game_id).map_err(err)
}
#[tauri::command]
pub fn create_background_interval(
    state: State<AppState>,
    session_id: i64,
    start: String,
    end: String,
) -> Cmd<i64> {
    state
        .db
        .create_interval(session_id, &start, &end)
        .map_err(err)
}
#[tauri::command]
pub fn update_background_interval(
    state: State<AppState>,
    id: i64,
    start: String,
    end: Option<String>,
) -> Cmd<()> {
    state
        .db
        .update_interval(id, &start, end.as_deref())
        .map_err(err)
}
#[tauri::command]
pub fn delete_background_interval(state: State<AppState>, id: i64) -> Cmd<()> {
    state.db.delete_interval(id).map_err(err)
}
#[tauri::command]
pub fn list_game_timestamps(state: State<AppState>, game_id: i64) -> Cmd<Vec<GameTimestamp>> {
    state.db.timestamps(game_id).map_err(err)
}
#[tauri::command]
pub fn create_game_timestamp(state: State<AppState>, game_id: i64, name: String) -> Cmd<i64> {
    state
        .db
        .create_timestamp(game_id, &name, &chrono::Utc::now().to_rfc3339())
        .map_err(err)
}
#[tauri::command]
pub fn update_game_timestamp(
    state: State<AppState>,
    id: i64,
    name: String,
    marked_at: String,
) -> Cmd<()> {
    state
        .db
        .update_timestamp(id, &name, &marked_at)
        .map_err(err)
}
#[tauri::command]
pub fn delete_game_timestamp(state: State<AppState>, id: i64) -> Cmd<()> {
    state.db.delete_timestamp(id).map_err(err)
}
#[tauri::command]
pub async fn fetch_erogamescape_metadata(
    state: State<'_, AppState>,
    value: String,
) -> Cmd<MetadataPreview> {
    let metadata = ErogameScapeProvider::new()
        .map_err(err)?
        .fetch_game(&value)
        .await
        .map_err(|e| {
            log::warn!("metadata fetch/parse failed: {e:#}");
            err(e)
        })?;
    let thumbnail_path = if let Some(url) = &metadata.thumbnail_url {
        match crate::thumbnail::download(&state.http, url, &state.thumbnails).await {
            Ok(path) => Some(path.to_string_lossy().to_string()),
            Err(error) => {
                log::warn!("thumbnail preview download failed: {error:#}");
                None
            }
        }
    } else {
        None
    };
    Ok(MetadataPreview {
        metadata,
        thumbnail_path,
    })
}
#[tauri::command]
pub fn import_thumbnail(state: State<AppState>, path: String) -> Cmd<String> {
    crate::thumbnail::import_local(path.trim(), &state.thumbnails)
        .map(|path| path.to_string_lossy().to_string())
        .map_err(err)
}
#[tauri::command]
pub fn save_cropped_thumbnail(state: State<AppState>, png_base64: String) -> Cmd<String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let bytes = STANDARD.decode(png_base64).map_err(err)?;
    crate::thumbnail::store_png(&bytes, &state.thumbnails)
        .map(|path| path.to_string_lossy().to_string())
        .map_err(err)
}
#[tauri::command]
pub async fn refresh_game_metadata(state: State<'_, AppState>, game_id: i64) -> Cmd<()> {
    let (id, url) = state.db.metadata_identity(game_id).map_err(err)?;
    let input = url
        .or_else(|| id.map(|x| x.to_string()))
        .ok_or("ErogameScape ID/URLがありません")?;
    let m = ErogameScapeProvider::new()
        .map_err(err)?
        .fetch_game(&input)
        .await
        .map_err(err)?;
    let thumb = if let Some(url) = &m.thumbnail_url {
        Some(
            crate::thumbnail::download(&state.http, url, &state.thumbnails)
                .await
                .map(|path| path.to_string_lossy().to_string())
                .map_err(err)?,
        )
    } else {
        None
    };
    state
        .db
        .apply_metadata(game_id, &m, thumb.as_deref())
        .map_err(err)
}
#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Cmd<AppSettings> {
    Ok(state.settings())
}
#[tauri::command]
pub fn update_settings(
    app: tauri::AppHandle,
    state: State<AppState>,
    mut settings: AppSettings,
) -> Cmd<()> {
    if !matches!(settings.theme.as_str(), "dark" | "light" | "pink" | "blue") {
        return Err("未対応のカラーテーマです".into());
    }
    crate::screenshot::validate_hotkey(&settings.screenshot_hotkey).map_err(err)?;
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    let autostart_enabled = manager.is_enabled().map_err(err)?;
    if settings.autostart != autostart_enabled {
        if settings.autostart {
            manager.enable().map_err(err)?
        } else {
            manager.disable().map_err(err)?
        }
    }
    state
        .screenshot_service
        .set_hotkey(settings.screenshot_hotkey.clone())
        .map_err(err)?;
    // A skip can be saved by the update prompt while the settings screen is open.
    // Preserve the latest value instead of overwriting it with the screen's snapshot.
    settings.skipped_update_version = state.settings().skipped_update_version;
    state
        .db
        .set_setting("app", &serde_json::to_string(&settings).map_err(err)?)
        .map_err(err)
}
#[tauri::command]
pub fn skip_update_version(state: State<AppState>, version: String) -> Cmd<()> {
    let mut settings = state.settings();
    settings.skipped_update_version = Some(version);
    state
        .db
        .set_setting("app", &serde_json::to_string(&settings).map_err(err)?)
        .map_err(err)
}
#[tauri::command]
pub fn validate_screenshot_hotkey(state: State<AppState>, hotkey: String) -> Cmd<()> {
    state.screenshot_service.check_hotkey(hotkey).map_err(err)
}
#[tauri::command]
pub fn suspend_screenshot_hotkey(state: State<AppState>) -> Cmd<()> {
    state
        .screenshot_service
        .set_hotkey(String::new())
        .map_err(err)
}
#[tauri::command]
pub fn resume_screenshot_hotkey(state: State<AppState>) -> Cmd<()> {
    state
        .screenshot_service
        .set_hotkey(state.settings().screenshot_hotkey)
        .map_err(err)
}
#[tauri::command]
pub fn get_tracking_status(state: State<AppState>) -> TrackingStatus {
    state.tracker.status()
}

#[tauri::command]
pub fn list_game_screenshots(state: State<AppState>, game_id: i64) -> Cmd<Vec<GameScreenshot>> {
    state.db.screenshots(game_id).map_err(err)
}

#[tauri::command]
pub fn delete_game_screenshot(state: State<AppState>, id: i64) -> Cmd<()> {
    if let Some(path) = state.db.remove_screenshot(id).map_err(err)? {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(err(e)),
        }
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn open_screenshot_directory(state: State<AppState>, game_id: i64) -> Cmd<()> {
    // Confirm the game exists before creating anything under its screenshot root.
    state.db.get_game(game_id).map_err(err)?;
    let directory = state.screenshots.join(game_id.to_string());
    std::fs::create_dir_all(&directory).map_err(err)?;
    let operation: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let target: Vec<u16> = directory
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        core::PCWSTR,
    };
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err("スクリーンショットの保存先を開けませんでした".into())
    } else {
        Ok(())
    }
}
