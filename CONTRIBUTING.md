# Contributing to LiterallySpotify

This repository maintains the Spotify-inspired interface and Windows releases of a community Spotifast mod. Use this repository's issues and pull requests for the mod; playback fixes that also affect upstream should be discussed with the original project separately.

Keep changes focused and preserve existing functionality. UI changes should include screenshots and relevant regression coverage. Preserve the Spotify.exe filename, Windows application identity, protected custom assets and updater identity.

For engine maintenance, follow [ENGINE_UPDATES.md](docs/ENGINE_UPDATES.md) and engine-policy.json. Do not replace src/ui or UI dependency versions with upstream copies. Shared API changes need review and native playback checks.

Run the checks in the README before submitting. Do not commit profiles, credentials, logs, build outputs or private signing material. Demo mode provides sample data for screenshots without a login.

This fork preserves the upstream MIT licence and contribution history. Attribute reused work and keep relevant notices intact.
