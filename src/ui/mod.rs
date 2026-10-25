//! Window layout: panels, overlays, keyboard shortcuts.

use std::sync::Arc;

pub mod artist;
pub mod collection;
pub(crate) mod devices;
mod dialogs;
pub mod home;
mod keys;
pub mod library;
pub mod login;
mod lyrics;
pub mod player_bar;
mod prerelease;
pub mod queue;
pub mod radio;
pub mod search;
pub mod settings;
pub mod show;
pub mod sidebar;
pub mod topbar;
mod update;
pub mod widgets;
pub mod winamp;

use egui::{Align, Align2, Color32, Context, CornerRadius, Frame, Id, Layout, Margin, Rect, Sense, Stroke, pos2, vec2};

use crate::api::models::pick_image;
use crate::app::App;
use crate::i18n::gettext;
use crate::backend::AuthStatus;
use crate::model::{Action, Loadable, Page, ToastKind};
use crate::theme::{self, Icon};

pub(crate) const LYRICS_CHROME_VIS_ID: &str = "lyrics-chrome-vis";

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let ctx = &ctx;
    keys::handle(app, ctx);
    for path in winamp::dropped_skins(ctx) {
        app.actions.push(Action::InstallSkin(path));
    }
    let signed_in = app.is_connected() && app.user.is_some();
    let connecting = matches!(app.auth, AuthStatus::Connecting | AuthStatus::Starting)
        || (app.is_connected() && app.user.is_none());
    if !signed_in {
        player_bar::end_tint_session(ctx);
        login::show(app, ui, connecting);
        update::show(app, ctx);
        toasts(app, ctx, 20.0);
        caption_buttons(ui, &app.palette, app.locale);
        fastframe_titlebar::resize(ui);
        return;
    }

    // Track mouse movement/clicks/scroll for 1-second idle chrome retraction in Expanded Lyrics
    let now_sec = ctx.input(|i| i.time);
    let mouse_active = ctx.input(|i| {
        i.pointer.delta().length_sq() > 0.25
            || i.pointer.any_pressed()
            || i.pointer.any_down()
            || i.smooth_scroll_delta.length_sq() > 0.1
    });
    let last_active_id = Id::new("lyrics-last-mouse-active");
    let mut last_active = ctx
        .data(|d| d.get_temp::<f64>(last_active_id))
        .unwrap_or(now_sec);
    if mouse_active || app.lyrics_fullscreen.is_none() {
        last_active = now_sec;
        ctx.data_mut(|d| d.insert_temp(last_active_id, last_active));
    }
    let idle_secs = (now_sec - last_active).max(0.0);
    if app.lyrics_fullscreen.is_some() && idle_secs < 1.08 {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
    let chrome_shown = app.lyrics_fullscreen.is_none() || idle_secs < 1.0;
    let chrome_vis = ctx.animate_bool_with_time_and_easing(
        Id::new("lyrics-chrome-anim"),
        chrome_shown,
        0.22,
        egui::emath::easing::cubic_out,
    );
    ctx.data_mut(|d| d.insert_temp(Id::new(LYRICS_CHROME_VIS_ID), chrome_vis));

    if chrome_vis > 0.05
        && titlebar_spans_window(cfg!(target_os = "macos"), fastframe_titlebar::enabled())
    {
        fastframe_titlebar::drag(
            ui,
            Rect::from_min_size(
                ui.max_rect().min,
                vec2(
                    ui.max_rect().width(),
                    theme::top_bar_height() + theme::titlebar_inset(ui.ctx()),
                ),
            ),
        );
    }
    player_bar::show(app, ui);
    if chrome_vis > 0.01 {
        egui::Panel::top("global-nav")
            .exact_size((theme::TOP_BAR_HEIGHT * chrome_vis).max(1.0))
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(app.palette.window))
            .show(ui, |ui| {
                if chrome_vis >= 0.35 {
                    topbar::show(app, ui);
                }
            });
    }

    let body_rect = ui.available_rect_before_wrap();
    let cinema_t = ctx.animate_bool_with_time_and_easing(
        Id::new("cinema-lyrics-slide-t"),
        app.lyrics_fullscreen.is_some(),
        0.26,
        egui::emath::easing::cubic_out,
    );

    sidebar::show(app, ui);
    right_bento_panel(app, ui);
    central(app, ui);

    if cinema_t > 0.001 {
        let slide_x = (1.0 - cinema_t) * body_rect.width();
        let cinema_pos = body_rect.min + vec2(slide_x, 0.0);
        let cinema_rect = Rect::from_min_size(cinema_pos, body_rect.size());
        egui::Area::new(Id::new("cinema-lyrics-slide-area"))
            .order(egui::Order::Middle)
            .movable(false)
            .constrain(false)
            .fixed_pos(cinema_pos)
            .show(ctx, |ui| {
                ui.set_clip_rect(body_rect);
                ui.set_min_size(body_rect.size());
                ui.painter()
                    .rect_filled(cinema_rect, 0.0, app.palette.panel);
                let mut content = ui.new_child(egui::UiBuilder::new().max_rect(cinema_rect));
                content.set_clip_rect(body_rect.intersect(cinema_rect));
                lyrics::fullscreen(app, &mut content);
            });
    }
    keep_room_for_panels(app, ctx);
    devices::popup(app, ctx);
    dialogs::show(app, ctx);
    update::show(app, ctx);
    widgets::drag_ghost(ctx, &app.palette, app.locale);
    toasts(app, ctx, (theme::PLAYER_BAR_HEIGHT * chrome_vis) + 16.0);
    if chrome_vis > 0.35 {
        caption_buttons(ui, &app.palette, app.locale);
    }
    fastframe_titlebar::resize(ui);
}

