//! Signed auto-updates.
//!
//! A release publishes `manifest.json` and a zip of `Strider.app`. The manifest carries an
//! ed25519 signature over `"<version>\n<sha256>"`, made with the key whose public half is baked
//! into the bundle's Info.plist (`StriderUpdatePublicKey`). The app refuses a manifest whose
//! signature does not verify, a download whose size or SHA-256 differs from the manifest, and a
//! new bundle whose identity differs from the running one.
//!
//! All network access goes through [`Transport`], so tests use a fake.
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

pub const BUNDLE_ID: &str = "dev.thaiv.strider";
/// The manifest is small; anything larger is refused.
const MAX_MANIFEST: u64 = 1 << 20;

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateError {
    NotConfigured,
    Manifest(String),
    Signature,
    Version(String),
    Insecure(String),
    Network(String),
    Size { expected: u64, actual: u64 },
    Hash,
    Io(String),
    Unpack(String),
    Identity(String),
    UnsupportedOs(String),
}
impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "Updates are not configured."),
            Self::Manifest(m) => write!(f, "The update manifest is invalid: {m}"),
            Self::Signature => write!(f, "The update signature could not be verified."),
            Self::Version(v) => write!(f, "Invalid version: {v}"),
            Self::Insecure(u) => write!(f, "Updates must use HTTPS: {u}"),
            Self::Network(m) => write!(f, "Could not reach the update server: {m}"),
            Self::Size { expected, actual } => {
                write!(f, "The download is {actual} bytes; expected {expected}.")
            }
            Self::Hash => write!(f, "The download does not match its checksum."),
            Self::Io(m) => write!(f, "File error: {m}"),
            Self::Unpack(m) => write!(f, "Could not unpack the update: {m}"),
            Self::Identity(m) => write!(f, "The update is not signed by the same publisher: {m}"),
            Self::UnsupportedOs(v) => write!(f, "This update needs macOS {v} or later."),
        }
    }
}
impl std::error::Error for UpdateError {}
pub type Result<T> = std::result::Result<T, UpdateError>;
fn io(e: std::io::Error) -> UpdateError {
    UpdateError::Io(e.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub signature: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub minimum_os: String,
}

/// The message that is signed: `"<version>\n<sha256>"`.
pub fn signed_message(version: &str, sha256: &str) -> Vec<u8> {
    format!("{version}\n{sha256}").into_bytes()
}

pub fn parse_manifest(text: &str) -> Result<Manifest> {
    let manifest: Manifest =
        serde_json::from_str(text).map_err(|e| UpdateError::Manifest(e.to_string()))?;
    parse_version(&manifest.version)?;
    if manifest.sha256.len() != 64 || !manifest.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(UpdateError::Manifest("sha256 must be 64 hex digits".into()));
    }
    if manifest.url.is_empty() || manifest.signature.is_empty() {
        return Err(UpdateError::Manifest(
            "url and signature are required".into(),
        ));
    }
    Ok(manifest)
}

fn parse_version(v: &str) -> Result<semver::Version> {
    semver::Version::parse(v).map_err(|_| UpdateError::Version(v.to_owned()))
}

pub fn public_key(base64_key: &str) -> Result<VerifyingKey> {
    let bytes = STANDARD
        .decode(base64_key.trim())
        .map_err(|_| UpdateError::Signature)?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| UpdateError::Signature)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| UpdateError::Signature)
}

/// Accept the manifest only if its signature verifies under `public_key_b64`.
pub fn verify_manifest(manifest: &Manifest, public_key_b64: &str) -> Result<()> {
    let key = public_key(public_key_b64)?;
    let bytes = STANDARD
        .decode(manifest.signature.trim())
        .map_err(|_| UpdateError::Signature)?;
    let bytes: [u8; 64] = bytes.try_into().map_err(|_| UpdateError::Signature)?;
    let signature = Signature::from_bytes(&bytes);
    key.verify_strict(
        &signed_message(&manifest.version, &manifest.sha256),
        &signature,
    )
    .map_err(|_| UpdateError::Signature)
}

/// Whether `manifest` is a newer version than `current`.
pub fn is_newer(current: &str, manifest: &Manifest) -> Result<bool> {
    Ok(parse_version(&manifest.version)? > parse_version(current)?)
}

/// Compare dotted numeric versions such as `13.0` and `14.2.1`.
pub fn os_at_least(current: &str, minimum: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim()
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    let (mut a, mut b) = (parse(current), parse(minimum));
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    a >= b
}

