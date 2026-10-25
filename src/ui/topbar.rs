//! Navigation arrows, search, and the account menu above every page.

use std::sync::Arc;

use egui::{Align, CornerRadius, Galley, Layout, Sense, Vec2, pos2, vec2};

use crate::api::models::pick_image;
use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, BackgroundActivity, Page};
use crate::theme::{self, Icon, Palette};
use super::widgets;

/// The gap the bar keeps between everything it lays out.
const ITEM_SPACING: f32 = 8.0;
/// The account avatar, and the icon in each of the three buttons beside it.
const AVATAR_SIZE: f32 = 36.0;
const ICON_BUTTON_ICON: f32 = 19.0;
/// `theme::icon_button` pads its icon by 12 px.
const ICON_BUTTON_SIZE: f32 = ICON_BUTTON_ICON + 12.0;
const SPINNER_SIZE: f32 = 15.0;
/// A badge is as tall as its text plus this, and as wide as its text plus
/// the padding its own label needs.
const BADGE_PADDING_Y: f32 = 12.0;
/// The width the search field aims for, the most it ever takes, and the
/// least it shrinks to before the badges give up their labels instead.
#[cfg(test)]
const SEARCH_IDEAL: f32 = 200.0;
#[cfg(test)]
const SEARCH_MAX: f32 = 440.0;
#[cfg(test)]
const SEARCH_FLOOR: f32 = 130.0;
// After the badges collapse, a right panel can leave less than 130 points.
// Keep the original 80-point minimum inside the page's own toolbar.
const SEARCH_MIN: f32 = 80.0;
/// Everything at the right end whose width never changes: the page padding,
/// the avatar, the gap the account menu leaves, the three icon buttons, and
/// the spacing between them. The cursor stops at the left edge of the last
/// button, so this counts three gaps, not four. The spinner and the badges
/// are measured on top of it because they come and go.
const RIGHT_CONTROLS_WIDTH: f32 =
    super::widgets::PAGE_PADDING + AVATAR_SIZE + 4.0 + 3.0 * ICON_BUTTON_SIZE + 3.0 * ITEM_SPACING;

/// What precedes the field until the bar has drawn once: the page padding,
/// the back and forward buttons and the gaps after them.
const LEAD_GUESS: f32 = super::widgets::PAGE_PADDING + 2.0 * 32.0 + 3.0 * ITEM_SPACING + 8.0;
/// A badge collapsed to its icon: a square as tall as its 12.5 pt label.
const BADGE_CHIP: f32 = 15.0 + BADGE_PADDING_Y;

fn lead_id() -> egui::Id {
    egui::Id::new("topbar-lead")
}

/// The narrowest the bar, and so the page under it, can be before its
/// controls run into each other: the narrowest field, with the spinner and
/// both badges as icons. Counting them even while they are away keeps the
/// panels and the window from changing width as they come and go.
pub fn least_width(ctx: &egui::Context) -> f32 {
    let lead = ctx
        .data(|data| data.get_temp(lead_id()))
        .unwrap_or(LEAD_GUESS);
    least_width_after(lead)
}

fn least_width_after(lead: f32) -> f32 {
    lead + SEARCH_MIN
        + RIGHT_CONTROLS_WIDTH
        + SPINNER_SIZE
        + ITEM_SPACING
        + 2.0 * (ITEM_SPACING + BADGE_CHIP)
}

/// How the top bar divides itself for one window width.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct TopbarFit {
    /// How wide the search field may be.
    search: f32,
    /// Whether the badges have the room to spell themselves out.
    labels: bool,
}

/// Divide the bar. The search field keeps the half it has always had, but
/// never so much that the right end has to reach over it, and the badges
/// fall back to their icons before the field shrinks past reading size.
///
/// `labelled` and `icons` are what the badges ask for with and without their
/// text, each already including the spacing that precedes it.
#[cfg(test)]
fn topbar_fit(room: f32, controls: f32, labelled: f32, icons: f32) -> TopbarFit {
    // SEARCH_IDEAL is above SEARCH_FLOOR, so the clamp below is well ordered.
    let ideal = (room * 0.5).clamp(SEARCH_IDEAL, SEARCH_MAX);
    let labels = room - controls - labelled >= SEARCH_FLOOR;
    let badges = if labels { labelled } else { icons };
    TopbarFit {
        search: (room - controls - badges).clamp(SEARCH_MIN, ideal),
        labels,
    }
}

/// What a badge asks of the bar, including the spacing before it.
fn badge_width(galley: Option<&Arc<Galley>>, padding: f32, labels: bool) -> f32 {
    galley.map_or(0.0, |galley| {
        ITEM_SPACING
            + if labels {
                galley.size().x + padding
            } else {
                galley.size().y + BADGE_PADDING_Y
            }
    })
}

/// A pill at the right end of the bar: an icon with its label, or the icon
/// alone once the bar is too narrow to spare the room for words.
fn badge(
    ui: &mut egui::Ui,
    palette: &Palette,
    icon: Icon,
    galley: Arc<Galley>,
    padding: f32,
    labels: bool,
) -> egui::Response {
    let height = galley.size().y + BADGE_PADDING_Y;
    let size = if labels {
        vec2(galley.size().x + padding, height)
    } else {
        Vec2::splat(height)
    };
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    // Collapsed to its icon the badge has no text on screen, so its label
    // reaches a screen reader as the widget's name.
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), galley.text())
    });
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(14),
        palette.accent.gamma_multiply(0.16),
    );
    let icon_center = if labels {
        pos2(rect.left() + 14.0, rect.center().y)
    } else {
        rect.center()
    };
    icon.image(palette.accent, 13.0).paint_at(
        ui,
        egui::Rect::from_center_size(icon_center, Vec2::splat(13.0)),
    );
    if labels {
        ui.painter().galley(
            pos2(rect.left() + 24.0, rect.center().y - galley.size().y / 2.0),
            galley,
            palette.accent,
        );
    }
    response
}

