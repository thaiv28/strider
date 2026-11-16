//! Build tasks: `cargo xtask bundle | sign | notarize | release | keygen`.
//!
//! See `desktop/README.md` ("Distribution") for the environment variables each task reads.
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, SigningKey};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

type Result<T> = std::result::Result<T, String>;

const BUNDLE_ID: &str = "dev.thaiv.strider";
const MINIMUM_OS: &str = "13.0";
const ICON_SVG: &str = "../app/icon.svg";
/// Where releases are hosted (the `ThaivStriderReleases` stack in project-platform-infrastructure).
const DEFAULT_UPDATE_URL: &str = "https://strider-releases.thaiv.dev/manifest.json";
const RELEASES_STACK: &str = "ThaivStriderReleases";
const RELEASES_REGION: &str = "us-west-2";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    let result = match command.as_str() {
        "bundle" => has_release(&rest).and_then(bundle).map(|_| ()),
        "sign" => sign(),
        "notarize" => notarize(),
        "release" => release(),
        "keygen" => keygen(),
        "publish" => publish(),
        _ => Err(
            "usage: cargo xtask bundle [--release] | sign | notarize | release | publish | keygen"
                .into(),
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask {command}: {message}");
            ExitCode::FAILURE
        }
    }
}

fn has_release(args: &[String]) -> Result<bool> {
    match args {
        [] => Ok(false),
        [flag] if flag == "--release" => Ok(true),
        _ => Err("only --release is accepted".into()),
    }
}

/// `desktop/`, the workspace root.
fn root() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    path.canonicalize().unwrap_or(path)
}
fn target() -> PathBuf {
    root().join("target")
}
fn app_path() -> PathBuf {
    target().join("bundle/Strider.app")
}
fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn run(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .map_err(|e| format!("{:?}: {e}", command.get_program()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{:?} exited with {status}", command.get_program()))
    }
}

/// The version of `strider-app`, read from its Cargo.toml.
fn app_version() -> Result<String> {
    let text = fs::read_to_string(root().join("crates/strider-app/Cargo.toml"))
        .map_err(|e| format!("strider-app/Cargo.toml: {e}"))?;
    text.lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("version = \""))
        .and_then(|v| v.split('"').next())
        .map(str::to_owned)
        .ok_or_else(|| "no version in strider-app/Cargo.toml".into())
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn info_plist(version: &str, update_url: &str, update_key: &str) -> String {
    let (version, update_url, update_key) = (
        xml_escape(version),
        xml_escape(update_url),
        xml_escape(update_key),
    );
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>strider</string>
	<key>CFBundleIconFile</key>
	<string>AppIcon</string>
	<key>CFBundleIdentifier</key>
	<string>{BUNDLE_ID}</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>Strider</string>
	<key>CFBundleDisplayName</key>
	<string>Strider</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>{version}</string>
	<key>CFBundleVersion</key>
	<string>{version}</string>
	<key>LSMinimumSystemVersion</key>
	<string>{MINIMUM_OS}</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>NSPrincipalClass</key>
	<string>NSApplication</string>
	<key>CFBundleURLTypes</key>
	<array>
		<dict>
			<key>CFBundleURLName</key>
			<string>{BUNDLE_ID}</string>
			<key>CFBundleURLSchemes</key>
			<array>
				<string>strider</string>
			</array>
		</dict>
	</array>
	<key>CFBundleDocumentTypes</key>
	<array>
		<dict>
			<key>CFBundleTypeName</key>
			<string>GPX Route</string>
			<key>CFBundleTypeRole</key>
			<string>Viewer</string>
			<key>LSHandlerRank</key>
			<string>Alternate</string>
			<key>LSItemContentTypes</key>
			<array>
				<string>com.topografix.gpx</string>
			</array>
		</dict>
	</array>
	<key>UTImportedTypeDeclarations</key>
	<array>
		<dict>
			<key>UTTypeIdentifier</key>
			<string>com.topografix.gpx</string>
			<key>UTTypeDescription</key>
			<string>GPS Exchange Format</string>
			<key>UTTypeConformsTo</key>
			<array>
				<string>public.xml</string>
			</array>
			<key>UTTypeTagSpecification</key>
			<dict>
				<key>public.filename-extension</key>
				<array>
					<string>gpx</string>
				</array>
			</dict>
		</dict>
	</array>
	<key>StriderUpdateURL</key>
	<string>{update_url}</string>
	<key>StriderUpdatePublicKey</key>
	<string>{update_key}</string>
</dict>
</plist>
"#
    )
}

