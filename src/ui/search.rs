//! The Search page.

use std::sync::Arc;

use egui::{Align, CornerRadius, Layout, Rect, Sense, Vec2, pos2, vec2};

use crate::api::models::{Artist, ArtistRef, PlayableItem, SearchResults, pick_image};
use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Loadable, Page, RowContext, SearchFilter};
use crate::theme::{self, Icon};

use super::widgets::{self, TrackRow};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    if app.search.committed.is_empty() && app.search.typed_at.is_none() {
        recent(app, ui);
        return;
    }
    let typing =
        app.search.typed_at.is_some() || app.search.committed_run.is_some_and(|(_, typing)| typing);
    let displayed_id = ui.make_persistent_id(("submitted-search-results", app.user_id()));
    if !typing && let Some(results) = app.search.results.get() {
        ui.data_mut(|data| {
            data.insert_temp(
                displayed_id,
                (app.search.committed.clone(), results.clone()),
            )
        });
    }
    ui.add_space(4.0);
    let labels: Vec<_> = SearchFilter::ALL
        .iter()
        .map(|f| (*f, f.label(app.locale)))
        .collect();
    let options: Vec<_> = labels
        .iter()
        .map(|(filter, label)| (*filter, label.as_ref()))
        .collect();
    if let Some(filter) = widgets::chips(ui, &palette, &options, app.search.filter) {
        app.actions.push(Action::SetSearchFilter(filter));
    }
    ui.add_space(12.0);
    if !typing {
        let error = app
            .search
            .error
            .clone()
            .or_else(|| match &app.search.results {
                Loadable::Failed(error) => Some(error.clone()),
                _ => None,
            });
        if let Some(error) = error {
            widgets::error_row(ui, app, &error, None);
        }
    }
    let Some((query, results)) =
        ui.data(|data| data.get_temp::<(String, SearchResults)>(displayed_id))
    else {
        return;
    };
    if results.is_empty() {
        if app.search.catalogue_pending
            || app.search.playlists_pending
            || app.search.error.is_some()
        {
            return;
        }
        widgets::empty_state(
            ui,
            &palette,
            Icon::Search,
            &gettext(app.locale, "No results for “{query}”").replace("{query}", &query),
            &gettext(app.locale, "Try a different spelling or keyword."),
        );
        return;
    }
    match app.search.filter {
        SearchFilter::All => all(app, ui, &results, &query),
        SearchFilter::Songs => songs(app, ui, &results, usize::MAX),
        SearchFilter::Artists => artists_grid(app, ui, &results),
        SearchFilter::Albums => albums_grid(app, ui, &results),
        SearchFilter::Playlists => playlists_grid(app, ui, &results),
        SearchFilter::Podcasts => shows_grid(app, ui, &results),
        SearchFilter::Episodes => episodes(app, ui, &results, usize::MAX),
    }
}

fn recent(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    ui.add_space(6.0);
    if !app.network_reachable
        && app.settings.search_history.is_empty()
        && app.settings.recent_searches.is_empty()
    {
        widgets::empty_state(
            ui,
            &palette,
            Icon::WifiOff,
            &gettext(app.locale, "You're offline"),
            &gettext(
                app.locale,
                "Connect to the internet to search songs, artists, and playlists.",
            ),
        );
        return;
    }
    if app.settings.search_history.is_empty() && app.settings.recent_searches.is_empty() {
        widgets::empty_state(
            ui,
            &palette,
            Icon::Search,
            &gettext(app.locale, "Search Spotify"),
            &gettext(
                app.locale,
                "Find songs, artists, albums, playlists, and podcasts.",
            ),
        );
        return;
    }

    if !app.settings.recent_searches.is_empty() {
        let recents = app.settings.recent_searches.clone();
        ui.horizontal(|ui| {
            theme::section_title(ui, &palette, &gettext(locale, "Recent searches"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let clear_label = gettext(locale, "Clear recent searches");
                if theme::link(ui, clear_label.as_ref(), theme::bold(13.0), palette.secondary).clicked() {
                    app.actions.push(Action::ClearRecentItems);
                }
            });
        });
        ui.add_space(8.0);
        widgets::shelf(ui, &palette, app.locale, "search-recents-shelf", "", |ui| {
            for item in &recents {
                recent_search_card(app, ui, item);
            }
        });
        ui.add_space(16.0);
    } else {
        theme::section_title(ui, &palette, &gettext(app.locale, "Recent searches"));
        ui.add_space(6.0);
        let history = app.settings.search_history.clone();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for query in &history {
                let (response, forget) = theme::soft_button_dismiss(ui, &palette, Icon::Clock, query);
                if forget {
                    app.actions.push(Action::ForgetSearch(query.clone()));
                } else if response.clicked() {
                    app.actions.push(Action::Search(query.clone()));
                }
            }
        });
    }
}

