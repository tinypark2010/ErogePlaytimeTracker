use crate::models::{ReleaseChanges, UpdateCompletionNotice};
use anyhow::{Result, ensure};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path, path::PathBuf};

const STATE_FILE: &str = "update-notice-state.json";
const BUNDLED_NOTES: &str = include_str!("../../release-notes/ja.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: u32,
    releases: Vec<ReleaseChanges>,
}

// An unacknowledged checkpoint includes its own version in the pending range.
// This also preserves the first notice when upgrading from a pre-feature app.
#[derive(Deserialize, Serialize)]
struct Checkpoint {
    version: Version,
    acknowledged: bool,
}

pub struct UpdateNotices {
    path: PathBuf,
    current: Version,
    checkpoint: Checkpoint,
    releases: Vec<ReleaseChanges>,
}

impl UpdateNotices {
    pub fn open(root: &Path, current: Version, existing_installation: bool) -> Result<Self> {
        let path = root.join(STATE_FILE);
        let checkpoint = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let initial = Checkpoint {
                    version: current.clone(),
                    acknowledged: !existing_installation,
                };
                save(&path, &initial)?;
                initial
            }
            Err(error) => return Err(error.into()),
        };
        // Missing/invalid notes must never prevent the completion confirmation.
        let releases = parse_catalog(BUNDLED_NOTES).unwrap_or_else(|error| {
            log::warn!("could not read bundled update notes: {error:#}");
            Vec::new()
        });
        Ok(Self {
            path,
            current,
            checkpoint,
            releases,
        })
    }

    pub fn pending(&self) -> Option<UpdateCompletionNotice> {
        if self.current < self.checkpoint.version
            || (self.current == self.checkpoint.version && self.checkpoint.acknowledged)
        {
            return None;
        }
        let releases = self
            .releases
            .iter()
            .filter(|release| {
                release.version <= self.current
                    && (release.version > self.checkpoint.version
                        || (release.version == self.checkpoint.version
                            && !self.checkpoint.acknowledged))
                    && !release.changes.is_empty()
            })
            .cloned()
            .collect();
        Some(UpdateCompletionNotice {
            version: self.current.clone(),
            releases,
        })
    }

    pub fn acknowledge(&mut self, version: &str) -> Result<()> {
        ensure!(
            Version::parse(version)? == self.current,
            "notice version mismatch"
        );
        // Do not lower the checkpoint on a downgrade, or repeat an acknowledged notice.
        if self.pending().is_none() {
            return Ok(());
        }
        let checkpoint = Checkpoint {
            version: self.current.clone(),
            acknowledged: true,
        };
        save(&self.path, &checkpoint)?;
        self.checkpoint = checkpoint;
        Ok(())
    }
}

fn parse_catalog(source: &str) -> Result<Vec<ReleaseChanges>> {
    let mut catalog: Catalog = serde_json::from_str(source)?;
    ensure!(
        catalog.schema_version == 1,
        "unsupported release notes schema"
    );
    catalog.releases.sort_by(|a, b| b.version.cmp(&a.version));
    for release in &catalog.releases {
        ensure!(
            release.version.pre.is_empty() && release.version.build.is_empty(),
            "release notes require stable versions"
        );
        ensure!(
            release.changes.iter().all(|text| !text.trim().is_empty()),
            "empty release note item"
        );
    }
    ensure!(
        catalog
            .releases
            .windows(2)
            .all(|pair| pair[0].version != pair[1].version),
        "duplicate release notes version"
    );
    Ok(catalog.releases)
}