pub trait Transport {
    /// GET a small document (the manifest).
    fn get(&self, url: &str) -> Result<Vec<u8>>;
    /// Stream `url` into `dest`, calling `progress(done, total)`. Never writes more than
    /// `max_bytes`.
    fn download(
        &self,
        url: &str,
        dest: &Path,
        max_bytes: u64,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<()>;
}

/// HTTPS over `ureq`.
pub struct HttpTransport {
    agent: ureq::Agent,
}
impl Default for HttpTransport {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(900)))
            .https_only(true)
            .build()
            .into();
        Self { agent }
    }
}
impl Transport for HttpTransport {
    fn get(&self, url: &str) -> Result<Vec<u8>> {
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| UpdateError::Network(e.to_string()))?;
        response
            .body_mut()
            .with_config()
            .limit(MAX_MANIFEST)
            .read_to_vec()
            .map_err(|e| UpdateError::Network(e.to_string()))
    }
    fn download(
        &self,
        url: &str,
        dest: &Path,
        max_bytes: u64,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<()> {
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| UpdateError::Network(e.to_string()))?;
        let total = response.body().content_length();
        let mut reader = response.body_mut().with_config().limit(max_bytes).reader();
        let mut file = File::create(dest).map_err(io)?;
        let mut buffer = vec![0u8; 64 * 1024];
        let mut done = 0u64;
        loop {
            let n = reader
                .read(&mut buffer)
                .map_err(|e| UpdateError::Network(e.to_string()))?;
            if n == 0 {
                break;
            }
            file.write_all(&buffer[..n]).map_err(io)?;
            done += n as u64;
            progress(done, total);
        }
        file.flush().map_err(io)
    }
}

/// Where to look for updates and the key that vouches for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub url: String,
    pub public_key: String,
}
impl Config {
    /// `None` unless both values are set. The URL must be HTTPS.
    pub fn new(url: &str, public_key: &str) -> Option<Config> {
        let (url, public_key) = (url.trim(), public_key.trim());
        if url.is_empty() || public_key.is_empty() || !url.starts_with("https://") {
            return None;
        }
        public_key_ok(public_key).then(|| Config {
            url: url.into(),
            public_key: public_key.into(),
        })
    }
    /// Read the running bundle's Info.plist. `None` when not running from a `.app`.
    pub fn from_bundle() -> Option<Config> {
        let plist = running_bundle()?.join("Contents/Info.plist");
        Config::new(
            &plist_value(&plist, "StriderUpdateURL")?,
            &plist_value(&plist, "StriderUpdatePublicKey")?,
        )
    }
}
fn public_key_ok(key: &str) -> bool {
    public_key(key).is_ok()
}

/// The `.app` directory containing the running executable, if any.
pub fn running_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    bundle_of(&exe)
}
/// `<x>.app/Contents/MacOS/<bin>` gives `<x>.app`.
pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    (macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && app.extension()? == "app")
        .then(|| app.to_path_buf())
}

/// One Info.plist string, via `plutil` (always present on macOS).
pub fn plist_value(plist: &Path, key: &str) -> Option<String> {
    let output = Command::new("/usr/bin/plutil")
        .args(["-extract", key, "raw", "-o", "-"])
        .arg(plist)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// A verified, newer release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Available {
    pub manifest: Manifest,
}