fn recent_search_card(app: &mut App, ui: &mut egui::Ui, item: &crate::settings::RecentSearchItem) {
    let palette = app.palette;
    let context_here = app.playing_context_uri().as_deref() == Some(item.uri.as_str());
    let playing_here = context_here && app.believed_playing();
    let card = widgets::card(
        ui,
        app,
        item.image.as_deref(),
        &item.title,
        &item.subtitle,
        if item.is_circle {
            widgets::CardCover::portrait(playing_here)
        } else {
            widgets::CardCover::square(playing_here)
        },
    );
    let close_rect = Rect::from_center_size(
        pos2(card.response.rect.right() - 20.0, card.response.rect.top() + 20.0),
        Vec2::splat(24.0),
    );
    let over_close = ui.rect_contains_pointer(close_rect);
    if card.response.hovered() || over_close {
        let close_resp = ui.interact(
            close_rect,
            ui.id().with(("recent-card-close", &item.uri)),
            Sense::click(),
        )
        .on_hover_text(gettext(app.locale, "Remove"));
        ui.painter().circle_filled(
            close_rect.center(),
            12.0,
            egui::Color32::from_black_alpha(180),
        );
        theme::paint_icon(
            ui,
            Icon::X,
            close_rect,
            13.0,
            if close_resp.hovered() {
                palette.text
            } else {
                palette.secondary
            },
        );
        if close_resp.clicked() {
            app.actions.push(Action::ForgetRecentItem(item.uri.clone()));
            return;
        }
    }
    if card.play {
        if context_here {
            app.actions.push(Action::TogglePlay);
        } else if item.uri.starts_with("spotify:track:") {
            let is_this_playing = app.now_playing().is_some_and(|np| np.playing && np.uri == item.uri);
            if is_this_playing {
                app.actions.push(Action::TogglePlay);
            } else {
                app.actions.push(Action::PlayUris {
                    uris: vec![item.uri.clone()],
                    index: 0,
                });
            }
        } else {
            app.actions.push(Action::PlayContext {
                uri: item.uri.clone(),
                offset_uri: None,
                offset_index: None,
            });
        }
    }
    if card.clicked && !over_close {
        if item.uri.starts_with("spotify:track:") {
            app.actions.push(Action::PlayUris {
                uris: vec![item.uri.clone()],
                index: 0,
            });
        } else if let Some(page) = Page::from_uri(&item.uri) {
            app.actions.push(Action::Open(page));
        } else {
            app.actions.push(Action::Search(item.title.clone()));
        }
    }
}