/// The main window's narrowest width with these panels open: their least
/// widths and the page's.
fn main_min_width(page: f32, sidebar: bool, right_panel: bool) -> f32 {
    let sidebar = if sidebar { SIDEBAR_MIN_WIDTH } else { 72.0 };
    let right = if right_panel {
        theme::SIDE_PANEL_MIN_WIDTH
    } else {
        36.0
    };
    (sidebar + right + page).max(crate::window::MAIN_MIN_SIZE[0])
}

/// The sidebar's narrowest width.
pub(crate) const SIDEBAR_MIN_WIDTH: f32 = 210.0;

/// Raise the window's minimum width while the Queue or Lyrics panel is open,
/// so even at its narrowest the page beside the panels keeps the room its
/// top bar needs (#624), and lower it again once they close. Window managers
/// that tile ignore the minimum; the panels still give way there.
fn keep_room_for_panels(app: &App, ctx: &Context) {
    if crate::window::fixed_size() {
        return;
    }
    let width = main_min_width(
        topbar::least_width(ctx),
        app.settings.sidebar_visible && app.lyrics_fullscreen.is_none(),
        app.lyrics_fullscreen.is_none()
            && (app.show_queue_panel
                || app.show_lyrics_panel
                || app.show_devices
                || app.show_friend_activity),
    )
    .round();
    let id = Id::new("main-min-width");
    let sent = ctx.data(|data| data.get_temp::<f32>(id));
    if sent.unwrap_or(crate::window::MAIN_MIN_SIZE[0]) != width {
        ctx.data_mut(|data| data.insert_temp(id, width));
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(vec2(
            width,
            crate::window::MAIN_MIN_SIZE[1],
        )));
    }
    let minimum = vec2(width, crate::window::MAIN_MIN_SIZE[1]);
    if crate::window::main_window_needs_size_recovery(ctx) {
        // Native restore can bypass the tracking minimum while dragging a maximized window.
        let restored = app
            .window_geometry
            .size
            .filter(|size| size.iter().all(|side| side.is_finite()))
            .map_or(minimum, |size| egui::Vec2::from(size) / ctx.zoom_factor());
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(restored.max(minimum)));
    }
}

/// How a resizable side panel fits beside the page.
pub(crate) struct PanelFit {
    /// The widths the panel may take this frame.
    pub range: std::ops::RangeInclusive<f32>,
    /// Whether the room the page needs holds the panel under the width the
    /// person chose for it.
    pub yielding: bool,
}

pub(crate) fn yielding_panel(
    ctx: &Context,
    id: &str,
    range: std::ops::RangeInclusive<f32>,
    chosen: f32,
    room: f32,
) -> PanelFit {
    let (min, max) = range.into_inner();
    let max = max.min(room).max(min);
    let width = chosen.clamp(min, max);
    let id = Id::new(id);
    if let Some(mut state) = egui::containers::panel::PanelState::load(ctx, id)
        && state.outer_rect.width() + 0.5 < width
    {
        state.outer_rect.set_width(width);
        ctx.data_mut(|data| data.insert_persisted(id, state));
    }
    PanelFit {
        range: min..=max,
        yielding: max < chosen - 0.5,
    }
}

pub(crate) fn panel_width_chosen(ctx: &Context, id: &str, fit: &PanelFit) -> bool {
    !fit.yielding || ctx.is_being_dragged(Id::new(id).with("__resize"))
}

fn loading_preview<T>(
    ctx: &Context,
    key: &str,
    details: &Loadable<T>,
    known: impl FnOnce() -> Option<T>,
) -> Option<Arc<T>>
where
    T: Send + Sync + 'static,
{
    let memory_id = Id::new("collection-loading-preview");
    let key = Id::new(key);
    if details.get().is_some() {
        ctx.data(|data| data.get_temp::<(Id, Arc<T>)>(memory_id))
            .filter(|(stored_key, _)| *stored_key == key)
            .map(|(_, preview)| preview)
    } else {
        let preview = known().map(Arc::new);
        ctx.data_mut(|data| match &preview {
            Some(preview) => {
                data.insert_temp(memory_id, (key, Arc::clone(preview)));
            }
            None => data.remove::<(Id, Arc<T>)>(memory_id),
        });
        preview
    }
}

fn page_tint(app: &mut App, ctx: &egui::Context) -> Option<Color32> {
    let page = app.page().clone();
    if page == Page::Home {
        let target = ctx
            .data(|data| data.get_temp::<Color32>(Id::new(home::HOME_HOVER_TINT_ID)))
            .unwrap_or(Color32::from_rgb(0x50, 0x38, 0xc8));
        let channel = |axis, value: u8| {
            ctx.animate_value_with_time(
                Id::new(("home-tint-channel", axis)),
                f32::from(value),
                crate::motion::seconds(ctx, 0.3),
            )
            .round() as u8
        };
        return Some(Color32::from_rgb(
            channel("r", target.r()),
            channel("g", target.g()),
            channel("b", target.b()),
        ));
    }
    let image = match &page {
        Page::Playlist(id) => app
            .playlist_pages
            .get(id)
            .and_then(|page| page.playlist.get())
            .or_else(|| app.known_playlist(id))
            .and_then(|playlist| pick_image(&playlist.images, 64))
            .map(str::to_string),
        Page::Album(id) => app
            .album_pages
            .get(id)
            .and_then(|page| page.album.get())
            .or_else(|| app.known_album(id))
            .and_then(|album| pick_image(&album.images, 64))
            .map(str::to_string),
        Page::Artist(id) => app
            .artist_pages
            .get(id)
            .and_then(|page| page.artist.get())
            .or_else(|| app.known_artist(id))
            .and_then(|artist| pick_image(&artist.images, 64))
            .map(str::to_string),
        Page::Show(id) => app
            .show_pages
            .get(id)
            .and_then(|page| page.show.get())
            .or_else(|| app.known_show(id))
            .and_then(|show| pick_image(&show.images, 64))
            .map(str::to_string),
        Page::Radio(seed) => pick_image(&app.radio_images(seed), 64).map(str::to_string),
        Page::LikedSongs => {
            return app
                .settings
                .accent_from_art
                .then_some(Color32::from_rgb(0x50, 0x38, 0xc8));
        }
        _ => None,
    };
    if !app.settings.accent_from_art && image.is_some() {
        return None;
    }
    match image {
        Some(url) => app.tint_for(Some(&url)).or_else(|| app.now_playing_tint()),
        None => app.now_playing_tint(),
    }
}