fn nav_button(
    ui: &mut egui::Ui,
    palette: &Palette,
    icon: Icon,
    enabled: bool,
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::splat(32.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if ui.is_rect_visible(rect) {
        let color = if !enabled {
            palette.dim
        } else if response.hovered() {
            palette.text
        } else {
            palette.secondary
        };
        theme::paint_icon(ui, icon, rect, 20.0, color);
    }
    if enabled {
        response.on_hover_text(tooltip)
    } else {
        response
    }
}

const RECENT_POPOVER_ID: &str = "search-recent-popover-open";

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let _ = (badge, badge_width);
    let palette = app.palette;
    let locale = app.locale;
    let width = ui.available_width();
    let footprint = fastframe_titlebar::footprint(ui.ctx(), theme::CUSTOM_TOP_BAR_HEIGHT);
    let right_caption_width = footprint.right;
    let left_caption_width = if cfg!(target_os = "linux") {
        footprint.left
    } else {
        0.0
    };
    let bar_height = theme::TOP_BAR_HEIGHT;
    let (bar_rect, _) = ui.allocate_exact_size(vec2(width, bar_height), Sense::hover());
    if fastframe_titlebar::enabled() {
        fastframe_titlebar::drag(ui, bar_rect);
    }

    // 1. LEFT CLUSTER: `...` (More menu), `<` (Back), `>` (Forward)
    let left_rect = egui::Rect::from_min_max(
        pos2(bar_rect.left() + 12.0 + left_caption_width, bar_rect.top()),
        pos2(
            (bar_rect.left() + 220.0).min(bar_rect.right()),
            bar_rect.bottom(),
        ),
    );
    let mut left_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(left_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    left_ui.spacing_mut().item_spacing.x = 6.0;
    let more_btn = nav_button(
        &mut left_ui,
        &palette,
        Icon::Ellipsis,
        true,
        &gettext(locale, "Menu"),
    );
    egui::Popup::menu(&more_btn)
        .frame(super::widgets::menu_frame(&palette))
        .align(egui::RectAlign::BOTTOM_START)
        .show(|ui| {
            ui.set_width(210.0);
            if super::widgets::menu_item(
                ui,
                &palette,
                Some(Icon::PanelLeft),
                &gettext(locale, "Your Library"),
            ) {
                app.actions.push(Action::ToggleSidebar);
            }
            if super::widgets::menu_item(
                ui,
                &palette,
                Some(Icon::AudioLines),
                &gettext(locale, "MilkDrop visualiser (Ctrl+Shift+K)"),
            ) {
                app.actions.push(Action::ToggleWinampMilkdrop);
            }
            if super::widgets::menu_item(
                ui,
                &palette,
                Some(Icon::Miniplayer),
                &gettext(locale, "Open Miniplayer"),
            ) {
                app.actions.push(Action::ToggleWinampWindow);
            }
            super::widgets::menu_separator(ui, &palette);
            if super::widgets::menu_item(
                ui,
                &palette,
                Some(Icon::Settings),
                &gettext(locale, "Settings"),
            ) {
                app.actions.push(Action::Open(Page::Settings));
            }
            if super::widgets::menu_item(
                ui,
                &palette,
                Some(Icon::Info),
                &gettext(locale, "Keyboard shortcuts"),
            ) {
                app.actions
                    .push(Action::ShowDialog(crate::model::Dialog::Shortcuts));
            }
        });

    if nav_button(
        &mut left_ui,
        &palette,
        Icon::ChevronLeft,
        app.can_go_back(),
        &gettext(locale, "Go back"),
    )
    .clicked()
    {
        app.actions.push(Action::Back);
    }
    if nav_button(
        &mut left_ui,
        &palette,
        Icon::ChevronRight,
        app.can_go_forward(),
        &gettext(locale, "Go forward"),
    )
    .clicked()
    {
        app.actions.push(Action::Forward);
    }

    // 2. CENTER CLUSTER: 48x48 Circular Home Button + Centered Search Pill with `| Browse`
    let search_width = (width * 0.36).clamp(240.0, 474.0);
    let home_size = 48.0;
    let center_gap = 8.0;
    let total_center = home_size + center_gap + search_width;
    let center_left = (bar_rect.center().x - total_center / 2.0).max(left_rect.left() + 124.0);
    let home_rect = egui::Rect::from_center_size(
        pos2(center_left + home_size / 2.0, bar_rect.center().y),
        Vec2::splat(home_size),
    );
    let home_resp = ui
        .interact(
            home_rect,
            egui::Id::new("topbar-home-circle"),
            Sense::click(),
        )
        .on_hover_text(gettext(locale, "Home"));
    let on_home = matches!(app.page(), Page::Home);
    let home_fill = if home_resp.hovered() {
        palette.surface_hover
    } else {
        palette.surface
    };
    ui.painter()
        .circle_filled(home_rect.center(), home_size / 2.0, home_fill);
    theme::paint_icon(
        ui,
        if on_home {
            Icon::HouseFilled
        } else {
            Icon::House
        },
        home_rect,
        22.0,
        if on_home || home_resp.hovered() {
            palette.text
        } else {
            palette.secondary
        },
    );
    if home_resp.clicked() {
        app.actions.push(Action::Open(Page::Home));
    }

    let search_rect = egui::Rect::from_center_size(
        pos2(
            home_rect.right() + center_gap + search_width / 2.0,
            bar_rect.center().y,
        ),
        vec2(search_width, 48.0),
    );
    let search_id = egui::Id::new("global-search");
    let has_focus = ui.memory(|m| m.has_focus(search_id));
    let search_hovered = ui.rect_contains_pointer(search_rect);
    let pill_fill = if has_focus || search_hovered {
        palette.surface_hover
    } else {
        palette.surface
    };
    ui.painter()
        .rect_filled(search_rect, CornerRadius::same(24), pill_fill);
    if has_focus {
        ui.painter().rect_stroke(
            search_rect,
            CornerRadius::same(24),
            egui::Stroke::new(2.0, palette.text),
            egui::StrokeKind::Inside,
        );
    } else if search_hovered {
        ui.painter().rect_stroke(
            search_rect,
            CornerRadius::same(24),
            egui::Stroke::new(1.0, palette.dim.gamma_multiply(0.5)),
            egui::StrokeKind::Inside,
        );
    }

    let search_icon_rect = egui::Rect::from_center_size(
        pos2(search_rect.left() + 22.0, search_rect.center().y),
        Vec2::splat(20.0),
    );
    theme::paint_icon(
        ui,
        Icon::Search,
        search_icon_rect,
        20.0,
        if has_focus || search_hovered {
            palette.text
        } else {
            palette.secondary
        },
    );

    let before = app.search.query.clone();
    let selected = egui::TextEdit::load_state(ui.ctx(), search_id)
        .and_then(|state| state.cursor.char_range())
        .is_some_and(|range| !range.is_empty());

    let right_reserved = if app.search.query.is_empty() {
        56.0
    } else {
        86.0
    };
    let field_rect = egui::Rect::from_min_max(
        pos2(search_rect.left() + 44.0, search_rect.top() + 4.0),
        pos2(
            (search_rect.right() - right_reserved).max(search_rect.left() + 60.0),
            search_rect.bottom() - 4.0,
        ),
    );
    let mut field_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(field_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let hint = gettext(locale, "What do you want to play?");
    let text_color = palette.text;
    let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, _wrap_width: f32| {
        let mut galley = ui
            .painter()
            .layout_job(egui::text::LayoutJob::simple_singleline(
                buffer.as_str().to_owned(),
                theme::regular(14.5),
                text_color,
            ));
        crate::bidi::reorder(&mut galley);
        galley
    };
    let response = super::widgets::text_edit(
        &mut field_ui,
        locale,
        egui::TextEdit::singleline(&mut app.search.query)
            .id(search_id)
            .hint_text(egui::RichText::new(hint.as_ref()).color(palette.secondary))
            .font(theme::regular(14.5))
            .text_color(palette.text)
            .frame(egui::Frame::NONE)
            .desired_width(field_rect.width())
            .vertical_align(Align::Center)
            .layouter(&mut layouter),
    );
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_label(hint.as_ref()));

    // Clear `×` button when query is non-empty
    if !app.search.query.is_empty() {
        let clear_rect = egui::Rect::from_center_size(
            pos2(search_rect.right() - 68.0, search_rect.center().y),
            Vec2::splat(28.0),
        );
        let mut clear_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(clear_rect)
                .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
        );
        if theme::icon_button(
            &mut clear_ui,
            Icon::X,
            16.0,
            palette.secondary,
            palette.text,
            &gettext(locale, "Clear"),
        )
        .clicked()
        {
            app.search.query.clear();
            ui.memory_mut(|m| m.request_focus(search_id));
        }
    }

    // Vertical divider `|` + Browse (`Icon::Browse`) icon on right side of search pill
    let divider_x = search_rect.right() - 48.0;
    ui.painter().vline(
        divider_x,
        (search_rect.center().y - 12.0)..=(search_rect.center().y + 12.0),
        egui::Stroke::new(1.0, palette.dim.gamma_multiply(0.55)),
    );
    let browse_rect = egui::Rect::from_center_size(
        pos2(search_rect.right() - 24.0, search_rect.center().y),
        Vec2::splat(30.0),
    );
    let browse_resp = ui
        .interact(
            browse_rect,
            egui::Id::new("topbar-browse-btn"),
            Sense::click(),
        )
        .on_hover_text(gettext(locale, "Browse"));
    let on_search_page = matches!(app.page(), Page::Search);
    theme::paint_icon(
        ui,
        Icon::Browse,
        browse_rect,
        20.0,
        if on_search_page || browse_resp.hovered() {
            palette.text
        } else {
            palette.secondary
        },
    );
    if browse_resp.clicked() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new(RECENT_POPOVER_ID), false));
        app.actions.push(Action::Open(Page::Search));
    }

    if app.search.focus_requested {
        app.search.focus_requested = false;
        response.request_focus();
    }
    if response.gained_focus() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new(RECENT_POPOVER_ID), true));
    }
    if app.search.query != before {
        if selected || app.search.query.is_empty() {
            app.actions.push(Action::EndSearchRun);
        }
        app.search.typed_at = Some(std::time::Instant::now());
        ui.data_mut(|data| {
            data.insert_temp(egui::Id::new(RECENT_POPOVER_ID), true);
            data.insert_temp(egui::Id::new("search-suggestion-selected"), 0usize);
        });
    }
    if response.lost_focus()
        && ui.input(|input| input.key_pressed(egui::Key::Enter))
        && ui
            .data(|data| data.get_temp::<usize>(egui::Id::new("search-suggestion-selected")))
            .unwrap_or(0)
            == 0
    {
        let query = app.search.query.clone();
        if !query.is_empty() {
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new(RECENT_POPOVER_ID), false));
            app.actions.push(Action::Search(query));
            if !matches!(app.page(), Page::Search) {
                app.actions.push(Action::Open(Page::Search));
            }
        }
    }
    if response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        response.surrender_focus();
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new(RECENT_POPOVER_ID), false));
    }

    // Search results stay above the current page until the query is submitted.
    let popover_open = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(egui::Id::new(RECENT_POPOVER_ID)))
        .unwrap_or(false);
    if popover_open {
        if app.search.query.is_empty() {
            recent_searches_popover(app, ui.ctx(), search_rect);
        } else {
            search_suggestions_popover(app, ui.ctx(), search_rect);
        }
    }

    // 3. RIGHT CLUSTER: Bell ("Novidades"), Friend Activity ("Atividade de amigos"), Ringed Avatar (48x48)
    let right_rect = egui::Rect::from_min_max(
        pos2(search_rect.right() + 8.0, bar_rect.top()),
        pos2(
            (bar_rect.right() - right_caption_width - 12.0).max(search_rect.right() + 8.0),
            bar_rect.bottom(),
        ),
    );
    let mut right_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right_rect)
            .layout(Layout::right_to_left(Align::Center)),
    );
    right_ui.spacing_mut().item_spacing.x = 8.0;

    // Account Avatar (48x48 #1f1f1f circle with 32x32 inner avatar)
    let (name, avatar) = app
        .user
        .as_ref()
        .map(|user| {
            (
                user.name().to_string(),
                pick_image(&user.images, app.art_px(32.0)).map(str::to_string),
            )
        })
        .unwrap_or_default();
    let (avatar_rect, avatar_resp) =
        right_ui.allocate_exact_size(Vec2::splat(48.0), Sense::click());
    if right_ui.is_rect_visible(avatar_rect) {
        let ring_fill = if avatar_resp.hovered() {
            palette.surface_hover
        } else {
            palette.surface
        };
        right_ui
            .painter()
            .circle_filled(avatar_rect.center(), 24.0, ring_fill);
        let inner = egui::Rect::from_center_size(avatar_rect.center(), Vec2::splat(32.0));
        match avatar.as_deref() {
            Some(url) => super::widgets::paint_cover(
                &right_ui,
                &palette,
                Some(url),
                inner,
                16.0,
                Icon::User,
                Some(app.backend.art()),
            ),
            None => {
                let initial = name
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_uppercase()
                    .to_string();
                right_ui
                    .painter()
                    .circle_filled(inner.center(), 16.0, palette.accent);
                right_ui.painter().text(
                    inner.center(),
                    egui::Align2::CENTER_CENTER,
                    initial,
                    theme::bold(14.0),
                    palette.on_accent,
                );
            }
        }
    }
    let avatar_resp = avatar_resp.on_hover_text(&name);
    egui::Popup::menu(&avatar_resp)
        .frame(super::widgets::menu_frame(&palette))
        .align(egui::RectAlign::BOTTOM_END)
        .show(|ui| {
            ui.set_width(210.0);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                theme::text(ui, &name, theme::semibold(14.0), palette.text);
            });
            if let Some(product) = app.user.as_ref().and_then(|user| user.product.clone()) {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    theme::text(
                        ui,
                        capitalize(&product),
                        theme::regular(12.0),
                        palette.secondary,
                    );
                });
            }
            super::widgets::menu_separator(ui, &palette);
            if super::widgets::menu_item(
                ui,
                &palette,
                Some(Icon::Settings),
                &gettext(locale, "Settings"),
            ) {
                app.actions.push(Action::Open(Page::Settings));
            }
            if super::widgets::menu_item(
                ui,
                &palette,
                Some(Icon::Info),
                &gettext(locale, "Keyboard shortcuts"),
            ) {
                app.actions
                    .push(Action::ShowDialog(crate::model::Dialog::Shortcuts));
            }
            super::widgets::menu_separator(ui, &palette);
            if super::widgets::menu_item(
                ui,
                &palette,
                Some(Icon::LogOut),
                &gettext(locale, "Sign out"),
            ) {
                app.actions.push(Action::SignOut);
            }
        });

    // Friend Activity ("Atividade de amigos")
    let friends_open = app.show_friend_activity;
    if theme::icon_button(
        &mut right_ui,
        Icon::Listening,
        18.0,
        if friends_open {
            palette.text
        } else {
            palette.secondary
        },
        palette.text,
        &gettext(locale, "Friend Activity"),
    )
    .clicked()
    {
        app.actions.push(Action::ToggleFriendActivity);
    }

    // What's New Bell ("Novidades")
    let has_update = app.update.is_some();
    let on_whats_new = matches!(app.page(), Page::WhatsNew);
    let bell_btn = theme::icon_button(
        &mut right_ui,
        Icon::Bell,
        18.0,
        if has_update {
            palette.accent
        } else if on_whats_new {
            palette.text
        } else {
            palette.secondary
        },
        palette.text,
        &gettext(locale, "What's New"),
    );
    if bell_btn.clicked() {
        if has_update {
            app.actions.push(Action::ShowUpdate);
        }
        if !on_whats_new {
            app.actions.push(Action::Open(Page::WhatsNew));
        }
    }

    if !app.network_reachable {
        let (rect, response) =
            right_ui.allocate_exact_size(Vec2::splat(ICON_BUTTON_SIZE), Sense::hover());
        if right_ui.is_rect_visible(rect) {
            right_ui.painter().circle_filled(
                rect.center(),
                14.5,
                egui::Color32::from_rgb(28, 92, 178),
            );
            theme::paint_icon(
                &right_ui,
                Icon::WifiOff,
                rect,
                ICON_BUTTON_ICON,
                egui::Color32::WHITE,
            );
        }
        response.on_hover_text(gettext(
            locale,
            "Spotifast is offline. Reconnecting automatically when connection returns.",
        ));
    }

    if let Some(activity) = app.background_activity() {
        let hint = match activity {
            BackgroundActivity::Library => gettext(locale, "Updating your library…"),
            BackgroundActivity::Updates => gettext(locale, "Checking for updates…"),
            BackgroundActivity::Spotify => gettext(locale, "Waiting for Spotify…"),
        };
        theme::spinner(&mut right_ui, SPINNER_SIZE, palette.secondary).on_hover_text(hint.as_ref());
    }
}

