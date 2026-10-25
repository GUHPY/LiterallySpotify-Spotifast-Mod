//! The words of the playing track, in a side panel that follows the song.

use egui::{Align, Color32, Frame, Layout, Margin, Rect, Sense, UiBuilder, Vec2, pos2, vec2};

use crate::app::{App, NowPlaying};
use crate::i18n::{gettext, pgettext};
use crate::model::{Action, Loadable};
use crate::theme::{self, Icon};

use super::widgets;
/// Where the line being sung sits, as a fraction of the visible lyrics from
/// the top: high up, so the lines to come fill most of the view.
const SUNG_LINE_AT: f32 = 0.2;

/// Scrolls so the middle of `line` sits `SUNG_LINE_AT` of the way down the
/// visible lyrics.
fn show_sung_line(ui: &egui::Ui, line: Rect, animation: Option<egui::style::ScrollAnimation>) {
    let above = (ui.clip_rect().height() * SUNG_LINE_AT - line.height() / 2.0).max(0.0);
    let target = Rect::from_min_max(pos2(line.left(), line.top() - above), line.max);
    match animation {
        Some(animation) => ui.scroll_to_rect_animation(target, Some(Align::Min), animation),
        None => ui.scroll_to_rect(target, Some(Align::Min)),
    }
}
/// How long a line takes to light up or fade.
const LIGHT_UP_SECONDS: f32 = 0.22;

fn blend(from: egui::Color32, to: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from(egui::Rgba::from(from) * (1.0 - t) + egui::Rgba::from(to) * t)
}

#[allow(dead_code)]
pub fn side_panel(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let fit = super::yielding_panel(
        ui.ctx(),
        "lyrics-panel",
        theme::SIDE_PANEL_MIN_WIDTH..=420.0,
        app.settings.lyrics_width,
        ui.available_width() - super::topbar::least_width(ui.ctx()),
    );
    let panel = egui::Panel::right("lyrics-panel")
        .resizable(true)
        .default_size(app.settings.lyrics_width)
        .size_range(fit.range.clone())
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(palette.window)
                .inner_margin(Margin {
                    left: 0,
                    right: theme::PANEL_GAP as i8,
                    top: 0,
                    bottom: 0,
                }),
        );
    let response = panel.show(ui, |ui| {
        let card_rect = ui.available_rect_before_wrap();
        Frame::new()
            .fill(palette.panel)
            .corner_radius(egui::CornerRadius::same(theme::RADIUS))
            .inner_margin(Margin::ZERO)
            .show(ui, |ui| {
                ui.set_min_height((card_rect.height() - 2.0).max(0.0));
                now_playing_view_content(app, ui);
            });
    });
    let current_width = response.response.rect.width();
    if (app.settings.lyrics_width - current_width).abs() > 1.0
        && super::panel_width_chosen(ui.ctx(), "lyrics-panel", &fit)
    {
        app.settings.lyrics_width = current_width;
        app.actions.push(Action::SettingsChanged);
    }
}