fn paint_corner_cutouts(ui: &egui::Ui, rect: Rect, left: bool, color: Color32) {
    let radius = theme::RADIUS as f32;
    let inward = if left { 1.0 } else { -1.0 };
    let edge = if left { rect.left() } else { rect.right() };
    for (y, direction) in [(rect.top(), 1.0), (rect.bottom(), -1.0)] {
        let corner = pos2(edge, y);
        let center = corner + vec2(inward * radius, direction * radius);
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(corner, color);
        for step in 0..=12 {
            let angle = std::f32::consts::FRAC_PI_2 * step as f32 / 12.0;
            mesh.colored_vertex(
                center
                    + vec2(
                        -inward * radius * angle.sin(),
                        -direction * radius * angle.cos(),
                    ),
                color,
            );
            if step > 0 {
                mesh.add_triangle(0, step, step + 1);
            }
        }
        ui.painter().add(egui::Shape::mesh(mesh));
    }
}

#[cfg(test)]
mod corner_cutout_tests {
    use super::*;

    #[test]
    fn corner_cutouts_cover_the_outside_of_each_rounded_edge() {
        let ctx = Context::default();
        let rect = Rect::from_min_max(pos2(100.0, 100.0), pos2(200.0, 300.0));
        let radius = theme::RADIUS as f32;
        for left in [false, true] {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                paint_corner_cutouts(ui, rect, left, Color32::BLACK);
            });
            let meshes: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Mesh(mesh) => Some(mesh),
                    _ => None,
                })
                .collect();
            assert_eq!(meshes.len(), 2);
            let inside_triangle = |p: egui::Pos2, a: egui::Pos2, b: egui::Pos2, c: egui::Pos2| {
                let cross = |u: egui::Vec2, v: egui::Vec2| u.x * v.y - u.y * v.x;
                let d = [
                    cross(b - a, p - a),
                    cross(c - b, p - b),
                    cross(a - c, p - c),
                ];
                cross(b - a, c - a).abs() > 0.0001
                    && (d.iter().all(|d| *d >= -0.001) || d.iter().all(|d| *d <= 0.001))
            };
            for (mesh, (y, direction)) in meshes
                .iter()
                .zip([(rect.top(), 1.0), (rect.bottom(), -1.0)])
            {
                assert!(mesh.is_valid());
                assert!(
                    mesh.vertices
                        .iter()
                        .all(|vertex| vertex.color == Color32::BLACK)
                );
                let inward = if left { 1.0 } else { -1.0 };
                let edge = if left { rect.left() } else { rect.right() };
                let center = pos2(edge + inward * radius, y + direction * radius);
                for dx in 0..theme::RADIUS {
                    for dy in 0..theme::RADIUS {
                        let point = pos2(
                            edge + inward * (f32::from(dx) + 0.5),
                            y + direction * (f32::from(dy) + 0.5),
                        );
                        let outside = (point - center).length_sq() - radius * radius;
                        if outside.abs() < 0.5 {
                            continue;
                        }
                        let covered = mesh.indices.as_chunks::<3>().0.iter().any(|triangle| {
                            inside_triangle(
                                point,
                                mesh.vertices[triangle[0] as usize].pos,
                                mesh.vertices[triangle[1] as usize].pos,
                                mesh.vertices[triangle[2] as usize].pos,
                            )
                        });
                        assert_eq!(
                            covered,
                            outside > 0.0,
                            "wrong corner coverage at {point:?}, left={left}"
                        );
                    }
                }
            }
            output.textures_delta.clear();
        }
    }
}

fn collapsed_bento_rect(body: Rect, hover_t: f32) -> Rect {
    Rect::from_min_max(
        pos2(body.right() - (32.0 + 16.0 * hover_t), body.top()),
        body.max,
    )
}