fn search_cover_scale(pressed: bool, elapsed: f32) -> f32 {
    if pressed {
        0.84
    } else if elapsed < 0.32 {
        let phase = elapsed.max(0.0) / 0.32;
        1.0 - 0.16 * (-8.0 * phase).exp() * (16.0 * phase).cos()
    } else {
        1.0
    }
}

fn search_item_cover(
    app: &mut App,
    ui: &mut egui::Ui,
    item: &crate::settings::RecentSearchItem,
    rect: egui::Rect,
    row_hovered: bool,
) -> bool {
    let ctx = ui.ctx().clone();
    let palette = app.palette;
    widgets::paint_cover(
        ui,
        &palette,
        item.image.as_deref(),
        rect,
        if item.is_circle { 20.0 } else { 4.0 },
        if item.is_circle {
            Icon::User
        } else {
            Icon::Music
        },
        Some(app.backend.art()),
    );
    let active = app.now_playing().is_some_and(|now| now.uri == item.uri);
    let playing = app
        .now_playing()
        .is_some_and(|now| now.uri == item.uri && now.playing);
    let response = ui.interact(
        rect,
        ui.id().with(("search-item-play", &item.uri)),
        Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            format!(
                "{} {}",
                gettext(app.locale, if playing { "Pause" } else { "Play" }),
                item.title
            ),
        )
    });
    let press_id = egui::Id::new(("search-item-press", &item.uri));
    if response.clicked() {
        let now = ctx.input(|input| input.time);
        ctx.data_mut(|data| data.insert_temp(press_id, now));
        app.actions.push(Action::RememberRecentItem(item.clone()));
        app.actions.push(if active {
            Action::TogglePlay
        } else if item.uri.starts_with("spotify:track:") || item.uri.starts_with("spotify:episode:")
        {
            Action::PlayUris {
                uris: vec![item.uri.clone()],
                index: 0,
            }
        } else {
            Action::PlayContext {
                uri: item.uri.clone(),
                offset_uri: None,
                offset_index: None,
            }
        });
    }
    if row_hovered || playing || response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(if item.is_circle { 20 } else { 4 }),
            egui::Color32::from_black_alpha(150),
        );
        let elapsed = ctx
            .data(|data| data.get_temp::<f64>(press_id))
            .map_or(1.0, |pressed_at| {
                (ctx.input(|input| input.time) - pressed_at) as f32
            });
        let reduced = crate::motion::reduced(&ctx);
        let scale = if reduced {
            1.0
        } else {
            search_cover_scale(response.is_pointer_button_down_on(), elapsed)
        };
        if !reduced && elapsed < 0.32 {
            ctx.request_repaint();
        }
        let mix = ctx.animate_bool_with_time(
            ui.id().with(("search-item-pause", &item.uri)),
            playing,
            if reduced { 0.0 } else { 0.12 },
        );
        let play = [
            [
                pos2(7.0, 4.0),
                pos2(12.0, 7.1),
                pos2(12.0, 16.9),
                pos2(7.0, 20.0),
            ],
            [
                pos2(12.0, 7.1),
                pos2(20.0, 12.0),
                pos2(20.0, 12.0),
                pos2(12.0, 16.9),
            ],
        ];
        let pause = [
            [
                pos2(6.0, 4.0),
                pos2(10.5, 4.0),
                pos2(10.5, 20.0),
                pos2(6.0, 20.0),
            ],
            [
                pos2(14.0, 4.0),
                pos2(18.5, 4.0),
                pos2(18.5, 20.0),
                pos2(14.0, 20.0),
            ],
        ];
        for (from, to) in play.into_iter().zip(pause) {
            let points = from
                .into_iter()
                .zip(to)
                .map(|(from, to)| rect.center() + (from.lerp(to, mix) - pos2(12.0, 12.0)) * scale)
                .collect();
            ui.painter().add(egui::Shape::convex_polygon(
                points,
                palette.text,
                egui::Stroke::NONE,
            ));
        }
    }
    response.clicked()
}