pub fn now_playing_view_content(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let card_w = ui.available_width();
    let pad = 16.0;

    // Track mouse hover over upper hero area (only when panel is active/enabled, not in collapsed preview)
    let hero_h = 320.0;
    let hero_rect = Rect::from_min_size(ui.cursor().min, vec2(card_w, hero_h));
    let hero_hovered = ui.is_enabled() && ui.rect_contains_pointer(hero_rect);
    let hover_t = if ui.is_enabled() {
        ui.ctx().animate_bool_with_time_and_easing(
            egui::Id::new("npv-hero-hover"),
            hero_hovered,
            0.22,
            egui::emath::easing::cubic_out,
        )
    } else {
        0.0
    };

    // 1. Cover Art fills upper area edge-to-edge
    if let Some(now) = app.now_playing() {
        let (rect, _) = ui.allocate_exact_size(vec2(card_w, hero_h), Sense::hover());
        // Clip to top rounded corners of the bento card
        let clip_radius = egui::CornerRadius {
            nw: theme::RADIUS,
            ne: theme::RADIUS,
            sw: 0,
            se: 0,
        };
        // Paint cover art across rect
        widgets::playing_cover(
            ui,
            &palette,
            app.backend.art(),
            "npv-cover-hero",
            now.art_url.as_deref().or(now.art_small.as_deref()),
            rect,
            theme::RADIUS as f32,
        );

        // Exposure overlay: dimmed when not hovered (higher alpha), lifted when hovered (lower alpha)
        let base_alpha = (140.0 * (1.0 - hover_t) + 30.0 * hover_t) as u8;
        ui.painter()
            .rect_filled(rect, clip_radius, Color32::from_black_alpha(base_alpha));

        // Soft Top Vignette for buttons
        widgets::paint_vertical_gradient(
            ui,
            Rect::from_min_size(rect.min, vec2(rect.width(), 70.0)),
            Color32::from_black_alpha(150),
            Color32::TRANSPARENT,
        );

        // Soft Bottom Fade Out into palette.panel (#121212) before "Prévia da letra"
        let fade_h = 95.0;
        let fade_rect = Rect::from_min_size(
            pos2(rect.left(), rect.bottom() - fade_h),
            vec2(rect.width(), fade_h),
        );
        widgets::paint_vertical_gradient(ui, fade_rect, Color32::TRANSPARENT, palette.panel);

        // Disabled previews keep the same content while their controls stay inert.
        {
            let header_rect = Rect::from_min_size(
                pos2(rect.left() + pad, rect.top() + 12.0),
                vec2(rect.width() - pad * 2.0, 32.0),
            );
            let mut header_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(header_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            header_ui.spacing_mut().item_spacing.x = 0.0;

            let context_title = if let Some(uri) = app.playing_context_uri() {
                if uri.ends_with(":collection") {
                    gettext(app.locale, "Liked Songs").into_owned()
                } else if let Some(id) = uri.strip_prefix("spotify:playlist:") {
                    app.known_playlist(id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| gettext(app.locale, "Liked Songs").into_owned())
                } else if let Some(id) = uri.strip_prefix("spotify:album:") {
                    app.known_album(id)
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| now.album_name.clone())
                } else if !now.album_name.is_empty() {
                    now.album_name.clone()
                } else {
                    gettext(app.locale, "Liked Songs").into_owned()
                }
            } else if !now.album_name.is_empty() {
                now.album_name.clone()
            } else {
                gettext(app.locale, "Liked Songs").into_owned()
            };

            // 1. Collapse button [|> (HideNowPlaying), pure white, slides in from left to right pushing title
            let collapse_w = 24.0 * hover_t;
            if collapse_w > 0.5 {
                let (c_rect, c_resp) =
                    header_ui.allocate_exact_size(vec2(collapse_w, 28.0), Sense::click());
                let icon_alpha = hover_t.clamp(0.0, 1.0);
                let c_color = Color32::WHITE.gamma_multiply(icon_alpha);
                let icon_rect = Rect::from_min_size(
                    pos2(c_rect.left(), c_rect.center().y - 9.0),
                    Vec2::splat(18.0),
                );
                theme::paint_icon(&header_ui, Icon::HideNowPlaying, icon_rect, 18.0, c_color);
                let c_resp = c_resp.on_hover_text(gettext(app.locale, "Close"));
                if c_resp.clicked() {
                    app.actions.push(Action::ToggleLyricsPanel);
                }
                if c_resp.hovered() {
                    header_ui
                        .ctx()
                        .set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                header_ui.add_space(6.0 * hover_t);
            }

            // 2. Context title stays visible at ALL times!
            theme::text(
                &mut header_ui,
                &context_title,
                theme::bold(14.5),
                palette.text,
            );

            // 3. Right actions: Full screen lyrics and "..." (fade in on hover, NOT visible when unhovered)
            header_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                if hover_t > 0.02 {
                    let right_ui_alpha = hover_t;
                    let mut actions_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(ui.available_rect_before_wrap())
                            .layout(Layout::right_to_left(Align::Center)),
                    );
                    actions_ui.set_opacity(right_ui_alpha);

                    if theme::icon_button(
                        &mut actions_ui,
                        Icon::Expand,
                        16.0,
                        Color32::WHITE,
                        Color32::WHITE,
                        &gettext(app.locale, "Full screen lyrics"),
                    )
                    .clicked()
                    {
                        app.actions.push(Action::SetLyricsFullscreen(true));
                    }

                    let loaded = matches!(&app.lyrics, Loadable::Loaded(Some(_)));
                    if loaded
                        && !app.lyrics_following
                        && theme::pill_button(
                            &mut actions_ui,
                            &palette,
                            &pgettext(app.locale, "lyrics", "Follow"),
                            false,
                        )
                        .clicked()
                    {
                        app.actions.push(Action::FollowLyrics);
                    }

                    let more = theme::icon_button(
                        &mut actions_ui,
                        Icon::Ellipsis,
                        16.0,
                        Color32::WHITE,
                        Color32::WHITE,
                        &gettext(app.locale, "More options"),
                    );
                    egui::Popup::menu(&more)
                        .frame(widgets::menu_frame(&palette))
                        .show(|ui| {
                            if let Some(item) = app.now_playing_item() {
                                let context = app.editable_context_playlist();
                                widgets::item_menu(ui, app, &item, context.as_ref(), None);
                            }
                        });
                }
            });
        }

        // Title, artist, like button, and copy link at bottom of hero area
        {
            let title_rect = Rect::from_min_size(
                pos2(rect.left() + pad, rect.bottom() - 52.0),
                vec2(rect.width() - pad * 2.0, 48.0),
            );
            let mut title_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(title_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            let text_w = (title_rect.width() - 64.0).max(60.0);
            title_ui.allocate_ui_with_layout(
                vec2(text_w, 48.0),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&now.title)
                                .font(theme::bold(18.0))
                                .color(palette.text),
                        )
                        .truncate(),
                    );
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&now.subtitle)
                                .font(theme::regular(14.0))
                                .color(palette.secondary),
                        )
                        .truncate(),
                    );
                },
            );
            title_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let saved = app.is_saved(&now.uri).unwrap_or(false);
                let tooltip = if saved {
                    gettext(app.locale, "Remove from Liked Songs")
                } else {
                    gettext(app.locale, "Save to Liked Songs")
                };
                if theme::liked_button(ui, &palette, saved, 20.0, &tooltip).clicked() {
                    app.actions.push(Action::ToggleSaved(now.uri.clone()));
                }
                if hover_t > 0.01 {
                    ui.set_opacity(hover_t);
                    if theme::icon_button(
                        ui,
                        Icon::Copy,
                        16.0,
                        palette.secondary,
                        palette.text,
                        &gettext(app.locale, "Copy link"),
                    )
                    .clicked()
                    {
                        app.actions.push(Action::CopyLink(now.uri.clone()));
                    }
                }
            });
        }

        ui.add_space(8.0);
    }

    {
        // The detail cards share the same scrollable panel as the lyrics preview.
        let inner_pad = egui::Margin::symmetric(pad as i8, 0);
        Frame::new().inner_margin(inner_pad).show(ui, |ui| {
            ui.spacing_mut().scroll.bar_width = 0.0;
            ui.spacing_mut().scroll.floating_width = 0.0;
            egui::ScrollArea::vertical()
                .id_salt("npv-scroll")
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    lyrics_preview_card(app, ui);

                    if let Some(now) = app.now_playing() {
                        ui.add_space(16.0);
                        artist_card(app, ui, &now);
                        credits_card(app, ui, &now);
                        next_up_card(app, ui);
                    }
                    ui.add_space(16.0);
                });
        });
    }
}