/// Official Spotify retracted Right Bento Strip (`Tela Tocando Agora` collapsed).
/// Shows a theme-colored rounded strip over the central content with `<` (`ChevronLeft`).
/// On hover, the strip and its black gap overlay the fixed central panel, revealing a dimmed preview of the
/// Now Playing View content behind a bright `<` button (`"Mostrar a tela Tocando agora"`).
/// Unified Right Bento Panel (Now Playing View, Queue, Connect Devices, Friend Activity, and Collapsed Strip).
pub fn right_bento_panel(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let palette = app.palette;

    let is_overlay = app.show_devices || app.show_queue_panel || app.show_friend_activity;
    let is_expanded = is_overlay || app.show_lyrics_panel;

    if app.lyrics_fullscreen.is_some() {
        return;
    }

    let expand_t = ctx.animate_bool_with_time_and_easing(
        Id::new("right-bento-expand-t"),
        is_expanded,
        0.22,
        egui::emath::easing::cubic_out,
    );

    let chosen_width = app.settings.lyrics_width.clamp(340.0, 520.0);
    app.settings.queue_width = chosen_width;

    let hover_anim_id = Id::new("npv-collapsed-hover-anim");
    let last_strip_rect_id = Id::new("npv-collapsed-strip-last-rect");
    let last_strip_rect: Option<Rect> = ctx.data(|d| d.get_temp(last_strip_rect_id));

    let is_strip_hovered = if !is_expanded {
        if let Some(r) = last_strip_rect {
            ctx.input(|i| i.pointer.hover_pos().is_some_and(|pos| r.contains(pos)))
        } else if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
            let screen = ctx.input(|i| i.viewport_rect());
            pos.x >= screen.right() - 32.0 - theme::PANEL_GAP
                && pos.x <= screen.right()
                && pos.y >= theme::top_bar_height()
                && pos.y <= screen.bottom() - theme::PLAYER_BAR_HEIGHT
        } else {
            false
        }
    } else {
        false
    };

    let hover_t = ctx.animate_bool_with_time_and_easing(
        hover_anim_id,
        is_strip_hovered,
        0.24,
        egui::emath::easing::quadratic_out,
    );
    ctx.data_mut(|d| d.insert_temp(hover_anim_id, hover_t));

    let panel_width = chosen_width * expand_t;

    let is_fully_expanded = expand_t >= 0.999;
    let outer_rect = ui.available_rect_before_wrap();
    if expand_t <= 0.02 {
        let rect = collapsed_bento_rect(outer_rect, hover_t);
        let overlay_rect =
            Rect::from_min_max(pos2(rect.left() - theme::PANEL_GAP, rect.top()), rect.max);
        ctx.data_mut(|d| d.insert_temp(last_strip_rect_id, overlay_rect));
        egui::Area::new(Id::new("collapsed-right-bento-overlay"))
            .order(egui::Order::Foreground)
            .movable(false)
            .constrain(false)
            .fixed_pos(overlay_rect.min)
            .show(&ctx, |ui| {
                ui.set_min_size(overlay_rect.size());
                ui.set_clip_rect(overlay_rect.expand2(vec2(theme::RADIUS as f32, 0.0)));
                // The separator travels with the strip and covers the central card beneath it.
                let gap_rect =
                    Rect::from_min_max(overlay_rect.min, pos2(rect.left(), rect.bottom()));
                ui.painter().rect_filled(gap_rect, 0.0, palette.window);
                paint_corner_cutouts(
                    ui,
                    Rect::from_min_max(
                        outer_rect.min,
                        pos2(overlay_rect.left(), outer_rect.bottom()),
                    ),
                    false,
                    palette.window,
                );

                let resp = ui.interact(rect, Id::new("collapsed-right-bento-open"), Sense::click());
                ui.set_clip_rect(rect);

                ui.painter().rect_filled(
                    rect,
                    CornerRadius {
                        nw: theme::RADIUS,
                        ne: 0,
                        sw: theme::RADIUS,
                        se: 0,
                    },
                    palette.panel,
                );

                // 2. Faint Now Playing View preview on hover (~18% opacity)
                if hover_t > 0.01 && app.now_playing().is_some() {
                    let preview_rect = Rect::from_min_size(
                        pos2(rect.left(), rect.top()),
                        vec2(chosen_width, rect.height()),
                    );
                    let mut preview_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(preview_rect)
                            .layout(Layout::top_down(Align::Min)),
                    );
                    preview_ui.set_clip_rect(rect);
                    preview_ui.disable();
                    preview_ui.set_opacity(hover_t * 0.18);
                    lyrics::now_playing_view_content(app, &mut preview_ui);
                }

                // Clear the corners after the preview so its artwork cannot leak through.
                paint_corner_cutouts(ui, rect, true, palette.window);

                // 3. Bare chevron icon centered over the strip
                let icon_color = blend(palette.secondary, palette.text, hover_t);
                let icon_rect = Rect::from_center_size(rect.center(), vec2(16.0, 16.0));
                theme::paint_icon(ui, Icon::ChevronLeft, icon_rect, 16.0, icon_color);

                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                let tooltip = gettext(app.locale, "Show Now Playing view");
                if resp.on_hover_text(tooltip.as_ref()).clicked() {
                    app.show_lyrics_panel = true;
                }
            });
        return;
    }

    let mut panel = egui::Panel::right("unified-right-bento")
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.window).inner_margin(Margin {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        }));

    if is_fully_expanded {
        panel = panel
            .resizable(true)
            .size_range(340.0..=520.0)
            .default_size(chosen_width);
    } else {
        ctx.data_mut(|d| {
            d.remove::<egui::containers::panel::PanelState>(Id::new("unified-right-bento"))
        });
        panel = panel.exact_size(panel_width).resizable(false);
    }

    let response = panel.show(ui, |ui| {
        let outer_rect = ui.available_rect_before_wrap();
        ui.ctx().data_mut(|d| d.remove::<Rect>(last_strip_rect_id));
        Frame::new()
            .fill(palette.panel)
            .corner_radius(CornerRadius {
                nw: theme::RADIUS,
                ne: 0,
                sw: theme::RADIUS,
                se: 0,
            })
            .show(ui, |ui| {
                ui.set_min_size(outer_rect.size());
                ui.set_max_size(outer_rect.size());
                ui.set_clip_rect(outer_rect);

                // Fixed-width content pinned to outer_rect.left():
                // As the panel collapses from chosen_width to 36px, the content smoothly slides off to the right!
                let content_rect = Rect::from_min_size(
                    pos2(outer_rect.left(), outer_rect.top()),
                    vec2(chosen_width, outer_rect.height()),
                );

                let overlay_t = ctx.animate_bool_with_time_and_easing(
                    Id::new("right-bento-overlay-t"),
                    is_overlay,
                    0.22,
                    egui::emath::easing::cubic_out,
                );

                let mut content_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(content_rect)
                        .layout(Layout::top_down(Align::Min)),
                );
                content_ui.set_clip_rect(outer_rect);

                if overlay_t <= 0.001 {
                    lyrics::now_playing_view_content(app, &mut content_ui);
                } else if overlay_t >= 0.999 {
                    render_overlay_content(app, &mut content_ui);
                } else {
                    // In transition: base Now Playing View in background
                    {
                        let mut base_ui = content_ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(content_rect)
                                .layout(Layout::top_down(Align::Min)),
                        );
                        base_ui.set_clip_rect(outer_rect);
                        lyrics::now_playing_view_content(app, &mut base_ui);
                    }
                    // Overlay slides up from bottom
                    let slide_y = (1.0 - overlay_t) * content_rect.height();
                    let overlay_rect = content_rect.translate(vec2(0.0, slide_y));
                    let mut overlay_ui = content_ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(overlay_rect)
                            .layout(Layout::top_down(Align::Min)),
                    );
                    overlay_ui.set_clip_rect(outer_rect);
                    overlay_ui.painter().rect_filled(
                        overlay_rect,
                        CornerRadius::same(theme::RADIUS),
                        palette.panel,
                    );
                    render_overlay_content(app, &mut overlay_ui);
                }
            });
    });

    let is_resizing = ctx.is_being_dragged(Id::new("unified-right-bento").with("__resize"));
    if is_fully_expanded && is_resizing {
        let actual_width = response.response.rect.width();
        let clamped = actual_width.clamp(340.0, 520.0);
        if (clamped - app.settings.lyrics_width).abs() > 0.5 {
            app.settings.lyrics_width = clamped;
            app.settings.queue_width = clamped;
        }
    } else if is_fully_expanded && ctx.input(|i| i.pointer.any_released()) {
        app.actions.push(Action::SettingsChanged);
    }
}