/// The (file name, pixel size) pairs `iconutil` expects in an `.iconset`.
const ICONSET: [(&str, u32); 10] = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
];

fn render_icon(svg: &[u8], pixels: u32) -> Result<Vec<u8>> {
    let tree = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default())
        .map_err(|e| format!("icon.svg: {e}"))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixels, pixels).ok_or("icon size")?;
    let scale = pixels as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|e| e.to_string())
}

fn make_icns(dest: &Path, scratch: &Path) -> Result<()> {
    let svg = fs::read(root().join(ICON_SVG)).map_err(|e| format!("app/icon.svg: {e}"))?;
    let iconset = scratch.join("AppIcon.iconset");
    let _ = fs::remove_dir_all(&iconset);
    fs::create_dir_all(&iconset).map_err(|e| e.to_string())?;
    for (name, pixels) in ICONSET {
        fs::write(iconset.join(name), render_icon(&svg, pixels)?).map_err(|e| e.to_string())?;
    }
    run(Command::new("/usr/bin/iconutil")
        .args(["-c", "icns", "-o"])
        .arg(dest)
        .arg(&iconset))
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            fs::copy(&src, &dst).map_err(|e| format!("{}: {e}", src.display()))?;
        }
    }
    Ok(())
}

/// Build `strider` and assemble `target/bundle/Strider.app`. Returns the bundle path.
fn bundle(release: bool) -> Result<PathBuf> {
    let mut build = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    build
        .current_dir(root())
        .args(["build", "-p", "strider-app"]);
    if release {
        build.arg("--release");
    }
    run(&mut build)?;
    let binary = target()
        .join(if release { "release" } else { "debug" })
        .join("strider");
    let app = app_path();
    let _ = fs::remove_dir_all(&app);
    let (macos, resources) = (app.join("Contents/MacOS"), app.join("Contents/Resources"));
    fs::create_dir_all(&macos).map_err(|e| e.to_string())?;
    fs::create_dir_all(&resources).map_err(|e| e.to_string())?;
    fs::copy(&binary, macos.join("strider")).map_err(|e| format!("{}: {e}", binary.display()))?;
    let version = app_version()?;
    let plist = info_plist(&version, &update_url(), &update_pubkey());
    let plist_path = app.join("Contents/Info.plist");
    fs::write(&plist_path, plist).map_err(|e| e.to_string())?;
    run(Command::new("/usr/bin/plutil")
        .arg("-lint")
        .arg(&plist_path))?;
    fs::write(app.join("Contents/PkgInfo"), "APPL????").map_err(|e| e.to_string())?;
    make_icns(&resources.join("AppIcon.icns"), &target())?;
    copy_dir(&root().join("assets"), &resources.join("assets"))?;
    println!("Bundled {} ({version})", app.display());
    Ok(app)
}

fn require_bundle() -> Result<PathBuf> {
    let app = app_path();
    if app.exists() {
        Ok(app)
    } else {
        Err("no bundle; run `cargo xtask bundle` first".into())
    }
}