fn artist_card(app: &mut App, ui: &mut egui::Ui, now: &NowPlaying) {
    let Some(reference) = now.artists.first() else {
        return;
    };
    let id = reference.id.as_deref().filter(|id| !id.is_empty());
    let palette = app.palette;
    let artist = id
        .and_then(|id| {
            app.artist_pages
                .get(id)
                .and_then(|page| page.artist.get())
                .or_else(|| app.known_artist(id))
        })
        .cloned();
    if let Some(id) = id.filter(|_| artist.is_none() && ui.is_enabled()) {
        app.actions.push(Action::LoadArtist(id.to_owned()));
    }
    let open_artist = || {
        id.map_or_else(
            || Action::Search(reference.name.clone()),
            |id| Action::Open(crate::model::Page::Artist(id.to_owned())),
        )
    };
    Frame::new()
        .fill(palette.surface)
        .corner_radius(8)
        .inner_margin(16)
        .show(ui, |ui| {
            theme::text(
                ui,
                gettext(app.locale, "About the artist"),
                theme::bold(14.0),
                palette.text,
            );
            ui.add_space(12.0);
            if let Some(image) = artist.as_ref().and_then(|artist| {
                crate::api::models::pick_image(&artist.images, app.art_px(ui.available_width()))
            }) {
                let (rect, response) = ui.allocate_exact_size(
                    vec2(ui.available_width(), ui.available_width() * 0.62),
                    Sense::click(),
                );
                widgets::paint_cover(
                    ui,
                    &palette,
                    Some(image),
                    rect,
                    6.0,
                    Icon::GoToArtist,
                    Some(app.backend.art()),
                );
                if response.clicked() {
                    app.actions.push(open_artist());
                }
                ui.add_space(12.0);
            }
            if theme::link(ui, &reference.name, theme::bold(16.0), palette.text).clicked() {
                app.actions.push(open_artist());
            }
            ui.add_space(8.0);
            if let Some(id) = id {
                let uri = format!("spotify:artist:{id}");
                let following = app.is_saved(&uri).unwrap_or(false);
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let follow = if following {
                            pgettext(app.locale, "artist", "Following")
                        } else {
                            pgettext(app.locale, "artist", "Follow")
                        };
                        if theme::pill_button(ui, &palette, &follow, false).clicked() {
                            app.actions.push(Action::ToggleSaved(uri));
                        }
                        if let Some(followers) =
                            artist.as_ref().and_then(|artist| artist.followers.as_ref())
                        {
                            let label = crate::i18n::ngettext(
                                app.locale,
                                "{count} follower",
                                "{count} followers",
                                u32::try_from(followers.total).unwrap_or(u32::MAX),
                            )
                            .replace("{count}", &crate::util::format_count(followers.total));
                            theme::text(ui, label, theme::regular(13.0), palette.secondary);
                        }
                    });
                });
            }
            if let Some(artist) = artist.as_ref().filter(|artist| !artist.genres.is_empty()) {
                ui.add_space(8.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(artist.genres.join(", "))
                            .font(theme::regular(13.0))
                            .color(palette.secondary),
                    )
                    .wrap(),
                );
            }
        });
    ui.add_space(16.0);
}

fn credits_card(app: &mut App, ui: &mut egui::Ui, now: &NowPlaying) {
    if now.artists.is_empty() {
        return;
    }
    let palette = app.palette;
    Frame::new()
        .fill(palette.surface)
        .corner_radius(8)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::text(
                    ui,
                    gettext(app.locale, "Credits"),
                    theme::bold(14.0),
                    palette.text,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::link(
                        ui,
                        gettext(app.locale, "Show all"),
                        theme::medium(12.0),
                        palette.secondary,
                    )
                    .clicked()
                        && let Some(crate::api::models::PlayableItem::Track(track)) =
                            app.now_playing_item()
                    {
                        app.actions
                            .push(Action::ShowDialog(crate::model::Dialog::Credits(Box::new(
                                track,
                            ))));
                    }
                });
            });
            for artist in &now.artists {
                ui.add_space(14.0);
                if theme::link(ui, &artist.name, theme::medium(14.0), palette.text).clicked()
                    && let Some(id) = artist.id.as_ref().filter(|id| !id.is_empty())
                {
                    app.actions
                        .push(Action::Open(crate::model::Page::Artist(id.clone())));
                }
                theme::text(
                    ui,
                    gettext(app.locale, "Artist"),
                    theme::regular(13.0),
                    palette.secondary,
                );
            }
        });
    ui.add_space(16.0);
}

fn next_up_card(app: &mut App, ui: &mut egui::Ui) {
    let Some(item) = app
        .queue
        .get()
        .and_then(|queue| queue.queue.first())
        .cloned()
    else {
        return;
    };
    let palette = app.palette;
    Frame::new()
        .fill(palette.surface)
        .corner_radius(8)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::text(
                    ui,
                    gettext(app.locale, "Up next"),
                    theme::bold(14.0),
                    palette.text,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::link(
                        ui,
                        gettext(app.locale, "Open queue"),
                        theme::medium(12.0),
                        palette.secondary,
                    )
                    .clicked()
                    {
                        app.actions.push(Action::ToggleQueuePanel);
                    }
                });
            });
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                widgets::paint_cover(
                    ui,
                    &palette,
                    item.image(app.art_px(48.0)),
                    rect,
                    4.0,
                    Icon::Music,
                    Some(app.backend.art()),
                );
                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(item.name())
                                .font(theme::medium(14.0))
                                .color(palette.text),
                        )
                        .truncate(),
                    );
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(item.subtitle())
                                .font(theme::regular(13.0))
                                .color(palette.secondary),
                        )
                        .truncate(),
                    );
                });
            });
        });
    ui.add_space(16.0);
}