fn all(app: &mut App, ui: &mut egui::Ui, results: &SearchResults, query: &str) {
    let palette = app.palette;
    let locale = app.locale;
    let query = query.to_lowercase();
    let top_artist = results
        .artists
        .as_ref()
        .and_then(|page| page.items.first())
        .filter(|artist| {
            artist.name.to_lowercase() == query
                || results.tracks.as_ref().is_none_or(|t| t.items.is_empty())
        });
    let top_art = app.art_px(TOP_RESULT_ART);
    if let Some(artist) = top_artist {
        top_result(
            app,
            ui,
            pick_image(&artist.images, top_art),
            &artist.name,
            TopResultSubtitle::Text(&gettext(locale, "Artist")),
            true,
            Some(artist.uri.clone()),
            Page::Artist(artist.id.clone()),
            false,
            |ui, app| {
                widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None);
            },
        );
    } else if let Some(track) = results.tracks.as_ref().and_then(|page| page.items.first()) {
        let page = track
            .album
            .as_ref()
            .map(|album| Page::Album(album.id.clone()))
            .unwrap_or(Page::Search);
        top_result(
            app,
            ui,
            track.image(top_art),
            &track.name,
            TopResultSubtitle::SongArtists(&track.artists),
            false,
            Some(track.uri.clone()),
            page,
            track.explicit,
            |ui, app| {
                widgets::item_menu(ui, app, &PlayableItem::Track(track.clone()), None, None);
            },
        );
    } else if let Some(album) = results.albums.as_ref().and_then(|page| page.items.first()) {
        top_result(
            app,
            ui,
            pick_image(&album.images, top_art),
            &album.name,
            TopResultSubtitle::Text(
                // Translators: {artists} is the album's artist names.
                &gettext(locale, "Album • {artists}").replace(
                    "{artists}",
                    &crate::api::models::join_names(album.artists.iter().map(|a| a.name.as_str())),
                ),
            ),
            false,
            Some(album.uri.clone()),
            Page::Album(album.id.clone()),
            false,
            |ui, app| {
                widgets::context_menu_items(ui, app, &album.uri, &album.name, None);
            },
        );
    } else if let Some(playlist) = results
        .playlists
        .as_ref()
        .and_then(|page| page.items.first())
    {
        top_result(
            app,
            ui,
            pick_image(&playlist.images, top_art),
            &playlist.name,
            TopResultSubtitle::Text(
                // Translators: {owner} is the name of the playlist's owner.
                &gettext(locale, "Playlist • {owner}").replace("{owner}", playlist.owner_name()),
            ),
            false,
            Some(playlist.uri.clone()),
            Page::Playlist(playlist.id.clone()),
            false,
            |ui, app| {
                let owned = app.user_id().is_some_and(|id| playlist.owned_by(id));
                widgets::context_menu_items(
                    ui,
                    app,
                    &playlist.uri,
                    &playlist.name,
                    owned.then_some(playlist),
                );
            },
        );
    } else if let Some(show) = results.shows.as_ref().and_then(|page| page.items.first()) {
        top_result(
            app,
            ui,
            pick_image(&show.images, top_art),
            &show.name,
            TopResultSubtitle::Text(
                // Translators: {publisher} is the podcast's publisher.
                &gettext(locale, "Podcast • {publisher}").replace("{publisher}", &show.publisher),
            ),
            false,
            Some(show.uri.clone()),
            Page::Show(show.id.clone()),
            false,
            |ui, app| {
                widgets::context_menu_items(ui, app, &show.uri, &show.name, None);
            },
        );
    }
    ui.add_space(16.0);
    songs(app, ui, results, 6);
    ui.add_space(8.0);
    shelf_artists(app, ui, results);
    shelf_albums(app, ui, results);
    shelf_playlists(app, ui, results);
    shelf_shows(app, ui, results);
    if results
        .episodes
        .as_ref()
        .is_some_and(|page| !page.items.is_empty())
    {
        theme::section_title(ui, &palette, &gettext(locale, "Episodes"));
        ui.add_space(4.0);
        episodes(app, ui, results, 4);
    }
}