fn codesign(path: &Path, identity: Option<&str>, entitlements: &Path) -> Result<()> {
    let mut command = Command::new("/usr/bin/codesign");
    command.args(["--force", "--options", "runtime"]);
    match identity {
        Some(_) => command.arg("--timestamp"),
        None => command.arg("--timestamp=none"),
    };
    command
        .arg("--entitlements")
        .arg(entitlements)
        .arg("--sign")
        .arg(identity.unwrap_or("-"))
        .arg(path);
    run(&mut command)
}

fn sign() -> Result<()> {
    let app = require_bundle()?;
    let entitlements = root().join("packaging/Strider.entitlements");
    let identity = env_var("STRIDER_SIGN_IDENTITY");
    if identity.is_none() {
        println!(
            "STRIDER_SIGN_IDENTITY is not set: signing ad hoc (Gatekeeper will not trust this build)."
        );
    }
    // Inner binaries first, then the bundle with --deep.
    codesign(
        &app.join("Contents/MacOS/strider"),
        identity.as_deref(),
        &entitlements,
    )?;
    let mut deep = Command::new("/usr/bin/codesign");
    deep.args(["--force", "--deep", "--options", "runtime"]);
    match identity {
        Some(_) => deep.arg("--timestamp"),
        None => deep.arg("--timestamp=none"),
    };
    deep.arg("--entitlements")
        .arg(&entitlements)
        .arg("--sign")
        .arg(identity.as_deref().unwrap_or("-"))
        .arg(&app);
    run(&mut deep)?;
    run(Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&app))?;
    println!("Signed and verified {}", app.display());
    Ok(())
}

fn zip_app(app: &Path, zip: &Path) -> Result<()> {
    if let Some(parent) = zip.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let _ = fs::remove_file(zip);
    run(Command::new("/usr/bin/ditto")
        .args(["-c", "-k", "--keepParent"])
        .arg(app)
        .arg(zip))
}

/// Returns false (after saying why) when no profile is configured.
fn notarize_if_configured() -> Result<bool> {
    let Some(profile) = env_var("STRIDER_NOTARY_PROFILE") else {
        println!(
            "Not notarizing: STRIDER_NOTARY_PROFILE is not set. Create a profile with\n  xcrun notarytool store-credentials <name>\nand export its name."
        );
        return Ok(false);
    };
    let app = require_bundle()?;
    let zip = target().join("notarize/Strider.zip");
    zip_app(&app, &zip)?;
    run(Command::new("/usr/bin/xcrun")
        .args(["notarytool", "submit"])
        .arg(&zip)
        .args(["--keychain-profile", &profile, "--wait"]))?;
    run(Command::new("/usr/bin/xcrun")
        .args(["stapler", "staple"])
        .arg(&app))?;
    run(Command::new("/usr/sbin/spctl")
        .args(["-a", "-vv"])
        .arg(&app))?;
    Ok(true)
}

fn notarize() -> Result<()> {
    notarize_if_configured().map(|_| ())
}