fn lyrics_preview_card(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let expanded_id =
        egui::Id::new("npv-lyrics-expanded").with(app.now_playing().map(|now| now.uri));
    let expanded = ui
        .data(|data| data.get_temp::<bool>(expanded_id))
        .unwrap_or(false);
    let card_fill = app
        .now_playing_tint()
        .map(|tint| blend(palette.surface, tint, 0.6))
        .unwrap_or(palette.surface);
    let pad = 16.0;
    let header_h = 18.0;
    let body_h = if expanded { 360.0 } else { 116.0 };
    let width = ui.available_width();
    let height = pad + header_h + 12.0 + body_h + pad;
    let (card, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    if !ui.is_rect_visible(card) {
        return;
    }
    let clip = card.intersect(ui.clip_rect());
    let painter = ui.painter().with_clip_rect(clip);
    painter.rect_filled(card, egui::CornerRadius::same(8), card_fill);
    painter.text(
        pos2(card.left() + pad, card.top() + pad + header_h / 2.0),
        egui::Align2::LEFT_CENTER,
        if expanded {
            gettext(app.locale, "Lyrics")
        } else {
            gettext(app.locale, "Lyrics preview")
        },
        theme::bold(14.0),
        palette.text,
    );
    let body = Rect::from_min_size(
        pos2(card.left() + pad, card.top() + pad + header_h + 12.0),
        vec2(width - pad * 2.0, body_h),
    );
    let body_painter = ui.painter().with_clip_rect(body.intersect(clip));
    let quiet = blend(card_fill, palette.text, 0.62);

    let message = |text: &str| {
        body_painter.text(
            body.left_top(),
            egui::Align2::LEFT_TOP,
            text,
            theme::bold(16.0),
            quiet,
        );
    };
    match (&app.lyrics, app.now_playing()) {
        (_, None) => message(&gettext(app.locale, "Nothing playing")),
        (Loadable::NotLoaded | Loadable::Loading, _) => message(&gettext(app.locale, "Loading…")),
        (Loadable::Failed(_), _) => {
            message(&gettext(app.locale, "No lyrics"));
            let retry_rect = Rect::from_min_size(
                pos2(card.left() + pad, card.bottom() - 44.0),
                vec2(100.0, 32.0),
            );
            let mut retry_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(retry_rect)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            let label = gettext(app.locale, "Try again");
            if theme::pill_button(&mut retry_ui, &palette, &label, false).clicked() {
                app.actions.push(Action::RetryLyrics);
            }
        }
        (Loadable::Loaded(None), _) => message(&gettext(app.locale, "No lyrics")),
        (Loadable::Loaded(Some(lyrics)), _) if lyrics.instrumental => {
            message(&gettext(app.locale, "Instrumental"))
        }
        (Loadable::Loaded(Some(lyrics)), Some(now)) => {
            let active = lyrics.active_line(now.position_ms);
            let font = theme::bold(20.0);
            let gap = 8.0;
            let mut y = 0.0;
            let mut target = 0.0;
            let mut rows = Vec::with_capacity(lyrics.lines.len());
            for (index, line) in lyrics.lines.iter().enumerate() {
                if Some(index) == active {
                    target = y;
                }
                let galley = ui.painter().layout(
                    line.text.clone(),
                    font.clone(),
                    Color32::PLACEHOLDER,
                    body.width(),
                );
                let h = galley.size().y;
                rows.push((y, galley));
                y += h + gap;
            }
            let offset = ui.ctx().animate_value_with_time(
                egui::Id::new("npv-lyrics-preview-offset"),
                (target - if expanded { body_h * 0.35 } else { 0.0 })
                    .clamp(0.0, (y - body_h).max(0.0)),
                0.35,
            );
            for (index, (row_y, galley)) in rows.into_iter().enumerate() {
                let top = body.top() + row_y - offset;
                if top > body.bottom() || top + galley.size().y < body.top() {
                    continue;
                }
                let lit = ui.ctx().animate_bool_with_time(
                    egui::Id::new("npv-preview-line").with(index),
                    active == Some(index) || (!lyrics.synced && index == 0),
                    LIGHT_UP_SECONDS,
                );
                body_painter.galley(
                    pos2(body.left(), top),
                    galley,
                    blend(quiet, palette.text, lit),
                );
            }
        }
    }

    // Fade the last visible line into the card.
    let fade = Rect::from_min_max(pos2(card.left(), card.bottom() - 44.0), card.max);
    let fade_painter = ui.painter().with_clip_rect(clip);
    let mut mesh = egui::Mesh::default();
    let transparent = card_fill.gamma_multiply(0.0);
    mesh.colored_vertex(fade.left_top(), transparent);
    mesh.colored_vertex(fade.right_top(), transparent);
    mesh.colored_vertex(pos2(fade.left(), fade.bottom() - 8.0), card_fill);
    mesh.colored_vertex(pos2(fade.right(), fade.bottom() - 8.0), card_fill);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    fade_painter.add(egui::Shape::mesh(mesh));
    fade_painter.rect_filled(
        Rect::from_min_max(pos2(card.left(), card.bottom() - 8.0), card.max),
        egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: 8,
            se: 8,
        },
        card_fill,
    );

    // "Mostrar mais": white pill, only on hover, sliding up from below.
    let hovered = ui.rect_contains_pointer(card) || expanded;
    let t = ui.ctx().animate_bool_with_time_and_easing(
        egui::Id::new("npv-lyrics-more"),
        hovered,
        0.2,
        egui::emath::easing::cubic_out,
    );
    if t > 0.01 {
        let label = if expanded {
            gettext(app.locale, "Show less")
        } else {
            gettext(app.locale, "Show more")
        };
        let galley =
            ui.painter()
                .layout_no_wrap(label.to_string(), theme::bold(13.0), Color32::PLACEHOLDER);
        let size = vec2(galley.size().x + 32.0, 32.0);
        let rest = pos2(card.right() - 12.0 - size.x, card.bottom() - 12.0 - size.y);
        let slide = (1.0 - t) * (size.y + 12.0);
        let rect = Rect::from_min_size(rest + vec2(0.0, slide), size);
        let response = ui.interact(rect, egui::Id::new("npv-lyrics-more-btn"), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label.as_ref())
        });
        let fill = if response.hovered() {
            blend(palette.text, palette.secondary, 0.15)
        } else {
            palette.text
        };
        let fill = fill.gamma_multiply(t);
        painter.rect_filled(rect, egui::CornerRadius::same(16), fill);
        painter.galley(
            rect.center() - galley.size() / 2.0,
            galley,
            palette.window.gamma_multiply(t),
        );
        if t > 0.5
            && response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
        {
            ui.data_mut(|data| data.insert_temp(expanded_id, !expanded));
            ui.ctx().request_repaint();
        }
    }
}