enum TopResultSubtitle<'a> {
    Text(&'a str),
    SongArtists(&'a [ArtistRef]),
}

/// The top result's cover size.
const TOP_RESULT_ART: f32 = 80.0;

#[allow(clippy::too_many_arguments)]
fn top_result(
    app: &mut App,
    ui: &mut egui::Ui,
    image: Option<&str>,
    title: &str,
    subtitle: TopResultSubtitle<'_>,
    round: bool,
    play_uri: Option<String>,
    page: Page,
    explicit: bool,
    menu: impl FnOnce(&mut egui::Ui, &mut App),
) {
    let palette = app.palette;
    let mut subtitle_clicked = false;
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 112.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = ui.rect_contains_pointer(rect);
        let fill = if hovered {
            palette.surface_hover
        } else {
            palette.surface
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(theme::RADIUS), fill);
        let image_rect =
            Rect::from_min_size(rect.min + vec2(16.0, 16.0), Vec2::splat(TOP_RESULT_ART));
        widgets::paint_shadow(ui, &palette, image_rect, if round { 48.0 } else { 6.0 });
        widgets::paint_cover(
            ui,
            &palette,
            image,
            image_rect,
            if round { 48.0 } else { 6.0 },
            if round { Icon::User } else { Icon::Music },
            Some(app.backend.art()),
        );
        let text_clip = Rect::from_min_max(
            pos2(image_rect.right() + 16.0, rect.top() + 18.0),
            pos2(rect.right() - 84.0, rect.bottom()),
        );
        let painter = ui.painter().with_clip_rect(text_clip);
        crate::bidi::paint_line(
            &painter,
            text_clip.left(),
            text_clip.right(),
            text_clip.top() + 16.0,
            title,
            theme::bold(20.0),
            palette.text,
        );
        match subtitle {
            TopResultSubtitle::SongArtists(artists) => {
                let subtitle_rect = Rect::from_min_max(
                    pos2(text_clip.left(), text_clip.top() + 36.0),
                    pos2(text_clip.right(), text_clip.top() + 56.0),
                );
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(subtitle_rect)
                        .layout(Layout::left_to_right(Align::Center)),
                );
                child.set_clip_rect(subtitle_rect.intersect(ui.clip_rect()));
                child.spacing_mut().item_spacing.x = 0.0;
                theme::text(
                    &mut child,
                    // Translators: Precedes the song's artist names, which follow as links.
                    gettext(app.locale, "Song • ").as_ref(),
                    theme::regular(13.5),
                    palette.secondary,
                );
                if explicit {
                    let badge_w = 14.0;
                    let (badge_rect, _) =
                        child.allocate_exact_size(vec2(badge_w + 5.0, 14.0), Sense::hover());
                    let inner = Rect::from_center_size(
                        pos2(badge_rect.left() + 7.0, badge_rect.center().y),
                        vec2(badge_w, 12.0),
                    );
                    child.painter().rect_filled(
                        inner,
                        CornerRadius::same(2),
                        palette.secondary.gamma_multiply(0.4),
                    );
                    child.painter().text(
                        inner.center(),
                        egui::Align2::CENTER_CENTER,
                        "E",
                        theme::bold(9.0),
                        palette.text,
                    );
                }
                subtitle_clicked = widgets::artist_links(
                    &mut child,
                    app,
                    artists,
                    theme::regular(13.5),
                    palette.secondary,
                );
            }
            TopResultSubtitle::Text(subtitle) => {
                crate::bidi::paint_line(
                    &painter,
                    text_clip.left(),
                    text_clip.right(),
                    text_clip.top() + 46.0,
                    subtitle,
                    theme::regular(13.5),
                    palette.secondary,
                );
            }
        }
        if hovered && let Some(uri) = &play_uri {
            let is_this_playing = if uri.starts_with("spotify:track:") {
                app.now_playing()
                    .is_some_and(|np| np.playing && np.uri == *uri)
            } else {
                app.playing_context_uri().as_deref() == Some(uri.as_str()) && app.believed_playing()
            };
            let icon = if is_this_playing {
                Icon::PauseFilled
            } else {
                Icon::PlayFilled
            };
            let button = Rect::from_center_size(
                pos2(rect.right() - 40.0, rect.center().y),
                Vec2::splat(48.0),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(button)
                    .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
            );
            let tooltip = if is_this_playing {
                gettext(app.locale, "Pause")
            } else {
                gettext(app.locale, "Play")
            };
            if theme::circle_button(
                &mut child,
                icon,
                48.0,
                palette.accent,
                palette.accent_hover,
                palette.on_accent,
                &tooltip,
            )
            .clicked()
            {
                if is_this_playing {
                    app.actions.push(Action::TogglePlay);
                } else {
                    let sub_text = match &subtitle {
                        TopResultSubtitle::Text(s) => s.to_string(),
                        TopResultSubtitle::SongArtists(artists) => {
                            format!(
                                "{} • {}",
                                gettext(app.locale, "Song"),
                                crate::api::models::join_names(
                                    artists.iter().map(|a| a.name.as_str())
                                )
                            )
                        }
                    };
                    app.actions.push(Action::RememberRecentItem(
                        crate::settings::RecentSearchItem {
                            title: title.to_string(),
                            subtitle: sub_text,
                            image: image.map(str::to_string),
                            uri: uri.clone(),
                            explicit,
                            is_circle: round,
                            verified: false,
                        },
                    ));
                    if uri.starts_with("spotify:track:") {
                        app.actions.push(Action::PlayUris {
                            uris: vec![uri.clone()],
                            index: 0,
                        });
                    } else {
                        app.actions.push(Action::PlayContext {
                            uri: uri.clone(),
                            offset_uri: None,
                            offset_index: None,
                        });
                    }
                }
            }
        }
    }
    if response.clicked() && !subtitle_clicked {
        let sub_text = match &subtitle {
            TopResultSubtitle::Text(s) => s.to_string(),
            TopResultSubtitle::SongArtists(artists) => {
                format!(
                    "{} • {}",
                    gettext(app.locale, "Song"),
                    crate::api::models::join_names(artists.iter().map(|a| a.name.as_str()))
                )
            }
        };
        app.actions.push(Action::RememberRecentItem(
            crate::settings::RecentSearchItem {
                title: title.to_string(),
                subtitle: sub_text,
                image: image.map(str::to_string),
                uri: play_uri.clone().unwrap_or_default(),
                explicit,
                is_circle: round,
                verified: false,
            },
        ));
        if let Some(uri) = &play_uri
            && uri.starts_with("spotify:track:")
        {
            let is_this_playing = app
                .now_playing()
                .is_some_and(|np| np.playing && np.uri == *uri);
            if is_this_playing {
                app.actions.push(Action::TogglePlay);
            } else {
                app.actions.push(Action::PlayUris {
                    uris: vec![uri.clone()],
                    index: 0,
                });
            }
        } else if page != Page::Search {
            app.actions.push(Action::Open(page));
        }
    }
    egui::Popup::context_menu(&response)
        .frame(widgets::menu_frame(&palette))
        .show(|ui| menu(ui, app));
}

