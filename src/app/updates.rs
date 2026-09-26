//! Opt-in updates. All networking and installer work stay outside the UI thread.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rivet::update_protocol::{FEED_URL, MAX_METADATA_BYTES, Release, SignedRelease};
use serde::{Deserialize, Serialize};

use crate::app::session;
use crate::platform::update_io::{self, Installation, Scope};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DAY: u64 = 24 * 60 * 60;
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn public_key() -> io::Result<[u8; 32]> {
    hex::decode(include_str!("../../assets/update-public-key.hex").trim())
        .map_err(io::Error::other)?
        .try_into()
        .map_err(|_| io::Error::other("Invalid update public key"))
}

fn cache_dir() -> io::Result<PathBuf> {
    session::data_dir()
        .map(|dir| dir.join("updates"))
        .map_err(io::Error::other)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<T> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_METADATA_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_METADATA_BYTES {
        return Err(io::Error::other("Update metadata is too large"));
    }
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

fn write_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    crate::storage::atomic_write::atomic_write_json(path, value).map_err(io::Error::other)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn check_due(last: u64, current: u64) -> bool {
    last == 0 || current < last || current.saturating_sub(last) >= DAY
}

enum Event {
    Progress(u64),
    Done(io::Result<Option<SignedRelease>>),
}

struct Worker {
    events: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    manual: bool,
}

pub struct Controller {
    installation: Option<Installation>,
    enabled: bool,
    started: Instant,
    inspected_cache: bool,
    worker: Option<Worker>,
    ready: Option<SignedRelease>,
    status: String,
}

impl Controller {
    pub fn new(enabled: bool) -> Self {
        let installation = std::env::current_exe()
            .ok()
            .and_then(|exe| update_io::detect_installation(&exe));
        let status = cache_dir()
            .ok()
            .and_then(|dir| {
                let path = dir.join("result.json");
                let result: HelperResult = read_json(&path).ok()?;
                let _ = fs::remove_file(path);
                Some(if result.success {
                    String::new()
                } else {
                    crate::logging::log_error(&format!(
                        "Update helper postponed installation: {}",
                        result.detail
                    ));
                    if result.cancelled {
                        "Update postponed - approval cancelled".into()
                    } else {
                        "Update postponed - check Help".into()
                    }
                })
            })
            .unwrap_or_default();
        Self {
            installation,
            enabled,
            started: Instant::now(),
            inspected_cache: false,
            worker: None,
            ready: None,
            status,
        }
    }

    pub fn supported(&self) -> bool {
        self.installation.is_some()
    }

    pub fn requires_approval(&self) -> bool {
        self.installation
            .as_ref()
            .is_some_and(|install| install.scope == Scope::Machine)
    }

    fn ready_message(&self) -> &'static str {
        if self.requires_approval() {
            "Update ready - click to approve restart"
        } else {
            "Update ready - click to restart"
        }
    }

    pub fn status(&self) -> &str {
        &self.status
    }
    pub fn ready(&self) -> bool {
        self.ready.is_some()
    }
    pub fn install_on_exit(&self) -> bool {
        self.enabled && self.ready() && !self.requires_approval()
    }

    #[cfg(test)]
    pub(crate) fn simulate_ready(&mut self) {
        self.enabled = true;
        self.ready = Some(SignedRelease {
            payload: String::new(),
            signature: String::new(),
        });
        self.status = self.ready_message().into();
    }

    #[cfg(test)]
    pub(crate) fn simulate_machine_install(&mut self) {
        self.installation = Some(Installation {
            directory: PathBuf::from(r"C:\Program Files\Rivet"),
            scope: Scope::Machine,
        });
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled && self.supported();
        if !self.enabled {
            if let Some(worker) = &self.worker {
                worker.cancel.store(true, Ordering::Relaxed);
            }
            self.ready = None;
            self.status.clear();
        } else {
            self.inspected_cache = false;
            self.started = Instant::now() - Duration::from_secs(15);
        }
    }

    /// Returns a new last-check timestamp only when a network check is started.
    pub fn tick(&mut self, last_check: u64) -> Option<u64> {
        self.poll();
        if self.enabled
            && self.supported()
            && self.worker.is_none()
            && !self.ready()
            && self.started.elapsed() >= Duration::from_secs(15)
        {
            let due = check_due(last_check, now());
            if due || !self.inspected_cache {
                return self.start(false, due);
            }
        }
        None
    }

    pub fn check_now(&mut self) -> Option<u64> {
        if !self.supported() {
            self.status = "Updates require an installed copy of Rivet".into();
            return None;
        }
        if let Some(worker) = &mut self.worker {
            if !worker.cancel.load(Ordering::Relaxed) {
                worker.manual = true;
            }
            return None;
        }
        self.start(true, true)
    }

    pub fn report(&mut self, text: &str) {
        self.status = text.into();
    }

    fn start(&mut self, manual: bool, network: bool) -> Option<u64> {
        let cancel = Arc::new(AtomicBool::new(false));
        let child_cancel = cancel.clone();
        let (tx, rx) = mpsc::channel();
        match std::thread::Builder::new()
            .name("rivet-updates".into())
            .spawn(move || {
                let result = cache_dir()
                    .and_then(|cache| check_and_stage(&cache, network, &child_cancel, &tx));
                let _ = tx.send(Event::Done(result));
            }) {
            Ok(_) => {
                self.ready = None;
                self.worker = Some(Worker {
                    events: rx,
                    cancel,
                    manual,
                });
                self.inspected_cache = true;
                self.status = if manual {
                    "Checking for updates...".into()
                } else {
                    String::new()
                };
                network.then(now)
            }
            Err(error) => {
                crate::logging::log_error(&format!("Cannot start update worker: {error}"));
                self.inspected_cache = true;
                if manual {
                    self.status = "Could not check for updates".into();
                }
                // Back off even if the OS cannot create a worker.
                Some(now())
            }
        }
    }

    fn poll(&mut self) {
        let mut finished = false;
        let ready_message = self.ready_message();
        if let Some(worker) = &self.worker {
            loop {
                let event = match worker.events.try_recv() {
                    Ok(event) => event,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if !finished && worker.manual && !worker.cancel.load(Ordering::Relaxed) {
                            self.status = "Could not check for updates - try later".into();
                        }
                        finished = true;
                        break;
                    }
                };
                if worker.cancel.load(Ordering::Relaxed) {
                    finished |= matches!(event, Event::Done(_));
                    continue;
                }
                match event {
                    Event::Progress(percent) => {
                        self.status = format!("Downloading update: {percent}%")
                    }
                    Event::Done(result) => {
                        finished = true;
                        match result {
                            Ok(Some(signed)) => {
                                self.ready = Some(signed);
                                self.status = ready_message.into();
                            }
                            Ok(None) => {
                                self.status = if worker.manual {
                                    "No updates available".into()
                                } else {
                                    String::new()
                                };
                            }
                            Err(error) => {
                                crate::logging::log_error(&format!("Update check failed: {error}"));
                                self.status = if worker.manual {
                                    "Could not check for updates - try later".into()
                                } else {
                                    String::new()
                                };
                            }
                        }
                    }
                }
            }
        }
        if finished {
            self.worker = None;
        }
    }

    /// Called only after normal close checks and the session checkpoint succeed.
    pub fn handoff(&self, restart: bool) -> io::Result<()> {
        let installation = self
            .installation
            .as_ref()
            .filter(|_| self.supported())
            .ok_or_else(|| io::Error::other("This installation requires a manual update"))?;
        require_explicit_machine_update(installation.scope, restart)?;
        let signed = self
            .ready
            .as_ref()
            .ok_or_else(|| io::Error::other("No update is ready"))?;
        let cache = cache_dir()?;
        let parent_created = update_io::process_birth()?;
        let id = format!("{}-{parent_created}", std::process::id());
        let helper = cache.join(format!("helper-{id}.exe"));
        fs::copy(std::env::current_exe()?, &helper)?;
        let request = Handoff {
            parent: std::process::id(),
            parent_created,
            installation: installation.clone(),
            signed: signed.clone(),
            restart,
        };
        let request_path = cache.join(format!("handoff-{id}.json"));
        write_json(&request_path, &request)?;
        hidden_command(&helper)
            .arg("--apply-update")
            .arg(&request_path)
            .spawn()?;
        Ok(())
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        if let Some(worker) = &self.worker {
            worker.cancel.store(true, Ordering::Relaxed);
        }
    }
}

