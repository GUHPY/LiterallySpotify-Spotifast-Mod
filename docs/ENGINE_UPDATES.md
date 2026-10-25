# Engine updates and release maintenance

LiterallySpotify maintains its own interface and signed releases while integrating selected changes from [upstream Spotifast](https://github.com/crmne/spotifast). It remains one native executable.

## Version and identity

| Identity | Source of truth |
| --- | --- |
| Fork release | Cargo.toml, beginning at 0.1.0. |
| Engine anchor | engine-source.json, recording the exact upstream Git commit. |
| Windows app | Spotify.exe, Spotify Start menu shortcut and existing per-user installation path. |

The initial source came from the customized Debloat/Spotify commit recorded in engine-source.json. It identified itself as 0.13.4 and included local changes beyond upstream v0.13.3. **The initial release does not claim to contain the complete upstream v0.14.1 engine.** Version 0.14.1 is the original-client visual reference and an integration dry-run target.

## Protected boundary

`engine-policy.json` allowlists playback, authentication, API, session, audio, discovery/device files and engine dependency names.

The integration script excludes the UI tree, theme/motion code, assets, window shell, entrypoint, updater, documentation, packaging and workflows. It fingerprints protected files and checks that non-engine dependency entries remain unchanged. Shared app/model files are not automatically imported; changes at that interface need a reviewed adaptation.

A three-way patch against the recorded upstream anchor preserves local core changes when they can be reconciled. Conflicts stop automatic integration and leave a reviewable candidate. The process never resolves a conflict by replacing the custom UI with upstream files.

## Preview and integrate

Use Python 3.11+ and the repository's Rust toolchain. Start from a clean working tree. Add the public upstream remote if it is not present:

```powershell
git remote add upstream https://github.com/crmne/spotifast.git
python packaging/update-engine.py --ref v0.14.1
```

The default action only fetches and writes a plan and Cargo preview under `target/engine-sync/`. Inspect that plan; excluded upstream files are outside the import boundary.

```powershell
python packaging/update-engine.py --ref v0.14.1 --apply
```

This creates a `codex/engine-…` branch and applies only the allowed patch. The successful path checks the UI boundary, resolves/builds the resulting dependency graph and runs the declared UI/update regression gate before recording the new anchor.

If the patch or API boundary conflicts, reconcile the engine files and any required shared-code adaptation while preserving protected UI files and UI dependencies. Finish with:

```powershell
python packaging/update-engine.py --finalize
```

The script does not commit or publish automatically. Review the diff and test native login, playback, seeking, queue and devices before merging. Fingerprints and regression tests guard a defined boundary; they do not prove every runtime behaviour or arbitrary future compatibility.

The **Integrate upstream engine** Actions workflow provides the same path and opens a draft pull request only after automatic checks succeed. Failed integrations upload the available report for manual review.

## Build Windows packages

```powershell
python -m pip install -r packaging/requirements-release.txt
python -m unittest discover -s packaging -p test_maintenance.py
cargo check --locked --all-targets
python packaging/verify-ui.py
./packaging/windows/build-release.ps1
```

Inno Setup 6 is required. The build script checks repository identity and `literallyspotify VERSION` output, then produces the installer and portable ZIP in dist/. The ZIP contains the versioned directory, Spotify.exe, licence, README and literallyspotify-portable.txt.

Preserve the installer application ID, legacy marker recognition, binary name and portable-directory contract. They allow the updater to recognize existing custom installations, including `%LOCALAPPDATA%\Programs\Spotify`.

## Sign and release

The updater targets `GUHPY/LiterallySpotify-Spotifast-Mod` and embeds `assets/update-public-key.hex`. It requires an Ed25519 signature over checksums.txt before accepting an update.

The private signing key is a project publisher key, not a Spotify account key. It is never included in Git, the installer or the portable ZIP. Local .private/ files are ignored; the release workflow uses the dedicated `LITERALLYSPOTIFY_UPDATE_SIGNING_KEY` secret. Users do not provide this key or a GitHub login to download or use the app.

Local maintainer signing:

```powershell
python packaging/sign-release.py dist --key-file PATH_TO_PRIVATE_PROJECT_KEY
```

The signer checks that the key matches the embedded public key and verifies its signature. The manifest covers the installer/ZIP by SHA-256. This is the app updater's signature, not a Windows Authenticode certificate.

Set the version, commit reviewed source/documentation and tag that commit `vVERSION`. Run **Release Windows** manually with that tag. It validates tag/version identity, runs maintenance/UI checks, builds, signs and publishes:

```text
literallyspotify-vVERSION-x86_64-pc-windows-msvc-setup.exe
literallyspotify-vVERSION-x86_64-pc-windows-msvc.zip
checksums.txt
checksums.txt.sig
```

Back up the project signing key securely. Changing the embedded public key affects verification by installed versions and needs a deliberate migration.