fn search_suggestions_popover(app: &mut App, ctx: &egui::Context, search_rect: egui::Rect) {
    struct Suggestion {
        title: String,
        subtitle: String,
        image: Option<String>,
        round: bool,
        uri: Option<String>,
        explicit: bool,
        action: Action,
    }
    impl Suggestion {
        fn recent_item(&self) -> Option<crate::settings::RecentSearchItem> {
            Some(crate::settings::RecentSearchItem {
                title: self.title.clone(),
                subtitle: self.subtitle.clone(),
                image: self.image.clone(),
                uri: self.uri.clone()?,
                explicit: self.explicit,
                is_circle: self.round,
                verified: false,
            })
        }
    }
    let palette = app.palette;
    let query = app.search.query.trim().to_owned();
    let mut suggestions = vec![Suggestion {
        title: query.clone(),
        subtitle: gettext(app.locale, "Search").into_owned(),
        image: None,
        round: false,
        uri: None,
        explicit: false,
        action: Action::Search(query.clone()),
    }];
    for recent in app
        .settings
        .search_history
        .iter()
        .rev()
        .filter(|recent| {
            recent.to_lowercase().contains(&query.to_lowercase()) && recent.trim() != query
        })
        .take(3)
    {
        suggestions.push(Suggestion {
            title: recent.clone(),
            subtitle: String::new(),
            image: None,
            round: false,
            uri: None,
            explicit: false,
            action: Action::Search(recent.clone()),
        });
    }
    if app.search.committed == query
        && let Some(results) = app.search.results.get()
    {
        if let Some(tracks) = &results.tracks {
            for track in &tracks.items {
                suggestions.push(Suggestion {
                    title: track.name.clone(),
                    subtitle: format!("{} • {}", gettext(app.locale, "Song"), track.artist_names()),
                    image: track.image(app.art_px(40.0)).map(str::to_owned),
                    round: false,
                    uri: Some(track.uri.clone()),
                    explicit: track.explicit,
                    action: Action::PlayUris {
                        uris: vec![track.uri.clone()],
                        index: 0,
                    },
                });
            }
        }
        if let Some(artists) = &results.artists {
            for artist in &artists.items {
                suggestions.push(Suggestion {
                    title: artist.name.clone(),
                    subtitle: gettext(app.locale, "Artist").into_owned(),
                    image: pick_image(&artist.images, app.art_px(40.0)).map(str::to_owned),
                    round: true,
                    uri: Some(artist.uri.clone()),
                    explicit: false,
                    action: Action::Open(Page::Artist(artist.id.clone())),
                });
            }
        }
        if let Some(albums) = &results.albums {
            for album in &albums.items {
                suggestions.push(Suggestion {
                    title: album.name.clone(),
                    subtitle: gettext(app.locale, "Album").into_owned(),
                    image: pick_image(&album.images, app.art_px(40.0)).map(str::to_owned),
                    round: false,
                    uri: Some(album.uri.clone()),
                    explicit: false,
                    action: Action::Open(Page::Album(album.id.clone())),
                });
            }
        }
    }
    if app.search.committed == query
        && let Some(playlists) = app
            .search
            .results
            .get()
            .and_then(|results| results.playlists.as_ref())
    {
        for playlist in &playlists.items {
            suggestions.push(Suggestion {
                title: playlist.name.clone(),
                subtitle: gettext(app.locale, "Playlist").into_owned(),
                image: pick_image(&playlist.images, app.art_px(40.0)).map(str::to_owned),
                round: false,
                uri: Some(playlist.uri.clone()),
                explicit: false,
                action: Action::Open(Page::Playlist(playlist.id.clone())),
            });
        }
    }
    let popup_height = suggestions
        .iter()
        .map(|suggestion| {
            if suggestion.image.is_some() {
                58.0
            } else {
                44.0
            }
        })
        .sum::<f32>()
        .min(440.0)
        .min(
            (ctx.input(|input| input.viewport_rect()).bottom() - search_rect.bottom() - 20.0)
                .max(44.0),
        );
    let selection_id = egui::Id::new("search-suggestion-selected");
    let mut selected = ctx
        .data(|data| data.get_temp::<usize>(selection_id))
        .unwrap_or(0)
        .min(suggestions.len() - 1);
    ctx.input_mut(|input| {
        if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
            selected = (selected + 1) % suggestions.len();
        }
        if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
            selected = (selected + suggestions.len() - 1) % suggestions.len();
        }
    });
    let enter = ctx.input(|input| input.key_pressed(egui::Key::Enter)) && selected > 0;
    let mut chosen = None;
    let area = egui::Area::new(egui::Id::new("search-suggestions-popover"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(search_rect.left(), search_rect.bottom() + 4.0))
        .show(ctx, |ui| {
            widgets::menu_frame(&palette).show(ui, |ui| {
                ui.set_width(search_rect.width() - 8.0);
                ui.set_height(popup_height);
                egui::ScrollArea::vertical()
                    .max_height(popup_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for (index, suggestion) in suggestions.iter().enumerate() {
                            let height = if suggestion.image.is_some() {
                                56.0
                            } else {
                                42.0
                            };
                            let (rect, response) = ui.allocate_exact_size(
                                vec2(ui.available_width(), height),
                                Sense::click(),
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    ui.is_enabled(),
                                    &suggestion.title,
                                )
                            });
                            let row_hovered = ui.rect_contains_pointer(rect);
                            if row_hovered || index == selected {
                                ui.painter().rect_filled(rect, 4.0, palette.surface_hover);
                            }
                            if row_hovered && ctx.input(|input| input.pointer.delta() != Vec2::ZERO)
                            {
                                selected = index;
                            }
                            let cover = egui::Rect::from_center_size(
                                pos2(rect.left() + 28.0, rect.center().y),
                                Vec2::splat(40.0),
                            );
                            let cover_clicked = if let Some(item) = suggestion.recent_item() {
                                search_item_cover(app, ui, &item, cover, row_hovered)
                            } else {
                                theme::paint_icon(
                                    ui,
                                    Icon::Search,
                                    cover.shrink(10.0),
                                    20.0,
                                    palette.secondary,
                                );
                                false
                            };
                            let painter = ui.painter().with_clip_rect(rect);
                            let x = cover.right() + 12.0;
                            let y = rect.center().y
                                - if suggestion.subtitle.is_empty() {
                                    0.0
                                } else {
                                    9.0
                                };
                            crate::bidi::paint_line(
                                &painter,
                                x,
                                rect.right() - 12.0,
                                y,
                                &suggestion.title,
                                theme::medium(14.0),
                                palette.text,
                            );
                            if !suggestion.subtitle.is_empty() {
                                crate::bidi::paint_line(
                                    &painter,
                                    x,
                                    rect.right() - 12.0,
                                    rect.center().y + 11.0,
                                    &suggestion.subtitle,
                                    theme::regular(12.0),
                                    palette.secondary,
                                );
                            }
                            if !cover_clicked
                                && (response.clicked() || (enter && index == selected))
                            {
                                chosen = Some(index);
                            }
                        }
                    });
            });
        });
    ctx.data_mut(|data| data.insert_temp(selection_id, selected));
    let clicked_outside = ctx.input(|input| {
        input.pointer.any_pressed()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|pos| !area.response.rect.contains(pos) && !search_rect.contains(pos))
    });
    if let Some(index) = chosen {
        let suggestion = suggestions.remove(index);
        if let Some(item) = suggestion.recent_item() {
            app.actions.push(Action::RememberRecentItem(item));
        }
        app.actions.push(suggestion.action);
        ctx.memory_mut(|memory| memory.surrender_focus(egui::Id::new("global-search")));
    }
    if chosen.is_some() || clicked_outside {
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new(RECENT_POPOVER_ID), false);
            data.insert_temp(selection_id, 0usize);
        });
    }
}

