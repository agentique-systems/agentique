//! Run results in the app's per-project data (ROADMAP §4.9, §8.3): one JSON
//! file per run, written atomically, never committed. They survive
//! reopening the project; their freshness is computed when they are shown.

use crate::result::{FORMAT, Mode, RunResult, RunStatus, Verdict};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The results of one project.
#[derive(Clone, Debug, PartialEq)]
pub struct RunStore {
    folder: PathBuf,
}

/// A result without its trace, for lists.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub id: String,
    pub scenario: u64,
    pub scenario_name: String,
    pub mode: Mode,
    pub started: String,
    pub status: RunStatus,
    pub tally: Vec<(Verdict, usize)>,
    pub model_digest: String,
    pub all_passed: bool,
}

impl RunStore {
    /// Results kept in `folder` (created when the first is saved).
    pub fn new(folder: impl Into<PathBuf>) -> RunStore {
        RunStore {
            folder: folder.into(),
        }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Saves a result, replacing one with the same id.
    pub fn save(&self, result: &RunResult) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.folder)?;
        let path = self.folder.join(format!("{}.json", result.id));
        let temporary = self.folder.join(format!("{}.json.tmp", result.id));
        let text = serde_json::to_string(result).map_err(std::io::Error::other)?;
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
        }
        std::fs::rename(&temporary, &path)
    }

    pub fn load(&self, id: &str) -> Option<RunResult> {
        if id.contains(['/', '\\']) || id.contains("..") {
            return None;
        }
        let text = std::fs::read_to_string(self.folder.join(format!("{id}.json"))).ok()?;
        let result: RunResult = serde_json::from_str(&text).ok()?;
        (result.format == FORMAT).then_some(result)
    }

    /// Every readable result, newest first. Unreadable files are skipped.
    pub fn list(&self) -> Vec<RunSummary> {
        let Ok(entries) = std::fs::read_dir(&self.folder) else {
            return Vec::new();
        };
        let mut out: Vec<RunSummary> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .filter_map(|text| serde_json::from_str::<RunResult>(&text).ok())
            .filter(|r| r.format == FORMAT)
            .map(|r| RunSummary {
                id: r.id.clone(),
                scenario: r.scenario,
                scenario_name: r.scenario_name.clone(),
                mode: r.mode,
                started: r.started.clone(),
                status: r.status,
                tally: r.tally(),
                model_digest: r.provenance.model_digest.clone(),
                all_passed: r.all_passed(),
            })
            .collect();
        out.sort_by(|a, b| b.started.cmp(&a.started).then(b.id.cmp(&a.id)));
        out
    }

    /// The newest result of a scenario in a mode.
    pub fn latest(&self, scenario: u64, mode: Mode) -> Option<RunResult> {
        let summary = self
            .list()
            .into_iter()
            .find(|s| s.scenario == scenario && s.mode == mode)?;
        self.load(&summary.id)
    }

    pub fn delete(&self, id: &str) -> std::io::Result<()> {
        if id.contains(['/', '\\']) || id.contains("..") {
            return Ok(());
        }
        std::fs::remove_file(self.folder.join(format!("{id}.json")))
    }
}
