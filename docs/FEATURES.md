# Feature inventory

This describes the final interface and the functionality retained from Spotifast. The [visual tour](UI_COMPARISON.md) shows the corresponding views. A screenshot demonstrates a view, not every service or hardware action available from it.

## Desktop interface

| Area | Functionality |
| --- | --- |
| Application shell | Persistent navigation/search bar, profile and notifications, window controls, custom title bar, resizable layout and fixed bottom player. |
| Home | Artwork-coloured background, category chips, playlist shortcuts, recommendations, recently played items and horizontal shelves with independent hover controls. |
| Library | Playlists, liked songs, albums, artists and podcasts; library search and sorting; list/grid layouts; adjustable library width. |
| Collections | Playlist and album headers, artwork, track lists, sorting/filtering and actions appropriate to the user's permissions. |
| Artist | Artist overview, popular tracks, releases and catalogue navigation where data is available. |
| Podcasts | Show pages, episode lists, episode playback and saved episodes. |
| Search preview | Results in the top-bar dropdown without navigating away; keyboard selection; Enter for a full search. |
| Search results | Top result, tracks and catalogue categories, with navigation into artists, albums, playlists and shows. |
| Recent searches | Persisted media selections with artwork; selecting a song replaces its text query in recent history. Unselected queries can remain as text history. |
| Search playback | Whole-row highlight, darkened cover, cover play/pause, subtle elastic press and play/pause icon transition. |
| Player | Previous/next, play/pause, seek, volume, shuffle/repeat, metadata and shortcuts to lyrics, queue and devices. |
| Now Playing | Artwork and track actions, collapsed panel with hover preview, lyrics preview, artist card, available credits and next item. |
| Lyrics | Synchronized lines when provided, seek by line, expanded preview, dedicated view and track context actions. |
| Context menus | Playlist/track actions, add to playlist, queue, artist/album navigation, credits and sharing, depending on the item and service support. |
| Playlist creation | Creation dialog and the existing editing workflow; folder/collaboration entries follow underlying capability limits. |
| Listening activity | Spotify-inspired onboarding/settings presentation. The mod does not invent friends or listening activity. |
| Appearance | Light/dark/system appearance, theme colours, interface zoom/density and title-bar preference. |
| Input | Keyboard shortcuts, accessible labels and reduced-motion handling for search-cover feedback. |

## Inherited playback and tools

| Area | Functionality and scope |
| --- | --- |
| Playback | Native playback through Spotifast's librespot integration; existing account/service restrictions apply. |
| Spotify Connect | Discovery, local/remote target selection and active-session controls. |
| Other devices | Existing Google Cast and Sonos paths; hardware compatibility is inherited, not newly certified by the mod. |
| Queue | Current/upcoming items, manual queue actions, recent playback and existing ordering rules. |
| Winamp-style mini player | Classic skins, playlist window, shaded layouts and built-in skin. |
| Audio controls | Existing equalizer, presets and sound-display options. |
| Visualizer | Existing MilkDrop integration, subject to platform/runtime support. |
| Windows | Spotify.exe, Start menu shortcut, media controls, tray and existing autostart preference. |
| Updates | Fork-owned signed releases, installer/portable packages and protected upstream-engine integration. |

## Service boundaries

A familiar interface does not imply complete official-client feature parity. Friend activity, concerts, video features, exact editorial recommendations and some collaboration/folder operations depend on APIs available to Spotifast. Missing data is omitted or shown as honest onboarding/empty states.

Duplicate related-video banners do not fill the Now Playing panel. Artist information, credits, lyrics and next-item cards depend on real data availability. See [What Spotify allows](_reference/what-spotify-allows.md), [connection architecture](_reference/how-it-connects.md) and [queue rules](_reference/queue.md).

Version 0.1.0 packages Windows x64. Source retains upstream platform code; this release does not certify macOS, Linux, every audio device or every speaker integration.