fn render_overlay_content(app: &mut App, ui: &mut egui::Ui) {
    Frame::new()
        .inner_margin(Margin {
            left: 20,
            right: 20,
            top: 20,
            bottom: 16,
        })
        .show(ui, |ui| {
            if app.show_devices {
                devices::content(app, ui);
            } else if app.show_queue_panel {
                queue::content(app, ui);
            } else if app.show_friend_activity {
                friend_activity_content(app, ui);
            }
        });
}

pub fn friend_activity_content(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    ui.horizontal(|ui| {
        theme::text(
            ui,
            gettext(locale, "Listening activity"),
            theme::bold(16.0),
            palette.text,
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::icon_button(
                ui,
                Icon::X,
                16.0,
                palette.secondary,
                palette.text,
                &gettext(locale, "Close"),
            )
            .clicked()
            {
                app.actions.push(Action::ToggleFriendActivity);
            }
        });
    });
    ui.add_space(32.0);
    Frame::new().inner_margin(Margin::symmetric(16, 0)).show(ui, |ui| {
        ui.add(egui::Label::new(egui::RichText::new(gettext(locale, "Let your friends and followers on Spotify see what you're listening to.")).font(theme::medium(14.0)).color(palette.secondary)).wrap());
        ui.add_space(18.0);
        for index in 0..3 {
            ui.horizontal(|ui| {
                let (avatar, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
                ui.painter().circle_filled(avatar.center(), 24.0, palette.surface_hover);
                if index == 0 {
                    theme::paint_icon(ui, Icon::User, avatar.shrink(12.0), 24.0, palette.secondary);
                }
                if index < 2 {
                    ui.painter().circle_filled(pos2(avatar.right() - 4.0, avatar.top() + 4.0), 4.0, Color32::from_rgb(45, 126, 221));
                }
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    for width in [80.0, 62.0, 62.0] {
                        let (line, _) = ui.allocate_exact_size(vec2(width, 7.0), Sense::hover());
                        ui.painter().rect_filled(line, 4.0, palette.surface_hover);
                    }
                });
            });
            ui.add_space(12.0);
        }
        ui.add(egui::Label::new(egui::RichText::new(gettext(locale, "Go to Settings > Listening activity and insights in the Spotify app to manage your listening activity.")).font(theme::medium(14.0)).color(palette.secondary)).wrap());
        ui.add_space(8.0);
        ui.add(egui::Label::new(egui::RichText::new(gettext(locale, "Real friend activity is not available to this client through Spotify.")).font(theme::regular(12.0)).color(palette.secondary)).wrap());
        ui.add_space(18.0);
        ui.vertical_centered(|ui| {
            if ui.add(egui::Button::new(egui::RichText::new(gettext(locale, "Settings"))
                .font(theme::semibold(14.0)).color(palette.window))
                .fill(palette.text).stroke(Stroke::NONE).corner_radius(24.0)
                .min_size(vec2(158.0, 48.0))).clicked() {
                app.actions.push(Action::OpenUrl("https://support.spotify.com/article/listening-activity/".into()));
            }
        });
    });
}

fn whats_new_page(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let filter_id = Id::new("whats-new-filter");
    let mut filter = ui
        .ctx()
        .data(|d| d.get_temp::<u8>(filter_id))
        .unwrap_or(0);

    ui.add_space(8.0);
    theme::text(ui, gettext(app.locale, "What's New"), theme::bold(32.0), palette.text);
    ui.add_space(4.0);
    theme::text(
        ui,
        gettext(app.locale, "The latest releases from artists, podcasts and shows you follow."),
        theme::regular(14.0),
        palette.secondary,
    );
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (idx, label) in [(1u8, gettext(app.locale, "Music")), (2u8, gettext(app.locale, "Podcasts and shows"))] {
            let active = filter == idx;
            if theme::pill_button(ui, &palette, &label, active).clicked() {
                filter = if active { 0 } else { idx };
                ui.ctx().data_mut(|d| d.insert_temp(filter_id, filter));
            }
        }
    });
    ui.add_space(72.0);
    ui.vertical_centered(|ui| {
        theme::text(
            ui,
            gettext(app.locale, "There are no new releases for you right now."),
            theme::bold(24.0),
            palette.text,
        );
        ui.add_space(8.0);
        theme::text(
            ui,
            gettext(app.locale, "New releases will appear here. Follow your favorite artists and podcasts to stay up to date."),
            theme::regular(14.0),
            palette.secondary,
        );
    });
}

