# UI comparison and visual tour

This documents the finished LiterallySpotify interface rather than the development steps used to build it. The three native application captures below establish the comparison; the offline gallery then covers the individual views and inherited tools.

## The three versions

| | Original Spotifast | LiterallySpotify | Official Spotify |
| --- | --- | --- | --- |
| Captured version | Upstream 0.14.1 | Fork 0.1.0 | Windows Store package 1.305.3058.0 |
| Role | Native client this project builds on | The mod distributed here | Visual reference |
| Capture | Authenticated isolated portable profile | Installed production build | Authenticated official desktop app |

### Original Spotifast

[![Original Spotifast Home](screenshots/original/live-home.png)](screenshots/original/live-home.png)

### LiterallySpotify

[![LiterallySpotify production Home](screenshots/mod/live-home.png)](screenshots/mod/live-home.png)

### Official Spotify reference

[![Official Spotify Home and Now Playing](screenshots/spotify/live-home.png)](screenshots/spotify/live-home.png)

These are native application captures from 25 October 2026. Their window sizes, scroll positions, selected tracks and recommendations differ; the images are a visual comparison, not a pixel-difference measurement or catalogue-parity claim. Click an image for full resolution. Versions and capture origins are recorded in [sources.json](screenshots/sources.json).

## Matching native views

These frames use the same 1600×900 window size. Recommendations, active tracks and device availability can still differ between sessions.

<details>
<summary><strong>Liked Songs: all three clients</strong></summary>

| Original Spotifast | LiterallySpotify | Official Spotify |
| --- | --- | --- |
| [![Liked Songs — original](screenshots/original/live-liked.png)](screenshots/original/live-liked.png) | [![Liked Songs — mod](screenshots/mod/live-liked.png)](screenshots/mod/live-liked.png) | [![Liked Songs — spotify](screenshots/spotify/live-liked.png)](screenshots/spotify/live-liked.png) |

</details>

<details>
<summary><strong>Queue: all three clients</strong></summary>

| Original Spotifast | LiterallySpotify | Official Spotify |
| --- | --- | --- |
| [![Queue — original](screenshots/original/live-queue.png)](screenshots/original/live-queue.png) | [![Queue — mod](screenshots/mod/live-queue.png)](screenshots/mod/live-queue.png) | [![Queue — spotify](screenshots/spotify/live-queue.png)](screenshots/spotify/live-queue.png) |

</details>

<details>
<summary><strong>Device picker: all three clients</strong></summary>

| Original Spotifast | LiterallySpotify | Official Spotify |
| --- | --- | --- |
| [![Device picker — original](screenshots/original/live-devices.png)](screenshots/original/live-devices.png) | [![Device picker — mod](screenshots/mod/live-devices.png)](screenshots/mod/live-devices.png) | [![Device picker — spotify](screenshots/spotify/live-devices.png)](screenshots/spotify/live-devices.png) |

</details>

### Inline search in the installed mod

[![Live search suggestions](screenshots/mod/live-search-preview.png)](screenshots/mod/live-search-preview.png)

### Lyrics in the two native clients

| Original Spotifast | LiterallySpotify |
| --- | --- |
| [![Original lyrics](screenshots/original/live-lyrics.png)](screenshots/original/live-lyrics.png) | [![LiterallySpotify lyrics](screenshots/mod/live-lyrics.png)](screenshots/mod/live-lyrics.png) |

The original uses a lyrics side panel; the pictured mod uses its dedicated artwork/lyrics view. No matching official-client lyrics frame was captured in this session.

## What the mod changes

| Area | Final behaviour in LiterallySpotify |
| --- | --- |
| Desktop shell | Global search/navigation bar, library at the left, persistent bottom player and a separate Now Playing panel. |
| Home | Artwork-derived colour, shortcut tiles and horizontal shelves that extend to the content-panel boundary. Shelf controls appear for the hovered shelf. |
| Search | Inline suggestions preserve the current page; Enter opens full results. Covers support play/pause, row highlight and a subtle elastic press. Recent media is stored by identity with artwork, across restarts. |
| Now Playing | A collapsed panel previews its own content on hover over the content edge. Artist information, available credits and the next item depend on actual data. |
| Lyrics | A compact preview can expand in place; the dedicated lyrics view provides track actions and window controls. The gallery below shows the preview and the dedicated artwork/lyrics layout. |
| Menus and details | Track/playlist actions, sharing, creation dialogs, listening-activity onboarding, interface zoom and appearance controls follow the same visual direction. |
| Playback tools | Spotifast's native playback, Connect, queue, podcasts, mini player, equalizer and visualizer paths remain available. |