fn recent_searches_popover(app: &mut App, ctx: &egui::Context, search_rect: egui::Rect) {
    let palette = app.palette;
    let locale = app.locale;
    let width = search_rect.width().max(360.0);
    let pos = pos2(search_rect.left(), search_rect.bottom() + 8.0);

    let area = egui::Area::new(egui::Id::new("recent-searches-popover"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(palette.overlay)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(16, 14))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(150),
                })
                .show(ui, |ui| {
                    ui.set_width(width - 32.0);
                    theme::text(
                        ui,
                        gettext(locale, "Recent searches"),
                        theme::bold(15.0),
                        palette.text,
                    );
                    ui.add_space(10.0);

                    let recent_items = app.settings.recent_searches.clone();
                    let history = app.settings.search_history.clone();
                    if recent_items.is_empty() && history.is_empty() {
                        theme::text(
                            ui,
                            gettext(
                                locale,
                                "Find songs, artists, albums, playlists, and podcasts.",
                            ),
                            theme::regular(13.0),
                            palette.secondary,
                        );
                        ui.add_space(4.0);
                        return;
                    }

                    if !recent_items.is_empty() {
                        for item in recent_items.iter().take(8) {
                            let row_h = 56.0;
                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                vec2(ui.available_width(), row_h),
                                Sense::click(),
                            );
                            let close_rect = egui::Rect::from_center_size(
                                pos2(row_rect.right() - 18.0, row_rect.center().y),
                                Vec2::splat(28.0),
                            );
                            let art_rect = egui::Rect::from_center_size(
                                pos2(row_rect.left() + 26.0, row_rect.center().y),
                                Vec2::splat(40.0),
                            );
                            let is_row_hovered = ui.rect_contains_pointer(row_rect);
                            let over_close = ui.rect_contains_pointer(close_rect);
                            let over_art = ui.rect_contains_pointer(art_rect);

                            if is_row_hovered {
                                ui.painter().rect_filled(
                                    row_rect,
                                    CornerRadius::same(6),
                                    palette.surface_hover,
                                );
                            }

                            search_item_cover(app, ui, item, art_rect, is_row_hovered);
                            let is_this_active =
                                app.now_playing().is_some_and(|now| now.uri == item.uri);

                            let text_left = art_rect.right() + 12.0;
                            let text_right = close_rect.left() - 8.0;
                            let painter = ui.painter().with_clip_rect(egui::Rect::from_min_max(
                                pos2(text_left, row_rect.top()),
                                pos2(text_right, row_rect.bottom()),
                            ));
                            let title_color = if is_this_active {
                                palette.accent
                            } else {
                                palette.text
                            };
                            painter.text(
                                pos2(text_left, row_rect.center().y - 10.0),
                                egui::Align2::LEFT_CENTER,
                                &item.title,
                                theme::medium(14.0),
                                title_color,
                            );
                            let sub_y = row_rect.center().y + 10.0;
                            if item.explicit {
                                let badge_w = 14.0;
                                let badge_rect = egui::Rect::from_center_size(
                                    pos2(text_left + 7.0, sub_y),
                                    vec2(badge_w, 12.0),
                                );
                                painter.rect_filled(
                                    badge_rect,
                                    CornerRadius::same(2),
                                    palette.secondary.gamma_multiply(0.4),
                                );
                                painter.text(
                                    badge_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "E",
                                    theme::bold(9.0),
                                    palette.text,
                                );
                                painter.text(
                                    pos2(text_left + badge_w + 5.0, sub_y),
                                    egui::Align2::LEFT_CENTER,
                                    &item.subtitle,
                                    theme::regular(12.0),
                                    palette.secondary,
                                );
                            } else {
                                painter.text(
                                    pos2(text_left, sub_y),
                                    egui::Align2::LEFT_CENTER,
                                    &item.subtitle,
                                    theme::regular(12.0),
                                    palette.secondary,
                                );
                            }

                            if is_row_hovered || over_close {
                                let close_resp = ui
                                    .interact(
                                        close_rect,
                                        egui::Id::new(("forget-recent-item", &item.uri)),
                                        Sense::click(),
                                    )
                                    .on_hover_text(gettext(locale, "Remove"));
                                theme::paint_icon(
                                    ui,
                                    Icon::X,
                                    close_rect,
                                    16.0,
                                    if over_close {
                                        palette.text
                                    } else {
                                        palette.secondary
                                    },
                                );
                                if close_resp.clicked() {
                                    app.actions.push(Action::ForgetRecentItem(item.uri.clone()));
                                }
                            }

                            if row_resp.clicked() && !over_close && !over_art {
                                ctx.data_mut(|d| {
                                    d.insert_temp(egui::Id::new(RECENT_POPOVER_ID), false)
                                });
                                if item.uri.starts_with("spotify:track:") {
                                    app.actions.push(Action::PlayUris {
                                        uris: vec![item.uri.clone()],
                                        index: 0,
                                    });
                                } else if let Some(page) = Page::from_uri(&item.uri) {
                                    app.actions.push(Action::Open(page));
                                } else {
                                    app.actions.push(Action::Search(item.title.clone()));
                                    if !matches!(app.page(), Page::Search) {
                                        app.actions.push(Action::Open(Page::Search));
                                    }
                                }
                            }
                        }
                    } else {
                        for query in history.iter().take(8) {
                            let matched_artist = app
                                .library
                                .artists
                                .items
                                .iter()
                                .find(|a| a.name.eq_ignore_ascii_case(query))
                                .cloned();
                            let matched_playlist = app.library.playlists.get().and_then(|list| {
                                list.iter()
                                    .find(|p| p.name.eq_ignore_ascii_case(query))
                                    .cloned()
                            });
                            let (art_url, subtitle, is_circle) =
                                if let Some(artist) = &matched_artist {
                                    (
                                        pick_image(&artist.images, 64).map(str::to_string),
                                        gettext(locale, "Artist").into_owned(),
                                        true,
                                    )
                                } else if let Some(playlist) = &matched_playlist {
                                    (
                                        pick_image(&playlist.images, 64).map(str::to_string),
                                        gettext(locale, "Playlist").into_owned(),
                                        false,
                                    )
                                } else {
                                    (None, gettext(locale, "Song").into_owned(), false)
                                };

                            let row_h = 54.0;
                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                vec2(ui.available_width(), row_h),
                                Sense::click(),
                            );
                            let close_rect = egui::Rect::from_center_size(
                                pos2(row_rect.right() - 18.0, row_rect.center().y),
                                Vec2::splat(28.0),
                            );
                            let art_rect = egui::Rect::from_center_size(
                                pos2(row_rect.left() + 26.0, row_rect.center().y),
                                Vec2::splat(40.0),
                            );
                            let is_row_hovered = ui.rect_contains_pointer(row_rect);
                            let over_close = ui.rect_contains_pointer(close_rect);
                            let over_art = ui.rect_contains_pointer(art_rect);

                            if is_row_hovered {
                                ui.painter().rect_filled(
                                    row_rect,
                                    CornerRadius::same(6),
                                    palette.surface_hover,
                                );
                            }

                            super::widgets::paint_cover(
                                ui,
                                &palette,
                                art_url.as_deref(),
                                art_rect,
                                if is_circle { 20.0 } else { 4.0 },
                                if is_circle { Icon::User } else { Icon::Search },
                                Some(app.backend.art()),
                            );

                            let painter = ui.painter().with_clip_rect(row_rect);
                            painter.text(
                                pos2(art_rect.right() + 12.0, row_rect.center().y - 9.0),
                                egui::Align2::LEFT_CENTER,
                                query,
                                theme::medium(14.0),
                                palette.text,
                            );
                            painter.text(
                                pos2(art_rect.right() + 12.0, row_rect.center().y + 9.0),
                                egui::Align2::LEFT_CENTER,
                                &subtitle,
                                theme::regular(12.5),
                                palette.secondary,
                            );

                            if is_row_hovered || over_close {
                                let close_resp = ui
                                    .interact(
                                        close_rect,
                                        egui::Id::new(("forget-recent", query)),
                                        Sense::click(),
                                    )
                                    .on_hover_text(gettext(locale, "Remove"));
                                theme::paint_icon(
                                    ui,
                                    Icon::X,
                                    close_rect,
                                    16.0,
                                    if over_close {
                                        palette.text
                                    } else {
                                        palette.secondary
                                    },
                                );
                                if close_resp.clicked() {
                                    app.actions.push(Action::ForgetSearch(query.clone()));
                                }
                            }

                            if row_resp.clicked() && !over_close && !over_art {
                                ctx.data_mut(|d| {
                                    d.insert_temp(egui::Id::new(RECENT_POPOVER_ID), false)
                                });
                                app.actions.push(Action::Search(query.clone()));
                                if !matches!(app.page(), Page::Search) {
                                    app.actions.push(Action::Open(Page::Search));
                                }
                            }
                        }
                    }
                });
        });

    let popup_rect = area.response.rect;
    let clicked_outside = ctx.input(|input| {
        input.pointer.any_pressed()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|pos| !popup_rect.contains(pos) && !search_rect.contains(pos))
    });
    if clicked_outside {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new(RECENT_POPOVER_ID), false));
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod topbar_fit_tests {
    use super::*;

    #[test]
    fn search_cover_press_has_a_small_overshoot_and_settles() {
        assert_eq!(search_cover_scale(true, 0.0), 0.84);
        assert!(search_cover_scale(false, 0.0) < 1.0);
        let peak = (0..32)
            .map(|frame| search_cover_scale(false, frame as f32 / 100.0))
            .fold(0.0_f32, f32::max);
        assert!(peak > 1.0 && peak < 1.05);
        assert_eq!(search_cover_scale(false, 0.32), 1.0);
        assert_eq!(search_cover_scale(false, 1.0), 1.0);
    }

    // What the badges measure on a bar showing "Playing on MacBook de Luis"
    // and "Update to 0.7.1", each including the spacing before it.
    const DEVICE: f32 = ITEM_SPACING + 176.0;
    const UPDATE: f32 = ITEM_SPACING + 152.0;
    // Collapsed, a badge is a square chip as tall as its text.
    const CHIP: f32 = ITEM_SPACING + 15.0 + BADGE_PADDING_Y;

    /// The narrowest bar the app can produce: a 760 px window, its sidebar,
    /// and the navigation buttons all taken out.
    const NARROWEST_BAR: f32 = 398.0;

    fn right_end(room: f32, labelled: f32, icons: f32) -> f32 {
        let fit = topbar_fit(room, RIGHT_CONTROLS_WIDTH, labelled, icons);
        let badges = if fit.labels { labelled } else { icons };
        RIGHT_CONTROLS_WIDTH + badges - (room - fit.search)
    }

    #[test]
    fn a_wide_bar_keeps_the_field_it_always_had() {
        let fit = topbar_fit(2000.0, RIGHT_CONTROLS_WIDTH, DEVICE + UPDATE, CHIP * 2.0);
        assert_eq!(fit.search, SEARCH_MAX);
        assert!(fit.labels);
        // Half the room, as before, while half still fits.
        let fit = topbar_fit(700.0, RIGHT_CONTROLS_WIDTH, 0.0, 0.0);
        assert_eq!(fit.search, 350.0);
    }

    #[test]
    fn the_right_end_never_reaches_over_the_search_field() {
        let mut room = NARROWEST_BAR;
        while room <= 2400.0 {
            for (labelled, icons) in [
                (0.0, 0.0),
                (DEVICE, CHIP),
                (UPDATE, CHIP),
                (DEVICE + UPDATE, CHIP * 2.0),
            ] {
                let over = right_end(room, labelled, icons);
                assert!(
                    over <= 0.0,
                    "badges overlap the field by {over} px on a {room} px bar"
                );
            }
            room += 1.0;
        }
    }

    #[test]
    fn a_right_panel_can_narrow_search_after_the_badges_collapse() {
        let room = RIGHT_CONTROLS_WIDTH + CHIP * 2.0 + 100.0;
        let fit = topbar_fit(room, RIGHT_CONTROLS_WIDTH, DEVICE + UPDATE, CHIP * 2.0);
        assert!(!fit.labels);
        assert_eq!(fit.search, 100.0);
        assert_eq!(right_end(room, DEVICE + UPDATE, CHIP * 2.0), 0.0);
    }

    #[test]
    fn a_narrow_bar_trades_the_badge_labels_for_their_icons() {
        assert!(topbar_fit(1400.0, RIGHT_CONTROLS_WIDTH, DEVICE, CHIP).labels);
        // The 1080 px window of the report that started this.
        assert!(topbar_fit(952.0, RIGHT_CONTROLS_WIDTH, DEVICE + UPDATE, CHIP * 2.0).labels);
        assert!(!topbar_fit(NARROWEST_BAR, RIGHT_CONTROLS_WIDTH, DEVICE, CHIP).labels);
    }

    /// At the least width the panels leave it, the bar still holds the
    /// spinner and both badges beside the narrowest field (#624).
    #[test]
    fn the_least_width_holds_every_control_beside_the_field() {
        let lead = LEAD_GUESS;
        let room = least_width_after(lead) - lead;
        let controls = RIGHT_CONTROLS_WIDTH + SPINNER_SIZE + ITEM_SPACING;
        let fit = topbar_fit(room, controls, DEVICE + UPDATE, CHIP * 2.0);
        assert!(!fit.labels);
        assert_eq!(fit.search, SEARCH_MIN);
        assert!(controls + CHIP * 2.0 + fit.search <= room);
    }

    #[test]
    fn the_field_stays_readable_however_tight_the_bar_gets() {
        let mut room = NARROWEST_BAR;
        while room <= 2400.0 {
            let fit = topbar_fit(room, RIGHT_CONTROLS_WIDTH, DEVICE + UPDATE, CHIP * 2.0);
            assert!(fit.search >= SEARCH_FLOOR, "field is {} px", fit.search);
            assert!(fit.search <= SEARCH_MAX);
            room += 1.0;
        }
    }
}