fn songs(app: &mut App, ui: &mut egui::Ui, results: &SearchResults, limit: usize) {
    let palette = app.palette;
    let Some(page) = &results.tracks else {
        return;
    };
    if page.items.is_empty() {
        return;
    }
    if limit == usize::MAX {
        theme::section_title(ui, &palette, &gettext(app.locale, "Songs"));
        ui.add_space(4.0);
    }
    let uris: Arc<[String]> = page
        .items
        .iter()
        .map(|track| track.uri.clone())
        .collect::<Vec<_>>()
        .into();
    let context = RowContext::Uris(Arc::clone(&uris));
    let items: Vec<PlayableItem> = page
        .items
        .iter()
        .cloned()
        .map(PlayableItem::Track)
        .collect();
    for (index, item) in items.iter().take(limit).enumerate() {
        let (resp, _pick) = widgets::track_row_response(
            ui,
            app,
            TrackRow {
                index,
                number: None,
                item,
                context: &context,
                show_cover: true,
                show_album: limit == usize::MAX,
                added_at: None,
                added_by: None,
                show_added_by: false,
                compact: limit != usize::MAX,
                thin: false,
                shift: 0.0,
                picked: false,
                picked_songs: &[],
            },
        );
        if resp.clicked() || resp.double_clicked() {
            if let PlayableItem::Track(track) = item {
                app.actions.push(Action::RememberRecentItem(
                    crate::settings::RecentSearchItem {
                        title: track.name.clone(),
                        subtitle: format!(
                            "{} • {}",
                            gettext(app.locale, "Song"),
                            crate::api::models::join_names(
                                track.artists.iter().map(|a| a.name.as_str())
                            )
                        ),
                        image: track.image(app.art_px(40.0)).map(str::to_string),
                        uri: track.uri.clone(),
                        explicit: track.explicit,
                        is_circle: false,
                        verified: false,
                    },
                ));
            }
        }
    }
}

fn artist_card(app: &mut App, ui: &mut egui::Ui, artist: &Artist) {
    let context_here = app.playing_context_uri().as_deref() == Some(artist.uri.as_str());
    let playing_here = context_here && app.believed_playing();
    let card_art = app.art_px(widgets::CARD_ART);
    let card = widgets::card(
        ui,
        app,
        pick_image(&artist.images, card_art),
        &artist.name,
        &gettext(app.locale, "Artist"),
        widgets::CardCover::portrait(playing_here),
    );
    if card.play {
        app.actions.push(Action::RememberRecentItem(crate::settings::RecentSearchItem {
            title: artist.name.clone(),
            subtitle: gettext(app.locale, "Artist").into_owned(),
            image: pick_image(&artist.images, card_art).map(str::to_string),
            uri: artist.uri.clone(),
            explicit: false,
            is_circle: true,
            verified: false,
        }));
        if context_here {
            app.actions.push(Action::TogglePlay);
        } else {
            app.actions.push(Action::PlayContext {
                uri: artist.uri.clone(),
                offset_uri: None,
                offset_index: None,
            });
        }
    }
    if card.clicked {
        app.actions.push(Action::RememberRecentItem(crate::settings::RecentSearchItem {
            title: artist.name.clone(),
            subtitle: gettext(app.locale, "Artist").into_owned(),
            image: pick_image(&artist.images, card_art).map(str::to_string),
            uri: artist.uri.clone(),
            explicit: false,
            is_circle: true,
            verified: false,
        }));
        app.actions
            .push(Action::Open(Page::Artist(artist.id.clone())));
    }
    egui::Popup::context_menu(&card.response)
        .id(ui.make_persistent_id(("search-artist-menu", &artist.uri)))
        .frame(widgets::menu_frame(&app.palette))
        .show(|ui| {
            widgets::context_menu_items(ui, app, &artist.uri, &artist.name, None);
        });
}

