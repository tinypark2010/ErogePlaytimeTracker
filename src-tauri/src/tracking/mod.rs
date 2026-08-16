mod platform;
pub mod state;
use crate::{
    database::{Database, normalize_path},
    models::{RunningGameStatus, TrackingStatus},
};
use chrono::Utc;
use parking_lot::Mutex;
use state::{Action, TrackerState};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter};
#[derive(Clone)]
pub struct TrackingService {
    inner: Arc<Inner>,
}
struct Inner {
    db: Database,
    state: Mutex<TrackerState>,
    open_interval: Mutex<Option<i64>>,
    observed_foreground: Mutex<Option<(u32, String)>>,
    // PID -> (game ID, normalized executable path). Keeping the path prevents
    // a recycled PID from inheriting a stale game association.
    known_pid_games: Mutex<HashMap<u32, (i64, String)>>,
    stop: AtomicBool,
    app: AppHandle,
}
impl TrackingService {
    pub fn start(db: Database, app: AppHandle, seconds: u64) -> Self {
        let this = Self {
            inner: Arc::new(Inner {
                db,
                state: Mutex::new(TrackerState::default()),
                open_interval: Mutex::new(None),
                observed_foreground: Mutex::new(None),
                known_pid_games: Mutex::new(HashMap::new()),
                stop: AtomicBool::new(false),
                app,
            }),
        };
        let worker = this.clone();
        let events = platform::foreground_events();
        thread::spawn(move || {
            log::info!("tracker started with EVENT_SYSTEM_FOREGROUND hook");
            while !worker.inner.stop.load(Ordering::Relaxed) {
                if let Err(e) = worker.tick() {
                    log::error!("tracking reconciliation failed: {e:#}")
                }
                let _ = events.recv_timeout(Duration::from_secs(seconds.clamp(2, 30)));
            }
            log::info!("tracker stopped")
        });
        this
    }
    fn tick(&self) -> anyhow::Result<()> {
        let registered = self.inner.db.registered_executables()?;
        let by_path: HashMap<String, i64> = registered
            .iter()
            .map(|(g, _, p)| (normalize_path(p), *g))
            .collect();
        let mut game_roots: HashMap<i64, Vec<String>> = HashMap::new();
        for (game, _, path) in &registered {
            if let Some(root) = executable_root(path) {
                game_roots.entry(*game).or_default().push(root);
            }
        }
        let processes = platform::processes()?;
        let mut pid_game = self.inner.known_pid_games.lock();
        pid_game.retain(|pid, (_, known_path)| {
            processes
                .get(pid)
                .and_then(|process| process.path.as_deref())
                .is_some_and(|path| normalize_path(path) == *known_path)
        });
        for (pid, info) in &processes {
            if let Some(g) = info
                .path
                .as_deref()
                .and_then(|path| by_path.get(&normalize_path(path)))
                && let Some(path) = info.path.as_deref()
            {
                pid_game.insert(*pid, (*g, normalize_path(path)));
            }
        }
        // Associate descendants with a registered launcher/game process only
        // when their executable is inside the registered game's directory.
        // DRM services and other global helpers can outlive the game and must
        // not keep its session open.
        let mut changed = true;
        while changed {
            changed = false;
            for (pid, info) in &processes {
                if !pid_game.contains_key(pid)
                    && let Some(game) = pid_game.get(&info.parent_pid).map(|known| known.0)
                    && let Some(path) = info.path.as_deref()
                    && is_in_game_root(path, game_roots.get(&game))
                {
                    pid_game.insert(*pid, (game, normalize_path(path)));
                    log::info!(
                        "associated child process pid={pid} executable={} game={game}",
                        info.path
                            .as_deref()
                            .and_then(|p| std::path::Path::new(p).file_name())
                            .and_then(|p| p.to_str())
                            .unwrap_or("<unavailable>")
                    );
                    changed = true;
                }
            }
        }
        let alive: HashSet<i64> = pid_game.values().map(|known| known.0).collect();
        let at = Utc::now().to_rfc3339();
        let mut state = self.inner.state.lock();
        let actions =
            state.reconcile_running(&alive, |g| match self.inner.db.start_session(g, &at) {
                Ok(id) => {
                    log::info!("session started game={g} session={id}");
                    id
                }
                Err(e) => {
                    log::error!("session start failed: {e:#}");
                    -1
                }
            });
        drop(state);
        self.apply(actions, &at)?;
        let foreground = platform::foreground_pid().map(|pid| {
            let path = platform::process_path(pid);
            let game = pid_game.get(&pid).map(|known| known.0).or_else(|| {
                path.as_deref()
                    .and_then(|p| by_path.get(&normalize_path(p)).copied())
            });
            (pid, path, game)
        });
        if !alive.is_empty() {
            let observed = foreground
                .as_ref()
                .map(|(pid, path, _)| (*pid, path.clone().unwrap_or_default()));
            let mut previous = self.inner.observed_foreground.lock();
            if *previous != observed {
                match &foreground {
                    Some((pid, path, game)) => log::info!(
                        "foreground transition pid={pid} executable={} matched_game={game:?}",
                        path.as_deref()
                            .and_then(|p| std::path::Path::new(p).file_name())
                            .and_then(|p| p.to_str())
                            .unwrap_or("<unavailable>")
                    ),
                    None => log::info!("foreground transition: no foreground window"),
                }
                *previous = observed;
            }
        }
        let fg = foreground.and_then(|(_, _, game)| game);
        drop(pid_game);
        let actions = self.inner.state.lock().foreground(fg);
        self.apply(actions, &at)?;
        self.inner.db.set_setting("last_seen", &at)?;
        self.emit();
        Ok(())
    }
    fn apply(&self, actions: Vec<Action>, at: &str) -> anyhow::Result<()> {
        for a in actions {
            match a {
                Action::SessionStarted { game_id } => {
                    log::info!("registered process detected game={game_id}")
                }
                Action::SessionEnded {
                    game_id,
                    session_id,
                } => {
                    if session_id >= 0 {
                        self.inner.db.end_session(session_id, at)?
                    }
                    log::info!("session ended game={game_id}")
                }
                Action::FocusStarted {
                    game_id,
                    session_id,
                } => {
                    if session_id >= 0 {
                        *self.inner.open_interval.lock() =
                            Some(self.inner.db.start_interval(session_id, at)?)
                    }
                    log::info!("foreground entered game={game_id}")
                }
                Action::FocusEnded { game_id } => {
                    if let Some(id) = self.inner.open_interval.lock().take() {
                        self.inner.db.end_interval(id, at)?
                    }
                    log::info!("foreground left game={game_id}")
                }
            }
        }
        Ok(())
    }
    fn emit(&self) {
        let status = self.status();
        if let Some(tray) = self.inner.app.tray_by_id("tracker") {
            let label = if status.running_games.is_empty() {
                "Eroge Playtime Tracker - idle".to_string()
            } else {
                format!(
                    "Eroge Playtime Tracker - tracking {} game(s)",
                    status.running_games.len()
                )
            };
            let _ = tray.set_tooltip(Some(label));
        }
        let _ = self.inner.app.emit("tracking-status", status);
    }
    pub fn status(&self) -> TrackingStatus {
        let s = self.inner.state.lock();
        let names: HashMap<i64, String> = self
            .inner
            .db
            .registered_executables()
            .unwrap_or_default()
            .into_iter()
            .map(|(g, n, _)| (g, n))
            .collect();
        TrackingStatus {
            running_games: s
                .running()
                .iter()
                .map(|(g, x)| RunningGameStatus {
                    game_id: *g,
                    title: names.get(g).cloned().unwrap_or_default(),
                    session_id: *x,
                })
                .collect(),
            foreground_game_id: s.focused(),
        }
    }
    pub fn shutdown(&self) {
        self.inner.stop.store(true, Ordering::Relaxed);
        let at = Utc::now().to_rfc3339();
        let mut s = self.inner.state.lock();
        let actions = s.reconcile_running(&HashSet::new(), |_| -1);
        drop(s);
        if let Err(e) = self.apply(actions, &at) {
            log::error!("clean shutdown failed: {e:#}")
        }
    }
}

fn executable_root(path: &str) -> Option<String> {
    let path = normalize_path(path);
    let (root, _) = path.rsplit_once('\\')?;
    Some(format!("{root}\\"))
}

fn is_in_game_root(path: &str, roots: Option<&Vec<String>>) -> bool {
    let path = normalize_path(path);
    roots.is_some_and(|roots| roots.iter().any(|root| path.starts_with(root)))
}

#[cfg(test)]
mod process_association_tests {
    use super::*;

    #[test]
    fn descendants_must_stay_in_the_game_directory() {
        let roots = vec![executable_root(r"D:\vn\game\launcher.exe").unwrap()];
        assert!(is_in_game_root(r"D:\vn\game\main.bin", Some(&roots)));
        assert!(is_in_game_root(r"D:\vn\game\engine\game.exe", Some(&roots)));
        assert!(!is_in_game_root(
            r"C:\Program Files (x86)\SoftDenchi\SdProxy.exe",
            Some(&roots)
        ));
        assert!(!is_in_game_root(r"D:\vn\game-old\main.exe", Some(&roots)));
    }
}