fn central(app: &mut App, ui: &mut egui::Ui) {
    let _ = (linux_left_controls_inset, window_controls_reservation);
    let palette = app.palette;
    let tint = page_tint(app, ui.ctx());
    let gap = theme::PANEL_GAP as i8;
    let radius = theme::RADIUS;
    egui::CentralPanel::default()
        .frame(Frame::new().fill(palette.window).inner_margin(Margin {
            left: gap,
            right: gap,
            top: 0,
            bottom: 0,
        }))
        .show(ui, |ui| {
            let outer_rect = ui.available_rect_before_wrap();
            // Paint card background strictly on outer_rect so it can NEVER invade the 8px black gap!
            ui.painter()
                .rect_filled(outer_rect, CornerRadius::same(radius), palette.panel);
            ui.set_clip_rect(outer_rect);
            let rect = outer_rect;
            let page = app.page().clone();
            let overview = matches!(
                page,
                Page::Home | Page::Search | Page::Settings | Page::Queue | Page::WhatsNew
            );
            let scrolled_id = Id::new(("page-scrolled", page.encode()));
            let scrolled = ui
                .ctx()
                .data(|data| data.get_temp::<f32>(scrolled_id))
                .unwrap_or(0.0);
            if let Some((top, _)) = header_wash(&palette, overview, tint) {
                let fade = (1.0 - scrolled / 260.0).clamp(0.0, 1.0);
                if fade > 0.01 {
                    let wash_rect =
                        Rect::from_min_size(rect.min, vec2(rect.width(), 340.0)).intersect(rect);
                    if wash_rect.height() > 0.0 {
                        widgets::paint_vertical_gradient_rounded(
                            ui,
                            wash_rect,
                            CornerRadius {
                                nw: radius,
                                ne: radius,
                                sw: 0,
                                se: 0,
                            },
                            top.gamma_multiply(fade),
                            Color32::TRANSPARENT,
                        );
                    }
                }
            }
            ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
            ui.spacing_mut().scroll.fade.strength = 0.0;

            // Middle section scrollbar 3-second persistence upon pointer exit
            let center_hovered = ui.rect_contains_pointer(rect);
            let last_hover_id = Id::new("central_scrollbar_last_active");
            let now_inst = std::time::Instant::now();
            let last_active = if center_hovered {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(last_hover_id, now_inst));
                now_inst
            } else {
                ui.ctx()
                    .data(|d| d.get_temp::<std::time::Instant>(last_hover_id))
                    .unwrap_or(now_inst - std::time::Duration::from_secs(10))
            };
            let elapsed = last_active.elapsed().as_secs_f32();
            let scroll_alpha = if elapsed < 3.0 {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(50));
                1.0f32
            } else if elapsed < 3.4 {
                let fade = (elapsed - 3.0) / 0.4;
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(16));
                (1.0 - fade).clamp(0.0, 1.0)
            } else {
                0.0f32
            };
            ui.spacing_mut().scroll.floating = true;
            let scroll_style = ui.spacing().scroll;
            ui.spacing_mut().scroll.dormant_handle_opacity =
                scroll_style.active_handle_opacity * scroll_alpha;
            ui.spacing_mut().scroll.active_handle_opacity =
                scroll_style.active_handle_opacity * scroll_alpha;
            ui.spacing_mut().scroll.interact_handle_opacity = if scroll_alpha > 0.01 {
                scroll_style.interact_handle_opacity
            } else {
                0.0
            };
            ui.spacing_mut().scroll.dormant_background_opacity = 0.0;
            ui.spacing_mut().scroll.active_background_opacity = 0.0;
            ui.spacing_mut().scroll.interact_background_opacity = 0.0;
            ui.spacing_mut().scroll.floating_allocated_width = 0.0;
            ui.spacing_mut().scroll.floating_width = 8.0;
            ui.spacing_mut().scroll.bar_width = 8.0;
            let scrollbar_inset = ui
                .ctx()
                .data(|data| data.get_temp::<Rect>(Id::new("npv-collapsed-strip-last-rect")))
                .map_or(0.0, |overlay| {
                    let resting_left =
                        collapsed_bento_rect(overlay, 0.0).left() - theme::PANEL_GAP;
                    (outer_rect.right() - resting_left).max(0.0)
                });
            // The hover overlay grows over the scrollbar's fixed resting position.
            ui.spacing_mut().scroll.bar_outer_margin = scrollbar_inset + 2.0;
            ui.spacing_mut().scroll.bar_inner_margin = 0.0;

            // Smooth page switching transition
            let last_page = ui
                .ctx()
                .data(|d| d.get_temp::<Page>(Id::new("central_last_page")));
            let page_changed = last_page.as_ref() != Some(&page);
            let start_id = Id::new("central_page_transition_start");
            let start_time = if page_changed {
                ui.ctx().data_mut(|d| {
                    d.insert_temp(Id::new("central_last_page"), page.clone());
                    d.insert_temp(start_id, now_inst);
                });
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(16));
                now_inst
            } else {
                ui.ctx()
                    .data(|d| d.get_temp::<std::time::Instant>(start_id))
                    .unwrap_or(now_inst)
            };
            let page_opacity = if page_changed {
                0.0f32
            } else {
                let elapsed = start_time.elapsed().as_secs_f32();
                let dur = 0.18f32;
                let t = (elapsed / dur).min(1.0);
                if t < 1.0 {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(16));
                }
                let inv = 1.0 - t;
                1.0 - inv * inv * inv
            };

            let scroll = crate::autoscroll::show(
                ui,
                egui::ScrollArea::vertical()
                    .id_salt(("page", page.encode()))
                    .auto_shrink([false, false]),
                egui::Vec2b::new(false, true),
                |ui| {
                    ui.set_clip_rect(outer_rect);
                    let page_padding = if page == Page::Home {
                        40.0
                    } else {
                        widgets::PAGE_PADDING
                    };
                    let content_w = (rect.width() - page_padding * 2.0).max(100.0);
                    Frame::new()
                        .inner_margin(Margin {
                            left: page_padding as i8,
                            right: page_padding as i8,
                            top: 16,
                            bottom: 48,
                        })
                        .show(ui, |ui| {
                            ui.set_width(content_w);
                            ui.multiply_opacity(page_opacity);
                            match page {
                                Page::Home => home::show(app, ui),
                                Page::WhatsNew => whats_new_page(app, ui),
                                Page::TopSongs => collection::top_songs(app, ui),
                                Page::Search => search::show(app, ui),
                                Page::LikedSongs => collection::liked(app, ui),
                                Page::Albums | Page::Artists | Page::Podcasts | Page::Episodes => {
                                    library::show(app, ui, page)
                                }
                                Page::Playlist(id) => collection::playlist(app, ui, &id),
                                Page::Album(id) => collection::album(app, ui, &id),
                                Page::Artist(id) => artist::show(app, ui, &id),
                                Page::Show(id) => show::show(app, ui, &id),
                                Page::Radio(seed) => radio::radio(app, ui, &seed),
                                Page::Queue => queue::page(app, ui),
                                Page::Settings => settings::show(app, ui),
                            }
                        });
                },
            );
            ui.ctx()
                .data_mut(|data| data.insert_temp(scrolled_id, scroll.state.offset.y));
        });
}