fn verified_release(signed: &SignedRelease) -> io::Result<Release> {
    let release = signed.verify(&public_key()?)?;
    if !release.is_newer_than(VERSION)? {
        return Err(io::Error::other("Update is not newer than Rivet"));
    }
    Ok(release)
}

fn cached_release(cache: &Path, key: &[u8; 32]) -> io::Result<SignedRelease> {
    let signed: SignedRelease = read_json(&cache.join("pending.json"))?;
    let release = signed.verify(key)?;
    if !release.is_newer_than(VERSION)? {
        return Err(io::Error::other("Cached update is no longer needed"));
    }
    release.verify_installer(&mut File::open(cache.join(release.filename()))?)?;
    Ok(signed)
}

fn check_and_stage(
    cache: &Path,
    network: bool,
    cancel: &AtomicBool,
    tx: &Sender<Event>,
) -> io::Result<Option<SignedRelease>> {
    fs::create_dir_all(cache)?;
    // Old helpers cannot remove themselves. Clean only our own, old cache files.
    cleanup_cache(cache);
    let pending = cache.join("pending.json");
    if let Ok(signed) = cached_release(cache, &public_key()?) {
        update_io::cancelled(cancel)?;
        return Ok(Some(signed));
    }
    let _ = fs::remove_file(&pending);
    if !network {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    match update_io::download(
        FEED_URL,
        &mut bytes,
        MAX_METADATA_BYTES as u64,
        cancel,
        |_| {},
    ) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    }
    let signed: SignedRelease = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    let release = signed.verify(&public_key()?)?;
    if !release.is_newer_than(VERSION)? {
        return Ok(None);
    }
    let destination = cache.join(release.filename());
    let partial = cache.join("download.partial");
    let result = (|| {
        let mut file = File::create(&partial)?;
        let mut last_percent = u64::MAX;
        update_io::download(
            &release.download_url(),
            &mut file,
            release.size,
            cancel,
            |bytes| {
                let percent = bytes * 100 / release.size;
                if percent != last_percent {
                    let _ = tx.send(Event::Progress(percent));
                    last_percent = percent;
                }
            },
        )?;
        file.sync_all()?;
        drop(file);
        release.verify_installer(&mut File::open(&partial)?)?;
        update_io::cancelled(cancel)?;
        // Replace a damaged/orphaned staged file from an interrupted check.
        match fs::remove_file(&destination) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::rename(&partial, &destination)?;
        write_json(&pending, &signed)?;
        Ok(Some(signed))
    })();
    let _ = fs::remove_file(partial);
    result
}

