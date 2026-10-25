# LiterallySpotify 0.1.0

The first Windows release brings a Spotify-inspired desktop interface to Spotifast's native client, with its own update stream and a protected boundary for engine maintenance.

## New

- **Familiar desktop layout:** persistent navigation/search, library panel, artwork-coloured Home, horizontal shelves and bottom playback bar.
- **Search inside the top bar:** inline results, full results on Enter, persistent recent media with artwork, and cover play/pause with row highlight and subtle elastic feedback.
- **Richer Now Playing:** collapsed panel and hover preview, expandable lyrics, artist information, available credits and next queued item.
- **Spotify-inspired details:** collection/artist views, lyrics controls, menus, playlist creation, listening-activity onboarding, zoom and appearance settings.
- **Spotifast tools retained:** native playback, Spotify Connect, queue, podcasts, themes, keyboard controls, Winamp-style mini player, equalizer and visualizer integration.
- **Fork-owned updates:** Windows installer/portable ZIP, Ed25519-signed checksums and an engine-integration workflow that protects the UI.

[Visual comparison](https://github.com/GUHPY/LiterallySpotify-Spotifast-Mod/blob/main/docs/UI_COMPARISON.md) · [Feature inventory](https://github.com/GUHPY/LiterallySpotify-Spotifast-Mod/blob/main/docs/FEATURES.md) · [Engine updates](https://github.com/GUHPY/LiterallySpotify-Spotifast-Mod/blob/main/docs/ENGINE_UPDATES.md)

## Installation

Use the setup.exe installer or extract the ZIP and run Spotify.exe. The installer updates the custom app at `%LOCALAPPDATA%\Programs\Spotify`. No account data or private key is included; sign in through the normal app flow.

The release targets **Windows x64**. Its engine snapshot and service limits are documented in the maintenance guide. Inherited device integrations are not a claim of complete official-client parity.

## Thanks

Thanks to Carmine Paolino and the [Spotifast contributors](https://github.com/crmne/spotifast) for the native client and playback foundation. Spotify is the visual reference; LiterallySpotify is an independent community project.

[First-release source](https://github.com/GUHPY/LiterallySpotify-Spotifast-Mod/tree/v0.1.0)
