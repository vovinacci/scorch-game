//! Crash reports. If the game panics, a report (version, system, error,
//! backtrace) is written to `crash-<time>.log` in the data folder. On the next
//! start the main menu offers to copy it to the clipboard for sending; after
//! that it is renamed to `crash-<time>.seen.log` so it's offered only once.

use std::fs;
use std::panic::PanicHookInfo;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Full build version, e.g. `0.1.0+3.gabc1234.dirty` (see `build.rs`).
pub const VERSION: &str = env!("SCORCH_BUILD");

/// "Scorch 0.1.0+3.gabc1234 (linux x86_64, release)"
pub fn version_line() -> String {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    format!(
        "Scorch {VERSION} ({} {}, {profile})",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

/// Writes a crash report on any panic, then lets the default handler run.
pub fn install() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = std::backtrace::Backtrace::force_capture().to_string();
        let text = report(&message(info), &location(info), &backtrace);
        match write(&text) {
            Some(path) => eprintln!("Scorch crashed. Report saved to {}", path.display()),
            None => eprintln!("Scorch crashed. Could not save a report:\n{text}"),
        }
        default(info);
    }));
}

fn message(info: &PanicHookInfo) -> String {
    let payload = info.payload();
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string())
}

fn location(info: &PanicHookInfo) -> String {
    info.location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn report(message: &str, location: &str, backtrace: &str) -> String {
    format!(
        "{}\ntime (unix): {}\n\nerror: {message}\nat: {location}\n\nbacktrace:\n{backtrace}",
        version_line(),
        now()
    )
}

fn dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("scorch"))
}

fn write(text: &str) -> Option<PathBuf> {
    let dir = dir()?;
    fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("crash-{}.log", now()));
    fs::write(&path, text).ok()?;
    Some(path)
}

/// Crash reports not yet shown to the player, offered together in one notice.
pub struct Pending {
    /// Newest first.
    pub paths: Vec<PathBuf>,
    /// All reports, newest first, ready to paste into a message.
    pub text: String,
}

impl Pending {
    pub fn count(&self) -> usize {
        self.paths.len()
    }

    /// The report's path, or its folder when there are several.
    pub fn location(&self) -> PathBuf {
        match self.paths.as_slice() {
            [one] => one.clone(),
            _ => self.paths[0]
                .parent()
                .map(PathBuf::from)
                .unwrap_or_default(),
        }
    }
}

/// Every unseen crash report, if there are any.
pub fn pending() -> Option<Pending> {
    pending_in(&dir()?)
}

fn pending_in(dir: &std::path::Path) -> Option<Pending> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with("crash-") && name.ends_with(".log") && !name.ends_with(".seen.log")
        })
        .collect();
    if paths.is_empty() {
        return None;
    }
    // crash-<unix time>.log: newest first.
    paths.sort_by(|a, b| b.cmp(a));
    let text = paths
        .iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_string_lossy();
            Some(format!("=== {name} ===\n{}", fs::read_to_string(p).ok()?))
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    Some(Pending { paths, text })
}

/// Marks the reports as seen so they aren't offered again.
pub fn mark_seen(pending: &Pending) {
    for path in &pending.paths {
        let _ = fs::rename(path, path.with_extension("seen.log"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_has_what_a_bug_report_needs() {
        let text = report("index out of bounds", "src/battle.rs:10:5", "0: main");
        assert!(text.starts_with(&format!("Scorch {VERSION}")));
        assert!(text.contains("error: index out of bounds"));
        assert!(text.contains("at: src/battle.rs:10:5"));
        assert!(text.contains("backtrace:\n0: main"));
    }

    #[test]
    fn all_unseen_reports_are_offered_once_together() {
        let dir = std::env::temp_dir().join(format!("scorch-crash-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("crash-100.log"), "old").unwrap();
        fs::write(dir.join("crash-200.log"), "new").unwrap();
        fs::write(dir.join("crash-50.seen.log"), "already sent").unwrap();

        let pending = pending_in(&dir).unwrap();
        assert_eq!(pending.count(), 2);
        assert_eq!(pending.location(), dir);
        let (new, old) = (
            pending.text.find("new").unwrap(),
            pending.text.find("old").unwrap(),
        );
        assert!(new < old, "newest first: {}", pending.text);
        assert!(!pending.text.contains("already sent"));

        mark_seen(&pending);
        assert!(pending_in(&dir).is_none(), "nothing is offered twice");
        let _ = fs::remove_dir_all(&dir);
    }
}