#[cfg(test)]
fn header_rect(panel: Rect, scrolled: f32) -> Rect {
    let top = panel.min.y - scrolled.max(0.0);
    Rect::from_min_size(egui::pos2(panel.min.x, top), vec2(panel.width(), 340.0))
}

/// The gradient behind a page's header, top to bottom: the art's colour
/// blended into the window, stronger on a cover's own page than on an
/// overview, or else the palette's `tint` fading out, whose alpha is its
/// strength. `None` leaves the plain window.
fn header_wash(
    palette: &theme::Palette,
    overview: bool,
    art: Option<Color32>,
) -> Option<(Color32, Color32)> {
    if let Some(art) = art {
        let strength = if overview { 0.45 } else { 0.85 };
        return Some((blend(palette.window, art, strength), palette.window));
    }
    (palette.tint.a() > 0).then_some((palette.tint, Color32::TRANSPARENT))
}


/// Without a native title bar, the top of every panel drags the window, not
/// only the page's top bar.
const fn titlebar_spans_window(on_macos: bool, custom_titlebar: bool) -> bool {
    on_macos || custom_titlebar
}

/// The room the top bar's own controls need beside the caption buttons: the
/// 760-point minimum with the default 250-point sidebar leaves 510 points.
const TOPBAR_MIN_CONTENT: f32 = 510.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WindowControlsReservation {
    pub topbar_width: f32,
    pub topbar_top: f32,
    pub queue_top: f32,
    pub lyrics_top: f32,
}

/// The caption buttons on the right belong to the outermost header: the
/// queue's or the lyrics' when either panel is open, otherwise the top bar,
/// beside its controls or above them when the page is too narrow.
fn controls_reservation(
    footprint: fastframe_titlebar::Footprint,
    queue: bool,
    lyrics: bool,
    topbar_width: f32,
) -> WindowControlsReservation {
    let mut space = WindowControlsReservation {
        topbar_width: 0.0,
        topbar_top: 0.0,
        queue_top: 0.0,
        lyrics_top: 0.0,
    };
    if footprint.right <= 0.0 {
        return space;
    }
    if queue {
        space.queue_top = footprint.height;
    } else if lyrics {
        space.lyrics_top = footprint.height;
    } else {
        let reserve = footprint.reserve(topbar_width, TOPBAR_MIN_CONTENT);
        space.topbar_width = reserve.inline_width;
        space.topbar_top = reserve.push_down;
    }
    space
}

/// Left-hand Linux controls sit above the sidebar's navigation rows.
pub(super) fn linux_left_controls_inset(ctx: &egui::Context) -> f32 {
    if !cfg!(target_os = "linux") {
        return 0.0;
    }
    let footprint = fastframe_titlebar::footprint(ctx, theme::CUSTOM_TOP_BAR_HEIGHT);
    if footprint.left > 0.0 {
        footprint.height
    } else {
        0.0
    }
}

pub(super) fn window_controls_reservation(
    ctx: &egui::Context,
    queue: bool,
    lyrics: bool,
    topbar_width: f32,
) -> WindowControlsReservation {
    controls_reservation(
        fastframe_titlebar::footprint(ctx, theme::CUSTOM_TOP_BAR_HEIGHT),
        queue,
        lyrics,
        topbar_width,
    )
}

/// Draws the caption buttons over the window's top corners in the palette's
/// colours: Windows' glyphs in the secondary text colour, GNOME's round
/// buttons in the text colour on the hover surface.
fn caption_buttons(ui: &mut egui::Ui, palette: &theme::Palette, locale: crate::i18n::Locale) {
    use crate::i18n::gettext;
    let style = fastframe_titlebar::Style {
        icon: if cfg!(target_os = "linux") {
            palette.text
        } else {
            palette.secondary
        },
        button: palette.surface_hover,
        header_height: theme::CUSTOM_TOP_BAR_HEIGHT,
    };
    // Literal calls, so xgettext extracts each label.
    let minimize = gettext(locale, "Minimize");
    let maximize = gettext(locale, "Maximize");
    let restore = gettext(locale, "Restore");
    let close = gettext(locale, "Close");
    let labels = fastframe_titlebar::Labels {
        minimize: &minimize,
        maximize: &maximize,
        restore: &restore,
        close: &close,
    };
    fastframe_titlebar::controls(ui, &style, &labels);
}

pub fn blend(base: Color32, tint: Color32, amount: f32) -> Color32 {
    let a = egui::Rgba::from(base);
    let b = egui::Rgba::from(tint);
    let mixed = a * (1.0 - amount) + b * amount;
    let mut color = Color32::from(mixed);
    color[3] = 255;
    color
}