fn save(path: &Path, checkpoint: &Checkpoint) -> Result<()> {
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(&serde_json::to_vec(checkpoint)?)?;
    file.sync_all()?;
    drop(file);
    // Rename replaces the destination atomically on Windows, without a delete gap.
    fs::rename(temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "ept-update-notice-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn open(&self, version: &str, existing: bool) -> UpdateNotices {
            let mut notices =
                UpdateNotices::open(&self.0, Version::parse(version).unwrap(), existing).unwrap();
            notices.releases = parse_catalog(
                r#"{"schema_version":1,"releases":[
                {"version":"0.1.9","changes":["以前の変更"]},
                {"version":"0.1.10","changes":["機能を追加しました。"]},
                {"version":"0.1.11","changes":[]},
                {"version":"0.1.12","changes":["不具合を修正しました。"]}
            ]}"#,
            )
            .unwrap();
            notices
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn fresh_install_and_same_version_restarts_do_not_notify() {
        let root = TestDirectory::new();
        assert!(root.open("0.1.9", false).pending().is_none());
        assert!(root.open("0.1.9", true).pending().is_none());
    }

    #[test]
    fn upgrade_notifies_until_acknowledged_including_after_restarts() {
        let root = TestDirectory::new();
        root.open("0.1.9", false);
        assert!(root.open("0.1.10", true).pending().is_some());
        let mut notices = root.open("0.1.10", true);
        assert_eq!(
            notices.pending().unwrap().releases[0].changes,
            ["機能を追加しました。"]
        );
        assert!(notices.acknowledge("0.1.12").is_err());
        assert!(notices.pending().is_some());
        notices.acknowledge("0.1.10").unwrap();
        assert!(notices.pending().is_none());
        assert!(root.open("0.1.10", true).pending().is_none());
    }

    #[test]
    fn legacy_install_only_shows_current_release_and_keeps_unread_range() {
        let root = TestDirectory::new();
        let notice = root.open("0.1.10", true).pending().unwrap();
        assert_eq!(notice.releases.len(), 1);
        let notice = root.open("0.1.12", true).pending().unwrap();
        assert_eq!(notice.releases.len(), 2);
        assert_eq!(
            notice.releases[1].version,
            Version::parse("0.1.10").unwrap()
        );
    }

    #[test]
    fn skipped_versions_are_numeric_newest_first_and_exclude_future_and_read_notes() {
        let root = TestDirectory::new();
        root.open("0.1.9", false);
        let mut notices = root.open("0.1.11", true);
        let notice = notices.pending().unwrap();
        assert_eq!(notice.version, Version::parse("0.1.11").unwrap());
        assert_eq!(notice.releases.len(), 1);
        assert_eq!(
            notice.releases[0].version,
            Version::parse("0.1.10").unwrap()
        );
        notices.acknowledge("0.1.11").unwrap();
        let notice = root.open("0.1.12", true).pending().unwrap();
        assert_eq!(notice.releases.len(), 1);
        assert_eq!(notice.releases[0].version, notice.version);
    }

    #[test]
    fn empty_or_missing_notes_still_confirm_completion_once() {
        let root = TestDirectory::new();
        root.open("0.1.10", false);
        let mut notices = root.open("0.1.11", true);
        assert!(notices.pending().unwrap().releases.is_empty());
        notices.acknowledge("0.1.11").unwrap();
        assert!(root.open("0.1.11", true).pending().is_none());
        let mut notices = root.open("0.1.13", true);
        notices.releases.clear();
        assert!(notices.pending().unwrap().releases.is_empty());
    }

    #[test]
    fn downgrade_and_reinstall_do_not_reset_acknowledgment() {
        let root = TestDirectory::new();
        root.open("0.1.12", false);
        let mut older = root.open("0.1.10", true);
        assert!(older.pending().is_none());
        older.acknowledge("0.1.10").unwrap();
        assert!(root.open("0.1.12", true).pending().is_none());
        assert!(root.open("0.1.13", true).pending().is_some());
    }

    #[test]
    fn failed_acknowledgment_remains_pending_and_can_be_retried() {
        let root = TestDirectory::new();
        let mut notices = root.open("0.1.10", true);
        fs::create_dir(root.0.join(STATE_FILE).with_extension("tmp")).unwrap();
        assert!(notices.acknowledge("0.1.10").is_err());
        assert!(notices.pending().is_some());
        fs::remove_dir(root.0.join(STATE_FILE).with_extension("tmp")).unwrap();
        assert!(root.open("0.1.10", true).pending().is_some());
        notices.acknowledge("0.1.10").unwrap();
        assert!(root.open("0.1.10", true).pending().is_none());
    }

    #[test]
    fn malformed_state_is_not_silently_overwritten() {
        let root = TestDirectory::new();
        fs::write(root.0.join(STATE_FILE), b"invalid").unwrap();
        assert!(UpdateNotices::open(&root.0, Version::parse("0.1.12").unwrap(), true).is_err());
        assert_eq!(fs::read(root.0.join(STATE_FILE)).unwrap(), b"invalid");
    }

    #[test]
    fn bundled_catalog_is_valid_and_preserves_explicit_empty_releases() {
        parse_catalog(BUNDLED_NOTES).unwrap();
        let empty = r#"{"schema_version":1,"releases":[{"version":"1.0.0","changes":[]}]}"#;
        assert_eq!(parse_catalog(empty).unwrap().len(), 1);
        assert!(parse_catalog(&empty.replace("\"changes\":[]", "\"other\":[]")).is_err());
        assert!(parse_catalog(&empty.replace("1.0.0", "1.0.0-beta.1")).is_err());
        assert!(
            parse_catalog(&empty.replace("\"schema_version\":1", "\"schema_version\":2")).is_err()
        );
    }
}
