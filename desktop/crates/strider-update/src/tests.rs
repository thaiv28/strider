use super::*;
use ed25519_dalek::{Signer, SigningKey};
use std::cell::RefCell;

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}
fn pub_b64(k: &SigningKey) -> String {
    STANDARD.encode(k.verifying_key().to_bytes())
}
fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn manifest_for(k: &SigningKey, version: &str, payload: &[u8]) -> Manifest {
    let sha256 = sha(payload);
    let signature = STANDARD.encode(k.sign(&signed_message(version, &sha256)).to_bytes());
    Manifest {
        version: version.into(),
        url: "https://example.test/Strider.zip".into(),
        sha256,
        size: payload.len() as u64,
        signature,
        notes: "Fixes".into(),
        minimum_os: "13.0".into(),
    }
}
fn json(m: &Manifest) -> Vec<u8> {
    serde_json::to_vec(m).expect("serialize")
}

struct Fake {
    manifest: Vec<u8>,
    payload: Vec<u8>,
    gets: RefCell<Vec<String>>,
}
impl Transport for Fake {
    fn get(&self, url: &str) -> Result<Vec<u8>> {
        self.gets.borrow_mut().push(url.into());
        Ok(self.manifest.clone())
    }
    fn download(
        &self,
        _: &str,
        dest: &Path,
        _: u64,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<()> {
        std::fs::write(dest, &self.payload).map_err(io)?;
        progress(self.payload.len() as u64, Some(self.payload.len() as u64));
        Ok(())
    }
}
fn fake(manifest: &Manifest, payload: &[u8]) -> Fake {
    Fake {
        manifest: json(manifest),
        payload: payload.to_vec(),
        gets: RefCell::default(),
    }
}
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("strider-update-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn parses_a_manifest_and_defaults_optional_fields() {
    let k = key(1);
    let m = manifest_for(&k, "1.2.3", b"zip");
    let text = format!(
        r#"{{"version":"1.2.3","url":"{}","sha256":"{}","size":3,"signature":"{}"}}"#,
        m.url, m.sha256, m.signature
    );
    let parsed = parse_manifest(&text).expect("parses");
    assert_eq!(parsed.notes, "");
    assert_eq!(parsed.minimum_os, "");
    assert_eq!(parsed.size, 3);
}

#[test]
fn rejects_malformed_manifests() {
    assert!(matches!(
        parse_manifest("nope"),
        Err(UpdateError::Manifest(_))
    ));
    let mut m = manifest_for(&key(1), "1.0.0", b"x");
    m.version = "one".into();
    assert!(matches!(
        parse_manifest(&String::from_utf8(json(&m)).expect("utf8")),
        Err(UpdateError::Version(_))
    ));
    let mut m = manifest_for(&key(1), "1.0.0", b"x");
    m.sha256 = "abc".into();
    assert!(matches!(
        parse_manifest(&String::from_utf8(json(&m)).expect("utf8")),
        Err(UpdateError::Manifest(_))
    ));
}

#[test]
fn accepts_a_good_signature() {
    let k = key(1);
    let m = manifest_for(&k, "1.2.3", b"zip");
    assert_eq!(verify_manifest(&m, &pub_b64(&k)), Ok(()));
}

#[test]
fn rejects_the_wrong_key() {
    let m = manifest_for(&key(1), "1.2.3", b"zip");
    assert_eq!(
        verify_manifest(&m, &pub_b64(&key(2))),
        Err(UpdateError::Signature)
    );
    assert_eq!(
        verify_manifest(&m, "not base64!"),
        Err(UpdateError::Signature)
    );
    assert_eq!(verify_manifest(&m, "AAAA"), Err(UpdateError::Signature));
}

#[test]
fn rejects_a_tampered_sha_or_version() {
    let k = key(1);
    let good = manifest_for(&k, "1.2.3", b"zip");
    let mut sha_changed = good.clone();
    sha_changed.sha256 = sha(b"other");
    assert_eq!(
        verify_manifest(&sha_changed, &pub_b64(&k)),
        Err(UpdateError::Signature)
    );
    let mut version_changed = good.clone();
    version_changed.version = "9.9.9".into();
    assert_eq!(
        verify_manifest(&version_changed, &pub_b64(&k)),
        Err(UpdateError::Signature)
    );
    let mut garbage = good;
    garbage.signature = STANDARD.encode([0u8; 10]);
    assert_eq!(
        verify_manifest(&garbage, &pub_b64(&k)),
        Err(UpdateError::Signature)
    );
}

#[test]
fn compares_versions_by_semver() {
    let k = key(1);
    let m = |v: &str| manifest_for(&k, v, b"z");
    assert_eq!(is_newer("1.2.3", &m("1.2.4")), Ok(true));
    assert_eq!(is_newer("1.2.3", &m("1.2.3")), Ok(false));
    assert_eq!(is_newer("1.2.3", &m("1.2.2")), Ok(false));
    assert_eq!(is_newer("1.9.0", &m("1.10.0")), Ok(true));
    assert_eq!(is_newer("1.0.0-beta.1", &m("1.0.0")), Ok(true));
    assert!(is_newer("garbage", &m("1.0.0")).is_err());
}

#[test]
fn compares_os_versions() {
    assert!(os_at_least("14.2.1", "13.0"));
    assert!(os_at_least("13", "13.0"));
    assert!(!os_at_least("12.7", "13.0"));
}

#[test]
fn check_returns_a_newer_verified_release() {
    let k = key(1);
    let m = manifest_for(&k, "0.2.0", b"zip");
    let transport = fake(&m, b"zip");
    let config = Config::new("https://updates.test/manifest.json", &pub_b64(&k)).expect("config");
    let found = check(&transport, &config, "0.1.0", "14.0").expect("check");
    assert_eq!(found.expect("update").manifest.version, "0.2.0");
    assert_eq!(*transport.gets.borrow(), vec![config.url.clone()]);
    assert_eq!(check(&transport, &config, "0.2.0", "14.0"), Ok(None));
    assert!(matches!(
        check(&transport, &config, "0.1.0", "12.0"),
        Err(UpdateError::UnsupportedOs(_))
    ));
}

#[test]
fn check_refuses_a_manifest_signed_by_another_key() {
    let m = manifest_for(&key(2), "0.2.0", b"zip");
    let transport = fake(&m, b"zip");
    let config = Config::new("https://updates.test/m.json", &pub_b64(&key(1))).expect("config");
    assert_eq!(
        check(&transport, &config, "0.1.0", "14.0"),
        Err(UpdateError::Signature)
    );
}

#[test]
fn config_needs_https_and_a_valid_key() {
    let k = pub_b64(&key(1));
    assert!(Config::new("", &k).is_none());
    assert!(Config::new("https://x.test/m.json", "").is_none());
    assert!(Config::new("http://x.test/m.json", &k).is_none());
    assert!(Config::new("https://x.test/m.json", "AAAA").is_none());
    assert!(Config::new("https://x.test/m.json", &k).is_some());
}

#[test]
fn download_accepts_matching_size_and_hash() {
    let k = key(1);
    let m = manifest_for(&k, "0.2.0", b"payload");
    let transport = fake(&m, b"payload");
    let dir = temp("ok");
    let mut seen = 0;
    let zip = download(&transport, &Available { manifest: m }, &dir, &mut |d, _| {
        seen = d
    })
    .expect("download");
    assert_eq!(std::fs::read(&zip).expect("read"), b"payload");
    assert_eq!(seen, 7);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn download_refuses_a_size_mismatch_and_deletes_the_file() {
    let k = key(1);
    let m = manifest_for(&k, "0.2.0", b"payload");
    let transport = fake(&m, b"payload+");
    let dir = temp("size");
    let result = download(&transport, &Available { manifest: m }, &dir, &mut |_, _| {});
    assert_eq!(
        result,
        Err(UpdateError::Size {
            expected: 7,
            actual: 8
        })
    );
    assert!(!dir.join("Strider-0.2.0.zip").exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn download_refuses_a_hash_mismatch() {
    let k = key(1);
    let m = manifest_for(&k, "0.2.0", b"payload");
    let transport = fake(&m, b"PAYLOAD");
    let dir = temp("hash");
    let result = download(&transport, &Available { manifest: m }, &dir, &mut |_, _| {});
    assert_eq!(result, Err(UpdateError::Hash));
    assert!(!dir.join("Strider-0.2.0.zip").exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn download_refuses_a_plain_http_url() {
    let k = key(1);
    let mut m = manifest_for(&k, "0.2.0", b"payload");
    m.url = "http://example.test/a.zip".into();
    let transport = fake(&m, b"payload");
    let dir = temp("http");
    assert!(matches!(
        download(&transport, &Available { manifest: m }, &dir, &mut |_, _| {}),
        Err(UpdateError::Insecure(_))
    ));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn finds_the_bundle_from_an_executable_path() {
    assert_eq!(
        bundle_of(Path::new(
            "/Applications/Strider.app/Contents/MacOS/strider"
        )),
        Some(PathBuf::from("/Applications/Strider.app"))
    );
    assert_eq!(bundle_of(Path::new("/repo/target/debug/strider")), None);
}

#[test]
fn parses_team_ids() {
    assert_eq!(
        parse_team_id("Identifier=x\nTeamIdentifier=ABCDE12345\n"),
        Some("ABCDE12345".into())
    );
    assert_eq!(parse_team_id("TeamIdentifier=not set\n"), None);
    assert_eq!(parse_team_id("nothing"), None);
}

#[test]
fn identities_must_match() {
    let id = |b: &str, t: Option<&str>| Identity {
        bundle_id: b.into(),
        team_id: t.map(Into::into),
    };
    let running = id(BUNDLE_ID, Some("TEAM1"));
    assert_eq!(
        identities_match(&running, &id(BUNDLE_ID, Some("TEAM1"))),
        Ok(())
    );
    assert!(identities_match(&running, &id(BUNDLE_ID, Some("TEAM2"))).is_err());
    assert!(identities_match(&running, &id(BUNDLE_ID, None)).is_err());
    assert!(identities_match(&running, &id("other", Some("TEAM1"))).is_err());
    // Ad-hoc running builds compare the bundle id only.
    let adhoc = id(BUNDLE_ID, None);
    assert_eq!(identities_match(&adhoc, &id(BUNDLE_ID, None)), Ok(()));
    assert!(identities_match(&adhoc, &id("other", None)).is_err());
}