/// Fetch the manifest, verify it and compare with `current`. `Ok(None)` means up to date.
pub fn check(
    transport: &dyn Transport,
    config: &Config,
    current: &str,
    os_version: &str,
) -> Result<Option<Available>> {
    if !config.url.starts_with("https://") {
        return Err(UpdateError::Insecure(config.url.clone()));
    }
    let bytes = transport.get(&config.url)?;
    let text = String::from_utf8(bytes).map_err(|e| UpdateError::Manifest(e.to_string()))?;
    let manifest = parse_manifest(&text)?;
    verify_manifest(&manifest, &config.public_key)?;
    if !is_newer(current, &manifest)? {
        return Ok(None);
    }
    if !manifest.minimum_os.is_empty() && !os_at_least(os_version, &manifest.minimum_os) {
        return Err(UpdateError::UnsupportedOs(manifest.minimum_os.clone()));
    }
    Ok(Some(Available { manifest }))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(io)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(io)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Download the release zip into `dir` and check its size and SHA-256 against the (already
/// verified) manifest. A mismatch deletes the file.
pub fn download(
    transport: &dyn Transport,
    available: &Available,
    dir: &Path,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<PathBuf> {
    let manifest = &available.manifest;
    if !manifest.url.starts_with("https://") {
        return Err(UpdateError::Insecure(manifest.url.clone()));
    }
    std::fs::create_dir_all(dir).map_err(io)?;
    let zip = dir.join(format!("Strider-{}.zip", manifest.version));
    let result = transport
        .download(&manifest.url, &zip, manifest.size, progress)
        .and_then(|()| verify_download(&zip, manifest));
    if result.is_err() {
        let _ = std::fs::remove_file(&zip);
    }
    result.map(|()| zip)
}

pub fn verify_download(zip: &Path, manifest: &Manifest) -> Result<()> {
    let actual = std::fs::metadata(zip).map_err(io)?.len();
    if actual != manifest.size {
        return Err(UpdateError::Size {
            expected: manifest.size,
            actual,
        });
    }
    if !sha256_file(zip)?.eq_ignore_ascii_case(&manifest.sha256) {
        return Err(UpdateError::Hash);
    }
    Ok(())
}

fn run(command: &mut Command, what: &str) -> Result<std::process::Output> {
    let output = command
        .output()
        .map_err(|e| UpdateError::Unpack(format!("{what}: {e}")))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(UpdateError::Unpack(format!(
            "{what}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

/// Unzip with `ditto` into `<dir>/unpacked` and return the single `.app` inside.
pub fn unpack(zip: &Path, dir: &Path) -> Result<PathBuf> {
    let out = dir.join("unpacked");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).map_err(io)?;
    run(
        Command::new("/usr/bin/ditto")
            .args(["-x", "-k"])
            .arg(zip)
            .arg(&out),
        "ditto",
    )?;
    let mut apps = std::fs::read_dir(&out)
        .map_err(io)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "app"));
    match (apps.next(), apps.next()) {
        (Some(app), None) => Ok(app),
        _ => Err(UpdateError::Unpack(
            "the archive must contain exactly one .app".into(),
        )),
    }
}

/// Identity of a signed bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub bundle_id: String,
    /// `None` for ad-hoc signatures.
    pub team_id: Option<String>,
}

/// Parse `TeamIdentifier=` from `codesign -dvv` output; "not set" is ad-hoc.
pub fn parse_team_id(codesign_output: &str) -> Option<String> {
    codesign_output
        .lines()
        .find_map(|l| l.strip_prefix("TeamIdentifier="))
        .map(str::trim)
        .filter(|t| !t.is_empty() && *t != "not set")
        .map(str::to_owned)
}

pub fn identity(app: &Path) -> Result<Identity> {
    let bundle_id = plist_value(&app.join("Contents/Info.plist"), "CFBundleIdentifier")
        .ok_or_else(|| UpdateError::Identity("no bundle identifier".into()))?;
    let output = run(
        Command::new("/usr/bin/codesign").arg("-dvv").arg(app),
        "codesign -dvv",
    )?;
    // codesign writes its description to stderr.
    let team_id = parse_team_id(&String::from_utf8_lossy(&output.stderr));
    Ok(Identity { bundle_id, team_id })
}

/// The new bundle may replace the running one only if the bundle ids match and, when the
/// running app has a Team ID, so do the Team IDs. Ad-hoc running apps check the bundle id only.
pub fn identities_match(running: &Identity, new: &Identity) -> Result<()> {
    if running.bundle_id != new.bundle_id {
        return Err(UpdateError::Identity(format!(
            "bundle id {} differs from {}",
            new.bundle_id, running.bundle_id
        )));
    }
    if running.team_id.is_some() && running.team_id != new.team_id {
        return Err(UpdateError::Identity("Team ID differs".into()));
    }
    Ok(())
}

/// `codesign --verify --deep --strict` plus the identity comparison.
pub fn verify_bundle(new_app: &Path, running_app: &Path) -> Result<Identity> {
    run(
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(new_app),
        "codesign --verify",
    )
    .map_err(|e| UpdateError::Identity(e.to_string()))?;
    let new = identity(new_app)?;
    identities_match(&identity(running_app)?, &new)?;
    Ok(new)
}

/// Hand the swap to a detached shell: wait for this process to exit, move the old bundle to a
/// temporary name, move the new one into place, relaunch with `open -n`, then delete the old.
/// If the move fails the old bundle is restored and relaunched. The caller then quits.
pub fn install_on_quit(new_app: &Path, running_app: &Path) -> Result<()> {
    const SCRIPT: &str = r#"
new="$1"; old="$2"; pid="$3"
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
tmp="${old}.old.$$"
if mv "$old" "$tmp"; then
  if mv "$new" "$old"; then
    open -n "$old"
    sleep 5
    rm -rf "$tmp"
  else
    mv "$tmp" "$old"
    open -n "$old"
  fi
else
  open -n "$old"
fi
"#;
    Command::new("/bin/sh")
        .arg("-c")
        .arg(SCRIPT)
        .arg("strider-update")
        .arg(new_app)
        .arg(running_app)
        .arg(std::process::id().to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(io)
}

/// Download, verify, unpack and check identity. Returns the new, verified bundle.
pub fn stage(
    transport: &dyn Transport,
    available: &Available,
    temp: &Path,
    running_app: &Path,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<PathBuf> {
    let zip = download(transport, available, temp, progress)?;
    let app = unpack(&zip, temp)?;
    verify_bundle(&app, running_app)?;
    Ok(app)
}

#[cfg(test)]
mod tests;