The feature list describes implemented behaviour; a still image cannot demonstrate every animation or persistence rule. Automated UI/update regression coverage is declared in [packaging/ui-regression-tests.txt](../packaging/ui-regression-tests.txt). Service and hardware limitations are listed in the [feature inventory](FEATURES.md).

## Complete interface gallery

The following screenshots use **the 0.1.0 source with its existing offline demo mode**. They use sample music, users and devices rather than a personal login. The demo build is used only to make documentation captures; the published installer and portable ZIP contain the production build. Most frames use 1600×900; classic-player windows use their native size. These are actual rendered app views, not design mockups.

<details>
<summary><strong>Home and recommendations</strong></summary>

Artwork colour, category chips, compact shortcuts, card shelves and the bottom player.

[![Home and recommendations — LiterallySpotify offline demo](screenshots/mod/01-home.png)](screenshots/mod/01-home.png)

</details>

<details>
<summary><strong>Liked Songs</strong></summary>

Collection header, track rows, artwork, duration and per-track controls.

[![Liked Songs — LiterallySpotify offline demo](screenshots/mod/02-liked.png)](screenshots/mod/02-liked.png)

</details>

<details>
<summary><strong>Playlist</strong></summary>

Playlist artwork and metadata, playback actions and the track table.

[![Playlist — LiterallySpotify offline demo](screenshots/mod/03-playlist.png)](screenshots/mod/03-playlist.png)

</details>

<details>
<summary><strong>Album</strong></summary>

Album identity, release information and tracks in a collection layout.

[![Album — LiterallySpotify offline demo](screenshots/mod/04-album.png)](screenshots/mod/04-album.png)

</details>

<details>
<summary><strong>Artist</strong></summary>

Artist header, popular tracks and release navigation.

[![Artist — LiterallySpotify offline demo](screenshots/mod/05-artist.png)](screenshots/mod/05-artist.png)

</details>

<details>
<summary><strong>Full search results</strong></summary>

Top result, songs, catalogue filters and artist results.

[![Full search results — LiterallySpotify offline demo](screenshots/mod/06-search.png)](screenshots/mod/06-search.png)

</details>

<details>
<summary><strong>Inline search</strong></summary>

Suggestions stay under the global search box while Home remains visible.

[![Inline search — LiterallySpotify offline demo](screenshots/mod/07-search-preview.png)](screenshots/mod/07-search-preview.png)

</details>

<details>
<summary><strong>Queue</strong></summary>

A side panel keeps the current item and upcoming tracks beside Home.

[![Queue — LiterallySpotify offline demo](screenshots/mod/08-queue.png)](screenshots/mod/08-queue.png)

</details>

<details>
<summary><strong>Connect and devices</strong></summary>

The device picker shows the active target and other available devices in the sample session.

[![Connect and devices — LiterallySpotify offline demo](screenshots/mod/09-devices.png)](screenshots/mod/09-devices.png)

</details>

<details>
<summary><strong>Listening-activity onboarding</strong></summary>

Settings guidance and neutral placeholders, without fabricated friends.

[![Listening-activity onboarding — LiterallySpotify offline demo](screenshots/mod/10-listening-activity.png)](screenshots/mod/10-listening-activity.png)

</details>

<details>
<summary><strong>Settings</strong></summary>

Account, playback and device settings in the native settings page. Additional appearance options are below the initial viewport.

[![Settings — LiterallySpotify offline demo](screenshots/mod/11-settings.png)](screenshots/mod/11-settings.png)

</details>

<details>
<summary><strong>Playlist creation</strong></summary>

Name and visibility controls in the creation dialog.

[![Playlist creation — LiterallySpotify offline demo](screenshots/mod/12-create-playlist.png)](screenshots/mod/12-create-playlist.png)

</details>

<details>
<summary><strong>Now Playing and lyrics preview</strong></summary>

Artwork, track metadata, lyrics preview and the artist card share the right panel.