fn shelf_artists(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    let palette = app.palette;
    let Some(page) = &results.artists else { return };
    if page.items.is_empty() {
        return;
    }
    widgets::shelf(
        ui,
        &palette, app.locale,
        "search-artists",
        &gettext(app.locale, "Artists"),
        |ui| {
            for artist in &page.items {
                artist_card(app, ui, artist);
            }
        },
    );
}

fn artists_grid(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    let Some(page) = &results.artists else { return };
    widgets::grid(ui, |ui| {
        for artist in &page.items {
            artist_card(app, ui, artist);
        }
    });
}

fn album_card(app: &mut App, ui: &mut egui::Ui, album: &crate::api::models::Album) {
    let subtitle = format!(
        "{} • {}",
        album.year().unwrap_or(""),
        crate::api::models::join_names(album.artists.iter().map(|a| a.name.as_str()))
    );
    let context_here = app.playing_context_uri().as_deref() == Some(album.uri.as_str());
    let playing_here = context_here && app.believed_playing();
    let card_art = app.art_px(widgets::CARD_ART);
    let card = widgets::card(
        ui,
        app,
        pick_image(&album.images, card_art),
        &album.name,
        subtitle.trim_start_matches(" • "),
        widgets::CardCover::square(playing_here),
    );
    if card.play {
        app.actions.push(Action::RememberRecentItem(crate::settings::RecentSearchItem {
            title: album.name.clone(),
            subtitle: subtitle.trim_start_matches(" • ").to_string(),
            image: pick_image(&album.images, card_art).map(str::to_string),
            uri: album.uri.clone(),
            explicit: false,
            is_circle: false,
            verified: false,
        }));
        if context_here {
            app.actions.push(Action::TogglePlay);
        } else {
            app.actions.push(Action::PlayContext {
                uri: album.uri.clone(),
                offset_uri: None,
                offset_index: None,
            });
        }
    }
    if card.clicked {
        app.actions.push(Action::RememberRecentItem(crate::settings::RecentSearchItem {
            title: album.name.clone(),
            subtitle: subtitle.trim_start_matches(" • ").to_string(),
            image: pick_image(&album.images, card_art).map(str::to_string),
            uri: album.uri.clone(),
            explicit: false,
            is_circle: false,
            verified: false,
        }));
        app.actions
            .push(Action::Open(Page::Album(album.id.clone())));
    }
    egui::Popup::context_menu(&card.response)
        .id(ui.make_persistent_id(("search-album-menu", &album.uri)))
        .frame(widgets::menu_frame(&app.palette))
        .show(|ui| {
            widgets::context_menu_items(ui, app, &album.uri, &album.name, None);
        });
}

fn shelf_albums(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    let palette = app.palette;
    let Some(page) = &results.albums else { return };
    if page.items.is_empty() {
        return;
    }
    widgets::shelf(
        ui,
        &palette, app.locale,
        "search-albums",
        &gettext(app.locale, "Albums"),
        |ui| {
            for album in &page.items {
                album_card(app, ui, album);
            }
        },
    );
}

fn albums_grid(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    let Some(page) = &results.albums else { return };
    widgets::grid(ui, |ui| {
        for album in &page.items {
            album_card(app, ui, album);
        }
    });
}

