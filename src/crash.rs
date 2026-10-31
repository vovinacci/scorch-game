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

/// A crash report not yet shown to the player.
pub struct Pending {
    pub path: PathBuf,
    pub text: String,
}

/// The newest unseen crash report, if any.
pub fn pending() -> Option<Pending> {
    let path = fs::read_dir(dir()?)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with("crash-") && name.ends_with(".log") && !name.ends_with(".seen.log")
        })
        .max()?;
    let text = fs::read_to_string(&path).ok()?;
    Some(Pending { path, text })
}

/// Marks a report as seen so it isn't offered again.
pub fn mark_seen(pending: &Pending) {
    let seen = pending.path.with_extension("seen.log");
    let _ = fs::rename(&pending.path, seen);
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
}
