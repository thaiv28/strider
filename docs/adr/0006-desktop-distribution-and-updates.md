# Distribute Strider Desktop with Developer ID and signed self-hosted updates

Strider Desktop is given to a few trip partners as `Strider.app`, outside the
Mac App Store. Builds are signed with an Apple Developer ID Application
certificate with the hardened runtime and notarized, so Gatekeeper opens them
with a double-click. `cargo xtask release` produces the signed zip and the
update manifest.

Updates use a small self-hosted protocol instead of Sparkle. A public HTTPS
`manifest.json` lists `{ version, url, sha256, size, signature, notes,
minimum_os }`. The signature is ed25519 over `"<version>\n<sha256>"`; the
public key and the manifest URL are baked into each build's Info.plist. The
app verifies the signature, compares semver, downloads the zip, checks size and
SHA-256, verifies the unzipped bundle with `codesign` and requires the same
bundle id and Team ID (bundle id only for ad-hoc builds), then swaps the bundle
after quitting and relaunches.

Why not Sparkle: it is an Objective-C framework that needs a framework bundle,
its own signing and an appcast, and it brings a bridge into a pure Rust/GPUI
app. The protocol here is under 500 lines of Rust with the locked crates
(`ed25519-dalek`, `sha2`, `semver`, `ureq`), is unit-tested with fake
transports, and the update key is independent of Apple's, so a compromised host
cannot push code without the signing key and a compromised key still cannot
pass the Team ID check.

## Consequences

- The manifest's `url` is not covered by the signature, but the zip it points
  to must match the signed SHA-256, so redirecting it gains nothing.
- There is no rollback protection beyond semver: an attacker with the host
  cannot install an older signed release only because the version must be newer.
- Losing the update signing key requires a manual reinstall for every user.
- No delta updates, no release channels, and no automatic install without a
  click.

## Open items

- An Apple Developer account, a Developer ID Application certificate and a
  `notarytool` keychain profile do not exist yet. Until then builds are ad-hoc
  signed and need right-click > Open on other Macs.
- Updates are hosted as GitHub releases (`desktop-v<version>`) built by
  `.github/workflows/desktop-release.yml`; apps read
  `releases/latest/download/manifest.json`. Version 0.1.0 shipped with the
  earlier AWS host (`strider-releases.thaiv.dev`) baked in, so that host keeps
  serving a manifest that points at the newest GitHub release until 0.1.0 is gone.
- The update seed lives in the login Keychain (`dev.thaiv.strider.update-signing`)
  and the `STRIDER_UPDATE_SIGNING_KEY` repository secret.
- Notarization and a real end-to-end update have not been exercised.
