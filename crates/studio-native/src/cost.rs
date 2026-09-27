//! What the Assistant has cost, as estimates from the dated price table
//! (C-37, R-42): the running turn's cost, and each day's total, kept in
//! `usage.json` beside the session file so the day's total survives a
//! restart. Days are UTC days. Nothing here limits spending.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Days kept in the file.
const DAYS_KEPT: usize = 31;

#[derive(Default, Serialize, Deserialize)]
struct File {
    format: u32,
    /// Estimated US dollars per UTC day (`2026-09-28`).
    days: BTreeMap<String, f64>,
}

pub struct DailyCost {
    path: PathBuf,
    file: File,
}

impl DailyCost {
    /// Reads the file beside `session`; a missing or unreadable file starts
    /// from nothing (the figures are estimates, not records).
    pub fn load(session: &Path) -> DailyCost {
        let path = session.with_file_name("usage.json");
        let file = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<File>(&text).ok())
            .filter(|file| file.format == 1)
            .unwrap_or(File {
                format: 1,
                days: BTreeMap::new(),
            });
        DailyCost { path, file }
    }

    /// Today's total so far.
    pub fn today(&self) -> f64 {
        self.file.days.get(&today()).copied().unwrap_or(0.0)
    }

    /// Adds to today's total and saves; a failed save is not worth a
    /// message, as the next one tries again.
    pub fn add(&mut self, cost: f64) {
        if cost <= 0.0 || !cost.is_finite() {
            return;
        }
        *self.file.days.entry(today()).or_insert(0.0) += cost;
        while self.file.days.len() > DAYS_KEPT {
            let oldest = self.file.days.keys().next().cloned();
            if let Some(oldest) = oldest {
                self.file.days.remove(&oldest);
            }
        }
        if let Ok(text) = serde_json::to_string_pretty(&self.file) {
            let _ = std::fs::write(&self.path, text);
        }
    }
}

/// Today's UTC date as `YYYY-MM-DD`.
fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() / 86_400) as i64;
    date(days)
}

/// A day number since 1970-01-01 as a civil date (Howard Hinnant's
/// `civil_from_days`).
fn date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// A cost as the Studio shows it: `≈ $0.021`, `≈ $1.35`.
pub fn dollars(cost: f64) -> String {
    if cost < 0.1 {
        format!("≈ ${cost:.3}")
    } else {
        format!("≈ ${cost:.2}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_become_dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(19_723), "2024-01-01");
        assert_eq!(date(20_724), "2026-09-28");
        assert_eq!(date(11_016), "2000-02-29");
    }

    #[test]
    fn the_day_total_survives_a_restart() {
        let folder = std::env::temp_dir().join(format!("agq-cost-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let session = folder.join("session.json");
        let mut cost = DailyCost::load(&session);
        cost.add(0.25);
        cost.add(0.5);
        cost.add(f64::NAN);
        assert!((DailyCost::load(&session).today() - 0.75).abs() < 1e-9);
        assert_eq!(dollars(0.0213), "≈ $0.021");
        assert_eq!(dollars(1.349), "≈ $1.35");
        let _ = std::fs::remove_dir_all(folder);
    }
}
