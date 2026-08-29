mod commands;
mod database;
mod metadata;
mod models;
mod screenshot;
mod thumbnail;
mod tracking;
use crate::{database::Database, models::AppSettings, tracking::TrackingService};
use chrono::Utc;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{
    Manager,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
};
pub struct AppState {
    db: Database,
    tracker: TrackingService,
    thumbnails: PathBuf,
    screenshots: PathBuf,
    social_images: PathBuf,
    screenshot_service: screenshot::ScreenshotService,
    http: reqwest::Client,
    quitting: AtomicBool,
}
impl AppState {
    fn settings(&self) -> AppSettings {
        self.db
            .get_setting("app")
            .ok()
            .flatten()
            .and_then(|x| serde_json::from_str(&x).ok())
            .unwrap_or_default()
    }
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--autostart"])
                .build(),
        )
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            let dirs = directories::BaseDirs::new()
                .ok_or_else(|| anyhow::anyhow!("Application Data directoryを取得できません"))?;
            let root = dirs.data_local_dir().join("ErogePlaytimeTracker");
            std::fs::create_dir_all(&root)?;
            let thumbs = root.join("thumbnails");
            std::fs::create_dir_all(&thumbs)?;
            let screenshots = root.join("screenshots");
            std::fs::create_dir_all(&screenshots)?;
            let social_images = root.join("social-images");
            std::fs::create_dir_all(&social_images)?;
            let db = Database::open(&root.join("app.db"))?;
            let last = db
                .get_setting("last_seen")?
                .unwrap_or_else(|| Utc::now().to_rfc3339());
            let recovered = db.recover_orphans(&last)?;
            if recovered > 0 {
                log::warn!("recovered {recovered} orphan focus intervals; histories require review")
            };
            let migrated = db.migrate_focus_intervals()?;
            if migrated > 0 {
                log::info!("migrated {migrated} sessions to background intervals")
            }
            db.set_setting("last_seen", &Utc::now().to_rfc3339())?;
            let settings = db
                .get_setting("app")?
                .and_then(|x| serde_json::from_str::<AppSettings>(&x).ok())
                .unwrap_or_default();
            let tracker = TrackingService::start(db.clone(), app.handle().clone());
            let screenshot_service = screenshot::ScreenshotService::start(
                app.handle().clone(),
                db.clone(),
                tracker.clone(),
                screenshots.clone(),
                settings.screenshot_hotkey.clone(),
            );
            let show = MenuItemBuilder::with_id("show", "メインウィンドウを開く").build(app)?;
            let status = MenuItemBuilder::with_id("status", "追跡状態: 待機中")
                .enabled(false)
                .build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "終了").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&show, &status, &quit])
                .build()?;
            TrayIconBuilder::with_id("tracker")
                .icon(tauri::include_image!("icons/32x32.png"))
                .menu(&menu)
                .tooltip("Eroge Playtime Tracker")
                .on_menu_event(|app, e| match e.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        if let Some(s) = app.try_state::<AppState>() {
                            s.quitting.store(true, Ordering::Relaxed);
                            s.tracker.shutdown();
                        }
                        app.exit(0)
                    }
                    _ => {}
                })
                .build(app)?;
            app.manage(AppState {
                db,
                tracker,
                thumbnails: thumbs,
                screenshots,
                social_images,
                screenshot_service,
                http: reqwest::Client::new(),
                quitting: AtomicBool::new(false),
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && let Some(s) = window.try_state::<AppState>()
                && !s.quitting.load(Ordering::Relaxed)
                && s.settings().close_to_tray
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_games,
            commands::list_brands,
            commands::list_system_fonts,
            commands::get_game,
            commands::create_game,
            commands::update_game,
            commands::update_game_thumbnail,
            commands::update_game_play_status,
            commands::open_external_url,
            commands::delete_game,
            commands::add_game_executable,
            commands::remove_game_executable,
            commands::launch_game,
            commands::list_sessions,
            commands::list_background_intervals,
            commands::create_manual_session,
            commands::update_session,
            commands::delete_session,
            commands::delete_all_sessions,
            commands::create_background_interval,
            commands::update_background_interval,
            commands::delete_background_interval,
            commands::list_game_timestamps,
            commands::create_game_timestamp,
            commands::update_game_timestamp,
            commands::delete_game_timestamp,
            commands::fetch_erogamescape_metadata,
            commands::import_thumbnail,
            commands::save_cropped_thumbnail,
            commands::refresh_game_metadata,
            commands::get_settings,
            commands::update_settings,
            commands::skip_update_version,
            commands::validate_screenshot_hotkey,
            commands::suspend_screenshot_hotkey,
            commands::resume_screenshot_hotkey,
            commands::get_tracking_status,
            commands::list_game_screenshots,
            commands::delete_game_screenshot,
            commands::open_screenshot_directory,
            commands::save_social_image,
            commands::open_social_image_directory
        ])
        .run(tauri::generate_context!())
        .expect("Tauri application failed");
}