fn toasts(app: &mut App, ctx: &egui::Context, bottom_offset: f32) {
    if app.toasts.is_empty() {
        return;
    }
    let palette = app.palette;
    egui::Area::new(egui::Id::new("toasts"))
        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -bottom_offset))
        .order(egui::Order::Tooltip)
        .interactable(false)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            for toast in &app.toasts {
                let age = toast.created.elapsed().as_secs_f32();
                let alpha = if age < 0.15 {
                    age / 0.15
                } else if age > 2.8 {
                    ((3.2 - age) / 0.4).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                ui.set_opacity(alpha);
                Frame::new()
                    .fill(if toast.kind == ToastKind::Info {
                        Color32::WHITE
                    } else {
                        palette.overlay
                    })
                    .stroke(Stroke::new(1.0, palette.outline))
                    .corner_radius(CornerRadius::same(theme::RADIUS))
                    .inner_margin(Margin::symmetric(14, 10))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 16,
                        spread: 0,
                        color: palette.shadow,
                    })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (icon, color) = match toast.kind {
                                ToastKind::Info => (Icon::CircleCheck, palette.accent),
                                ToastKind::Error => (Icon::CircleAlert, palette.danger),
                            };
                            if toast.kind == ToastKind::Error {
                                theme::icon(ui, icon, 16.0, color);
                            }
                            // Laid out at its own width. The area
                            // remembers its size, so after a short toast a
                            // label left to wrap at the area's width broke
                            // long messages on every word.
                            let galley = ui.painter().layout(
                                toast.message.clone(),
                                theme::medium(13.5),
                                if toast.kind == ToastKind::Info {
                                    Color32::BLACK
                                } else {
                                    palette.text
                                },
                                440.0,
                            );
                            ui.add(egui::Label::new(galley));
                        });
                    });
            }
        });
}

#[cfg(test)]
mod window_chrome_tests {
    use super::*;
    use fastframe_titlebar::Footprint;

    /// Windows' three caption buttons.
    const BUTTONS: Footprint = Footprint {
        left: 0.0,
        right: 3.0 * 36.0 + 5.0,
        height: theme::CUSTOM_TOP_BAR_HEIGHT,
    };

    #[test]
    fn the_side_panels_drag_a_window_without_a_native_title_bar() {
        assert!(titlebar_spans_window(true, false));
        assert!(titlebar_spans_window(false, true));
        assert!(!titlebar_spans_window(false, false));
    }

    #[test]
    fn caption_space_belongs_to_the_outermost_header() {
        let values = |footprint, queue, lyrics| {
            let space = controls_reservation(footprint, queue, lyrics, f32::INFINITY);
            [
                space.topbar_width,
                space.topbar_top,
                space.queue_top,
                space.lyrics_top,
            ]
        };
        let [width, height] = [BUTTONS.right, BUTTONS.height];
        assert_eq!(values(BUTTONS, false, false), [width, 0.0, 0.0, 0.0]);
        assert_eq!(values(BUTTONS, true, false), [0.0, 0.0, height, 0.0]);
        assert_eq!(values(BUTTONS, false, true), [0.0, 0.0, 0.0, height]);
        assert_eq!(values(BUTTONS, true, true), [0.0, 0.0, height, 0.0]);
        // No buttons on the right: full screen, the native frame, macOS's
        // traffic lights, or a GNOME layout with them all on the left.
        let none = Footprint {
            left: 50.0,
            ..Footprint::default()
        };
        for (queue, lyrics) in [(false, false), (true, false), (false, true)] {
            assert_eq!(values(none, queue, lyrics), [0.0; 4]);
        }
    }

    #[test]
    fn minimum_windows_window_stacks_caption_space_above_the_topbar() {
        let available = 760.0 - 250.0;
        let space = controls_reservation(BUTTONS, false, false, available);
        assert_eq!(space.topbar_width, 0.0);
        assert_eq!(space.topbar_top, BUTTONS.height);

        let inline =
            controls_reservation(BUTTONS, false, false, TOPBAR_MIN_CONTENT + BUTTONS.right);
        assert_eq!(inline.topbar_width, BUTTONS.right);
        assert_eq!(inline.topbar_top, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page fades from its cover's colour while art tints the interface,
    /// otherwise from the palette's own wash, and from nothing when the
    /// palette has none.
    #[test]
    fn the_header_fades_from_the_art_or_the_palettes_wash() {
        let mut palette = theme::Palette::dark();
        assert_eq!(header_wash(&palette, false, None), None);

        let art = Color32::from_rgb(0x50, 0x38, 0xc8);
        let (page, _) = header_wash(&palette, false, Some(art)).unwrap();
        let (overview, _) = header_wash(&palette, true, Some(art)).unwrap();
        assert_eq!(page, blend(palette.window, art, 0.85));
        assert_eq!(overview, blend(palette.window, art, 0.45));

        palette.tint = Color32::from_black_alpha(24);
        assert_eq!(
            header_wash(&palette, false, None),
            Some((palette.tint, Color32::TRANSPARENT))
        );
        assert_eq!(
            header_wash(&palette, false, Some(art)).unwrap().0,
            page,
            "art wins over the wash"
        );
    }

    /// The wash sits at the top of an unscrolled page and leaves with the
    /// hero as the page scrolls, instead of staying behind the page.
    #[test]
    fn the_header_wash_scrolls_away_with_the_page() {
        let panel = Rect::from_min_size(egui::pos2(0.0, 80.0), vec2(800.0, 600.0));
        assert_eq!(header_rect(panel, 0.0).min, panel.min);
        assert_eq!(header_rect(panel, 0.0).width(), 800.0);
        assert_eq!(header_rect(panel, 120.0).min.y, panel.min.y - 120.0);
        assert_eq!(header_rect(panel, 120.0).max.y, panel.min.y + 220.0);
        assert!(
            header_rect(panel, 400.0).max.y < panel.min.y,
            "gone once scrolled past"
        );
        assert_eq!(
            header_rect(panel, -5.0).min,
            panel.min,
            "no overscroll bounce"
        );
    }
}