fn track_change_opacity(ui: &mut egui::Ui, app: &App) -> f32 {
    let Some(now) = app.now_playing() else {
        return 1.0;
    };
    if cfg!(test) {
        return 1.0;
    }
    let tracker_id = egui::Id::new("fullscreen_track_change_tracker");
    let (last_uri, start_inst): (Option<String>, Option<std::time::Instant>) =
        ui.ctx().data_mut(|d| d.get_temp(tracker_id).unwrap_or((None, None)));
    let cur_uri = Some(now.uri.clone());
    if last_uri.is_some() && last_uri != cur_uri {
        let now_inst = std::time::Instant::now();
        ui.ctx().data_mut(|d| d.insert_temp(tracker_id, (cur_uri, Some(now_inst))));
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        0.05
    } else if let Some(start) = start_inst {
        let elapsed = start.elapsed().as_secs_f32();
        let dur = 0.35;
        let raw_t = (elapsed / dur).min(1.0);
        if raw_t < 1.0 {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        }
        let inv = 1.0 - raw_t;
        0.05 + 0.95 * (1.0 - inv * inv)
    } else {
        ui.ctx().data_mut(|d| d.insert_temp(tracker_id, (cur_uri, None::<std::time::Instant>)));
        1.0
    }
}

pub fn fullscreen(app: &mut App, ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    background(app, ui, rect);
    let chrome = ui
        .ctx()
        .data(|data| data.get_temp::<f32>(egui::Id::new(super::LYRICS_CHROME_VIS_ID)))
        .unwrap_or(1.0);
    widgets::paint_vertical_gradient(
        ui,
        Rect::from_min_size(rect.min, vec2(rect.width(), 110.0)).intersect(rect),
        Color32::from_black_alpha((210.0 * chrome) as u8),
        Color32::TRANSPARENT,
    );
    let track_t = track_change_opacity(ui, app);
    let top = 24.0;
    if app.now_playing().is_some() && rect.width() >= COVER_BESIDE_MIN_WIDTH {
        with_cover(app, ui, rect, top, track_t);
        return;
    }
    let width = fullscreen_content_width(rect.width());
    let region = Rect::from_min_max(
        pos2(rect.center().x - width / 2.0, rect.top() + top),
        pos2(rect.center().x + width / 2.0, rect.bottom()),
    );
    let mut content = ui.new_child(UiBuilder::new().max_rect(region));
    fullscreen_header(app, &mut content);
    content.add_space(20.0);
    track_heading(app, &mut content, track_t);
    content.add_space(16.0);
    fullscreen_contents(app, &mut content, track_t);
}

/// The widest the lyrics get beside the cover, so lines stay easy to read.
const LYRICS_BESIDE_WIDTH: f32 = 640.0;

/// The narrowest window that shows the cover beside the lyrics; narrower
/// ones keep a single column with a small cover in the heading.
const COVER_BESIDE_MIN_WIDTH: f32 = 900.0;

