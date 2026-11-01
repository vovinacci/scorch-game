//! Self-update from GitHub Releases.
//!
//! A check asks GitHub for the latest release; when it's newer, the main menu offers it. Updating downloads
//! `SHA256SUMS` and its minisign signature, verifies the signature with the public key built into the game
//! (`assets/release-key.pub`), downloads this platform's file, checks its hash, replaces the game and restarts it.
//! A file that fails any check is never installed. Network work runs on a background thread.
//!
//! Off without the `self-update` Cargo feature (package managers update the game themselves) and for development
//! builds (any version with `+` build metadata).

// Without the feature only `enabled()` (false) and the status type are used; the rest is compiled for the feature.
#![cfg_attr(not(feature = "self-update"), allow(dead_code))]

use std::sync::{Arc, Mutex};

use crate::crash::VERSION;

/// Where the game looks for updates.
pub const REPO: &str = "vovinacci/scorch-game";

/// What the updater is doing, shown on the main menu and in Settings.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Idle,
    Checking,
    UpToDate,
    /// The check didn't get an answer (offline, GitHub down); silent unless asked for.
    CheckFailed(String),
    /// A newer release, its version without "v".
    Available {
        version: String,
    },
    Downloading {
        version: String,
    },
    /// Installed; restart into it.
    Installed {
        version: String,
    },
    InstallFailed(String),
}

/// Runs checks and updates in the background; the UI polls `status()` every frame.
pub struct Updater {
    status: Arc<Mutex<Status>>,
}

impl Updater {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(Status::Idle)),
        }
    }

    pub fn status(&self) -> Status {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(Status::Idle)
    }

    /// Forgets an offered update until the next check ("Later").
    pub fn dismiss(&self) {
        self.set(Status::Idle);
    }

    fn set(&self, status: Status) {
        if let Ok(mut s) = self.status.lock() {
            *s = status;
        }
    }

    fn busy(&self) -> bool {
        matches!(self.status(), Status::Checking | Status::Downloading { .. })
    }

    /// Starts a check in the background, unless one is already running.
    pub fn check(&self) {
        if !enabled() || self.busy() {
            return;
        }
        self.set(Status::Checking);
        #[cfg(feature = "self-update")]
        {
            let status = self.status.clone();
            std::thread::spawn(move || {
                let result = match net::latest() {
                    Ok(version) if is_newer(&version, current_version()) => {
                        Status::Available { version }
                    }
                    Ok(_) => Status::UpToDate,
                    Err(e) => Status::CheckFailed(e),
                };
                if let Ok(mut s) = status.lock() {
                    *s = result;
                }
            });
        }
    }

    /// Downloads, verifies and installs the offered version in the background.
    pub fn install(&self) {
        let Status::Available { version } = self.status() else {
            return;
        };
        self.set(Status::Downloading {
            version: version.clone(),
        });
        #[cfg(feature = "self-update")]
        {
            let status = self.status.clone();
            std::thread::spawn(move || {
                let result = match net::install(&version) {
                    Ok(()) => Status::Installed { version },
                    Err(e) => Status::InstallFailed(e),
                };
                if let Ok(mut s) = status.lock() {
                    *s = result;
                }
            });
        }
    }
}

/// Whether this build can update itself.
pub fn enabled() -> bool {
    cfg!(feature = "self-update") && !VERSION.contains('+')
}

/// "0.1.2" from "0.1.2+3.gabc1234".
fn current_version() -> &'static str {
    VERSION.split('+').next().unwrap_or(VERSION)
}

/// `latest` is a newer version than `current` (both "X.Y.Z").
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let mut parts = v.trim_start_matches('v').split('.').map(|p| p.parse().ok());
        Some((parts.next()??, parts.next()??, parts.next()??))
    };
    match (parse(latest), parse(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// `scorch --update`: checks and installs from the command line. Returns the process exit code.
pub fn command_line() -> i32 {
    if !enabled() {
        println!("Self-update is off in this build (Scorch {VERSION}).");
        return 1;
    }
    #[cfg(feature = "self-update")]
    {
        let latest = match net::latest() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Couldn't check for updates: {e}");
                return 1;
            }
        };
        if !is_newer(&latest, current_version()) {
            println!("Scorch {VERSION} is the latest version.");
            return 0;
        }
        println!("Updating Scorch {VERSION} to {latest}...");
        match net::install(&latest) {
            Ok(()) => {
                println!("Scorch {latest} is installed.");
                0
            }
            Err(e) => {
                eprintln!("Update failed: {e}");
                1
            }
        }
    }
    #[cfg(not(feature = "self-update"))]
    1
}