fn playlist_card(app: &mut App, ui: &mut egui::Ui, playlist: &crate::api::models::Playlist) {
    let context_here = app.playing_context_uri().as_deref() == Some(playlist.uri.as_str());
    let playing_here = context_here && app.believed_playing();
    let card_art = app.art_px(widgets::CARD_ART);
    let card = widgets::card(
        ui,
        app,
        pick_image(&playlist.images, card_art),
        &playlist.name,
        // Translators: {owner} is the name of the playlist's owner.
        &gettext(app.locale, "By {owner}").replace("{owner}", playlist.owner_name()),
        widgets::CardCover::square(playing_here),
    );
    if card.play {
        app.actions.push(Action::RememberRecentItem(crate::settings::RecentSearchItem {
            title: playlist.name.clone(),
            subtitle: gettext(app.locale, "By {owner}").replace("{owner}", playlist.owner_name()),
            image: pick_image(&playlist.images, card_art).map(str::to_string),
            uri: playlist.uri.clone(),
            explicit: false,
            is_circle: false,
            verified: false,
        }));
        if context_here {
            app.actions.push(Action::TogglePlay);
        } else {
            app.actions.push(Action::PlayContext {
                uri: playlist.uri.clone(),
                offset_uri: None,
                offset_index: None,
            });
        }
    }
    if card.clicked {
        app.actions.push(Action::RememberRecentItem(crate::settings::RecentSearchItem {
            title: playlist.name.clone(),
            subtitle: gettext(app.locale, "By {owner}").replace("{owner}", playlist.owner_name()),
            image: pick_image(&playlist.images, card_art).map(str::to_string),
            uri: playlist.uri.clone(),
            explicit: false,
            is_circle: false,
            verified: false,
        }));
        app.actions
            .push(Action::Open(Page::Playlist(playlist.id.clone())));
    }
    egui::Popup::context_menu(&card.response)
        .id(ui.make_persistent_id(("search-playlist-menu", &playlist.uri)))
        .frame(widgets::menu_frame(&app.palette))
        .show(|ui| {
            let owned = app.user_id().is_some_and(|id| playlist.owned_by(id));
            widgets::context_menu_items(
                ui,
                app,
                &playlist.uri,
                &playlist.name,
                owned.then_some(playlist),
            );
        });
}

fn shelf_playlists(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    let palette = app.palette;
    let Some(page) = &results.playlists else {
        return;
    };
    if page.items.is_empty() {
        return;
    }
    widgets::shelf(
        ui,
        &palette, app.locale,
        "search-playlists",
        &gettext(app.locale, "Playlists"),
        |ui| {
            for playlist in &page.items {
                playlist_card(app, ui, playlist);
            }
        },
    );
}

fn playlists_grid(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    let Some(page) = &results.playlists else {
        return;
    };
    widgets::grid(ui, |ui| {
        for playlist in &page.items {
            playlist_card(app, ui, playlist);
        }
    });
}

fn show_card(app: &mut App, ui: &mut egui::Ui, show: &crate::api::models::Show) {
    let card_art = app.art_px(widgets::CARD_ART);
    let card = widgets::card(
        ui,
        app,
        pick_image(&show.images, card_art),
        &show.name,
        &show.publisher,
        widgets::CardCover::default(),
    );
    if card.clicked {
        app.actions.push(Action::RememberRecentItem(crate::settings::RecentSearchItem {
            title: show.name.clone(),
            subtitle: gettext(app.locale, "Podcast • {publisher}").replace("{publisher}", &show.publisher),
            image: pick_image(&show.images, card_art).map(str::to_string),
            uri: show.uri.clone(),
            explicit: false,
            is_circle: false,
            verified: false,
        }));
        app.actions.push(Action::Open(Page::Show(show.id.clone())));
    }
    egui::Popup::context_menu(&card.response)
        .id(ui.make_persistent_id(("search-show-menu", &show.uri)))
        .frame(widgets::menu_frame(&app.palette))
        .show(|ui| {
            widgets::context_menu_items(ui, app, &show.uri, &show.name, None);
        });
}

fn shelf_shows(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    let palette = app.palette;
    let Some(page) = &results.shows else { return };
    if page.items.is_empty() {
        return;
    }
    widgets::shelf(
        ui,
        &palette, app.locale,
        "search-shows",
        &gettext(app.locale, "Podcasts"),
        |ui| {
            for show in &page.items {
                show_card(app, ui, show);
            }
        },
    );
}

fn shows_grid(app: &mut App, ui: &mut egui::Ui, results: &SearchResults) {
    let Some(page) = &results.shows else { return };
    widgets::grid(ui, |ui| {
        for show in &page.items {
            show_card(app, ui, show);
        }
    });
}

fn episodes(app: &mut App, ui: &mut egui::Ui, results: &SearchResults, limit: usize) {
    let Some(page) = &results.episodes else {
        return;
    };
    for episode in page.items.iter().take(limit) {
        super::show::episode_row(app, ui, episode, None);
    }
}

#[allow(dead_code)]
fn align_right(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.with_layout(Layout::right_to_left(Align::Center), add);
}