/// Full screen with the cover large: beside the lyrics when there are
/// words to follow, and alone in the middle when there are none, a calm
/// view of what is playing.
fn with_cover(app: &mut App, ui: &mut egui::Ui, rect: Rect, top: f32, track_t: f32) {
    let outer = Rect::from_min_max(
        pos2(rect.left() + 48.0, rect.top() + top),
        pos2(rect.right() - 48.0, rect.bottom() - 40.0),
    );
    let mut header = ui.new_child(UiBuilder::new().max_rect(outer));
    fullscreen_header(app, &mut header);
    let below = Rect::from_min_max(
        pos2(outer.left(), header.min_rect().bottom() + 24.0),
        outer.max,
    );
    // The cover moves aside only for words to read. While they load it
    // stays in the middle, so a song that turns out to have none never
    // moves at all.
    let words = matches!(&app.lyrics, Loadable::Loaded(Some(lyrics)) if !lyrics.instrumental);
    if words {
        // The cover and the lyrics are one group, centred in the window.
        let gap = 64.0;
        let side = (below.width() * 0.38)
            .min(below.height() - 90.0)
            .clamp(200.0, 520.0);
        let lyrics_width = (below.width() - side - gap).min(LYRICS_BESIDE_WIDTH);
        let left = below.center().x - (side + gap + lyrics_width) / 2.0;
        let column = Rect::from_min_size(
            pos2(left, below.center().y - (side + 90.0) / 2.0),
            vec2(side, side + 90.0),
        );
        big_cover(app, ui, column, Align::Min, track_t);
        let lyrics = Rect::from_min_max(
            pos2(column.right() + gap, below.top()),
            pos2(column.right() + gap + lyrics_width, below.bottom()),
        );
        let mut content = ui.new_child(UiBuilder::new().max_rect(lyrics));
        fullscreen_contents(app, &mut content, track_t);
    } else {
        let side = (below.height() - 140.0)
            .min(below.width() * 0.5)
            .clamp(200.0, 560.0);
        let column = Rect::from_center_size(below.center(), vec2(side, side + 90.0));
        big_cover(app, ui, column, Align::Center, track_t);
        // Why there are no words, quietly, under the song, or that they
        // are still being fetched.
        let (heading, detail) = match &app.lyrics {
            Loadable::Loaded(Some(_)) => (
                gettext(app.locale, "Instrumental"),
                gettext(app.locale, "No timed lyrics for this track."),
            ),
            Loadable::Loaded(None) => (
                gettext(app.locale, "No lyrics"),
                gettext(app.locale, "No lyrics found for this track."),
            ),
            Loadable::Failed(error) => (
                gettext(
                    app.locale,
                    // Translators: Keep {error} unchanged. It is the original failure detail.
                    "Couldn't fetch the lyrics: {error}",
                )
                .replace("{error}", error)
                .into(),
                Default::default(),
            ),
            Loadable::NotLoaded | Loadable::Loading => {
                (gettext(app.locale, "Loading…"), Default::default())
            }
        };
        let heading = ui.painter().text(
            pos2(column.center().x, column.bottom() + 8.0),
            egui::Align2::CENTER_TOP,
            heading,
            theme::semibold(13.0),
            Color32::from_gray(200),
        );
        if matches!(app.lyrics, Loadable::Failed(_)) {
            let retry = Rect::from_center_size(
                pos2(column.center().x, heading.bottom() + 24.0),
                vec2(column.width(), 32.0),
            );
            let mut retry_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(retry)
                    .layout(Layout::top_down(Align::Center)),
            );
            let label = gettext(app.locale, "Try again");
            if theme::pill_button(&mut retry_ui, &theme::Palette::dark(), &label, false).clicked() {
                app.actions.push(Action::RetryLyrics);
            }
            return;
        }
        ui.painter().text(
            pos2(column.center().x, heading.bottom() + 4.0),
            egui::Align2::CENTER_TOP,
            detail,
            theme::regular(13.0),
            Color32::from_gray(170),
        );
    }
}

/// The playing song's cover filling the top of `column`, with its title and
/// artists beneath, aligned to its left edge or centred.
fn big_cover(app: &App, ui: &mut egui::Ui, column: Rect, align: Align, opacity: f32) {
    let Some(now) = app.now_playing() else {
        return;
    };
    let side = column.width();
    let cover = Rect::from_min_size(column.min, vec2(side, side));
    let radius = 10.0;
    ui.scope(|ui| {
        ui.multiply_opacity(opacity);
        ui.painter().add(
            egui::epaint::Shadow {
                offset: [0, 18],
                blur: 48,
                spread: 0,
                color: Color32::from_black_alpha(140),
            }
            .as_shape(cover, radius),
        );
        widgets::playing_cover(
            ui,
            &theme::Palette::dark(),
            app.backend.art(),
            "fullscreen-lyrics",
            now.art_url.as_deref().or(now.art_small.as_deref()),
            cover,
            radius,
        );
        let words = Rect::from_min_max(pos2(column.left(), cover.bottom() + 18.0), column.max);
        let mut text = ui.new_child(
            UiBuilder::new()
                .max_rect(words)
                .layout(Layout::top_down(align)),
        );
        text.spacing_mut().item_spacing.y = 4.0;
        text.add(
            egui::Label::new(
                egui::RichText::new(&now.title)
                    .font(theme::semibold(22.0))
                    .color(Color32::WHITE),
            )
            .truncate(),
        );
        text.add(
            egui::Label::new(
                egui::RichText::new(&now.subtitle)
                    .font(theme::regular(14.0))
                    .color(Color32::from_gray(225)),
            )
            .truncate(),
        );
    });
}

fn fullscreen_content_width(viewport_width: f32) -> f32 {
    let available = (viewport_width - 48.0).max(0.0);
    (viewport_width * 0.72).clamp(400.0, 960.0).min(available)
}

fn preferred_backdrop_art(small: Option<String>, large: Option<String>) -> Option<String> {
    small.or(large)
}

fn background(app: &mut App, ui: &mut egui::Ui, rect: Rect) {
    theme::apply_local(ui, &theme::Palette::dark());
    let art = app
        .now_playing()
        .and_then(|now| preferred_backdrop_art(now.art_small, now.art_url));
    let painter = ui.painter().with_clip_rect(rect);
    let (cur_tex, prev_tex, t) = app
        .lyrics_backdrop
        .textures(ui.ctx(), app.backend.art(), art.as_deref());

    if let Some(prev) = prev_tex {
        let prev_tint = Color32::from_gray(180).gamma_multiply((1.0 - t).clamp(0.0, 1.0));
        if prev_tint.a() > 0 {
            painter.image(
                prev.id(),
                rect,
                cover_uv(rect.size(), prev.size_vec2()),
                prev_tint,
            );
        }
    }

    if let Some(cur) = cur_tex {
        let cur_tint = Color32::from_gray(180).gamma_multiply(t.clamp(0.0, 1.0));
        if cur_tint.a() > 0 {
            painter.image(
                cur.id(),
                rect,
                cover_uv(rect.size(), cur.size_vec2()),
                cur_tint,
            );
        }
    }

    painter.rect_filled(rect, 0.0, Color32::from_black_alpha(120));
    widgets::paint_vertical_gradient(
        ui,
        rect,
        Color32::from_black_alpha(0),
        Color32::from_black_alpha(95),
    );
}