/// Restarts into the installed version: starts it, then exits this one.
pub fn restart() -> ! {
    #[cfg(feature = "self-update")]
    if let Some(exe) = net::installed_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
    std::process::exit(0);
}

#[cfg(feature = "self-update")]
mod net {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use sha2::{Digest, Sha256};

    use super::{REPO, VERSION};

    /// The release signing key (minisign public key file).
    const PUBLIC_KEY: &str = include_str!("../assets/release-key.pub");
    /// Largest download accepted (the biggest release file is a few MB).
    const MAX_DOWNLOAD: u64 = 100 * 1024 * 1024;

    fn agent(timeout: u64) -> ureq::Agent {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(timeout)))
            .user_agent(format!("scorch/{VERSION}"))
            .build()
            .into()
    }

    fn get(url: &str, timeout: u64) -> Result<Vec<u8>, String> {
        agent(timeout)
            .get(url)
            .call()
            .map_err(|e| format!("{url}: {e}"))?
            .body_mut()
            .with_config()
            .limit(MAX_DOWNLOAD)
            .read_to_vec()
            .map_err(|e| format!("{url}: {e}"))
    }

    /// The version of the latest published release, without "v".
    pub fn latest() -> Result<String, String> {
        let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
        let json: serde_json::Value =
            serde_json::from_slice(&get(&url, 20)?).map_err(|e| e.to_string())?;
        let tag = json["tag_name"]
            .as_str()
            .ok_or("no tag_name in the release")?;
        Ok(tag.trim_start_matches('v').to_string())
    }

    fn download_url(version: &str, file: &str) -> String {
        format!("https://github.com/{REPO}/releases/download/v{version}/{file}")
    }

    /// The release file for this platform and how to install it.
    enum Target {
        /// A plain executable replacing the running one (Linux, Windows).
        Binary(String),
        /// An AppImage replacing the AppImage file the game runs from.
        AppImage { file: String, path: PathBuf },
        /// A zip with `Scorch.app`, replacing the bundle the game runs from.
        MacApp { file: String, bundle: PathBuf },
    }

    fn target(version: &str) -> Result<Target, String> {
        let arch = std::env::consts::ARCH;
        let tag = format!("v{version}");
        if cfg!(target_os = "macos") {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            // .../Scorch.app/Contents/MacOS/scorch
            let bundle = exe
                .ancestors()
                .nth(3)
                .filter(|b| b.extension().is_some_and(|e| e == "app"))
                .ok_or("the game isn't running from Scorch.app")?;
            Ok(Target::MacApp {
                file: format!("scorch-{tag}-macos.zip"),
                bundle: bundle.to_path_buf(),
            })
        } else if let Some(path) = std::env::var_os("APPIMAGE") {
            Ok(Target::AppImage {
                file: format!("Scorch-{tag}-{arch}.AppImage"),
                path: PathBuf::from(path),
            })
        } else if cfg!(windows) {
            Ok(Target::Binary(format!("scorch-{tag}-windows-{arch}.exe")))
        } else {
            Ok(Target::Binary(format!("scorch-{tag}-linux-{arch}")))
        }
    }

    /// Checks `sums` against its signature made with `public_key` (a minisign public key file) and returns the
    /// SHA-256 listed for `file`.
    pub(super) fn expected_hash(
        public_key: &str,
        sums: &str,
        signature: &str,
        file: &str,
    ) -> Result<String, String> {
        let key = minisign_verify::PublicKey::decode(public_key).map_err(|e| e.to_string())?;
        let signature =
            minisign_verify::Signature::decode(signature).map_err(|e| format!("signature: {e}"))?;
        key.verify(sums.as_bytes(), &signature, false)
            .map_err(|_| "SHA256SUMS signature doesn't match the release key".to_string())?;
        sums.lines()
            .find_map(|line| {
                let (hash, name) = line.split_once("  ")?;
                (name == file).then(|| hash.to_lowercase())
            })
            .ok_or_else(|| format!("{file} isn't in SHA256SUMS"))
    }

    pub(super) fn sha256_hex(data: &[u8]) -> String {
        Sha256::digest(data)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    pub fn install(version: &str) -> Result<(), String> {
        let target = target(version)?;
        let file = match &target {
            Target::Binary(file) | Target::AppImage { file, .. } | Target::MacApp { file, .. } => {
                file.clone()
            }
        };
        let text = |bytes: Vec<u8>| String::from_utf8(bytes).map_err(|e| e.to_string());
        let sums = text(get(&download_url(version, "SHA256SUMS"), 30)?)?;
        let signature = text(get(&download_url(version, "SHA256SUMS.minisig"), 30)?)?;
        let expected = expected_hash(PUBLIC_KEY, &sums, &signature, &file)?;
        let data = get(&download_url(version, &file), 300)?;
        if sha256_hex(&data) != expected {
            return Err(format!("{file}: checksum mismatch"));
        }

        match target {
            Target::Binary(_) => {
                let new =
                    std::env::temp_dir().join(format!("scorch-update-{}", std::process::id()));
                write_executable(&new, &data)?;
                let result = self_replace::self_replace(&new).map_err(|e| e.to_string());
                let _ = fs::remove_file(&new);
                result
            }
            Target::AppImage { path, .. } => {
                let new = path.with_extension("update");
                write_executable(&new, &data)?;
                fs::rename(&new, &path).map_err(|e| e.to_string())
            }
            Target::MacApp { bundle, .. } => replace_bundle(&bundle, &data),
        }
    }

    fn write_executable(path: &Path, data: &[u8]) -> Result<(), String> {
        fs::write(path, data).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Unpacks the release zip next to the running `Scorch.app` (same volume, so renames work) with the system's
    /// `ditto`, then swaps the bundles.
    fn replace_bundle(bundle: &Path, zip: &[u8]) -> Result<(), String> {
        let parent = bundle.parent().ok_or("Scorch.app has no parent folder")?;
        let work = parent.join(format!(".scorch-update-{}", std::process::id()));
        let _ = fs::remove_dir_all(&work);
        fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let result = (|| {
            let zip_path = work.join("update.zip");
            fs::write(&zip_path, zip).map_err(|e| e.to_string())?;
            let ok = std::process::Command::new("/usr/bin/ditto")
                .args(["-x", "-k"])
                .arg(&zip_path)
                .arg(&work)
                .status()
                .map_err(|e| e.to_string())?
                .success();
            if !ok {
                return Err("couldn't unpack the update".to_string());
            }
            let new_bundle = fs::read_dir(&work)
                .map_err(|e| e.to_string())?
                .filter_map(|e| e.ok().map(|e| e.path().join("Scorch.app")))
                .find(|p| p.is_dir())
                .ok_or("Scorch.app isn't in the update")?;
            let old = work.join("old.app");
            fs::rename(bundle, &old).map_err(|e| e.to_string())?;
            if let Err(e) = fs::rename(&new_bundle, bundle) {
                let _ = fs::rename(&old, bundle);
                return Err(e.to_string());
            }
            Ok(())
        })();
        let _ = fs::remove_dir_all(&work);
        result
    }

    /// The executable to start after an update: the same path as now (the file or bundle was replaced in place).
    pub fn installed_exe() -> Option<PathBuf> {
        match std::env::var_os("APPIMAGE") {
            Some(path) => Some(PathBuf::from(path)),
            None => std::env::current_exe().ok(),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn sha256_is_lowercase_hex() {
            assert_eq!(
                sha256_hex(b"abc"),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            );
        }

        // A throwaway key pair, used only here; SUMS was signed with its secret key.
        const TEST_KEY: &str = "untrusted comment: minisign public key 2E4BB988727CB55F
RWRftXxyiLlLLmIi+pr7Z4bgCQOQSk6fkxA1syzsXpEckcKA46jYWKI9
";
        const SUMS: &str = "abc123  scorch-v9.9.9-linux-x86_64\n";
        const SIGNATURE: &str = "untrusted comment: signature from minisign secret key
RURftXxyiLlLLuc/1K6ikfyG1AUVf8/dLjHRAoEeLp6bf3DhZjIidTJnTJaWw1jaxEXCEe1TEz5Jc2ZexDH6ARTRnAHWg9m0yAc=
trusted comment: test
/YGpmmmfeh1lfchRq6/bfAl976Jxlrj1r44UEN3AuF08fDu6E1fNg7bf45pePsRsM+mq5nQTQ6g29hYefeYrDQ==
";

        #[test]
        fn a_signed_checksum_list_gives_the_hash() {
            let hash = expected_hash(TEST_KEY, SUMS, SIGNATURE, "scorch-v9.9.9-linux-x86_64");
            assert_eq!(hash, Ok("abc123".to_string()));
            assert!(expected_hash(TEST_KEY, SUMS, SIGNATURE, "other-file").is_err());
        }

        #[test]
        fn a_changed_checksum_list_is_rejected() {
            let forged = SUMS.replace("abc123", "def456");
            assert!(
                expected_hash(TEST_KEY, &forged, SIGNATURE, "scorch-v9.9.9-linux-x86_64").is_err()
            );
        }

        #[test]
        fn the_release_key_parses() {
            assert!(minisign_verify::PublicKey::decode(PUBLIC_KEY).is_ok());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(is_newer("v1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.2", "0.1.2"));
        assert!(!is_newer("0.1.1", "0.1.2"));
        assert!(!is_newer("garbage", "0.1.2"));
    }
}