fn sha256_hex(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    Ok(Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Keychain item holding the release signing seed (`security add-generic-password -s ...`).
const SIGNING_KEYCHAIN_SERVICE: &str = "dev.thaiv.strider.update-signing";

/// The public key baked into builds: `STRIDER_UPDATE_PUBKEY`, else the committed
/// `packaging/update-public-key.txt`.
fn update_pubkey() -> String {
    env_var("STRIDER_UPDATE_PUBKEY").unwrap_or_else(|| {
        fs::read_to_string(root().join("packaging/update-public-key.txt"))
            .map(|s| s.trim().to_owned())
            .unwrap_or_default()
    })
}

/// The signing key from `STRIDER_UPDATE_SIGNING_KEY` (base64 32-byte seed), else from the
/// login Keychain item `dev.thaiv.strider.update-signing`, if either exists.
fn signing_key() -> Result<Option<SigningKey>> {
    let text = match env_var("STRIDER_UPDATE_SIGNING_KEY") {
        Some(text) => text,
        None => {
            let out = Command::new("/usr/bin/security")
                .args([
                    "find-generic-password",
                    "-s",
                    SIGNING_KEYCHAIN_SERVICE,
                    "-w",
                ])
                .output()
                .map_err(|e| format!("security: {e}"))?;
            if !out.status.success() {
                return Ok(None);
            }
            String::from_utf8_lossy(&out.stdout).trim().to_owned()
        }
    };
    let seed: [u8; 32] = STANDARD
        .decode(text.trim())
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or("STRIDER_UPDATE_SIGNING_KEY must be a base64 32-byte seed")?;
    let key = SigningKey::from_bytes(&seed);
    let public = update_pubkey();
    if !public.is_empty() && public != STANDARD.encode(key.verifying_key().to_bytes()) {
        return Err("STRIDER_UPDATE_SIGNING_KEY does not match STRIDER_UPDATE_PUBKEY".into());
    }
    Ok(Some(key))
}

/// The zip's public URL: `STRIDER_RELEASE_URL`, else next to the manifest URL.
fn zip_url(zip_name: &str) -> String {
    if let Some(url) = env_var("STRIDER_RELEASE_URL") {
        return url;
    }
    match update_url().rsplit_once('/') {
        Some((base, _)) => format!("{base}/{zip_name}"),
        None => zip_name.to_owned(),
    }
}

/// The manifest URL baked into builds: `STRIDER_UPDATE_URL`, else the AWS releases host.
/// Updates stay off in a build without `STRIDER_UPDATE_PUBKEY`, whatever the URL.
fn update_url() -> String {
    env_var("STRIDER_UPDATE_URL").unwrap_or_else(|| DEFAULT_UPDATE_URL.to_owned())
}

fn aws_output(stack_output: &str) -> Result<String> {
    let out = Command::new("aws")
        .args([
            "cloudformation",
            "describe-stacks",
            "--region",
            RELEASES_REGION,
            "--stack-name",
            RELEASES_STACK,
        ])
        .args([
            "--query",
            &format!("Stacks[0].Outputs[?OutputKey=='{stack_output}'].OutputValue"),
            "--output",
            "text",
        ])
        .output()
        .map_err(|e| format!("aws: {e} (install the AWS CLI and sign in, e.g. `aws sso login`)"))?;
    let value = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if !out.status.success() || value.is_empty() || value == "None" {
        return Err(format!(
            "could not read {stack_output} from the {RELEASES_STACK} stack: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(value)
}

/// Uploads the artifacts `release` wrote: the zip first (immutable), then the manifest
/// (no-cache), then invalidates `/manifest.json`, so clients never see a manifest whose zip
/// is missing. Uses the caller's AWS session.
fn publish() -> Result<()> {
    let version = app_version()?;
    let out = target().join("release-artifacts");
    let name = format!("Strider-{version}.zip");
    let (zip, manifest) = (out.join(&name), out.join("manifest.json"));
    for path in [&zip, &manifest] {
        if !path.exists() {
            return Err(format!(
                "{} is missing; run `cargo xtask release` with STRIDER_UPDATE_SIGNING_KEY set",
                path.display()
            ));
        }
    }
    let manifest_text = fs::read_to_string(&manifest).map_err(|e| e.to_string())?;
    let parsed: serde_json::Value =
        serde_json::from_str(&manifest_text).map_err(|e| e.to_string())?;
    if parsed["version"] != version.as_str() {
        return Err(format!(
            "manifest.json is for {}, not {version}",
            parsed["version"]
        ));
    }
    let bucket = aws_output("DeploymentBucket")?;
    let distribution = aws_output("CloudFrontDistributionId")?;
    let zip_target = format!("s3://{bucket}/current/{name}");
    run(Command::new("aws")
        .args(["s3", "cp"])
        .arg(&zip)
        .arg(&zip_target)
        .args([
            "--content-type",
            "application/zip",
            "--cache-control",
            "public, max-age=31536000, immutable",
        ]))?;
    run(Command::new("aws")
        .args(["s3", "cp"])
        .arg(&manifest)
        .arg(format!("s3://{bucket}/current/manifest.json"))
        .args([
            "--content-type",
            "application/json",
            "--cache-control",
            "no-cache",
        ]))?;
    run(Command::new("aws").args([
        "cloudfront",
        "create-invalidation",
        "--distribution-id",
        &distribution,
        "--paths",
        "/manifest.json",
    ]))?;
    println!("Published Strider {version} to {}", update_url());
    Ok(())
}

fn manifest(
    key: &SigningKey,
    version: &str,
    url: &str,
    sha256: &str,
    size: u64,
    notes: &str,
) -> String {
    let signature = key.sign(format!("{version}\n{sha256}").as_bytes());
    let value = serde_json::json!({
        "version": version,
        "url": url,
        "sha256": sha256,
        "size": size,
        "signature": STANDARD.encode(signature.to_bytes()),
        "notes": notes,
        "minimum_os": MINIMUM_OS,
    });
    format!("{value:#}\n")
}

fn release() -> Result<()> {
    let key = signing_key()?;
    let version = app_version()?;
    let app = bundle(true)?;
    sign()?;
    notarize_if_configured()?;
    let out = target().join("release-artifacts");
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let name = format!("Strider-{version}.zip");
    let zip = out.join(&name);
    zip_app(&app, &zip)?;
    println!("Wrote {}", zip.display());
    let Some(key) = key else {
        println!(
            "Skipping manifest.json: STRIDER_UPDATE_SIGNING_KEY is not set (see `cargo xtask keygen`)."
        );
        return Ok(());
    };
    let size = fs::metadata(&zip).map_err(|e| e.to_string())?.len();
    let text = manifest(
        &key,
        &version,
        &zip_url(&name),
        &sha256_hex(&zip)?,
        size,
        &env_var("STRIDER_RELEASE_NOTES").unwrap_or_default(),
    );
    let path = out.join("manifest.json");
    fs::write(&path, text).map_err(|e| e.to_string())?;
    println!("Wrote {}", path.display());
    Ok(())
}

fn keygen() -> Result<()> {
    let mut seed = [0u8; 32];
    rand::rng().fill(&mut seed);
    let key = SigningKey::from_bytes(&seed);
    println!("STRIDER_UPDATE_SIGNING_KEY={}", STANDARD.encode(seed));
    println!(
        "STRIDER_UPDATE_PUBKEY={}",
        STANDARD.encode(key.verifying_key().to_bytes())
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_signature_covers_version_and_hash() {
        let key = SigningKey::from_bytes(&[3u8; 32]);
        let text = manifest(
            &key,
            "1.0.0",
            "https://x.test/a.zip",
            &"ab".repeat(32),
            5,
            "n",
        );
        let value: serde_json::Value = serde_json::from_str(&text).expect("json");
        let signature: [u8; 64] = STANDARD
            .decode(value["signature"].as_str().expect("signature"))
            .expect("base64")
            .try_into()
            .expect("64 bytes");
        let message = format!("1.0.0\n{}", "ab".repeat(32));
        let signature = ed25519_dalek::Signature::from_bytes(&signature);
        assert!(
            key.verifying_key()
                .verify_strict(message.as_bytes(), &signature)
                .is_ok()
        );
        for field in [
            "version",
            "url",
            "sha256",
            "size",
            "signature",
            "notes",
            "minimum_os",
        ] {
            assert!(value.get(field).is_some(), "{field}");
        }
    }

    #[test]
    fn plist_escapes_values_and_names_the_scheme() {
        let plist = info_plist("1.2.3", "https://x.test/m.json?a=1&b=2", "KEY");
        assert!(plist.contains("a=1&amp;b=2"));
        assert!(plist.contains("<string>strider</string>"));
        assert!(plist.contains("com.topografix.gpx"));
    }
}