fn cover_uv(view: egui::Vec2, image: egui::Vec2) -> Rect {
    let ratio = (view.x / view.y.max(1.0)) / (image.x / image.y.max(1.0));
    let size = if ratio > 1.0 {
        vec2(1.0, 1.0 / ratio)
    } else {
        vec2(ratio, 1.0)
    };
    Rect::from_center_size(pos2(0.5, 0.5), size)
}

fn fullscreen_header(app: &mut App, ui: &mut egui::Ui) {
    let palette = theme::Palette::dark();
    let chrome_vis = ui
        .ctx()
        .data(|d| d.get_temp::<f32>(egui::Id::new(super::LYRICS_CHROME_VIS_ID)))
        .unwrap_or(1.0);
    ui.horizontal(|ui| {
        ui.set_min_height(32.0);
        if chrome_vis < 0.02 {
            return;
        }
        ui.set_opacity(chrome_vis);
        let title = app
            .now_playing()
            .map(|now| now.title)
            .unwrap_or_else(|| gettext(app.locale, "Lyrics").into_owned());
        theme::text(ui, title, theme::bold(18.0), palette.text);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if theme::icon_button(
                ui,
                Icon::MinimizeView,
                18.0,
                palette.text,
                palette.text,
                &gettext(app.locale, "Minimize"),
            )
            .clicked()
            {
                app.actions.push(Action::SetLyricsFullscreen(false));
            }
            let fullscreen = ui
                .ctx()
                .input(|input| input.viewport().fullscreen.unwrap_or(false));
            if theme::icon_button(
                ui,
                Icon::Fullscreen,
                18.0,
                palette.text,
                palette.text,
                &if fullscreen {
                    gettext(app.locale, "Leave full screen (Esc)")
                } else {
                    gettext(app.locale, "Enter full screen")
                },
            )
            .clicked()
            {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
            }
            let more = theme::icon_button(
                ui,
                Icon::Ellipsis,
                18.0,
                palette.text,
                palette.text,
                &gettext(app.locale, "More options"),
            );
            egui::Popup::menu(&more)
                .frame(widgets::menu_frame(&palette))
                .show(|ui| {
                    if let Some(item) = app.now_playing_item() {
                        let context = app.editable_context_playlist();
                        widgets::item_menu(ui, app, &item, context.as_ref(), None);
                    }
                });
            let loaded = matches!(&app.lyrics, Loadable::Loaded(Some(_)));
            if loaded
                && !app.lyrics_following
                && theme::pill_button(
                    ui,
                    &palette,
                    &pgettext(app.locale, "lyrics", "Follow"),
                    false,
                )
                .clicked()
            {
                app.actions.push(Action::FollowLyrics);
            }
        });
    });
}

fn track_heading(app: &App, ui: &mut egui::Ui, opacity: f32) {
    if let Some(now) = app.now_playing() {
        ui.scope(|ui| {
            ui.multiply_opacity(opacity);
            ui.horizontal(|ui| {
                let size = 52.0;
                let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
                widgets::playing_cover(
                    ui,
                    &theme::Palette::dark(),
                    app.backend.art(),
                    "lyrics-heading",
                    app.small_art(&now, size),
                    rect,
                    4.0,
                );
                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&now.title)
                                .font(theme::semibold(22.0))
                                .color(Color32::WHITE),
                        )
                        .truncate(),
                    );
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&now.subtitle)
                                .font(theme::regular(13.0))
                                .color(Color32::from_gray(235)),
                        )
                        .truncate(),
                    );
                });
            });
        });
    }
}

fn fullscreen_contents(app: &mut App, ui: &mut egui::Ui, opacity: f32) {
    ui.scope(|ui| {
        ui.multiply_opacity(opacity);
        fullscreen_contents_inner(app, ui);
    });
}

