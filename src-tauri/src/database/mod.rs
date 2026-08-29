use crate::models::*;
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use std::{path::Path, sync::Arc};

#[derive(Clone)]
pub struct Database(Arc<Mutex<Connection>>);
const MIGRATION_1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
CREATE TABLE brands(id INTEGER PRIMARY KEY, erogamescape_id INTEGER, name TEXT NOT NULL COLLATE NOCASE UNIQUE);
CREATE TABLE games(id INTEGER PRIMARY KEY, erogamescape_id INTEGER UNIQUE, title TEXT NOT NULL, brand_id INTEGER REFERENCES brands(id) ON DELETE SET NULL, release_date TEXT, thumbnail_path TEXT, source_url TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE game_executables(id INTEGER PRIMARY KEY, game_id INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE, path TEXT NOT NULL COLLATE NOCASE UNIQUE, file_name TEXT NOT NULL COLLATE NOCASE, created_at TEXT NOT NULL);
CREATE TABLE play_sessions(id INTEGER PRIMARY KEY, game_id INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE, launched_at TEXT NOT NULL, exited_at TEXT, needs_review INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, CHECK(exited_at IS NULL OR exited_at >= launched_at));
CREATE TABLE focus_intervals(id INTEGER PRIMARY KEY, play_session_id INTEGER NOT NULL REFERENCES play_sessions(id) ON DELETE CASCADE, started_at TEXT NOT NULL, ended_at TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, CHECK(ended_at IS NULL OR ended_at >= started_at));
CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE INDEX idx_games_brand ON games(brand_id);CREATE INDEX idx_exec_path ON game_executables(path);CREATE INDEX idx_session_game_time ON play_sessions(game_id,launched_at);CREATE INDEX idx_interval_session_time ON focus_intervals(play_session_id,started_at);
"#;
const MIGRATION_2: &str = r#"
CREATE TABLE game_timestamps(
    id INTEGER PRIMARY KEY,
    game_id INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    marked_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_game_timestamps_game_time ON game_timestamps(game_id,marked_at);
"#;
const MIGRATION_3: &str = r#"
ALTER TABLE games ADD COLUMN play_status TEXT NOT NULL DEFAULT 'unplayed'
    CHECK(play_status IN ('unplayed','playing','completed','retired'));
CREATE INDEX idx_games_play_status ON games(play_status);
"#;
const MIGRATION_4: &str = r#"
CREATE TABLE background_intervals(
    id INTEGER PRIMARY KEY,
    play_session_id INTEGER NOT NULL REFERENCES play_sessions(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK(ended_at IS NULL OR ended_at >= started_at)
);
CREATE INDEX idx_background_interval_session_time
    ON background_intervals(play_session_id,started_at);
ALTER TABLE play_sessions ADD COLUMN background_migrated INTEGER NOT NULL DEFAULT 0;
"#;
const MIGRATION_5: &str = r#"
CREATE TABLE game_screenshots(
    id INTEGER PRIMARY KEY,
    game_id INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    play_session_id INTEGER REFERENCES play_sessions(id) ON DELETE SET NULL,
    path TEXT NOT NULL UNIQUE,
    captured_at TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL
);
CREATE INDEX idx_game_screenshots_game_time ON game_screenshots(game_id,captured_at);
"#;

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("databaseを開けません: {}", path.display()))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let db = Self(Arc::new(Mutex::new(conn)));
        db.migrate()?;
        Ok(db)
    }
    #[cfg(test)]
    pub fn memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Self(Arc::new(Mutex::new(conn)));
        db.migrate()?;
        Ok(db)
    }
    fn migrate(&self) -> Result<()> {
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);")?;
        let done: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=1)",
            [],
            |r| r.get(0),
        )?;
        if !done {
            tx.execute_batch(MIGRATION_1)?;
            tx.execute("INSERT INTO schema_migrations VALUES(1,?)", [now()])?;
        }
        let done: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=2)",
            [],
            |r| r.get(0),
        )?;
        if !done {
            tx.execute_batch(MIGRATION_2)?;
            tx.execute("INSERT INTO schema_migrations VALUES(2,?)", [now()])?;
        }
        let done: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=3)",
            [],
            |r| r.get(0),
        )?;
        if !done {
            tx.execute_batch(MIGRATION_3)?;
            tx.execute("INSERT INTO schema_migrations VALUES(3,?)", [now()])?;
        }
        let done: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=4)",
            [],
            |r| r.get(0),
        )?;
        if !done {
            tx.execute_batch(MIGRATION_4)?;
            tx.execute("INSERT INTO schema_migrations VALUES(4,?)", [now()])?;
        }
        let done: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=5)",
            [],
            |r| r.get(0),
        )?;
        if !done {
            tx.execute_batch(MIGRATION_5)?;
            tx.execute("INSERT INTO schema_migrations VALUES(5,?)", [now()])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn create_game(&self, input: &CreateGameInput, thumbnail: Option<&str>) -> Result<i64> {
        if input.title.trim().is_empty() {
            bail!("タイトルは必須です")
        };
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let brand = brand_id(&tx, input.brand.as_deref())?;
        let n = now();
        tx.execute("INSERT INTO games(erogamescape_id,title,brand_id,release_date,thumbnail_path,source_url,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?)",params![input.erogamescape_id,input.title.trim(),brand,input.release_date,thumbnail,input.source_url,n,n])?;
        let id = tx.last_insert_rowid();
        for p in &input.executable_paths {
            insert_executable(&tx, id, p)?
        }
        tx.commit()?;
        Ok(id)
    }
    pub fn update_game(&self, id: i64, input: &UpdateGameInput) -> Result<()> {
        if input.title.trim().is_empty() {
            bail!("タイトルは必須です")
        };
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let brand = brand_id(&tx, input.brand.as_deref())?;
        let source_url = input
            .source_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let erogamescape_id = source_url
            .and_then(|value| reqwest::Url::parse(value).ok())
            .and_then(|url| {
                url.query_pairs()
                    .find(|(key, _)| key == "game")
                    .and_then(|(_, value)| value.parse::<i64>().ok())
            });
        tx.execute(
            "UPDATE games SET title=?,brand_id=?,release_date=?,source_url=?,erogamescape_id=COALESCE(?,erogamescape_id),updated_at=? WHERE id=?",
            params![input.title.trim(), brand, input.release_date, source_url, erogamescape_id, now(), id],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn update_game_thumbnail(&self, id: i64, thumbnail: Option<&str>) -> Result<()> {
        let updated = self.0.lock().execute(
            "UPDATE games SET thumbnail_path=?,updated_at=? WHERE id=?",
            params![thumbnail, now(), id],
        )?;
        if updated == 0 {
            bail!("ゲームが見つかりません")
        }
        Ok(())
    }
    pub fn delete_game(&self, id: i64) -> Result<()> {
        self.0
            .lock()
            .execute("DELETE FROM games WHERE id=?", [id])?;
        Ok(())
    }
    pub fn add_executable(&self, game: i64, path: &str) -> Result<()> {
        insert_executable(&self.0.lock(), game, path)
    }
    pub fn remove_executable(&self, id: i64) -> Result<()> {
        self.0
            .lock()
            .execute("DELETE FROM game_executables WHERE id=?", [id])?;
        Ok(())
    }
    pub fn launcher_path(&self, game: i64) -> Result<String> {
        self.0
            .lock()
            .query_row(
                "SELECT path FROM game_executables WHERE game_id=? ORDER BY id LIMIT 1",
                [game],
                |row| row.get(0),
            )
            .optional()?
            .context("起動する実行ファイルが登録されていません")
    }
    pub fn registered_executables(&self) -> Result<Vec<(i64, String, String)>> {
        let c = self.0.lock();
        let mut q=c.prepare("SELECT e.game_id,g.title,e.path FROM game_executables e JOIN games g ON g.id=e.game_id")?;
        Ok(q.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn list_games(
        &self,
        search: &str,
        brand: Option<&str>,
        play_status: Option<&str>,
        sort: &str,
        descending: bool,
    ) -> Result<Vec<GameSummary>> {
        let order = match sort {
            "title" => "g.title",
            "brand" => "b.name",
            "release_date" => "g.release_date",
            "created_at" => "g.created_at",
            "total_playtime" => "total_playtime_seconds",
            "session_count" => "session_count",
            _ => "last_played",
        };
        let sql = format!(
            "{} ORDER BY {} {} NULLS LAST,g.title COLLATE NOCASE",
            GAME_QUERY,
            order,
            if descending { "DESC" } else { "ASC" }
        );
        let like = format!("%{}%", search);
        let c = self.0.lock();
        let mut q = c.prepare(&sql)?;
        let rows = q.query_map(
            params![like, brand, brand, play_status, play_status],
            game_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
    pub fn list_brands(&self) -> Result<Vec<String>> {
        let connection = self.0.lock();
        let mut query =
            connection.prepare("SELECT name FROM brands ORDER BY name COLLATE NOCASE")?;
        Ok(query
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn get_game(&self, id: i64) -> Result<GameDetail> {
        let c = self.0.lock();
        let summary = c
            .query_row(
                &format!("{} AND g.id=?", GAME_QUERY),
                params![
                    "%",
                    Option::<String>::None,
                    Option::<String>::None,
                    Option::<String>::None,
                    Option::<String>::None,
                    id
                ],
                game_row,
            )
            .optional()?
            .context("ゲームが見つかりません")?;
        let (eid, url) = c.query_row(
            "SELECT erogamescape_id,source_url FROM games WHERE id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let mut q=c.prepare("SELECT id,game_id,path,file_name,created_at FROM game_executables WHERE game_id=? ORDER BY id")?;
        let executables = q
            .query_map([id], |r| {
                Ok(Executable {
                    id: r.get(0)?,
                    game_id: r.get(1)?,
                    path: r.get(2)?,
                    file_name: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(GameDetail {
            summary,
            erogamescape_id: eid,
            source_url: url,
            executables,
        })
    }
    pub fn list_sessions(&self, game: i64) -> Result<Vec<PlaySession>> {
        let c = self.0.lock();
        let mut q=c.prepare("SELECT s.id,s.game_id,s.launched_at,s.exited_at,s.needs_review,MAX(0,CAST(strftime('%s',COALESCE(s.exited_at,'now')) AS INTEGER)-CAST(strftime('%s',s.launched_at) AS INTEGER)-COALESCE(SUM(MAX(0,CAST(strftime('%s',COALESCE(b.ended_at,'now')) AS INTEGER)-CAST(strftime('%s',b.started_at) AS INTEGER))),0)),COALESCE(SUM(MAX(0,CAST(strftime('%s',COALESCE(b.ended_at,'now')) AS INTEGER)-CAST(strftime('%s',b.started_at) AS INTEGER))),0),CASE WHEN s.exited_at IS NULL THEN NULL ELSE CAST(strftime('%s',s.exited_at) AS INTEGER)-CAST(strftime('%s',s.launched_at) AS INTEGER) END FROM play_sessions s LEFT JOIN background_intervals b ON b.play_session_id=s.id WHERE s.game_id=? GROUP BY s.id ORDER BY s.launched_at DESC")?;
        Ok(q.query_map([game], |r| {
            Ok(PlaySession {
                id: r.get(0)?,
                game_id: r.get(1)?,
                launched_at: r.get(2)?,
                exited_at: r.get(3)?,
                needs_review: r.get::<_, i64>(4)? != 0,
                playtime_seconds: r.get(5)?,
                background_seconds: r.get(6)?,
                running_seconds: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    pub fn intervals(&self, session: i64) -> Result<Vec<BackgroundInterval>> {
        let c = self.0.lock();
        let mut q=c.prepare("SELECT id,play_session_id,started_at,ended_at FROM background_intervals WHERE play_session_id=? ORDER BY started_at")?;
        Ok(q.query_map([session], |r| {
            Ok(BackgroundInterval {
                id: r.get(0)?,
                play_session_id: r.get(1)?,
                started_at: r.get(2)?,
                ended_at: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    pub fn start_session(&self, game: i64, at: &str) -> Result<i64> {
        let n = now();
        let c = self.0.lock();
        c.execute(
            "INSERT INTO play_sessions(game_id,launched_at,created_at,updated_at,background_migrated) VALUES(?,?,?,?,1)",
            params![game, at, n, n],
        )?;
        Ok(c.last_insert_rowid())
    }
    pub fn end_session(&self, id: i64, at: &str) -> Result<()> {
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        tx.execute("UPDATE focus_intervals SET ended_at=?,updated_at=? WHERE play_session_id=? AND ended_at IS NULL",params![at,now(),id])?;
        tx.execute("UPDATE background_intervals SET ended_at=?,updated_at=? WHERE play_session_id=? AND ended_at IS NULL",params![at,now(),id])?;
        tx.execute(
            "UPDATE play_sessions SET exited_at=?,updated_at=? WHERE id=?",
            params![at, now(), id],
        )?;
        rebuild_focus_mirror(&tx, id)?;
        tx.commit()?;
        Ok(())
    }
    pub fn start_interval(&self, session: i64, at: &str) -> Result<i64> {
        validate_interval(
            &self.0.lock(),
            "background_intervals",
            session,
            None,
            at,
            None,
        )?;
        let n = now();
        let c = self.0.lock();
        c.execute("INSERT INTO background_intervals(play_session_id,started_at,created_at,updated_at) VALUES(?,?,?,?)",params![session,at,n,n])?;
        Ok(c.last_insert_rowid())
    }
    pub fn end_interval(&self, id: i64, at: &str) -> Result<()> {
        let c = self.0.lock();
        let (session, start): (i64, String) = c.query_row(
            "SELECT play_session_id,started_at FROM background_intervals WHERE id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        validate_interval(
            &c,
            "background_intervals",
            session,
            Some(id),
            &start,
            Some(at),
        )?;
        if start == at {
            c.execute("DELETE FROM background_intervals WHERE id=?", [id])?;
        } else {
            c.execute(
                "UPDATE background_intervals SET ended_at=?,updated_at=? WHERE id=?",
                params![at, now(), id],
            )?;
        }
        Ok(())
    }
    pub fn start_legacy_focus_interval(&self, session: i64, at: &str) -> Result<i64> {
        validate_interval(&self.0.lock(), "focus_intervals", session, None, at, None)?;
        let n = now();
        let c = self.0.lock();
        c.execute("INSERT INTO focus_intervals(play_session_id,started_at,created_at,updated_at) VALUES(?,?,?,?)",params![session,at,n,n])?;
        Ok(c.last_insert_rowid())
    }
    pub fn end_legacy_focus_interval(&self, id: i64, at: &str) -> Result<()> {
        let c = self.0.lock();
        let (session, start): (i64, String) = c.query_row(
            "SELECT play_session_id,started_at FROM focus_intervals WHERE id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        validate_interval(&c, "focus_intervals", session, Some(id), &start, Some(at))?;
        if start == at {
            c.execute("DELETE FROM focus_intervals WHERE id=?", [id])?;
        } else {
            c.execute(
                "UPDATE focus_intervals SET ended_at=?,updated_at=? WHERE id=?",
                params![at, now(), id],
            )?;
        }
        Ok(())
    }
    pub fn manual_session(&self, game: i64, start: &str, end: &str) -> Result<i64> {
        parse_range(start, end)?;
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let n = now();
        tx.execute("INSERT INTO play_sessions(game_id,launched_at,exited_at,created_at,updated_at,background_migrated) VALUES(?,?,?,?,?,1)",params![game,start,end,n,n])?;
        let id = tx.last_insert_rowid();
        tx.execute("INSERT INTO focus_intervals(play_session_id,started_at,ended_at,created_at,updated_at) VALUES(?,?,?,?,?)",params![id,start,end,n,n])?;
        tx.commit()?;
        Ok(id)
    }
    pub fn update_session(&self, id: i64, start: &str, end: Option<&str>) -> Result<()> {
        DateTime::parse_from_rfc3339(start).context("開始日時が不正です")?;
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let existing_end: Option<String> = tx.query_row(
            "SELECT exited_at FROM play_sessions WHERE id=?",
            [id],
            |r| r.get(0),
        )?;
        if existing_end.is_some() != end.is_some() {
            bail!("セッションの実行状態は手動変更できません")
        }
        if let Some(end) = end {
            parse_range(start, end)?;
        }
        let invalid:i64=tx.query_row("SELECT COUNT(*) FROM background_intervals WHERE play_session_id=? AND (started_at < ? OR (? IS NOT NULL AND (ended_at IS NULL OR ended_at > ?)))",params![id,start,end,end],|r|r.get(0))?;
        if invalid > 0 {
            bail!("バックグラウンド区間がSession範囲外になります")
        };
        tx.execute("UPDATE play_sessions SET launched_at=?,exited_at=?,needs_review=0,updated_at=? WHERE id=?",params![start,end,now(),id])?;
        rebuild_focus_mirror(&tx, id)?;
        tx.commit()?;
        Ok(())
    }
    pub fn delete_session(&self, id: i64) -> Result<()> {
        self.0
            .lock()
            .execute("DELETE FROM play_sessions WHERE id=?", [id])?;
        Ok(())
    }
    pub fn delete_game_sessions(&self, game: i64) -> Result<usize> {
        Ok(self
            .0
            .lock()
            .execute("DELETE FROM play_sessions WHERE game_id=?", [game])?)
    }
    pub fn create_interval(&self, session: i64, start: &str, end: &str) -> Result<i64> {
        parse_range(start, end)?;
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        validate_interval(&tx, "background_intervals", session, None, start, Some(end))?;
        let n = now();
        tx.execute("INSERT INTO background_intervals(play_session_id,started_at,ended_at,created_at,updated_at) VALUES(?,?,?,?,?)",params![session,start,end,n,n])?;
        let id = tx.last_insert_rowid();
        rebuild_focus_mirror(&tx, session)?;
        tx.commit()?;
        Ok(id)
    }
    pub fn update_interval(&self, id: i64, start: &str, end: Option<&str>) -> Result<()> {
        DateTime::parse_from_rfc3339(start).context("開始日時が不正です")?;
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let (session, existing_end): (i64, Option<String>) = tx.query_row(
            "SELECT play_session_id,ended_at FROM background_intervals WHERE id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if existing_end.is_some() != end.is_some() {
            bail!("除外区間の記録状態は手動変更できません")
        }
        validate_interval(&tx, "background_intervals", session, Some(id), start, end)?;
        tx.execute(
            "UPDATE background_intervals SET started_at=?,ended_at=?,updated_at=? WHERE id=?",
            params![start, end, now(), id],
        )?;
        rebuild_focus_mirror(&tx, session)?;
        tx.commit()?;
        Ok(())
    }
    pub fn delete_interval(&self, id: i64) -> Result<()> {
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let session: i64 = tx.query_row(
            "SELECT play_session_id FROM background_intervals WHERE id=?",
            [id],
            |r| r.get(0),
        )?;
        tx.execute("DELETE FROM background_intervals WHERE id=?", [id])?;
        rebuild_focus_mirror(&tx, session)?;
        tx.commit()?;
        Ok(())
    }
    pub fn update_play_status(&self, game: i64, status: &str) -> Result<()> {
        if !matches!(status, "unplayed" | "playing" | "completed" | "retired") {
            bail!("未対応のプレイ状況です")
        }
        let changed = self.0.lock().execute(
            "UPDATE games SET play_status=?,updated_at=? WHERE id=?",
            params![status, now(), game],
        )?;
        if changed == 0 {
            bail!("ゲームが見つかりません")
        }
        Ok(())
    }
    pub fn create_timestamp(&self, game: i64, name: &str, marked_at: &str) -> Result<i64> {
        let name = validate_timestamp_name(name)?;
        DateTime::parse_from_rfc3339(marked_at).context("記録日時が不正です")?;
        let c = self.0.lock();
        let n = now();
        c.execute(
            "INSERT INTO game_timestamps(game_id,name,marked_at,created_at) VALUES(?,?,?,?)",
            params![game, name, marked_at, n],
        )?;
        Ok(c.last_insert_rowid())
    }
    pub fn update_timestamp(&self, id: i64, name: &str, marked_at: &str) -> Result<()> {
        let name = validate_timestamp_name(name)?;
        DateTime::parse_from_rfc3339(marked_at).context("記録日時が不正です")?;
        let changed = self.0.lock().execute(
            "UPDATE game_timestamps SET name=?,marked_at=? WHERE id=?",
            params![name, marked_at, id],
        )?;
        if changed == 0 {
            bail!("プレイ記録ポイントが見つかりません")
        }
        Ok(())
    }
    pub fn timestamps(&self, game: i64) -> Result<Vec<GameTimestamp>> {
        let c = self.0.lock();
        let mut query = c.prepare(
            "SELECT id,game_id,name,marked_at FROM game_timestamps WHERE game_id=? ORDER BY marked_at,id",
        )?;
        let rows: Vec<(i64, i64, String, String)> = query
            .query_map([game], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut previous = 0;
        rows.into_iter()
            .map(|(id, game_id, name, marked_at)| {
                let playtime_seconds: i64 = c.query_row(
                    "SELECT MAX(0,COALESCE(SUM(MAX(0,CAST(strftime('%s',MIN(COALESCE(s.exited_at,?),?)) AS INTEGER)-CAST(strftime('%s',s.launched_at) AS INTEGER))),0)-COALESCE((SELECT SUM(MAX(0,CAST(strftime('%s',MIN(COALESCE(b.ended_at,?),?)) AS INTEGER)-CAST(strftime('%s',MAX(b.started_at,s2.launched_at)) AS INTEGER))) FROM background_intervals b JOIN play_sessions s2 ON s2.id=b.play_session_id WHERE s2.game_id=? AND b.started_at<?),0)) FROM play_sessions s WHERE s.game_id=? AND s.launched_at<?",
                    params![marked_at, marked_at, marked_at, marked_at, game_id, marked_at, game_id, marked_at],
                    |r| r.get(0),
                )?;
                let since_previous_seconds = (playtime_seconds - previous).max(0);
                previous = playtime_seconds;
                Ok(GameTimestamp {
                    id,
                    game_id,
                    name,
                    marked_at,
                    playtime_seconds,
                    since_previous_seconds,
                })
            })
            .collect()
    }
    pub fn delete_timestamp(&self, id: i64) -> Result<()> {
        self.0
            .lock()
            .execute("DELETE FROM game_timestamps WHERE id=?", [id])?;
        Ok(())
    }
    pub fn recover_orphans(&self, at: &str) -> Result<usize> {
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let n = tx.execute(
            "UPDATE focus_intervals SET ended_at=?,updated_at=? WHERE ended_at IS NULL",
            params![at, now()],
        )?;
        tx.execute(
            "UPDATE background_intervals SET ended_at=?,updated_at=? WHERE ended_at IS NULL",
            params![at, now()],
        )?;
        tx.execute("UPDATE play_sessions SET exited_at=?,needs_review=1,updated_at=? WHERE exited_at IS NULL",params![at,now()])?;
        tx.commit()?;
        Ok(n)
    }
    pub fn migrate_focus_intervals(&self) -> Result<usize> {
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let sessions: Vec<(i64, String, String)> = {
            let mut query = tx.prepare("SELECT id,launched_at,exited_at FROM play_sessions WHERE background_migrated=0 AND exited_at IS NOT NULL ORDER BY id")?;
            query
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<rusqlite::Result<_>>()?
        };
        for (session, start, end) in &sessions {
            let focus = load_ranges(&tx, "focus_intervals", *session)?;
            insert_complement(&tx, "background_intervals", *session, start, end, &focus)?;
            let background = load_ranges(&tx, "background_intervals", *session)?;
            let total = range_seconds(start, end)?;
            let foreground_seconds: i64 = focus
                .iter()
                .map(|(start, end)| range_seconds(start, end))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .sum();
            let background_seconds: i64 = background
                .iter()
                .map(|(start, end)| range_seconds(start, end))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .sum();
            if total - background_seconds != foreground_seconds {
                bail!("セッション {session} の旧プレイ時間を安全に移行できません")
            }
            tx.execute(
                "UPDATE play_sessions SET background_migrated=1,updated_at=? WHERE id=?",
                params![now(), session],
            )?;
        }
        tx.commit()?;
        Ok(sessions.len())
    }
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .0
            .lock()
            .query_row("SELECT value FROM settings WHERE key=?", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.0.lock().execute("INSERT INTO settings(key,value,updated_at) VALUES(?,?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at",params![key,value,now()])?;
        Ok(())
    }
    pub fn add_screenshot(
        &self,
        game_id: i64,
        session_id: Option<i64>,
        path: &str,
        captured_at: &str,
        width: i64,
        height: i64,
    ) -> Result<i64> {
        let c = self.0.lock();
        c.execute(
            "INSERT INTO game_screenshots(game_id,play_session_id,path,captured_at,width,height) VALUES(?,?,?,?,?,?)",
            params![game_id, session_id, path, captured_at, width, height],
        )?;
        Ok(c.last_insert_rowid())
    }
    pub fn screenshots(&self, game_id: i64) -> Result<Vec<GameScreenshot>> {
        let c = self.0.lock();
        let mut q = c.prepare("SELECT id,game_id,play_session_id,path,captured_at,width,height FROM game_screenshots WHERE game_id=? ORDER BY captured_at DESC,id DESC")?;
        Ok(q.query_map([game_id], |r| {
            Ok(GameScreenshot {
                id: r.get(0)?,
                game_id: r.get(1)?,
                play_session_id: r.get(2)?,
                path: r.get(3)?,
                captured_at: r.get(4)?,
                width: r.get(5)?,
                height: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn remove_screenshot(&self, id: i64) -> Result<Option<String>> {
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let path = tx
            .query_row("SELECT path FROM game_screenshots WHERE id=?", [id], |r| {
                r.get(0)
            })
            .optional()?;
        if path.is_some() {
            tx.execute("DELETE FROM game_screenshots WHERE id=?", [id])?;
        }
        tx.commit()?;
        Ok(path)
    }
    pub fn metadata_identity(&self, id: i64) -> Result<(Option<i64>, Option<String>)> {
        Ok(self.0.lock().query_row(
            "SELECT erogamescape_id,source_url FROM games WHERE id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }
    pub fn apply_metadata(
        &self,
        id: i64,
        m: &crate::metadata::GameMetadata,
        thumb: Option<&str>,
    ) -> Result<()> {
        let mut c = self.0.lock();
        let tx = c.transaction()?;
        let brand = brand_id(&tx, m.brand.as_deref())?;
        tx.execute("UPDATE games SET erogamescape_id=?,title=?,brand_id=?,release_date=?,thumbnail_path=COALESCE(?,thumbnail_path),source_url=?,updated_at=? WHERE id=?",params![m.erogamescape_id,m.title,brand,m.release_date,thumb,m.source_url,now(),id])?;
        tx.commit()?;
        Ok(())
    }
}

const GAME_QUERY: &str = "SELECT g.id,g.title,b.name,g.release_date,g.thumbnail_path,g.created_at,
MAX(0,COALESCE((SELECT SUM(MAX(0,CAST(strftime('%s',COALESCE(s.exited_at,'now')) AS INTEGER)-CAST(strftime('%s',s.launched_at) AS INTEGER))) FROM play_sessions s WHERE s.game_id=g.id),0)-COALESCE((SELECT SUM(MAX(0,CAST(strftime('%s',COALESCE(b.ended_at,'now')) AS INTEGER)-CAST(strftime('%s',b.started_at) AS INTEGER))) FROM background_intervals b JOIN play_sessions bs ON bs.id=b.play_session_id WHERE bs.game_id=g.id),0)) total_playtime_seconds,
COALESCE((SELECT SUM(MAX(0,CAST(strftime('%s',COALESCE(s.exited_at,'now')) AS INTEGER)-CAST(strftime('%s',s.launched_at) AS INTEGER))) FROM play_sessions s WHERE s.game_id=g.id),0) total_running_seconds,
(SELECT MAX(COALESCE(f.ended_at,f.started_at)) FROM focus_intervals f JOIN play_sessions fs ON fs.id=f.play_session_id WHERE fs.game_id=g.id) last_played,
(SELECT COUNT(*) FROM play_sessions s WHERE s.game_id=g.id) session_count,
g.play_status
FROM games g LEFT JOIN brands b ON b.id=g.brand_id WHERE g.title LIKE ? AND (? IS NULL OR b.name=?) AND (? IS NULL OR g.play_status=?)";
fn game_row(r: &rusqlite::Row) -> rusqlite::Result<GameSummary> {
    Ok(GameSummary {
        id: r.get(0)?,
        title: r.get(1)?,
        brand: r.get(2)?,
        release_date: r.get(3)?,
        thumbnail_path: r.get(4)?,
        created_at: r.get(5)?,
        total_playtime_seconds: r.get(6)?,
        total_running_seconds: r.get(7)?,
        last_played: r.get(8)?,
        session_count: r.get(9)?,
        play_status: r.get(10)?,
    })
}
fn now() -> String {
    Utc::now().to_rfc3339()
}
fn parse_range(start: &str, end: &str) -> Result<()> {
    let s = DateTime::parse_from_rfc3339(start).context("開始日時が不正です")?;
    let e = DateTime::parse_from_rfc3339(end).context("終了日時が不正です")?;
    if e < s {
        bail!("終了日時は開始日時以降にしてください")
    };
    Ok(())
}
fn range_seconds(start: &str, end: &str) -> Result<i64> {
    let start = DateTime::parse_from_rfc3339(start).context("開始日時が不正です")?;
    let end = DateTime::parse_from_rfc3339(end).context("終了日時が不正です")?;
    // Match SQLite strftime('%s') exactly. Subtracting the chrono Duration and
    // truncating each interval independently loses fractional seconds and can
    // make a perfectly partitioned session fail migration validation.
    Ok(end.timestamp() - start.timestamp())
}
fn brand_id(c: &Connection, name: Option<&str>) -> Result<Option<i64>> {
    let Some(name) = name.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    c.execute(
        "INSERT INTO brands(name) VALUES(?) ON CONFLICT(name) DO NOTHING",
        [name],
    )?;
    Ok(Some(c.query_row(
        "SELECT id FROM brands WHERE name=?",
        [name],
        |r| r.get(0),
    )?))
}
fn insert_executable(c: &Connection, game: i64, path: &str) -> Result<()> {
    let p = normalize_path(path);
    if p.is_empty() {
        bail!("実行ファイルパスが空です")
    };
    let file = Path::new(&p)
        .file_name()
        .and_then(|x| x.to_str())
        .context("実行ファイル名を取得できません")?;
    c.execute(
        "INSERT INTO game_executables(game_id,path,file_name,created_at) VALUES(?,?,?,?)",
        params![game, p, file, now()],
    )?;
    Ok(())
}
fn validate_timestamp_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        bail!("プレイ記録ポイントの名称を入力してください")
    }
    if name.chars().count() > 100 {
        bail!("プレイ記録ポイントの名称は100文字以内にしてください")
    }
    Ok(name)
}
pub fn normalize_path(path: &str) -> String {
    let p = path.trim().trim_matches('"').replace('/', "\\");
    p.strip_prefix("\\\\?\\").unwrap_or(&p).to_lowercase()
}
fn validate_interval(
    c: &Connection,
    table: &str,
    session: i64,
    exclude: Option<i64>,
    start: &str,
    end: Option<&str>,
) -> Result<()> {
    DateTime::parse_from_rfc3339(start).context("開始日時が不正です")?;
    if let Some(e) = end {
        parse_range(start, e)?
    }
    let (ss, se): (String, Option<String>) = c.query_row(
        "SELECT launched_at,exited_at FROM play_sessions WHERE id=?",
        [session],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if start < ss.as_str() || se.as_deref().is_some_and(|x| end.is_none_or(|e| e > x)) {
        bail!("区間はSession範囲内にしてください")
    };
    let overlap:i64=c.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE play_session_id=? AND (? IS NULL OR id<>?) AND COALESCE(ended_at,'9999') > ? AND COALESCE(?, '9999') > started_at"),params![session,exclude,exclude,start,end],|r|r.get(0))?;
    if overlap > 0 {
        bail!("区間が既存の区間と重複しています")
    };
    Ok(())
}

fn load_ranges(c: &Connection, table: &str, session: i64) -> Result<Vec<(String, String)>> {
    let mut query = c.prepare(&format!("SELECT started_at,ended_at FROM {table} WHERE play_session_id=? AND ended_at IS NOT NULL ORDER BY started_at"))?;
    Ok(query
        .query_map([session], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?)
}

fn insert_complement(
    c: &Connection,
    table: &str,
    session: i64,
    start: &str,
    end: &str,
    excluded: &[(String, String)],
) -> Result<()> {
    let mut cursor = start.to_string();
    for (range_start, range_end) in excluded {
        let clamped_start = range_start.as_str().max(start).min(end);
        let clamped_end = range_end.as_str().max(start).min(end);
        if cursor.as_str() < clamped_start {
            insert_range(c, table, session, &cursor, clamped_start)?;
        }
        if clamped_end > cursor.as_str() {
            cursor = clamped_end.to_string();
        }
    }
    if cursor.as_str() < end {
        insert_range(c, table, session, &cursor, end)?;
    }
    Ok(())
}

fn insert_range(c: &Connection, table: &str, session: i64, start: &str, end: &str) -> Result<()> {
    let n = now();
    c.execute(&format!("INSERT INTO {table}(play_session_id,started_at,ended_at,created_at,updated_at) VALUES(?,?,?,?,?)"), params![session,start,end,n,n])?;
    Ok(())
}

fn rebuild_focus_mirror(c: &Connection, session: i64) -> Result<()> {
    let (start, end): (String, Option<String>) = c.query_row(
        "SELECT launched_at,exited_at FROM play_sessions WHERE id=?",
        [session],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let Some(end) = end else { return Ok(()) };
    let background = load_ranges(c, "background_intervals", session)?;
    c.execute(
        "DELETE FROM focus_intervals WHERE play_session_id=?",
        [session],
    )?;
    insert_complement(c, "focus_intervals", session, &start, &end, &background)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(db: &Database) -> i64 {
        db.create_game(
            &CreateGameInput {
                title: "A".into(),
                brand: Some("B".into()),
                release_date: None,
                thumbnail_path: None,
                erogamescape_id: None,
                source_url: None,
                executable_paths: vec![r#"C:\G\a.exe"#.into(), r#"C:\G\launcher.exe"#.into()],
            },
            None,
        )
        .unwrap()
    }
    #[test]
    fn aggregate_and_sort() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        d.manual_session(g, "2026-01-01T00:00:00Z", "2026-01-01T01:00:00Z")
            .unwrap();
        let x = d
            .list_games("", None, None, "total_playtime", true)
            .unwrap();
        assert_eq!(x[0].total_playtime_seconds, 3600);
        assert_eq!(x[0].session_count, 1);
    }
    #[test]
    fn thumbnail_can_be_replaced_and_cleared() {
        let d = Database::memory().unwrap();
        let g = game(&d);

        d.update_game_thumbnail(g, Some(r#"C:\thumbnails\custom.png"#))
            .unwrap();
        assert_eq!(
            d.get_game(g).unwrap().summary.thumbnail_path.as_deref(),
            Some(r#"C:\thumbnails\custom.png"#)
        );

        d.update_game_thumbnail(g, None).unwrap();
        assert!(d.get_game(g).unwrap().summary.thumbnail_path.is_none());
        assert!(d.update_game_thumbnail(i64::MAX, None).is_err());
    }
    #[test]
    fn erogamescape_metadata_replaces_an_existing_thumbnail() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        d.update_game_thumbnail(g, Some(r#"C:\thumbnails\manual.png"#))
            .unwrap();
        let metadata = crate::metadata::GameMetadata {
            erogamescape_id: 42,
            title: "Updated".into(),
            brand: Some("Updated Brand".into()),
            release_date: Some("2026-08-29".into()),
            thumbnail_url: Some("https://example.test/cover.jpg".into()),
            source_url: "https://example.test/game.php?game=42".into(),
        };

        d.apply_metadata(g, &metadata, Some(r#"C:\thumbnails\erogamescape.jpg"#))
            .unwrap();

        assert_eq!(
            d.get_game(g).unwrap().summary.thumbnail_path.as_deref(),
            Some(r#"C:\thumbnails\erogamescape.jpg"#)
        );
    }
    #[test]
    fn rejects_overlap_and_outside() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        let s = d
            .manual_session(g, "2026-01-01T00:00:00Z", "2026-01-01T01:00:00Z")
            .unwrap();
        d.create_interval(s, "2026-01-01T00:30:00Z", "2026-01-01T00:40:00Z")
            .unwrap();
        assert!(
            d.create_interval(s, "2026-01-01T00:35:00Z", "2026-01-01T00:45:00Z")
                .is_err()
        );
        assert!(
            d.update_session(s, "2026-01-01T00:35:00Z", Some("2026-01-01T01:00:00Z"))
                .is_err()
        );
    }
    #[test]
    fn session_update_preserves_running_state() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        let closed = d
            .manual_session(g, "2026-01-01T00:00:00Z", "2026-01-01T01:00:00Z")
            .unwrap();
        assert!(
            d.update_session(closed, "2026-01-01T00:00:00Z", None)
                .is_err()
        );

        let running = d.start_session(g, "2026-01-01T02:00:00Z").unwrap();
        assert!(
            d.update_session(
                running,
                "2026-01-01T02:05:00Z",
                Some("2026-01-01T03:00:00Z")
            )
            .is_err()
        );
        d.update_session(running, "2026-01-01T02:05:00Z", None)
            .unwrap();

        let sessions = d.list_sessions(g).unwrap();
        assert_eq!(sessions[0].id, running);
        assert_eq!(sessions[0].launched_at, "2026-01-01T02:05:00Z");
        assert_eq!(sessions[0].exited_at, None);
        assert_eq!(
            sessions[1].exited_at.as_deref(),
            Some("2026-01-01T01:00:00Z")
        );
    }
    #[test]
    fn interval_update_preserves_recording_state() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        let session = d.start_session(g, "2026-01-01T00:00:00Z").unwrap();
        let interval = d.start_interval(session, "2026-01-01T00:10:00Z").unwrap();

        assert!(
            d.update_interval(
                interval,
                "2026-01-01T00:15:00Z",
                Some("2026-01-01T00:20:00Z")
            )
            .is_err()
        );
        d.update_interval(interval, "2026-01-01T00:15:00Z", None)
            .unwrap();
        d.end_interval(interval, "2026-01-01T00:30:00Z").unwrap();

        let recorded = d.intervals(session).unwrap().remove(0);
        assert_eq!(recorded.started_at, "2026-01-01T00:15:00Z");
        assert_eq!(recorded.ended_at.as_deref(), Some("2026-01-01T00:30:00Z"));
        assert!(
            d.update_interval(interval, &recorded.started_at, None)
                .is_err()
        );
    }
    #[test]
    fn multiple_executables_one_game() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        assert_eq!(
            d.registered_executables()
                .unwrap()
                .iter()
                .filter(|x| x.0 == g)
                .count(),
            2
        );
    }
    #[test]
    fn timestamp_playtime_is_derived_from_sessions_minus_background() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        d.manual_session(g, "2026-01-01T00:00:00Z", "2026-01-01T01:00:00Z")
            .unwrap();
        d.manual_session(g, "2026-01-01T02:00:00Z", "2026-01-01T03:00:00Z")
            .unwrap();
        d.create_timestamp(g, "共通ルート終了", "2026-01-01T00:30:00Z")
            .unwrap();
        let second = d
            .create_timestamp(g, "個別ルート終了", "2026-01-01T02:15:00Z")
            .unwrap();
        let points = d.timestamps(g).unwrap();
        assert_eq!(points[0].playtime_seconds, 1800);
        assert_eq!(points[0].since_previous_seconds, 1800);
        assert_eq!(points[1].playtime_seconds, 4500);
        assert_eq!(points[1].since_previous_seconds, 2700);
        d.delete_timestamp(second).unwrap();
        assert_eq!(d.timestamps(g).unwrap().len(), 1);
    }

    #[test]
    fn timestamp_update_reorders_and_recalculates_playtime() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        d.manual_session(g, "2026-01-01T00:00:00Z", "2026-01-01T01:00:00Z")
            .unwrap();
        let first = d
            .create_timestamp(g, "共通ルート終了", "2026-01-01T00:30:00Z")
            .unwrap();
        let second = d
            .create_timestamp(g, "個別ルート終了", "2026-01-01T00:45:00Z")
            .unwrap();

        d.update_timestamp(second, "  真ルート終了  ", "2026-01-01T00:15:00Z")
            .unwrap();
        let updated = d.timestamps(g).unwrap();
        assert_eq!(updated[0].id, second);
        assert_eq!(updated[0].name, "真ルート終了");
        assert_eq!(updated[0].marked_at, "2026-01-01T00:15:00Z");
        assert_eq!(updated[0].playtime_seconds, 900);
        assert_eq!(updated[0].since_previous_seconds, 900);
        assert_eq!(updated[1].id, first);
        assert_eq!(updated[1].playtime_seconds, 1800);
        assert_eq!(updated[1].since_previous_seconds, 900);

        assert!(
            d.update_timestamp(first, "   ", "2026-01-01T00:30:00Z")
                .is_err()
        );
        assert!(d.update_timestamp(first, "有効な名称", "invalid").is_err());
        assert!(
            d.update_timestamp(i64::MAX, "存在しない記録", "2026-01-01T00:30:00Z")
                .is_err()
        );
    }

    #[test]
    fn legacy_focus_is_migrated_to_its_background_complement() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        let n = now();
        let c = d.0.lock();
        c.execute("INSERT INTO play_sessions(game_id,launched_at,exited_at,created_at,updated_at) VALUES(?,?,?,?,?)", params![g,"2026-01-01T00:00:00Z","2026-01-01T01:00:00Z",n,n]).unwrap();
        let session = c.last_insert_rowid();
        c.execute("INSERT INTO focus_intervals(play_session_id,started_at,ended_at,created_at,updated_at) VALUES(?,?,?,?,?)", params![session,"2026-01-01T00:10:00Z","2026-01-01T00:40:00Z",n,n]).unwrap();
        drop(c);

        assert_eq!(d.migrate_focus_intervals().unwrap(), 1);
        let intervals = d.intervals(session).unwrap();
        assert_eq!(intervals.len(), 2);
        assert_eq!(intervals[0].started_at, "2026-01-01T00:00:00Z");
        assert_eq!(
            intervals[0].ended_at.as_deref(),
            Some("2026-01-01T00:10:00Z")
        );
        assert_eq!(intervals[1].started_at, "2026-01-01T00:40:00Z");
        assert_eq!(d.list_sessions(g).unwrap()[0].playtime_seconds, 1800);
        assert_eq!(d.migrate_focus_intervals().unwrap(), 0);
    }
    #[test]
    fn migration_validation_matches_sqlite_second_rounding() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        let n = now();
        let c = d.0.lock();
        c.execute("INSERT INTO play_sessions(game_id,launched_at,exited_at,created_at,updated_at) VALUES(?,?,?,?,?)", params![g,"2026-01-01T00:00:00.900Z","2026-01-01T00:00:03.100Z",n,n]).unwrap();
        let session = c.last_insert_rowid();
        c.execute("INSERT INTO focus_intervals(play_session_id,started_at,ended_at,created_at,updated_at) VALUES(?,?,?,?,?)", params![session,"2026-01-01T00:00:00.900Z","2026-01-01T00:00:01.100Z",n,n]).unwrap();
        c.execute("INSERT INTO focus_intervals(play_session_id,started_at,ended_at,created_at,updated_at) VALUES(?,?,?,?,?)", params![session,"2026-01-01T00:00:02.900Z","2026-01-01T00:00:03.100Z",n,n]).unwrap();
        drop(c);

        assert_eq!(d.migrate_focus_intervals().unwrap(), 1);
        assert_eq!(d.list_sessions(g).unwrap()[0].playtime_seconds, 2);
    }
    #[test]
    fn background_tracking_derives_playtime_and_keeps_focus_mirror() {
        let d = Database::memory().unwrap();
        let g = game(&d);
        let session = d.start_session(g, "2026-01-01T00:00:00Z").unwrap();
        let background = d.start_interval(session, "2026-01-01T00:00:00Z").unwrap();
        d.end_interval(background, "2026-01-01T00:10:00Z").unwrap();
        let focus = d
            .start_legacy_focus_interval(session, "2026-01-01T00:10:00Z")
            .unwrap();
        d.end_legacy_focus_interval(focus, "2026-01-01T00:20:00Z")
            .unwrap();
        let background = d.start_interval(session, "2026-01-01T00:20:00Z").unwrap();
        d.end_interval(background, "2026-01-01T00:30:00Z").unwrap();
        d.end_session(session, "2026-01-01T00:30:00Z").unwrap();

        let recorded = d.list_sessions(g).unwrap().remove(0);
        assert_eq!(recorded.running_seconds, Some(1800));
        assert_eq!(recorded.background_seconds, 1200);
        assert_eq!(recorded.playtime_seconds, 600);
        let legacy_focus_seconds: i64 = d
            .0
            .lock()
            .query_row(
                "SELECT SUM(strftime('%s',ended_at)-strftime('%s',started_at)) FROM focus_intervals WHERE play_session_id=?",
                [session],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(legacy_focus_seconds, recorded.playtime_seconds);
    }
    #[test]
    fn brand_list_is_independent_from_game_filtering() {
        let d = Database::memory().unwrap();
        game(&d);
        d.create_game(
            &CreateGameInput {
                title: "Other".into(),
                brand: Some("Another Brand".into()),
                release_date: None,
                thumbnail_path: None,
                erogamescape_id: None,
                source_url: None,
                executable_paths: vec![],
            },
            None,
        )
        .unwrap();
        assert_eq!(d.list_brands().unwrap(), vec!["Another Brand", "B"]);
        assert_eq!(
            d.list_games("", Some("B"), None, "title", false)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(d.list_brands().unwrap().len(), 2);
    }
    #[test]
    fn play_status_defaults_updates_and_filters() {
        let d = Database::memory().unwrap();
        let first = game(&d);
        let second = d
            .create_game(
                &CreateGameInput {
                    title: "Completed".into(),
                    brand: Some("B".into()),
                    release_date: None,
                    thumbnail_path: None,
                    erogamescape_id: None,
                    source_url: None,
                    executable_paths: vec![],
                },
                None,
            )
            .unwrap();
        assert_eq!(d.get_game(first).unwrap().summary.play_status, "unplayed");
        d.update_play_status(second, "completed").unwrap();
        let completed = d
            .list_games("", None, Some("completed"), "title", false)
            .unwrap();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].id, second);
        assert!(d.update_play_status(first, "invalid").is_err());
    }
}
