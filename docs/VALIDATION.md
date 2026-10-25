# Initial release validation

The 0.1.0 Windows x64 production binary was built from this fork with the locked dependency graph. The documentation gallery uses the same source with the existing demo feature enabled, plus native captures of the installed production app and the two reference clients.

| Check | Result |
| --- | --- |
| Cargo check, all targets | Passed. |
| Declared UI/update regression gate | All 39 named tests passed. |
| Maintenance tools | All seven tests passed, including a core-only three-way patch fixture, dependency conflict handling and signing verification. |
| Clippy, all targets | Completed successfully with four existing warnings. |
| Windows production build | Passed with `cargo build --release --locked --bin Spotify -j 1`. |
| Windows install | Production executable copied to the existing per-user Spotify installation with a backup; installed SHA-256 matches the release binary. |
| Update installation discovery | The fork updater identifies that installed executable as `WindowsInstaller`. |
| Inno Setup package | Installer compiled successfully. |
| Portable archive | Contains the four expected files; its Spotify.exe matches the production build. |
| Release manifest | Both asset SHA-256 hashes and the Ed25519 signature were verified locally. |
| Engine update preview | Upstream v0.14.1 fetched and a scoped plan generated without modifying working source files. |
| Visual review | Native comparison frames and 25 offline-demo views reviewed as image contact sheets; capture modes are labelled in the gallery. |

Production Spotify.exe SHA-256:

```text
a1c2b1df183215d1bea2212dc59bdd6316f7e8e01ee4e826e6b14f3e3530bb8b
```

The four Clippy warnings are existing style/complexity findings in images, search and sidebar code. This release does not claim a warning-free lint run.

The engine integration was previewed and its import boundary tested with a temporary Git fixture. The application was not upgraded wholesale to upstream v0.14.1. Future API conflicts, live playback after a core upgrade and hardware-specific integrations still require review and testing before that upgrade ships.

The native screenshots demonstrate rendered views, not pixel-perfect equivalence, performance measurements or complete official-client feature parity. macOS/Linux packages, every speaker type and every interactive menu branch were not validated for this Windows release. The Ed25519 updater signature is separate from Windows Authenticode signing.