[![Now Playing and lyrics preview — LiterallySpotify offline demo](screenshots/mod/13-lyrics.png)](screenshots/mod/13-lyrics.png)

</details>

<details>
<summary><strong>Dedicated lyrics view</strong></summary>

The artwork-and-lyrics layout with track actions in the top-right corner.

[![Dedicated lyrics view — LiterallySpotify offline demo](screenshots/mod/14-lyrics-expanded.png)](screenshots/mod/14-lyrics-expanded.png)

</details>

<details>
<summary><strong>Library grid</strong></summary>

An alternative library layout retains the surrounding application shell.

[![Library grid — LiterallySpotify offline demo](screenshots/mod/15-library-grid.png)](screenshots/mod/15-library-grid.png)

</details>

<details>
<summary><strong>Podcasts</strong></summary>

Show artwork and catalogue navigation.

[![Podcasts — LiterallySpotify offline demo](screenshots/mod/16-podcasts.png)](screenshots/mod/16-podcasts.png)

</details>

<details>
<summary><strong>Podcast show and episodes</strong></summary>

Show information and episode rows.

[![Podcast show and episodes — LiterallySpotify offline demo](screenshots/mod/17-podcast-show.png)](screenshots/mod/17-podcast-show.png)

</details>

<details>
<summary><strong>Keyboard shortcuts</strong></summary>

The shortcut reference displayed inside the app.

[![Keyboard shortcuts — LiterallySpotify offline demo](screenshots/mod/18-shortcuts.png)](screenshots/mod/18-shortcuts.png)

</details>

<details>
<summary><strong>Playing next</strong></summary>

The upcoming queue alongside the main page.

[![Playing next — LiterallySpotify offline demo](screenshots/mod/19-playing-next.png)](screenshots/mod/19-playing-next.png)

</details>

<details>
<summary><strong>Winamp-style mini player</strong></summary>

The inherited classic player in its own compact window.

[![Winamp-style mini player — LiterallySpotify offline demo](screenshots/mod/20-winamp.png)](screenshots/mod/20-winamp.png)

</details>

<details>
<summary><strong>Equalizer and classic playlist</strong></summary>

The inherited equalizer and playlist beside the classic player.

[![Equalizer and classic playlist — LiterallySpotify offline demo](screenshots/mod/21-equalizer.png)](screenshots/mod/21-equalizer.png)

</details>

<details>
<summary><strong>Light appearance</strong></summary>

The same layout in the light theme.

[![Light appearance — LiterallySpotify offline demo](screenshots/mod/22-light-theme.png)](screenshots/mod/22-light-theme.png)

</details>

<details>
<summary><strong>Recent playback empty state</strong></summary>

The recent-playback tab honestly shows an empty sample history.

[![Recent playback empty state — LiterallySpotify offline demo](screenshots/mod/23-recent-playback.png)](screenshots/mod/23-recent-playback.png)

</details>

<details>
<summary><strong>Saved albums</strong></summary>

Artwork-based album browsing.

[![Saved albums — LiterallySpotify offline demo](screenshots/mod/24-library-albums.png)](screenshots/mod/24-library-albums.png)

</details>

<details>
<summary><strong>Saved artists</strong></summary>

Artist browsing with circular artwork.

[![Saved artists — LiterallySpotify offline demo](screenshots/mod/25-library-artists.png)](screenshots/mod/25-library-artists.png)

</details>

## Capture scope

The production screenshot shows the installed fork. The original Spotifast and official Spotify screenshots show their real authenticated applications. The full feature gallery uses deterministic sample data so it can be regenerated without credentials. [Demo capture metadata](screenshots/mod/demo-captures.json) records each page and flag set.

Native comparisons cover Home, Liked Songs, queue and devices. The extra offline screenshots document the mod; they are not presented as matched original-client captures. The inline-search capture comes from the installed production mod.

The gallery covers the views listed above. Motion, a live Spotify login, every context-menu branch, all settings below the fold and hardware-specific playback paths require interactive testing; they are not proved by screenshots. The feature inventory also documents implemented surfaces not individually pictured here, including expandable lyrics, available credits, next-item cards, tray/media controls and visualizer integration.

[All features](FEATURES.md) · [Download](https://github.com/GUHPY/LiterallySpotify-Spotifast-Mod/releases/latest) · [Engine maintenance](ENGINE_UPDATES.md)