fn fullscreen_contents_inner(app: &mut App, ui: &mut egui::Ui) {
    let palette = theme::Palette::dark();
    let Some(now) = app.now_playing() else {
        widgets::empty_state(
            ui,
            &palette,
            Icon::Mic,
            &gettext(app.locale, "Nothing playing"),
            &gettext(app.locale, "Play a song to see its lyrics."),
        );
        return;
    };
    let lyrics = match &app.lyrics {
        Loadable::NotLoaded | Loadable::Loading => {
            widgets::loading_row(ui, &palette, app.locale);
            return;
        }
        Loadable::Failed(error) => {
            let message = gettext(
                app.locale,
                // Translators: Keep {error} unchanged. It is the original failure detail.
                "Couldn't fetch the lyrics: {error}",
            )
            .replace("{error}", error);
            ui.add_space(8.0);
            theme::text(ui, message, theme::regular(13.0), palette.text);
            ui.add_space(8.0);
            if theme::pill_button(ui, &palette, &gettext(app.locale, "Try again"), false).clicked()
            {
                app.actions.push(Action::RetryLyrics);
            }
            return;
        }
        Loadable::Loaded(None) => {
            widgets::empty_state(
                ui,
                &palette,
                Icon::Mic,
                &gettext(app.locale, "No lyrics"),
                &gettext(app.locale, "No lyrics found for this track."),
            );
            return;
        }
        Loadable::Loaded(Some(lyrics)) if lyrics.instrumental => {
            widgets::empty_state(
                ui,
                &palette,
                Icon::Music,
                &gettext(app.locale, "Instrumental"),
                &gettext(app.locale, "No timed lyrics for this track."),
            );
            return;
        }
        Loadable::Loaded(Some(lyrics)) => lyrics.clone(),
    };

    let active = lyrics.active_line(now.position_ms);
    // A Follow click resets the remembered line after drawing. Other frames
    // record the shown line before any line-click action restores following.
    if !app
        .actions
        .iter()
        .any(|action| matches!(action, Action::FollowLyrics))
    {
        app.actions.push(Action::LyricsLineShown(active));
    }
    let viewport = ui.available_rect_before_wrap();
    let manual_scroll = ui.rect_contains_pointer(viewport)
        && ui.input(|input| {
            input.smooth_scroll_delta.y != 0.0
                || (input.pointer.primary_down() && input.pointer.delta().y != 0.0)
        });
    let following = app.lyrics_following && !manual_scroll;
    let follow = following && app.lyrics_line_shown != Some(active);
    let animation = crate::motion::scroll(ui.ctx(), 0.45);
    let size = (ui.available_width() * 0.046).clamp(28.0, 42.0);
    // The line being sung brightens; all lines keep the same font metrics
    // so highlighting cannot rewrap the words during a transition.
    // A line takes 300 ms to light up or fade.
    let quiet = palette.text.gamma_multiply(0.68);
    ui.spacing_mut().scroll.fade.strength = 0.0;
    ui.spacing_mut().scroll.bar_width = 0.0;
    ui.spacing_mut().scroll.floating_width = 0.0;
    egui::ScrollArea::vertical()
        .id_salt(("fullscreen-lyrics-scroll", &now.uri))
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Before the first line there is nothing to highlight, so the
            // panel sits at the top rather than wherever it was left.
            if follow && lyrics.synced && active.is_none() {
                let top = ui.cursor().min;
                ui.scroll_to_rect_animation(
                    egui::Rect::from_min_size(top, egui::vec2(1.0, 1.0)),
                    Some(Align::Min),
                    animation,
                );
            }
            // Room for the first line to sit where a sung line sits, and for
            // the last to rise to it.
            let (padding, below) = if lyrics.synced {
                (
                    (viewport.height() * SUNG_LINE_AT - size).max(12.0),
                    viewport.height() * (1.0 - SUNG_LINE_AT),
                )
            } else {
                (12.0, 60.0)
            };
            ui.add_space(padding);
            for (index, line) in lyrics.lines.iter().enumerate() {
                let is_active = active == Some(index);
                let lit = ui.ctx().animate_bool_with_time(
                    egui::Id::new("lyric-line").with(("fullscreen", &now.uri, index)),
                    is_active,
                    crate::motion::seconds(ui.ctx(), 0.3),
                );
                let color = if lyrics.synced {
                    blend(quiet, palette.text, lit)
                } else {
                    palette.text
                };
                let font = theme::bold(size);
                // A timed line with no words is the band playing on.
                let text = if line.text.is_empty() && lyrics.synced {
                    "\u{266a}"
                } else {
                    line.text.as_str()
                };
                let sense = if lyrics.synced {
                    Sense::click()
                } else {
                    Sense::hover()
                };
                let galley = crate::bidi::layout(
                    ui.painter(),
                    text,
                    font,
                    color,
                    ui.available_width(),
                    usize::MAX,
                    None,
                );
                let center = ui.cursor().top() + galley.size().y * 0.5;
                let edge = ((center - viewport.top()).min(viewport.bottom() - center)
                    / (size * 2.0))
                    .clamp(0.0, 1.0);
                let response = ui
                    .scope(|ui| {
                        ui.multiply_opacity(edge * edge * (3.0 - 2.0 * edge));
                        ui.add(egui::Label::new(galley).sense(sense))
                    })
                    .inner;
                let rect = response.rect;
                if lyrics.synced
                    && response.clicked()
                    && let Some(at_ms) = line.at_ms
                {
                    app.actions.push(Action::Seek(at_ms));
                    app.actions.push(Action::FollowLyrics);
                }
                if is_active && follow {
                    show_sung_line(ui, rect, Some(animation));
                }
                ui.add_space(27.0);
            }
            // Words without timing can only be followed by the clock: sit
            // at the part of the text the song is probably at.
            if following && !lyrics.synced && now.duration_ms > 0 {
                let fraction =
                    (f64::from(now.position_ms) / f64::from(now.duration_ms)).clamp(0.0, 1.0);
                let content = ui.min_rect();
                let y = content.top() + content.height() * fraction as f32;
                ui.scroll_to_rect_animation(
                    egui::Rect::from_min_max(
                        egui::pos2(content.left(), y),
                        egui::pos2(content.right(), y + 1.0),
                    ),
                    Some(Align::Center),
                    animation,
                );
            }
            ui.add_space(below.max(60.0));
        });
    // Scrolling by hand means the reader wants to look elsewhere; the
    // Follow button in the header picks the song back up.
    if manual_scroll && app.lyrics_following {
        app.actions.push(Action::PauseLyricsFollow);
    }
    if now.playing
        && lyrics.synced
        && let Some(next) = lyrics
            .lines
            .iter()
            .filter_map(|line| line.at_ms)
            .find(|at| *at > now.position_ms)
    {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(u64::from(
                next - now.position_ms,
            )));
    }
}

#[cfg(test)]
mod tests {
    use super::{fullscreen_content_width, preferred_backdrop_art};

    #[test]
    fn fullscreen_backdrop_prefers_small_art_with_large_art_as_fallback() {
        let small = "small".to_string();
        let large = "large".to_string();
        assert_eq!(
            preferred_backdrop_art(Some(small.clone()), Some(large.clone())),
            Some(small)
        );
        assert_eq!(
            preferred_backdrop_art(None, Some(large.clone())),
            Some(large)
        );
        assert_eq!(preferred_backdrop_art(None, None), None);
    }

    #[test]
    fn fullscreen_content_width_never_inverts_a_narrow_viewport() {
        for viewport_width in [0.0, 24.0, 47.0, 48.0, 64.0, 760.0, 2_000.0] {
            let width = fullscreen_content_width(viewport_width);
            assert!(width >= 0.0);
            assert!(width <= (viewport_width - 48.0).max(0.0));
        }
        assert_eq!(fullscreen_content_width(47.0), 0.0);
        assert_eq!(fullscreen_content_width(2_000.0), 960.0);
    }
}
