//! Continuous log of recognized text as JSON Lines: one `{"time", "text"}` object per line.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Number of entries shown in the settings window.
const LIST_LIMIT: usize = 200;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    /// RFC 3339, UTC.
    pub time: String,
    pub text: String,
}

pub struct History {
    path: PathBuf,
    lock: Mutex<()>,
}

impl History {
    /// Creates the file if missing so that its folder can always be revealed.
    pub fn new(path: PathBuf) -> History {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = OpenOptions::new().create(true).append(true).open(&path);
        History {
            path,
            lock: Mutex::new(()),
        }
    }

    pub fn append(&self, text: &str) -> Result<Entry, String> {
        let entry = Entry {
            time: rfc3339_utc(SystemTime::now()),
            text: text.to_string(),
        };
        let line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
        let _guard = self.lock.lock().unwrap();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("history: {e}"))?;
        writeln!(file, "{line}").map_err(|e| format!("history: {e}"))?;
        Ok(entry)
    }

    /// Most recent entries first. Unreadable lines are skipped.
    pub fn list(&self) -> Vec<Entry> {
        let _guard = self.lock.lock().unwrap();
        let text = fs::read_to_string(&self.path).unwrap_or_default();
        text.lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .rev()
            .take(LIST_LIMIT)
            .collect()
    }

    pub fn clear(&self) -> Result<(), String> {
        let _guard = self.lock.lock().unwrap();
        fs::write(&self.path, "").map_err(|e| format!("history: {e}"))
    }
}

fn rfc3339_utc(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_history(name: &str) -> History {
        let dir =
            std::env::temp_dir().join(format!("jabberwrite-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        History::new(dir.join("history.jsonl"))
    }

    #[test]
    fn formats_timestamps() {
        let at = |s| rfc3339_utc(UNIX_EPOCH + Duration::from_secs(s));
        assert_eq!(at(0), "1970-01-01T00:00:00Z");
        assert_eq!(at(951_827_696), "2000-02-29T12:34:56Z");
        assert_eq!(at(1_793_278_275), "2026-10-29T12:51:15Z");
    }

    #[test]
    fn appends_lists_newest_first_and_clears() {
        let history = temp_history("roundtrip");
        assert!(history.list().is_empty());
        history.append("first").unwrap();
        history.append("second\nline \"quoted\"").unwrap();
        let entries = history.list();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "second\nline \"quoted\"");
        assert_eq!(entries[1].text, "first");
        let raw = fs::read_to_string(&history.path).unwrap();
        assert_eq!(raw.lines().count(), 2);
        history.clear().unwrap();
        assert!(history.list().is_empty());
        assert!(history.path.exists());
    }
}