fn cleanup_cache(cache: &Path) {
    let Ok(entries) = fs::read_dir(cache) else {
        return;
    };
    for entry in entries.take(1000).flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("helper-")
            || name.starts_with("handoff-")
            || name.starts_with("rivet-"))
        {
            continue;
        }
        if let Ok(meta) = entry.metadata()
            && meta.is_file()
            && meta
                .modified()
                .ok()
                .and_then(|time| time.elapsed().ok())
                .is_some_and(|age| age.as_secs() > 30 * DAY)
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Handoff {
    parent: u32,
    parent_created: u64,
    installation: Installation,
    signed: SignedRelease,
    restart: bool,
}

#[derive(Serialize, Deserialize)]
struct HelperResult {
    success: bool,
    detail: String,
    #[serde(default)]
    cancelled: bool,
}

fn hidden_command(exe: &Path) -> Command {
    let mut command = Command::new(exe);
    command
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn installer_arguments(installation: &Installation, log: &Path) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = [
        "/VERYSILENT",
        "/SUPPRESSMSGBOXES",
        "/SP-",
        "/NORESTART",
        "/NOCLOSEAPPLICATIONS",
        "/NORESTARTAPPLICATIONS",
        "/RESTARTEXITCODE=3010",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    args.push(
        match installation.scope {
            Scope::User => "/CURRENTUSER",
            Scope::Machine => "/ALLUSERS",
        }
        .into(),
    );
    let mut directory = std::ffi::OsString::from("/DIR=");
    directory.push(&installation.directory);
    args.push(directory);
    if installation.scope == Scope::User {
        let mut logging = std::ffi::OsString::from("/LOG=");
        logging.push(log);
        args.push(logging);
    } else {
        // Do not ask an elevated installer to overwrite a predictable path in
        // a user-writable cache. Inno creates its own unique temporary log.
        args.push("/LOG".into());
    }
    args
}

fn require_explicit_machine_update(scope: Scope, restart: bool) -> io::Result<()> {
    if scope == Scope::Machine && !restart {
        return Err(io::Error::other(
            "System-wide updates require an explicit restart request",
        ));
    }
    Ok(())
}

fn installer_finished(code: u32) -> io::Result<()> {
    match code {
        0 => Ok(()),
        3010 => Err(io::Error::other(
            "Windows restart required to finish the update",
        )),
        _ => Err(io::Error::other(format!("Installer exited with {code}"))),
    }
}

fn apply_update(cache: &Path, request: &Handoff) -> io::Result<()> {
    require_explicit_machine_update(request.installation.scope, request.restart)?;
    let release = verified_release(&request.signed)?;
    update_io::wait_for_parent(request.parent, request.parent_created)?;
    let executable = request.installation.directory.join("rivet.exe");
    if update_io::detect_installation(&executable).as_ref() != Some(&request.installation) {
        return Err(io::Error::other("Rivet's installation has changed"));
    }
    // A manual installation after this helper was launched must never be
    // replaced by an older staged release. The helper is a copy of the source.
    let current = rivet::update_protocol::digest(
        &mut File::open(&executable)?,
        rivet::update_protocol::MAX_INSTALLER_BYTES,
    )?;
    let source = rivet::update_protocol::digest(
        &mut File::open(std::env::current_exe()?)?,
        rivet::update_protocol::MAX_INSTALLER_BYTES,
    )?;
    if current != source {
        return Err(io::Error::other(
            "Rivet was changed after this update was prepared",
        ));
    }
    // Deny writes and deletion until the installer has finished, closing the
    // verification-to-execution race. Inno reads its own executable normally.
    let path = cache.join(release.filename());
    let mut locked = OpenOptions::new().read(true).share_mode(1).open(&path)?;
    release.verify_installer(&mut locked)?;
    let arguments = installer_arguments(&request.installation, &cache.join("install.log"));
    let code = match request.installation.scope {
        Scope::User => hidden_command(&path)
            .args(&arguments)
            .status()?
            .code()
            .ok_or_else(|| io::Error::other("Installer did not return an exit code"))?
            as u32,
        // Only the authenticated installer receives administrator privileges.
        // This helper stays in the original account for results and relaunch.
        Scope::Machine => update_io::run_elevated_installer(&path, &arguments)?,
    };
    installer_finished(code)?;
    let _ = fs::remove_file(cache.join("pending.json"));
    drop(locked);
    let _ = fs::remove_file(path);
    Ok(())
}

/// Helper mode deliberately returns before logging, single-instance forwarding,
/// or any window creation, including on malformed command lines or failures.
pub fn run_helper_if_requested() -> bool {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--apply-update")) {
        return false;
    }
    let result = (|| -> io::Result<()> {
        let cache = std::path::absolute(cache_dir()?)?;
        let canonical_cache = cache.canonicalize()?;
        let path = PathBuf::from(
            args.next()
                .ok_or_else(|| io::Error::other("Missing update request"))?,
        );
        if args.next().is_some()
            || path
                .parent()
                .and_then(|dir| dir.canonicalize().ok())
                .as_ref()
                != Some(&canonical_cache)
            || !path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("handoff-"))
        {
            return Err(io::Error::other("Invalid update request path"));
        }
        let request: Handoff = read_json(&path)?;
        let result = apply_update(&cache, &request);
        let outcome = HelperResult {
            success: result.is_ok(),
            cancelled: result
                .as_ref()
                .is_err_and(|error| error.kind() == io::ErrorKind::Interrupted),
            detail: result
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_default(),
        };
        let _ = write_json(&cache.join("result.json"), &outcome);
        let _ = fs::remove_file(path);
        // Only relaunch a verified registered installation after parent exit.
        if request.restart
            && update_io::wait_for_parent(request.parent, request.parent_created).is_ok()
        {
            let executable = request.installation.directory.join("rivet.exe");
            if update_io::detect_installation(&executable).as_ref() == Some(&request.installation) {
                let _ = hidden_command(&executable).spawn();
            }
        }
        result
    })();
    if let Err(error) = result {
        crate::logging::log_error(&format!("Update helper failed: {error}"));
    }
    true
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn machine_updates_are_downloadable_but_never_automatic_on_exit() {
        let mut controller = Controller {
            installation: None,
            enabled: false,
            started: Instant::now(),
            inspected_cache: true,
            worker: None,
            ready: None,
            status: String::new(),
        };
        assert!(!controller.supported());
        controller.simulate_machine_install();
        assert!(controller.supported());
        controller.set_enabled(true);
        assert!(controller.enabled);
        controller.simulate_ready();
        assert!(controller.status().contains("approve"));
        assert!(!controller.install_on_exit());
        assert!(controller.handoff(false).is_err()); // Guard before any filesystem work.
        assert!(require_explicit_machine_update(Scope::Machine, false).is_err());
        assert!(require_explicit_machine_update(Scope::Machine, true).is_ok());
        controller.installation.as_mut().unwrap().scope = Scope::User;
        assert!(controller.install_on_exit());
        controller.set_enabled(false);
        assert!(!controller.install_on_exit());
    }

    #[test]
    fn machine_arguments_preserve_shared_scope_and_avoid_user_log_paths() {
        let installation = Installation {
            directory: PathBuf::from(r"C:\Program Files\Rivet"),
            scope: Scope::Machine,
        };
        let args = installer_arguments(&installation, Path::new(r"C:\User\updates\install.log"));
        for required in [
            "/ALLUSERS",
            "/LOG",
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/NOCLOSEAPPLICATIONS",
            "/NORESTARTAPPLICATIONS",
            r"/DIR=C:\Program Files\Rivet",
        ] {
            assert!(args.iter().any(|arg| arg == required));
        }
        assert!(
            !args
                .iter()
                .any(|arg| arg == "/CURRENTUSER" || arg.to_string_lossy().starts_with("/LOG="))
        );
        assert!(installer_finished(0).is_ok());
        assert!(installer_finished(3010).is_err());
        assert!(installer_finished(5).is_err());
    }

    #[test]
    #[ignore = "Requires target/release/rivet.exe to have been built"]
    fn release_helper_rejects_unsigned_requests_without_opening_the_app() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("Rivet").join("updates");
        fs::create_dir_all(&cache).unwrap();
        let request = Handoff {
            parent: std::process::id(),
            parent_created: update_io::process_birth().unwrap(),
            installation: Installation {
                directory: dir.path().join("never-installed"),
                scope: Scope::User,
            },
            signed: SignedRelease {
                payload: "{}".into(),
                signature: "0".repeat(128),
            },
            restart: false,
        };
        let path = cache.join("handoff-test.json");
        write_json(&path, &request).unwrap();
        let exe = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/rivet.exe");
        let mut child = hidden_command(&exe)
            .arg("--apply-update")
            .arg(&path)
            .env("LOCALAPPDATA", dir.path())
            .env("APPDATA", dir.path())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                assert!(
                    Instant::now() < deadline,
                    "Helper did not reject invalid metadata promptly"
                );
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let outcome: HelperResult = read_json(&cache.join("result.json")).unwrap();
        assert!(!outcome.success);
        assert!(outcome.detail.contains("signature"));
        assert!(!path.exists());
        assert!(!dir.path().join("Rivet/sessions/session.json").exists());
    }

    #[test]
    fn cached_updates_are_reverified_and_bounded() {
        use ed25519_dalek::{Signer, SigningKey};
        let dir = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[27; 32]);
        let (_, sha256) = rivet::update_protocol::digest(&mut &b"abc"[..], 3).unwrap();
        let release = Release {
            version: "99.0.0".into(),
            platform: "windows-x86_64".into(),
            size: 3,
            sha256,
        };
        let payload = serde_json::to_string(&release).unwrap();
        let signed = SignedRelease {
            signature: hex::encode(key.sign(payload.as_bytes()).to_bytes()),
            payload,
        };
        let manifest = dir.path().join("pending.json");
        write_json(&manifest, &signed).unwrap();
        fs::write(dir.path().join(release.filename()), b"abc").unwrap();
        assert!(cached_release(dir.path(), &key.verifying_key().to_bytes()).is_ok());
        fs::write(dir.path().join(release.filename()), b"bad").unwrap();
        assert!(cached_release(dir.path(), &key.verifying_key().to_bytes()).is_err());
        fs::write(&manifest, vec![b' '; MAX_METADATA_BYTES + 1]).unwrap();
        assert!(read_json::<SignedRelease>(&manifest).is_err());
    }

    #[test]
    fn installer_lock_blocks_replacement_until_execution_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("installer.exe");
        fs::write(&path, b"installer").unwrap();
        let locked = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        assert!(OpenOptions::new().write(true).open(&path).is_err());
        assert!(fs::remove_file(&path).is_err());
        assert!(File::open(&path).is_ok());
        drop(locked);
        assert!(fs::remove_file(&path).is_ok());
    }

    #[test]
    fn failed_automatic_checks_stay_quiet_and_manual_checks_report_inline() {
        let mut controller = Controller {
            installation: None,
            enabled: true,
            started: Instant::now(),
            inspected_cache: false,
            worker: None,
            ready: None,
            status: String::new(),
        };
        for manual in [false, true] {
            let (tx, rx) = mpsc::channel();
            controller.worker = Some(Worker {
                events: rx,
                cancel: Arc::new(AtomicBool::new(false)),
                manual,
            });
            tx.send(Event::Done(Err(io::Error::other("offline"))))
                .unwrap();
            controller.poll();
            assert_eq!(controller.status().is_empty(), !manual);
            assert!(!controller.ready());
        }
    }

    #[test]
    fn daily_checks_handle_clock_changes() {
        assert!(check_due(0, 5));
        assert!(!check_due(100, 100 + DAY - 1));
        assert!(check_due(100, 100 + DAY));
        assert!(check_due(100, 99));
    }

    #[test]
    fn disconnected_worker_does_not_leave_checks_permanently_busy() {
        let (sender, receiver) = mpsc::channel();
        drop(sender);
        let mut controller = Controller {
            installation: None,
            enabled: false,
            started: Instant::now(),
            inspected_cache: true,
            ready: None,
            status: "Checking for updates...".into(),
            worker: Some(Worker {
                events: receiver,
                cancel: Arc::new(AtomicBool::new(false)),
                manual: true,
            }),
        };
        controller.poll();
        assert!(controller.worker.is_none());
        assert!(!controller.ready());
        assert!(controller.status().contains("try later"));
    }

    #[test]
    fn cancellation_discards_late_readiness() {
        let (tx, rx) = mpsc::channel();
        let mut controller = Controller {
            installation: None,
            enabled: false,
            started: Instant::now(),
            inspected_cache: false,
            worker: None,
            ready: None,
            status: String::new(),
        };
        controller.worker = Some(Worker {
            events: rx,
            cancel: Arc::new(AtomicBool::new(false)),
            manual: false,
        });
        controller.set_enabled(false);
        tx.send(Event::Done(Ok(Some(SignedRelease {
            payload: String::new(),
            signature: String::new(),
        }))))
        .unwrap();
        controller.poll();
        assert!(!controller.ready());
        assert!(!controller.install_on_exit());
        assert!(controller.status().is_empty());
        assert!(controller.worker.is_none());
    }

    #[test]
    fn quiet_arguments_preserve_install_path_and_never_force_restart() {
        let installation = Installation {
            directory: PathBuf::from(r"C:\User Files\Rivet"),
            scope: Scope::User,
        };
        let args = installer_arguments(&installation, Path::new(r"C:\Update Cache\install.log"));
        for required in [
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/NOCLOSEAPPLICATIONS",
            "/NORESTARTAPPLICATIONS",
            "/CURRENTUSER",
            r"/DIR=C:\User Files\Rivet",
        ] {
            assert!(args.iter().any(|arg| arg == required));
        }
        assert!(
            !args
                .iter()
                .any(|arg| arg == "/CLOSEAPPLICATIONS" || arg == "/RESTARTAPPLICATIONS")
        );
    }
}
