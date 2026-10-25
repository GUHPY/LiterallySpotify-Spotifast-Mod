//! The Settings page.

use egui::{Align, CornerRadius, Frame, Layout, Margin, Stroke, Vec2};

use crate::api::models::pick_image;
use crate::app::App;
use crate::i18n::{gettext, ngettext, pgettext};
use crate::model::{Action, Dialog};
use crate::settings::{LanguageChoice, ProxyMode, ThemeChoice};
use crate::theme::{self, Icon, Palette};

use super::widgets;

const PLAYBACK_DIRTY_ID: &str = "playback-settings-dirty";
pub(crate) const PERSONAL_APP_FOCUS_ID: &str = "focus-personal-app-setup";
const SETTINGS_FILTER_ID: &str = "settings-filter";

/// Whether a settings row matches the filter query (case-insensitive).
/// An empty query matches everything, so the page reads exactly as
/// before when the search field is empty.
pub(crate) fn row_matches(needle: &str, title: &str, description: &str) -> bool {
    let needle = needle.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }
    title.to_lowercase().contains(&needle) || description.to_lowercase().contains(&needle)
}

/// Text shared by section selection and the rendered row. Conditional rows
/// participate only on the platforms and account states where they exist.
struct RowText<'a> {
    title: std::borrow::Cow<'a, str>,
    description: std::borrow::Cow<'a, str>,
    available: bool,
}

impl<'a> RowText<'a> {
    fn new(
        title: impl Into<std::borrow::Cow<'a, str>>,
        description: impl Into<std::borrow::Cow<'a, str>>,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            available: true,
        }
    }

    fn when(mut self, available: bool) -> Self {
        self.available = available;
        self
    }

    fn matches(&self, needle: &str, section: &str) -> bool {
        self.available
            && (needle.is_empty()
                || section.to_lowercase().contains(needle)
                || row_matches(needle, &self.title, &self.description))
    }
}

/// The guide to writing a palette file for the themes folder.
const THEMES_GUIDE_URL: &str = "https://spotifast.rocks/settings-and-files/#custom-themes";

fn section_matches(needle: &str, title: &str, rows: &[RowText<'_>]) -> bool {
    let needle = needle.trim().to_lowercase();
    rows.iter().any(|row| row.matches(&needle, title))
}

fn filtered_row(
    ui: &mut egui::Ui,
    palette: &Palette,
    needle: &str,
    section: &str,
    row: &RowText<'_>,
    control: impl FnOnce(&mut egui::Ui),
) {
    if row.matches(needle, section) {
        widgets::setting_row(ui, palette, &row.title, &row.description, control);
    }
}

/// `filtered_row` for a control that needs `control_width` points.
fn filtered_row_sized(
    ui: &mut egui::Ui,
    palette: &Palette,
    needle: &str,
    section: &str,
    row: &RowText<'_>,
    control_width: f32,
    control: impl FnOnce(&mut egui::Ui),
) {
    if row.matches(needle, section) {
        widgets::setting_row_sized(
            ui,
            palette,
            &row.title,
            &row.description,
            control_width,
            control,
        );
    }
}

/// Fills a settings slider's rail up to the handle with the accent. The
/// rail keeps the theme's colours and rounding: egui carries that fill past
/// the handle's centre by the widgets' corner radius, so a pill radius here
/// would show the accent beyond the handle.
fn slider_rail_style(style: &mut egui::Style, palette: &Palette) {
    style.visuals.selection.bg_fill = palette.accent;
    style.visuals.slider_trailing_fill = true;
}

/// Shapes a settings slider's value box like the pill-shaped buttons in the
/// other settings rows.
fn slider_value_style(style: &mut egui::Style, palette: &Palette) {
    style.visuals.widgets.inactive.bg_fill = palette.surface;
    style.visuals.widgets.hovered.bg_fill = palette.surface_hover;
    style.visuals.widgets.active.bg_fill = palette.surface_active;
    let pill = CornerRadius::same(14);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = pill;
    }
    style
        .text_styles
        .insert(egui::TextStyle::Button, theme::medium(13.0));
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
}

/// A settings slider with its value box beside it, where egui's own slider
/// would put it. The two are drawn apart so each keeps its own rounding;
/// `slider` and `value` set up each part for `number` over `range`.
fn setting_slider<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    palette: &Palette,
    number: &mut N,
    range: std::ops::RangeInclusive<N>,
    slider: impl FnOnce(egui::Slider<'_>) -> egui::Slider<'_>,
    value: impl FnOnce(egui::DragValue<'_>) -> egui::DragValue<'_>,
) -> egui::Response {
    ui.horizontal(|ui| {
        let moved = ui
            .scope(|ui| {
                slider_rail_style(ui.style_mut(), palette);
                ui.add(slider(egui::Slider::new(number, range.clone())).show_value(false))
            })
            .inner;
        let typed = ui
            .scope(|ui| {
                slider_value_style(ui.style_mut(), palette);
                ui.add(value(egui::DragValue::new(number).range(range)))
            })
            .inner;
        moved | typed
    })
    .inner
}

/// Forget the search text, so a flow that lands on a specific row (like
/// the Personal App setup) always finds that row visible and focusable.
pub(crate) fn clear_search(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove::<String>(egui::Id::new(SETTINGS_FILTER_ID)));
}
const PROXY_DIRTY_ID: &str = "proxy-settings-dirty";

/// A frame-rate dial: 30, 60, the screen's rate once known, the current rate
/// and Uncapped. Returns the rate a move lands on.
fn frame_rate_dial(
    ui: &mut egui::Ui,
    palette: &Palette,
    locale: crate::i18n::Locale,
    screen_hz: u32,
    fps: u32,
) -> Option<u32> {
    // The dial stops at the rates worth having and passes
    // through nothing in between, the way a gear lever does.
    let stops = crate::milkdrop::fps_stops(screen_hz, fps);
    let last = stops.len().saturating_sub(1);
    let mut at = stops.iter().position(|rate| *rate == fps).unwrap_or(1);
    let labels: Vec<String> = stops
        .iter()
        .map(|rate| crate::milkdrop::fps_label(locale, *rate, screen_hz))
        .collect();
    let shown = labels.clone();
    let typed = stops.clone();
    let changed = setting_slider(
        ui,
        palette,
        &mut at,
        0..=last,
        |slider| slider.step_by(1.0),
        // A drag moves one stop every twenty points.
        |value| {
            value
                .speed(0.05)
                .custom_formatter(move |value, _| {
                    shown
                        .get((value.round().max(0.0) as usize).min(shown.len() - 1))
                        .cloned()
                        .unwrap_or_default()
                })
                .custom_parser(move |text| {
                    // A rate typed in lands on the nearest stop, since
                    // the stops are all this dial can hold.
                    let text = text.trim().to_lowercase();
                    if text.starts_with("un") {
                        return Some(typed.len().saturating_sub(1) as f64);
                    }
                    let wanted: u32 = text
                        .trim_end_matches("fps")
                        .trim()
                        .split(',')
                        .next()?
                        .trim()
                        .parse()
                        .ok()?;
                    typed
                        .iter()
                        .enumerate()
                        .filter(|(_, rate)| **rate > 0)
                        .min_by_key(|(_, rate)| rate.abs_diff(wanted))
                        .map(|(index, _)| index as f64)
                })
        },
    )
    .changed();
    if changed {
        stops.get(at).copied()
    } else {
        None
    }
}

fn section(
    ui: &mut egui::Ui,
    palette: &Palette,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    ui.add_space(16.0);
    theme::text(ui, title, theme::bold(20.0), palette.text);
    ui.add_space(10.0);
    Frame::NONE
        .inner_margin(Margin::symmetric(0, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width().min(768.0));
            add_contents(ui);
        });
    ui.add_space(12.0);
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.left()..=(rect.left() + rect.width().min(768.0)),
        rect.top(),
        Stroke::new(1.0, egui::Color32::from_white_alpha(18)),
    );
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let total_width = ui.available_width();
    let column_width = total_width.min(768.0);
    let side_margin = ((total_width - column_width) / 2.0).max(0.0);
    ui.horizontal(|ui| {
        if side_margin > 0.5 {
            ui.add_space(side_margin);
        }
        ui.vertical(|ui| {
            ui.set_max_width(768.0);
            show_inner(app, ui);
        });
    });
}

fn show_inner(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    ui.add_space(12.0);
    let mut filter = ui
        .data(|data| data.get_temp::<String>(egui::Id::new(SETTINGS_FILTER_ID)))
        .unwrap_or_default();
    ui.horizontal(|ui| {
        theme::text(
            ui,
            gettext(locale, "Settings"),
            theme::bold(32.0),
            palette.text,
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            widgets::search_field(
                ui,
                &palette,
                app.locale,
                egui::Id::new("settings-search-field"),
                &mut filter,
                &gettext(locale, "Search settings"),
                ui.available_width().min(260.0),
            );
        });
    });
    let needle = filter.trim().to_lowercase();
    ui.data_mut(|data| data.insert_temp(egui::Id::new(SETTINGS_FILTER_ID), filter));
    ui.add_space(8.0);
    let dirty_id = egui::Id::new(PLAYBACK_DIRTY_ID);
    let mut playback_dirty = ui
        .data(|data| data.get_temp::<bool>(dirty_id))
        .unwrap_or(false);
    let proxy_dirty_id = egui::Id::new(PROXY_DIRTY_ID);
    let mut proxy_dirty = ui
        .data(|data| data.get_temp::<bool>(proxy_dirty_id))
        .unwrap_or(false);
    let mut changed = false;
    let mut any_visible = false;
    let open_folder = gettext(locale, "Open folder");

    let wanted = app
        .settings
        .web_client_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string);
    let in_use = wanted
        .as_deref()
        .is_some_and(|wanted| app.web_app.as_deref() == Some(wanted));
    let account = gettext(locale, "Account");
    let sign_out = gettext(locale, "Sign out");
    let account_rows = [
        RowText::new(
            gettext(locale, "Personal Spotify app"),
            gettext(
                locale,
                "Use a personal Development Mode app for a separate API quota. The shared app stays active.",
            ),
        ),
        RowText::new(
            gettext(locale, "Create an app"),
            gettext(
                locale,
                "Create one for free in Spotify's developer dashboard.",
            ),
        )
        .when(!in_use),
        RowText::new(
            gettext(locale, "Personal app ready"),
            gettext(
                locale,
                "Supported requests use your app. Other requests use the shared app.",
            ),
        )
        .when(in_use),
        RowText::new(
            gettext(locale, "Authorize your personal app"),
            gettext(
                locale,
                "Spotify opens in your browser to verify the account.",
            ),
        )
        .when(!in_use && wanted.is_some()),
        RowText::new(
            gettext(locale, "Remove personal app"),
            gettext(locale, "Shared access remains signed in."),
        )
        .when(!in_use && wanted.is_none() && app.web_app.is_some()),
        RowText::new(sign_out.clone(), account.clone()),
    ];
    if section_matches(&needle, &account, &account_rows) {
        any_visible = true;
        section(ui, &palette, &account, |ui| {
            if row_matches(&needle, &format!("{sign_out} {account}"), "") {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    let avatar = app.user.as_ref().and_then(|user| {
                        pick_image(&user.images, app.art_px(56.0)).map(str::to_string)
                    });
                    widgets::cover(ui, &palette, avatar.as_deref(), 56.0, 28.0, Icon::User);
                    ui.vertical(|ui| {
                        let name = app
                            .user
                            .as_ref()
                            .map(|user| user.name().to_string())
                            .unwrap_or_default();
                        theme::text(ui, name, theme::semibold(16.0), palette.text);
                        let product = app
                            .user
                            .as_ref()
                            .and_then(|user| user.product.clone())
                            .map(|product| match product.as_str() {
                                "premium" => "Spotify Premium".to_string(),
                                "free" | "open" => {
                                    gettext(locale, "Spotify Free, local playback needs Premium")
                                        .into_owned()
                                }
                                other => other.to_string(),
                            })
                            .unwrap_or_default();
                        theme::text(ui, product, theme::regular(13.0), palette.secondary);
                        if let Some(username) =
                            app.local.connected.then(|| app.local.username.clone())
                            && !username.is_empty()
                        {
                            theme::text(
                                ui,
                                // Translators: {username} is the Spotify account's user name.
                                gettext(locale, "Connected as {username}")
                                    .replace("{username}", &username),
                                theme::regular(12.0),
                                palette.dim,
                            );
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::pill_button(ui, &palette, &sign_out, false).clicked() {
                            app.actions.push(Action::SignOut);
                        }
                    });
                });
                ui.add_space(10.0);
            }
            let mut client_id = app.settings.web_client_id.clone().unwrap_or_default();
            filtered_row(ui, &palette, &needle, &account, &account_rows[0], |ui| {
                let response = Frame::new()
                    .fill(palette.surface)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        widgets::text_edit(
                            ui,
                            app.locale,
                            egui::TextEdit::singleline(&mut client_id)
                                .id(egui::Id::new("personal-web-client-id"))
                                .hint_text(
                                    egui::RichText::new(gettext(locale, "Client ID"))
                                        .color(palette.dim),
                                )
                                .font(theme::regular(13.0))
                                .frame(egui::Frame::NONE)
                                .desired_width(200.0),
                        )
                    })
                    .inner;
                if ui
                    .data_mut(|data| data.remove_temp::<bool>(egui::Id::new(PERSONAL_APP_FOCUS_ID)))
                    .unwrap_or(false)
                {
                    response.scroll_to_me(Some(Align::Center));
                    response.request_focus();
                }
                if response.changed() {
                    let trimmed = client_id.trim().to_string();
                    app.settings.web_client_id = (!trimmed.is_empty()).then_some(trimmed);
                    changed = true;
                }
            });
            filtered_row(ui, &palette, &needle, &account, &account_rows[1], |ui| {
                if theme::pill_button(ui, &palette, &gettext(locale, "Setup guide"), false)
                    .clicked()
                {
                    app.actions.push(Action::OpenUrl(
                        "https://spotifast.rocks/make-it-even-faster/#make-a-spotify-app".into(),
                    ));
                }
            });
            if in_use {
                filtered_row(ui, &palette, &needle, &account, &account_rows[2], |ui| {
                    if theme::pill_button(ui, &palette, &gettext(locale, "Remove"), false).clicked()
                    {
                        app.settings.web_client_id = None;
                        app.actions.push(Action::ConfigurePersonalWebApp);
                    }
                });
            } else if wanted.is_some() {
                filtered_row(ui, &palette, &needle, &account, &account_rows[3], |ui| {
                    if theme::pill_button(ui, &palette, &gettext(locale, "Authorize"), true)
                        .clicked()
                    {
                        app.actions.push(Action::ConfigurePersonalWebApp);
                    }
                });
            } else if app.web_app.is_some() {
                filtered_row(ui, &palette, &needle, &account, &account_rows[4], |ui| {
                    if theme::pill_button(ui, &palette, &gettext(locale, "Remove"), false).clicked()
                    {
                        app.actions.push(Action::ConfigurePersonalWebApp);
                    }
                });
            }
        });
    }

    let (status, detail, action) = match &app.local_playback {
        crate::backend::LocalPlayback::Ready { .. } => (
            pgettext(locale, "playback status", "Ready"),
            gettext(locale, "This computer is a Spotify Connect device."),
            None,
        ),
        crate::backend::LocalPlayback::Authorizing => (
            pgettext(locale, "playback status", "Setting up"),
            gettext(locale, "Finish authorizing in your browser."),
            None,
        ),
        crate::backend::LocalPlayback::Connecting => (
            pgettext(locale, "playback status", "Connecting"),
            gettext(locale, "Connecting to Spotify…"),
            None,
        ),
        crate::backend::LocalPlayback::WaitingForNetwork => (
            pgettext(locale, "playback status", "Waiting for network"),
            gettext(
                locale,
                "Waiting for network connectivity to resume playback.",
            ),
            None,
        ),
        crate::backend::LocalPlayback::Failed(message) => (
            pgettext(locale, "playback status", "Unavailable"),
            message.clone().into(),
            Some(gettext(locale, "Try again")),
        ),
        crate::backend::LocalPlayback::Unavailable => (
            pgettext(locale, "playback status", "Not set up"),
            gettext(
                locale,
                "Requires Spotify Premium and a one-time browser sign-in.",
            ),
            Some(gettext(locale, "Enable playback here")),
        ),
    };
    let playback = gettext(locale, "Playback on this computer");
    let normalize_volume = gettext(locale, "Normalize volume");
    let autoplay = gettext(locale, "Autoplay");
    let gapless = gettext(locale, "Gapless playback");
    let keep_playing = gettext(locale, "Keep music playing when the window closes");
    let update_checks = gettext(locale, "Automatic update checks");
    let audio_cache = gettext(locale, "Audio cache");
    let apply_playback = gettext(locale, "Apply and restart playback");
    let apply_playback_note = gettext(locale, "Restart local playback to apply these settings.");
    let download_updates = gettext(locale, "Download updates automatically");
    let playback_rows = [
        RowText::new(
            // Translators: {status} is a playback state such as Ready or Not set up.
            gettext(locale, "Status: {status}").replace("{status}", &status),
            detail,
        ),
        RowText::new(
            gettext(locale, "Device name"),
            gettext(locale, "How this computer appears in Spotify Connect."),
        ),
        RowText::new(
            gettext(locale, "Audio quality"),
            gettext(locale, "Higher bitrates use more data and cache space."),
        ),
        RowText::new(
            normalize_volume.clone(),
            gettext(locale, "Keep loud and quiet tracks at a similar level."),
        ),
        RowText::new(
            autoplay.clone(),
            gettext(locale, "Keep playing similar songs when your music ends."),
        ),
        RowText::new(
            gapless.clone(),
            gettext(locale, "Play tracks without silence between them."),
        ),
        RowText::new(
            keep_playing.clone(),
            super::keys::platform_shortcut(
                &gettext(
                    locale,
                    "Spotify hides to the system tray. Quit from the tray menu or with Ctrl+Q.",
                ),
                &gettext(
                    locale,
                    "Spotify hides to the system tray. Quit from the tray menu or with Cmd+Q.",
                ),
            )
            .to_owned(),
        ),
        RowText::new(
            update_checks.clone(),
            gettext(locale, "Checks GitHub once a day. No personal data is sent."),
        ),
        RowText::new(
            gettext(locale, "Audio output"),
            gettext(
                locale,
                "PulseAudio also covers PipeWire. Rodio talks to ALSA directly.",
            ),
        )
        .when(cfg!(target_os = "linux")),
        RowText::new(
            gettext(locale, "Output buffer"),
            gettext(
                locale,
                "More buffering can prevent clicks on busy computers. Less buffering makes controls respond sooner.",
            ),
        )
        .when(cfg!(windows)),
        RowText::new(
            audio_cache.clone(),
            gettext(locale, "Save downloaded audio for later playback."),
        ),
        RowText::new(apply_playback.clone(), apply_playback_note.clone()).when(playback_dirty),
        RowText::new(gettext(locale, "Playback settings applied"), "").when(!playback_dirty),
        RowText::new(
            download_updates.clone(),
            gettext(
                locale,
                "Downloads in the background. You choose when to restart.",
            ),
        ),
        RowText::new(
            gettext(locale, "MacBook notch widget"),
            gettext(
                locale,
                "Show interactive Now Playing widget when hovering over the notch.",
            ),
        )
        .when(cfg!(target_os = "macos")),
    ];
    if section_matches(&needle, &playback, &playback_rows) {
        any_visible = true;
        section(ui, &palette, &playback, |ui| {
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[0], |ui| {
                if let Some(label) = action {
                    if theme::pill_button(ui, &palette, &label, true).clicked() {
                        app.actions.push(Action::EnablePlayback);
                    }
                } else if app.local_ready
                    && theme::soft_button(
                        ui,
                        &palette,
                        Some(Icon::Refresh),
                        &gettext(locale, "Reconnect"),
                        false,
                    )
                    .clicked()
                {
                    app.actions.push(Action::RestartEngine);
                }
            });
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[1], |ui| {
                let response = Frame::new()
                    .fill(palette.surface)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        widgets::text_edit(
                            ui,
                            locale,
                            egui::TextEdit::singleline(&mut app.settings.device_name)
                                .font(theme::regular(14.0))
                                .frame(egui::Frame::NONE)
                                .desired_width(200.0),
                        )
                    })
                    .inner;
                if response.changed() {
                    changed = true;
                    playback_dirty = true;
                }
            });
            // Normal to Very high, left to right when side by side and top
            // to bottom in a column.
            let choices = [
                (96u16, gettext(locale, "Normal · 96 kbps")),
                (160, gettext(locale, "High · 160 kbps")),
                (320, gettext(locale, "Very high · 320 kbps")),
            ];
            let choice_gap = 6.0;
            let choices_width = choices
                .iter()
                .map(|(_, label)| theme::soft_button_width(ui, label))
                .sum::<f32>()
                + choice_gap * (choices.len() - 1) as f32;
            filtered_row_sized(
                ui,
                &palette,
                &needle,
                &playback,
                &playback_rows[2],
                choices_width,
                |ui| {
                    let mut choose = |ui: &mut egui::Ui, kbps: u16, label: &str| {
                        if theme::soft_button(
                            ui,
                            &palette,
                            None,
                            label,
                            app.settings.bitrate == kbps,
                        )
                        .clicked()
                            && app.settings.bitrate != kbps
                        {
                            app.settings.bitrate = kbps;
                            changed = true;
                            playback_dirty = true;
                        }
                    };
                    if ui.available_width() >= choices_width {
                        // Laid right to left, so the last choice goes first.
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = choice_gap;
                            for (kbps, label) in choices.iter().rev() {
                                choose(ui, *kbps, label);
                            }
                        });
                    } else {
                        // Too narrow even for a line of their own: a column.
                        ui.with_layout(Layout::top_down(Align::Max), |ui| {
                            ui.spacing_mut().item_spacing.y = choice_gap;
                            for (kbps, label) in &choices {
                                choose(ui, *kbps, label);
                            }
                        });
                    }
                },
            );
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[3], |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &normalize_volume,
                    &mut app.settings.normalisation,
                )
                .changed()
                {
                    changed = true;
                    playback_dirty = true;
                }
            });
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[4], |ui| {
                if widgets::switch(ui, &palette, &autoplay, &mut app.settings.autoplay).changed() {
                    changed = true;
                    playback_dirty = true;
                }
            });
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[5], |ui| {
                if widgets::switch(ui, &palette, &gapless, &mut app.settings.gapless).changed() {
                    changed = true;
                    playback_dirty = true;
                }
            });
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[6], |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &keep_playing,
                    &mut app.settings.keep_playing_in_background,
                )
                .changed()
                {
                    changed = true;
                }
            });
            #[cfg(target_os = "macos")]
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[14], |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &gettext(locale, "MacBook notch widget"),
                    &mut app.settings.mac_notch_widget,
                )
                .changed()
                {
                    changed = true;
                }
            });
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[7], |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &update_checks,
                    &mut app.settings.check_for_updates,
                )
                .changed()
                {
                    changed = true;
                }
            });
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[13], |ui| {
                if widgets::switch(
                    ui,
                    &palette,
                    &download_updates,
                    &mut app.settings.download_updates_automatically,
                )
                .changed()
                {
                    changed = true;
                }
            });
            if cfg!(target_os = "linux") {
                filtered_row(ui, &palette, &needle, &playback, &playback_rows[8], |ui| {
                    let current = app
                        .settings
                        .platform_backend()
                        .unwrap_or_else(|| "rodio".into());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for backend in ["rodio", "pulseaudio"] {
                            let label = if backend == "pulseaudio" {
                                "PulseAudio / PipeWire"
                            } else {
                                "ALSA (rodio)"
                            };
                            if theme::soft_button(ui, &palette, None, label, current == backend)
                                .clicked()
                                && current != backend
                            {
                                app.settings.audio_backend = Some(backend.to_string());
                                changed = true;
                                playback_dirty = true;
                            }
                        }
                    });
                });
            }
            #[cfg(windows)]
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[9], |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let current = app.settings.audio_buffer_ms;
                    for ms in [50u32, 100, 200] {
                        let label = format!("{ms} ms");
                        if theme::soft_button(ui, &palette, None, &label, current == ms).clicked()
                            && current != ms
                        {
                            app.settings.audio_buffer_ms = ms;
                            changed = true;
                            playback_dirty = true;
                        }
                    }
                });
            });
            filtered_row(ui, &palette, &needle, &playback, &playback_rows[10], |ui| {
                // The control area lays out right-to-left: add the rightmost item first.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if widgets::switch(ui, &palette, &audio_cache, &mut app.settings.audio_cache)
                        .changed()
                    {
                        changed = true;
                        playback_dirty = true;
                    }
                    if app.settings.audio_cache {
                        ui.add_space(6.0);
                        for (mb, label) in [(4096u64, "4 GB"), (1024, "1 GB"), (512, "512 MB")] {
                            if theme::soft_button(
                                ui,
                                &palette,
                                None,
                                label,
                                app.settings.audio_cache_mb == mb,
                            )
                            .clicked()
                                && app.settings.audio_cache_mb != mb
                            {
                                app.settings.audio_cache_mb = mb;
                                changed = true;
                                playback_dirty = true;
                            }
                        }
                    }
                });
            });
            ui.add_space(4.0);
            if playback_dirty
                || playback_rows[11].matches(&needle, &playback)
                || playback_rows[12].matches(&needle, &playback)
            {
                ui.horizontal(|ui| {
                    if playback_dirty {
                        if theme::pill_button(ui, &palette, &apply_playback, true).clicked() {
                            app.actions.push(Action::RestartEngine);
                            playback_dirty = false;
                        }
                        theme::subtle(ui, &palette, &apply_playback_note);
                    } else {
                        theme::subtle(ui, &palette, &gettext(locale, "Playback settings applied."));
                    }
                });
            }
        });
    }

    let appearance = gettext(locale, "Appearance");
    let theme_title = gettext(locale, "Theme");
    let theme_guide = gettext(locale, "How to make a theme");
    let themes_folder = gettext(locale, "Open themes folder");
    let accent_from_art = gettext(locale, "Colour from album art");
    let sidebar_compact = gettext(locale, "Compact library sidebar");
    let tracklist_compact = gettext(locale, "Compact track list");
    let middle_click = gettext(locale, "Middle-click autoscroll");
    let custom_titlebar = gettext(locale, "Custom title bar");
    let show_custom_titlebar = cfg!(target_os = "linux") || app.windows_controls_visible();
    let player_bar_vis = gettext(locale, "Player bar visualizer");
    let player_bar_vis_click = gettext(locale, "Click player bar to change visualizer");
    let appearance_rows = [
        RowText::new(theme_title.clone(), {
            let detail = theme::catalog_detail(
                &app.custom_themes,
                locale,
                app.settings.custom_theme.as_deref(),
            );
            if !detail.is_empty() {
                detail
            } else if app.custom_themes.follows_omarchy() {
                gettext(locale, "Follow system uses your Omarchy colours.")
            } else {
                gettext(
                    locale,
                    "Follow system uses your desktop's light or dark appearance.",
                )
            }
        }),
        RowText::new(
            gettext(locale, "Language"),
            gettext(
                locale,
                "System follows your computer's language. Untranslated text stays in English.",
            ),
        ),
        RowText::new(
            accent_from_art.clone(),
            gettext(
                locale,
                "Use the current cover's colour on pages and the player bar.",
            ),
        ),
        RowText::new(
            sidebar_compact.clone(),
            gettext(locale, "Show names without covers in the sidebar."),
        ),
        RowText::new(
            tracklist_compact.clone(),
            gettext(locale, "Show each track on one line without a cover."),
        ),
        RowText::new(
            gettext(locale, "Interface zoom"),
            super::keys::platform_shortcut(
                &gettext(
                    locale,
                    "Ctrl+Plus and Ctrl+Minus work anywhere; Ctrl+0 resets.",
                ),
                &gettext(
                    locale,
                    "Cmd+Plus and Cmd+Minus work anywhere; Cmd+0 resets.",
                ),
            )
            .to_owned(),
        ),
        RowText::new(
            middle_click.clone(),
            gettext(
                locale,
                "Middle-click a list, then move the pointer to scroll it. Off by default, because a middle click usually pastes on Linux.",
            ),
        )
        .when(cfg!(target_os = "linux")),
        RowText::new(
            custom_titlebar.clone(),
            gettext(
                locale,
                "Draw Spotify's own title bar and window buttons instead of the desktop's.",
            ),
        )
        .when(show_custom_titlebar),
        RowText::new(
            player_bar_vis.clone(),
            gettext(
                locale,
                "Show the song moving behind the player bar's controls while it plays here.",
            ),
        ),
        RowText::new(
            player_bar_vis_click.clone(),
            gettext(locale, "Allow clicks on empty player bar space to cycle the visualizer."),
        ),
        RowText::new(
            gettext(locale, "Frame rate"),
            gettext(
                locale,
                "How often the player bar and mini player visualizers draw. Lower rates use fewer resources. MilkDrop has its own.",
            ),
        ),
    ];
    if section_matches(&needle, &appearance, &appearance_rows) {
        any_visible = true;
        section(ui, &palette, &appearance, |ui| {
            // Wide enough for the theme's two buttons side by side.
            let theme_buttons_width = theme::soft_button_width(ui, &theme_guide)
                + theme::soft_button_width(ui, &themes_folder)
                + 6.0;
            filtered_row_sized(
                ui,
                &palette,
                &needle,
                &appearance,
                &appearance_rows[0],
                theme_buttons_width,
                |ui| {
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        let selected = app
                            .settings
                            .custom_theme
                            .as_deref()
                            .map(|filename| fastframe_theme::display_name(filename).into())
                            .unwrap_or_else(|| app.settings.theme.label(locale));
                        let response = egui::ComboBox::from_id_salt("appearance_theme")
                            .selected_text(selected.as_ref())
                            .width(200.0_f32.min(ui.available_width()))
                            .show_ui(ui, |ui| {
                                for choice in ThemeChoice::ALL {
                                    if ui
                                        .selectable_label(
                                            app.settings.custom_theme.is_none()
                                                && app.settings.theme == choice,
                                            choice.label(locale).as_ref(),
                                        )
                                        .clicked()
                                    {
                                        app.actions.push(Action::SetTheme(choice));
                                    }
                                }
                                if app.custom_themes.picker_themes().next().is_some() {
                                    ui.separator();
                                }
                                for theme in app.custom_themes.picker_themes() {
                                    if ui
                                        .selectable_label(
                                            app.settings.custom_theme.as_deref()
                                                == Some(theme.filename.as_str()),
                                            fastframe_theme::display_name(&theme.filename),
                                        )
                                        .clicked()
                                    {
                                        app.actions
                                            .push(Action::SetCustomTheme(theme.filename.clone()));
                                    }
                                }
                            });
                        response.response.widget_info(|| {
                            let mut info = egui::WidgetInfo::labeled(
                                egui::WidgetType::ComboBox,
                                ui.is_enabled(),
                                theme_title.as_ref(),
                            );
                            info.current_text_value = Some(selected.to_string());
                            info
                        });
                        // The guide to writing a theme sits beside the folder
                        // it goes in, and above it when both do not fit.
                        let (guide, folder) = (&theme_guide, &themes_folder);
                        let gap = 6.0;
                        let mut buttons = |ui: &mut egui::Ui| {
                            if theme::soft_button(ui, &palette, Some(Icon::Globe), guide, false)
                                .clicked()
                            {
                                app.actions.push(Action::OpenUrl(THEMES_GUIDE_URL.into()));
                            }
                            if theme::soft_button(
                                ui,
                                &palette,
                                Some(Icon::ExternalLink),
                                folder,
                                false,
                            )
                            .clicked()
                            {
                                app.actions.push(Action::OpenThemesFolder);
                            }
                        };
                        let both = theme::soft_button_width(ui, guide)
                            + theme::soft_button_width(ui, folder)
                            + gap;
                        if ui.available_width() >= both {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = gap;
                                buttons(ui);
                            });
                        } else {
                            ui.spacing_mut().item_spacing.y = gap;
                            buttons(ui);
                        }
                    });
                },
            );
            filtered_row(
                ui,
                &palette,
                &needle,
                &appearance,
                &appearance_rows[1],
                |ui| language_picker(app, ui),
            );
            filtered_row(
                ui,
                &palette,
                &needle,
                &appearance,
                &appearance_rows[2],
                |ui| {
                    if widgets::switch(
                        ui,
                        &palette,
                        &accent_from_art,
                        &mut app.settings.accent_from_art,
                    )
                    .changed()
                    {
                        changed = true;
                    }
                },
            );
            {
                use crate::settings::PlayerBarVis;
                let choices = [
                    (PlayerBarVis::Off, gettext(locale, "Off")),
                    (PlayerBarVis::Spectrum, gettext(locale, "Spectrum")),
                    (PlayerBarVis::Waveform, gettext(locale, "Waveform")),
                ];
                let choice_gap = 6.0;
                let choices_width = choices
                    .iter()
                    .map(|(_, label)| theme::soft_button_width(ui, label))
                    .sum::<f32>()
                    + choice_gap * (choices.len() - 1) as f32;
                filtered_row_sized(
                    ui,
                    &palette,
                    &needle,
                    &appearance,
                    &appearance_rows[8],
                    choices_width,
                    |ui| {
                        let mut choose = |ui: &mut egui::Ui, mode: PlayerBarVis, label: &str| {
                            if theme::soft_button(
                                ui,
                                &palette,
                                None,
                                label,
                                app.settings.player_bar_vis == mode,
                            )
                            .clicked()
                                && app.settings.player_bar_vis != mode
                            {
                                app.settings.player_bar_vis = mode;
                                changed = true;
                            }
                        };
                        if ui.available_width() >= choices_width {
                            // Laid right to left, so the last choice goes first.
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = choice_gap;
                                for (mode, label) in choices.iter().rev() {
                                    choose(ui, *mode, label);
                                }
                            });
                        } else {
                            ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                ui.spacing_mut().item_spacing.y = choice_gap;
                                for (mode, label) in &choices {
                                    choose(ui, *mode, label);
                                }
                            });
                        }
                    },
                );
            }
            filtered_row(
                ui,
                &palette,
                &needle,
                &appearance,
                &appearance_rows[9],
                |ui| {
                    if widgets::switch(
                        ui,
                        &palette,
                        &player_bar_vis_click,
                        &mut app.settings.player_bar_vis_click,
                    )
                    .changed()
                    {
                        changed = true;
                    }
                },
            );
            filtered_row(
                ui,
                &palette,
                &needle,
                &appearance,
                &appearance_rows[10],
                |ui| {
                    if let Some(rate) = frame_rate_dial(
                        ui,
                        &palette,
                        locale,
                        app.visualiser_screen_hz(),
                        app.settings.visualizer_fps,
                    ) {
                        app.settings.visualizer_fps = rate;
                        changed = true;
                    }
                },
            );
            filtered_row(
                ui,
                &palette,
                &needle,
                &appearance,
                &appearance_rows[3],
                |ui| {
                    if widgets::switch(
                        ui,
                        &palette,
                        &sidebar_compact,
                        &mut app.settings.sidebar_compact,
                    )
                    .changed()
                    {
                        changed = true;
                    }
                },
            );
            filtered_row(
                ui,
                &palette,
                &needle,
                &appearance,
                &appearance_rows[4],
                |ui| {
                    if widgets::switch(
                        ui,
                        &palette,
                        &tracklist_compact,
                        &mut app.settings.tracklist_compact,
                    )
                    .changed()
                    {
                        changed = true;
                    }
                },
            );
            if appearance_rows[5].matches(&needle, &appearance) {
                theme::text(ui, gettext(locale, "Zoom level"), theme::semibold(14.0), palette.text);
                ui.add_space(8.0);
                theme::text(ui, gettext(locale, "Adjust the zoom level to fit Spotify to your preferences and needs."), theme::regular(13.0), palette.secondary);
                ui.add_space(8.0);
                theme::text(ui, super::keys::platform_shortcut(
                    &gettext(locale, "You can also change the zoom with Ctrl+- or Ctrl++."),
                    &gettext(locale, "You can also change the zoom with Cmd+- or Cmd++."),
                ), theme::regular(13.0), palette.secondary);
                ui.add_space(20.0);
                zoom_level_card(app, ui);
                ui.add_space(10.0);
            }
            if cfg!(target_os = "linux") {
                filtered_row(
                    ui,
                    &palette,
                    &needle,
                    &appearance,
                    &appearance_rows[6],
                    |ui| {
                        if widgets::switch(
                            ui,
                            &palette,
                            &middle_click,
                            &mut app.settings.middle_click_autoscroll,
                        )
                        .changed()
                        {
                            changed = true;
                        }
                    },
                );
            }
            if show_custom_titlebar {
                filtered_row(
                    ui,
                    &palette,
                    &needle,
                    &appearance,
                    &appearance_rows[7],
                    |ui| {
                        let mut custom = app.settings.custom_titlebar;
                        if widgets::switch(ui, &palette, &custom_titlebar, &mut custom).changed() {
                            app.actions.push(Action::SetCustomTitlebar(custom));
                        }
                    },
                );
            }
        });
    }

    let proxy = gettext(locale, "Proxy");
    let proxy_rows = [RowText::new(
        gettext(locale, "Mode, host, port, username and password"),
        gettext(
            locale,
            "Off, System, HTTP or SOCKS5 proxy for network requests.",
        ),
    )];
    if section_matches(&needle, &proxy, &proxy_rows) {
        any_visible = true;
        ui.push_id("proxy-settings", |ui| {
    section(ui, &palette, &proxy, |ui| {
        widgets::setting_row(
            ui,
            &palette,
            &pgettext(locale, "proxy", "Mode"),
            &gettext(
                locale,
                "Off ignores environment variables. System uses them, and the OS proxy on macOS and Windows.",
            ),
            |_| {},
        );
        // The row's control slot is right-to-left and too narrow for four
        // choices; they sit on this line so they read Off, System, HTTP, SOCKS5.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for choice in ProxyMode::ALL {
                if theme::soft_button(
                    ui,
                    &palette,
                    None,
                    &choice.label(locale),
                    app.settings.proxy_mode == choice,
                )
                .clicked()
                    && app.settings.proxy_mode != choice
                {
                    app.settings.proxy_mode = choice;
                    changed = true;
                    app.actions.push(Action::ProxyEdited);
                    if choice.is_manual() {
                        proxy_dirty = true;
                    } else {
                        proxy_dirty = false;
                        app.actions.push(Action::ApplyProxy);
                    }
                }
            }
        });
        ui.add_space(10.0);
        if app.settings.proxy_mode.is_manual() {
            if widgets::proxy_manual_form(
                ui,
                &palette,
                app.locale,
                &mut app.settings.proxy_host,
                &mut app.settings.proxy_port,
                &mut app.settings.proxy_username,
                &mut app.settings.proxy_password,
            ) {
                changed = true;
                app.actions.push(Action::ProxyEdited);
                proxy_dirty = true;
            }
            ui.add_space(6.0);
            widgets::proxy_scope_note(ui, &palette, app.locale, app.settings.proxy_mode);
            ui.add_space(10.0);
        }
        if app.settings.proxy_mode.is_manual() {
            ui.horizontal(|ui| {
                if theme::pill_button(ui, &palette, &gettext(locale, "Apply settings"), true).clicked() {
                    app.actions.push(Action::ApplyProxy);
                    if app.settings.proxy_config().is_ok() {
                        proxy_dirty = false;
                    }
                }
            });
        } else {
            theme::subtle(
                ui,
                &palette,
                &match app.settings.proxy_mode {
                    ProxyMode::Off => gettext(locale, "Not using a proxy."),
                    ProxyMode::System => gettext(locale, "Using the system proxy."),
                    ProxyMode::Http | ProxyMode::Socks => "".into(),
                },
            );
        }
    });
        });
    }

    let skins_folder = app.dirs.skins_dir();
    app.winamp.refresh_choices(&skins_folder);
    let skins = gettext(locale, "Winamp skins");
    let always_on_top = gettext(locale, "Always on top");
    let show_in_taskbar = gettext(locale, "Show Winamp in taskbar");
    let skins_rows = [
        RowText::new(
            gettext(locale, "Mini player"),
            super::keys::platform_shortcut(
                &gettext(
                    locale,
                    "Use classic Winamp .wsz skins. Press Ctrl+M or click the skin logo to return. Drop a skin on either window to add it.",
                ),
                &gettext(
                    locale,
                    "Use classic Winamp .wsz skins. Press Cmd+Shift+M or click the skin logo to return. Drop a skin on either window to add it.",
                ),
            )
            .to_owned(),
        ),
        RowText::new(
            gettext(locale, "Skin"),
            gettext(
                locale,
                // Translators: {folder} is the path of the skins folder.
                "Installed skins are in {folder}. Find more at the Winamp Skin Museum.",
            )
            .replace("{folder}", &skins_folder.display().to_string()),
        ),
        RowText::new(
            gettext(locale, "Size"),
            gettext(locale, "Whole-number scaling keeps skin pixels sharp."),
        ),
        RowText::new(
            always_on_top.clone(),
            if app.window_level_supported {
                gettext(locale, "Keep the Winamp window above everything else.")
            } else {
                crate::window::on_top_unavailable(locale)
            },
        ),
        RowText::new(
            gettext(locale, "Show in taskbar"),
            gettext(
                locale,
                "Keep a taskbar button for the mini player. The tray icon stays available when hidden.",
            ),
        )
        .when(app.taskbar_setting_visible()),
        RowText::new(
            gettext(locale, "Installed skins"),
            app.winamp
                .choices
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        ),
    ];
    if section_matches(&needle, &skins, &skins_rows) {
        any_visible = true;
        section(ui, &palette, &skins, |ui| {
            filtered_row(ui, &palette, &needle, &skins, &skins_rows[0], |ui| {
                if theme::pill_button(ui, &palette, &gettext(locale, "Switch to it"), true)
                    .clicked()
                {
                    app.actions.push(Action::ToggleWinampWindow);
                }
            });
            filtered_row(ui, &palette, &needle, &skins, &skins_rows[1], |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if theme::soft_button(
                        ui,
                        &palette,
                        Some(Icon::Globe),
                        &gettext(locale, "Skin Museum"),
                        false,
                    )
                    .clicked()
                    {
                        app.actions
                            .push(Action::OpenUrl("https://skins.webamp.org/".into()));
                    }
                    if theme::soft_button(
                        ui,
                        &palette,
                        Some(Icon::ExternalLink),
                        &open_folder,
                        false,
                    )
                    .clicked()
                    {
                        app.actions.push(Action::OpenSkinsFolder);
                    }
                });
            });
            let choices = app.winamp.choices.clone();
            if skins_rows[1].matches(&needle, &skins) || skins_rows[5].matches(&needle, &skins) {
                // Random comes first; it picks among the others each time
                // the mini player opens.
                const RANDOM: usize = usize::MAX;
                let random = gettext(locale, "Random");
                let mut options: Vec<(usize, &str)> = vec![(RANDOM, &random), (0, "Spotify")];
                options.extend(
                    choices
                        .iter()
                        .enumerate()
                        .map(|(index, choice)| (index + 1, choice.label())),
                );
                let showing = app
                    .settings
                    .skin
                    .as_deref()
                    .and_then(|name| choices.iter().position(|choice| choice.name == name))
                    .map_or(0, |index| index + 1);
                let current = if app.settings.random_skin {
                    RANDOM
                } else {
                    showing
                };
                if let Some(picked) = widgets::chips(ui, &palette, &options, current)
                    && picked != current
                {
                    if picked == RANDOM {
                        app.actions.push(Action::SetRandomSkin(true));
                    } else {
                        let name = picked
                            .checked_sub(1)
                            .map(|index| choices[index].name.clone());
                        app.actions.push(Action::SetSkin(name));
                    }
                }
                if app.settings.random_skin {
                    let label = options
                        .iter()
                        .find(|(value, _)| *value == showing)
                        .map_or("Spotify", |(_, label)| label);
                    theme::subtle(
                        ui,
                        &palette,
                        &gettext(locale, "Now showing {skin}. Another one is picked each time the mini player opens.")
                            .replace("{skin}", label),
                    );
                }
                ui.add_space(4.0);
            }
            filtered_row(ui, &palette, &needle, &skins, &skins_rows[2], |ui| {
                let scale =
                    crate::winamp::WinampState::scale(&app.settings, ui.ctx().pixels_per_point());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for candidate in 1..=crate::winamp::MAX_SCALE {
                        let label = format!("{candidate}x");
                        if theme::soft_button(ui, &palette, None, &label, candidate == scale)
                            .clicked()
                            && candidate != scale
                        {
                            app.actions.push(Action::SetSkinScale(candidate as u8));
                        }
                    }
                });
            });
            filtered_row(ui, &palette, &needle, &skins, &skins_rows[3], |ui| {
                ui.add_enabled_ui(app.window_level_supported, |ui| {
                    let mut on_top = app.settings.winamp_on_top && app.window_level_supported;
                    if widgets::switch(ui, &palette, &always_on_top, &mut on_top).changed() {
                        app.actions.push(Action::ToggleWinampOnTop);
                    }
                });
            });
            if app.taskbar_setting_visible() {
                filtered_row(ui, &palette, &needle, &skins, &skins_rows[4], |ui| {
                    let mut visible = app.settings.winamp_show_taskbar;
                    let response = widgets::switch(ui, &palette, &show_in_taskbar, &mut visible);
                    if response.changed() {
                        app.actions.push(Action::SetWinampTaskbar(visible));
                    }
                    #[cfg(any(test, feature = "demo"))]
                    if app.demo_windows_controls {
                        let id = egui::Id::new("demo-winamp-taskbar-focus");
                        if !ui.data(|data| data.get_temp::<bool>(id)).unwrap_or(false) {
                            response.scroll_to_me(Some(Align::Center));
                            ui.data_mut(|data| data.insert_temp(id, true));
                        }
                    }
                });
            }
        });
    }

    let presets_folder = app.dirs.milkdrop_dir();
    app.winamp
        .presets
        .refresh(&presets_folder, Some(ui.ctx().clone()));
    let count = app.winamp.presets.count();
    let screen_hz = app.settings.milkdrop_screen_hz;
    let milkdrop_window = gettext(locale, "MilkDrop window");
    let folder = presets_folder.display().to_string();
    let milkdrop_rows = [
        RowText::new(
            milkdrop_window.clone(),
            super::keys::platform_shortcut(
                &gettext(
                    locale,
                    "A projectM visualiser for local playback. Open it here, from the top bar, with Ctrl+Shift+K, or from the mini player's V menu. Press ? or F1 for its shortcuts.",
                ),
                &gettext(
                    locale,
                    "A projectM visualiser for local playback. Open it here, from the top bar, with Cmd+Shift+K, or from the mini player's V menu. Press ? or F1 for its shortcuts.",
                ),
            )
            .to_owned(),
        ),
        RowText::new(
            gettext(locale, "Presets"),
            match count {
                0 => gettext(
                    locale,
                    // Translators: {folder} is the path of the MilkDrop presets folder.
                    "None yet in {folder}. Add .milk files here. Spotify downloads presets when MilkDrop first opens with an empty folder.",
                )
                .replace("{folder}", &folder),
                1 => gettext(
                    locale,
                    // Translators: {folder} is the path of the MilkDrop presets folder.
                    "One preset in {folder}. Add .milk files here. Spotify downloads presets when MilkDrop first opens with an empty folder.",
                )
                .replace("{folder}", &folder),
                n => ngettext(
                    locale,
                    // Translators: {count} is the number of presets, {folder} the path of the MilkDrop presets folder.
                    "{count} preset in {folder}. Add .milk files here. Spotify downloads presets when MilkDrop first opens with an empty folder.",
                    "{count} presets in {folder}. Add .milk files here. Spotify downloads presets when MilkDrop first opens with an empty folder.",
                    u32::try_from(n).unwrap_or(u32::MAX),
                )
                .replace("{count}", &n.to_string())
                .replace("{folder}", &folder),
            },
        ),
        RowText::new(
            gettext(locale, "Time per preset"),
            gettext(
                locale,
                "How long each preset plays before the next fades in.",
            ),
        ),
        RowText::new(
            gettext(locale, "Frame rate"),
            match screen_hz {
                hz if !crate::milkdrop::is_screen_rate(hz) => gettext(
                    locale,
                    "Lower rates use fewer resources. Uncapped draws as fast as possible.",
                ),
                hz => gettext(
                    locale,
                    // Translators: {hz} is the screen's refresh rate in hertz.
                    "Your screen refreshes at {hz} Hz. Higher rates do not add visible frames. Uncapped draws as fast as possible.",
                )
                .replace("{hz}", &hz.to_string())
                .into(),
            },
        ),
        RowText::new(
            gettext(locale, "Resolution"),
            gettext(
                locale,
                "Half and Quarter use fewer resources and scale the image back up.",
            ),
        ),
        RowText::new(
            format!(
                "{} {}",
                // Translators: search keywords for the MilkDrop preset download buttons.
                gettext(locale, "Get presets"),
                open_folder
            ),
            crate::milkdrop::PACKS
                .iter()
                .map(|pack| pack.name)
                .collect::<Vec<_>>()
                .join(" "),
        ),
    ];
    if section_matches(&needle, "MilkDrop", &milkdrop_rows) {
        any_visible = true;
        section(ui, &palette, "MilkDrop", |ui| {
            filtered_row(ui, &palette, &needle, "MilkDrop", &milkdrop_rows[0], |ui| {
                let mut open = app.settings.milkdrop_open;
                if widgets::switch(ui, &palette, &milkdrop_window, &mut open).changed() {
                    app.actions.push(Action::ToggleWinampMilkdrop);
                }
            });
            let downloading = app.winamp.presets.downloading();
            filtered_row(
                ui,
                &palette,
                &needle,
                "MilkDrop",
                &milkdrop_rows[1],
                |_ui| {},
            );
            // Three buttons are wider than a row's control slot; they get a
            // line of their own under the words.
            if milkdrop_rows[1].matches(&needle, "MilkDrop")
                || milkdrop_rows[5].matches(&needle, "MilkDrop")
            {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (index, pack) in crate::milkdrop::PACKS.iter().enumerate() {
                        let label = match downloading {
                            Some(name) if name == pack.name => {
                                gettext(locale, "Fetching...").into_owned()
                            }
                            // Translators: {pack} is the name of a MilkDrop preset pack.
                            _ => gettext(locale, "Get {pack}").replace("{pack}", pack.name),
                        };
                        if theme::soft_button(ui, &palette, Some(Icon::Globe), &label, false)
                            .on_hover_text(pack.note)
                            .clicked()
                            && downloading.is_none()
                        {
                            app.actions.push(Action::DownloadMilkdropPack(index));
                        }
                    }
                    if theme::soft_button(
                        ui,
                        &palette,
                        Some(Icon::ExternalLink),
                        &open_folder,
                        false,
                    )
                    .clicked()
                    {
                        app.actions.push(Action::OpenMilkdropFolder);
                    }
                });
                ui.add_space(10.0);
            }
            filtered_row(ui, &palette, &needle, "MilkDrop", &milkdrop_rows[2], |ui| {
                let mut seconds = app.settings.milkdrop_seconds.clamp(2, 300);
                let changed = setting_slider(
                    ui,
                    &palette,
                    &mut seconds,
                    2..=300,
                    |slider| slider.logarithmic(true),
                    |value| value.suffix(" s"),
                )
                .changed();
                if changed {
                    app.actions.push(Action::SetMilkdropSeconds(seconds));
                }
            });
            filtered_row(ui, &palette, &needle, "MilkDrop", &milkdrop_rows[3], |ui| {
                if let Some(rate) =
                    frame_rate_dial(ui, &palette, locale, screen_hz, app.settings.milkdrop_fps)
                {
                    app.actions.push(Action::SetMilkdropFps(rate));
                }
            });
            filtered_row(ui, &palette, &needle, "MilkDrop", &milkdrop_rows[4], |ui| {
                let current = app.settings.milkdrop_scale.max(1);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    for (scale, label) in [
                        (1u32, pgettext(locale, "resolution", "Full")),
                        (2, pgettext(locale, "resolution", "Half")),
                        (4, pgettext(locale, "resolution", "Quarter")),
                    ] {
                        if theme::soft_button(ui, &palette, None, &label, scale == current)
                            .clicked()
                            && scale != current
                        {
                            app.actions.push(Action::SetMilkdropScale(scale));
                        }
                    }
                });
            });
        });
    }

    let equalizer = gettext(locale, "Equalizer");
    let equalizer_rows = [
        RowText::new(
            equalizer.clone(),
            gettext(
                locale,
                "A ten-band equalizer for playback on this computer. It does not affect other devices.",
            ),
        ),
        RowText::new(
            format!(
                "{} {} {} dB",
                gettext(locale, "Presets"),
                // Translators: a search keyword for the equalizer's preamplifier slider.
                gettext(locale, "Preamp"),
                // Translators: a search keyword for the equalizer's frequency band sliders.
                gettext(locale, "Bands"),
            ),
            crate::eq::PRESETS
                .iter()
                .map(|preset| preset.name)
                .collect::<Vec<_>>()
                .join(" "),
        ),
    ];
    if section_matches(&needle, &equalizer, &equalizer_rows) {
        any_visible = true;
        section(ui, &palette, &equalizer, |ui| {
            filtered_row(
                ui,
                &palette,
                &needle,
                &equalizer,
                &equalizer_rows[0],
                |ui| {
                    let mut on = app.settings.eq_on;
                    if widgets::switch(ui, &palette, &equalizer, &mut on).changed() {
                        app.actions.push(Action::ToggleEq);
                    }
                },
            );
            let eq_extra_visible = equalizer_rows[1].matches(&needle, &equalizer);
            if eq_extra_visible {
                ui.add_space(6.0);
                spotify_6band_equalizer(app, ui);
            }
        });
    }

    let storage = gettext(locale, "Storage");
    let storage_rows = [
        RowText::new(
            gettext(locale, "Artwork cache"),
            // Translators: {folder} is the path of a cache folder.
            gettext(locale, "Stored in {folder}")
                .replace("{folder}", &app.dirs.art_cache_dir().display().to_string()),
        ),
        RowText::new(
            audio_cache.clone(),
            // Translators: {folder} is the path of a cache folder.
            gettext(locale, "Stored in {folder}").replace(
                "{folder}",
                &app.dirs.audio_cache_dir().display().to_string(),
            ),
        ),
        RowText::new(
            gettext(locale, "Play history"),
            gettext(
                locale,
                // Translators: {file} is the path of the play history file.
                "Tracks played here are stored in {file}. This file is never uploaded.",
            )
            .replace("{file}", &app.dirs.history_file().display().to_string()),
        ),
        RowText::new(
            gettext(locale, "Sign-in"),
            gettext(
                locale,
                "Sign-ins are saved in the system credential store when available.",
            ),
        ),
    ];
    if section_matches(&needle, &storage, &storage_rows) {
        any_visible = true;
        section(ui, &palette, &storage, |ui| {
            filtered_row(ui, &palette, &needle, &storage, &storage_rows[0], |ui| {
                if theme::soft_button(
                    ui,
                    &palette,
                    Some(Icon::Trash),
                    &gettext(locale, "Clear artwork"),
                    false,
                )
                .clicked()
                {
                    app.actions.push(Action::ClearArtCache);
                }
            });
            filtered_row(ui, &palette, &needle, &storage, &storage_rows[1], |_| {});
            filtered_row(ui, &palette, &needle, &storage, &storage_rows[2], |ui| {
                if theme::soft_button(
                    ui,
                    &palette,
                    Some(Icon::Trash),
                    &gettext(locale, "Clear history"),
                    false,
                )
                .clicked()
                {
                    app.actions.push(Action::ClearPlayHistory);
                }
            });
            filtered_row(ui, &palette, &needle, &storage, &storage_rows[3], |_| {});
        });
    }

    let about = gettext(locale, "About");
    let built_with = gettext(
        locale,
        "Built with Rust, egui, and librespot. Not affiliated with Spotify.",
    );
    let check_for_updates = gettext(locale, "Check for updates");
    let checking = gettext(locale, "Checking…");
    let keyboard_shortcuts = gettext(locale, "Keyboard shortcuts");
    let source_code = gettext(locale, "Source code");
    let about_rows = [
        RowText::new(
            format!("Spotify {}", env!("CARGO_PKG_VERSION")),
            built_with.clone(),
        ),
        RowText::new(
            format!("{check_for_updates} {checking}"),
            format!("{keyboard_shortcuts} {source_code}"),
        ),
    ];
    if section_matches(&needle, &about, &about_rows) {
        any_visible = true;
        section(ui, &palette, &about, |ui| {
            ui.horizontal(|ui| {
                let (logo, _) = ui.allocate_exact_size(Vec2::splat(40.0), egui::Sense::hover());
                theme::logo(ui, logo.center(), 40.0);
                ui.vertical(|ui| {
                    theme::text(
                        ui,
                        format!("Spotify {}", env!("CARGO_PKG_VERSION")),
                        theme::semibold(15.0),
                        palette.text,
                    );
                    theme::text(
                        ui,
                        built_with.as_ref(),
                        theme::regular(13.0),
                        palette.secondary,
                    );
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let check_label = if app.update_checking {
                    &checking
                } else {
                    &check_for_updates
                };
                if theme::soft_button(ui, &palette, Some(Icon::Refresh), check_label, false)
                    .clicked()
                    && !app.update_checking
                {
                    app.actions.push(Action::CheckForUpdates);
                }
                if theme::soft_button(ui, &palette, Some(Icon::Info), &keyboard_shortcuts, false)
                    .clicked()
                {
                    app.actions.push(Action::ShowDialog(Dialog::Shortcuts));
                }
                if theme::soft_button(ui, &palette, Some(Icon::ExternalLink), &source_code, false)
                    .clicked()
                {
                    ui.ctx()
                        .open_url(egui::OpenUrl::new_tab(env!("CARGO_PKG_REPOSITORY")));
                }
            });
            ui.add_space(14.0);
            if widgets::credit(ui, &palette, locale) {
                app.actions
                    .push(Action::OpenUrl(widgets::AUTHOR_URL.to_owned()));
            }
        });
    }

    if !needle.is_empty() && !any_visible {
        widgets::empty_state(
            ui,
            &palette,
            Icon::Search,
            // Translators: {query} is the text typed into the settings search field.
            &gettext(locale, "No settings for “{query}”").replace("{query}", &needle),
            &gettext(locale, "Try fewer words, or check the spelling."),
        );
    }

    ui.data_mut(|data| data.insert_temp(dirty_id, playback_dirty));
    ui.data_mut(|data| data.insert_temp(proxy_dirty_id, proxy_dirty));
    if changed {
        app.actions.push(Action::SettingsChanged);
    }
}

/// The interface language: System first, then each language by its own name,
/// so a reader can find theirs whatever language the app is showing.
fn language_picker(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let system = pgettext(locale, "language", "System");
    let current = app.settings.language;
    let selected = match current {
        LanguageChoice::System => system.clone(),
        LanguageChoice::Locale(chosen) => chosen.native_name().into(),
    };
    let response = egui::ComboBox::from_id_salt("interface_language")
        .selected_text(selected.as_ref())
        .width(200.0_f32.min(ui.available_width()))
        // As many languages as a menu holds before it scrolls, not five.
        .height(1000.0)
        .show_ui(ui, |ui| {
            let choices = std::iter::once((LanguageChoice::System, system.clone())).chain(
                crate::i18n::LOCALES
                    .iter()
                    .map(|&each| (LanguageChoice::Locale(each), each.native_name().into())),
            );
            for (choice, label) in choices {
                if ui
                    .selectable_label(current == choice, label.as_ref())
                    .clicked()
                    && current != choice
                {
                    app.actions.push(Action::SetLanguage(choice));
                }
            }
        });
    let name = gettext(locale, "Language");
    response.response.widget_info(|| {
        let mut info =
            egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, ui.is_enabled(), name.as_ref());
        info.current_text_value = Some(selected.to_string());
        info
    });
}

/// Visual Zoom Level Card (`Denso` / `Padrão` / `Amplo` + `70%`–`130%` radio ruler).
fn zoom_level_card(app: &mut App, ui: &mut egui::Ui) {
    use egui::{Rect, Sense, pos2, vec2};
    let palette = app.palette;
    let locale = app.locale;
    let mut zoom = app.settings.zoom;
    Frame::new()
        .fill(palette.surface)
        .corner_radius(8.0)
        .inner_margin(Margin::symmetric(24, 24))
        .show(ui, |ui| {
            let width = ui.available_width();
            let column = width / 3.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (index, (label, value)) in [
                    (gettext(locale, "Dense"), 0.9),
                    (gettext(locale, "Default"), 1.0),
                    (gettext(locale, "Spacious"), 1.1),
                ]
                .into_iter()
                .enumerate()
                {
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(column, 116.0), Sense::click());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            ui.is_enabled(),
                            label.as_ref(),
                        )
                    });
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let active = if index == 0 {
                        zoom < 0.95
                    } else if index == 1 {
                        (0.95..=1.05).contains(&zoom)
                    } else {
                        zoom > 1.05
                    };
                    let preview = Rect::from_center_size(
                        pos2(rect.center().x, rect.top() + 40.0),
                        vec2((column - 8.0).clamp(16.0, 80.0), 64.0),
                    );
                    let painter = ui.painter();
                    painter.rect_stroke(
                        preview,
                        1.0,
                        Stroke::new(1.0, palette.secondary),
                        egui::StrokeKind::Inside,
                    );
                    let inner = preview.shrink(4.0);
                    let columns = [5, 3, 3][index];
                    let rows = [5, 3, 2][index];
                    let fill = if active {
                        palette.accent
                    } else {
                        palette.secondary.gamma_multiply(0.45)
                    };
                    let cell = vec2(inner.width() / columns as f32, inner.height() / rows as f32);
                    for row in 0..rows {
                        for col in 0..columns {
                            painter.rect_filled(
                                Rect::from_min_size(
                                    inner.min + vec2(col as f32 * cell.x, row as f32 * cell.y),
                                    cell,
                                )
                                .shrink(1.0),
                                1.0,
                                fill,
                            );
                        }
                    }
                    painter.text(
                        pos2(rect.center().x, rect.bottom() - 14.0),
                        egui::Align2::CENTER_CENTER,
                        label.as_ref(),
                        theme::semibold(13.0),
                        palette.text,
                    );
                    if response.clicked() {
                        zoom = value;
                    }
                }
            });
            let stops = [70, 80, 90, 100, 110, 120, 130];
            let step = width / stops.len() as f32;
            let top = ui.cursor().top();
            ui.painter().hline(
                (ui.cursor().left() + step / 2.0)..=(ui.cursor().left() + width - step / 2.0),
                top + 12.0,
                Stroke::new(1.0, palette.outline),
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for pct in stops {
                    let (rect, response) = ui.allocate_exact_size(vec2(step, 56.0), Sense::click());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            ui.is_enabled(),
                            format!("{pct}%"),
                        )
                    });
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let center = pos2(rect.center().x, rect.top() + 12.0);
                    ui.painter().circle_filled(center, 7.0, palette.surface);
                    ui.painter().circle_stroke(
                        center,
                        6.0,
                        Stroke::new(
                            if (zoom * 100.0 - pct as f32).abs() < 1.0 {
                                3.0
                            } else {
                                1.0
                            },
                            if (zoom * 100.0 - pct as f32).abs() < 1.0 {
                                palette.accent
                            } else {
                                palette.secondary
                            },
                        ),
                    );
                    ui.painter().text(
                        pos2(rect.center().x, rect.top() + 38.0),
                        egui::Align2::CENTER_CENTER,
                        format!("{pct}%"),
                        theme::regular(13.0),
                        palette.secondary,
                    );
                    if response.clicked() {
                        zoom = pct as f32 / 100.0;
                    }
                }
            });
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::pill_button(ui, &palette, &gettext(locale, "Reset"), false)
                    .clicked()
                {
                    zoom = 1.0;
                }
            });
        });
    if (zoom - app.settings.zoom).abs() > 0.001 {
        app.settings.zoom = zoom;
        ui.ctx().set_zoom_factor(zoom);
        app.mark_settings_dirty();
    }
}

fn eq_preset_label(
    locale: crate::i18n::Locale,
    name: &'static str,
) -> std::borrow::Cow<'static, str> {
    match name {
        "Flat" => gettext(locale, "Flat"),
        "Classical" => gettext(locale, "Classical"),
        "Club" => gettext(locale, "Club"),
        "Dance" => gettext(locale, "Dance"),
        "Full Bass" => gettext(locale, "Full Bass"),
        "Full Bass & Treble" => gettext(locale, "Full Bass & Treble"),
        "Full Treble" => gettext(locale, "Full Treble"),
        "Laptop Speakers / Headphones" => gettext(locale, "Laptop Speakers / Headphones"),
        "Large Hall" => gettext(locale, "Large Hall"),
        "Live" => gettext(locale, "Live"),
        "Party" => gettext(locale, "Party"),
        "Pop" => gettext(locale, "Pop"),
        "Reggae" => gettext(locale, "Reggae"),
        "Rock" => gettext(locale, "Rock"),
        "Ska" => gettext(locale, "Ska"),
        "Soft" => gettext(locale, "Soft"),
        "Soft Rock" => gettext(locale, "Soft Rock"),
        "Techno" => gettext(locale, "Techno"),
        "Bass Booster" => gettext(locale, "Bass Booster"),
        "Bass Reducer" => gettext(locale, "Bass Reducer"),
        "Treble Booster" => gettext(locale, "Treble Booster"),
        "Vocal Booster" => gettext(locale, "Vocal Booster"),
        "Small Speakers" => gettext(locale, "Small Speakers"),
        "Spoken Word" => gettext(locale, "Spoken Word"),
        "Loudness" => gettext(locale, "Loudness"),
        "Night Listening" => gettext(locale, "Night Listening"),
        other => std::borrow::Cow::Borrowed(other),
    }
}

/// Official Spotify Desktop 6-Band Interactive Cubic Hermite Spline Equalizer
/// (`60Hz, 150Hz, 400Hz, 1KHz, 2.4KHz, 15KHz`), fully styled by `app.palette`.
fn spotify_6band_equalizer(app: &mut App, ui: &mut egui::Ui) {
    use egui::{Color32, Mesh, Pos2, Rect, Sense, Shape, Stroke, pos2, vec2};
    let palette = app.palette;
    let locale = app.locale;
    let current = crate::eq::PRESETS
        .iter()
        .position(|preset| preset.bands_db == app.settings.eq_bands_db)
        .unwrap_or(usize::MAX);

    Frame::new()
        .fill(palette.surface)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(24, 20))
        .show(ui, |ui| {
            // Top bar inside card: "Predefinições" + dropdown ("Plano ⌄") on the left
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                theme::text(
                    ui,
                    gettext(locale, "Presets"),
                    theme::medium(13.5),
                    palette.secondary,
                );
                let selected_name = crate::eq::PRESETS
                    .get(current)
                    .map(|p| eq_preset_label(locale, p.name))
                    .unwrap_or_else(|| gettext(locale, "Custom"));
                egui::ComboBox::from_id_salt("eq_presets_dropdown")
                    .selected_text(selected_name)
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for (index, preset) in crate::eq::PRESETS.iter().enumerate() {
                            let label = eq_preset_label(locale, preset.name);
                            if ui.selectable_label(current == index, label).clicked() {
                                app.actions.push(Action::ApplyEqPreset(index));
                            }
                        }
                    });
            });

            ui.add_space(16.0);

            let width = ui.available_width();
            let height = 220.0;
            let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                let plot = Rect::from_min_max(
                    pos2(rect.left() + 86.0, rect.top() + 14.0),
                    pos2(rect.right() - 48.0, rect.bottom() - 36.0),
                );
                let range = crate::eq::RANGE_DB;
                let y_of = |db: f32| {
                    plot.center().y - (db.clamp(-range, range) / range) * (plot.height() / 2.0)
                };
                let db_of_y = |y: f32| {
                    (((plot.center().y - y) / (plot.height() / 2.0)) * range).clamp(-range, range)
                };

                // dB axis labels (+12dB at top, -12dB at bottom) and single 0dB center line
                for (db, label) in [(12.0f32, "+12dB"), (-12.0, "-12dB")] {
                    let y = y_of(db);
                    painter.text(
                        pos2(rect.left() + 8.0, y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        theme::bold(12.0),
                        palette.secondary,
                    );
                }
                let zero_y = y_of(0.0);
                painter.hline(
                    (plot.left() - 34.0)..=(plot.right() + 34.0),
                    zero_y,
                    Stroke::new(1.0, palette.outline),
                );

                // 6 official Spotify bands mapped to our 10-band engine
                const LABELS: [&str; 6] = ["60Hz", "150Hz", "400Hz", "1KHz", "2.4KHz", "15KHz"];
                const PRIMARY_BAND: [usize; 6] = [0, 1, 2, 4, 5, 9];

                let mut nodes: [Pos2; 6] = [Pos2::ZERO; 6];
                for i in 0..6 {
                    let t = i as f32 / 5.0;
                    let x = plot.left() + t * plot.width();
                    let db = app.settings.eq_bands_db[PRIMARY_BAND[i]];
                    nodes[i] = pos2(x, y_of(db));

                    // Vertical guide line and bottom frequency label
                    painter.vline(x, plot.y_range(), Stroke::new(1.0, palette.outline));
                    painter.text(
                        pos2(x, rect.bottom() - 10.0),
                        egui::Align2::CENTER_CENTER,
                        LABELS[i],
                        theme::bold(12.0),
                        palette.secondary,
                    );
                }

                // Compute exact Cubic Hermite Spline tangents (Spotify's `ew` algorithm)
                let mut tangents: [egui::Vec2; 6] = [egui::Vec2::ZERO; 6];
                for i in 0..6 {
                    tangents[i] = if i == 0 {
                        nodes[1] - nodes[0]
                    } else if i == 5 {
                        nodes[5] - nodes[4]
                    } else {
                        let mut m = (nodes[i + 1] - nodes[i - 1]) * 0.5;
                        let dy_prev = nodes[i].y - nodes[i - 1].y;
                        let dy_next = nodes[i + 1].y - nodes[i].y;
                        if dy_prev * dy_next <= 0.0 {
                            m.y = 0.0;
                        }
                        m
                    };
                }

                // Sample spline curve across all 5 segments
                let steps_per_seg = 32usize;
                let mut curve_pts = Vec::with_capacity(5 * steps_per_seg + 1);
                for seg in 0..5 {
                    let p0 = nodes[seg];
                    let p1 = nodes[seg + 1];
                    let m0 = tangents[seg];
                    let m1 = tangents[seg + 1];
                    for s in 0..steps_per_seg {
                        let t = s as f32 / steps_per_seg as f32;
                        let t2 = t * t;
                        let t3 = t2 * t;
                        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
                        let h10 = t3 - 2.0 * t2 + t;
                        let h01 = -2.0 * t3 + 3.0 * t2;
                        let h11 = t3 - t2;
                        let x = p0.x + t * (p1.x - p0.x);
                        let y = (h00 * p0.y + h10 * m0.y + h01 * p1.y + h11 * m1.y)
                            .clamp(plot.top(), plot.bottom());
                        curve_pts.push(pos2(x, y));
                    }
                }
                curve_pts.push(nodes[5]);

                // Theme-colored (`palette.accent`) vertical gradient fill under the spline down to plot.bottom()
                let [ar, ag, ab, _] = palette.accent.to_array();
                let top_alpha = if app.settings.eq_on { 145 } else { 118 };
                let top_fill = Color32::from_rgba_unmultiplied(ar, ag, ab, top_alpha);
                let bot_fill = Color32::from_rgba_unmultiplied(ar, ag, ab, 0);
                let mut mesh = Mesh::default();
                for pt in &curve_pts {
                    mesh.colored_vertex(*pt, top_fill);
                    mesh.colored_vertex(pos2(pt.x, plot.bottom()), bot_fill);
                }
                for i in 0..(curve_pts.len() - 1) {
                    let idx = (i * 2) as u32;
                    mesh.add_triangle(idx, idx + 1, idx + 2);
                    mesh.add_triangle(idx + 1, idx + 3, idx + 2);
                }
                painter.add(Shape::mesh(mesh));

                // Spline stroke line in theme's `palette.accent`
                painter.add(Shape::line(curve_pts, Stroke::new(2.5, palette.accent)));

                // 6 interactive draggable circular handles
                for i in 0..6 {
                    let center = nodes[i];
                    let hit_rect = Rect::from_center_size(center, vec2(26.0, plot.height()));
                    let handle_id = ui.id().with(("spotify-eq-node", i));
                    let resp = ui.interact(hit_rect, handle_id, Sense::click_and_drag());
                    let mut new_db = None;
                    if resp.double_clicked() {
                        new_db = Some(0.0);
                    } else if (resp.dragged() || resp.clicked())
                        && let Some(pos) = resp.interact_pointer_pos()
                    {
                        let db = (db_of_y(pos.y) * 10.0).round() / 10.0;
                        new_db = Some(db);
                    }
                    if let Some(db) = new_db {
                        if !app.settings.eq_on {
                            app.actions.push(Action::ToggleEq);
                        }
                        match i {
                            0 => app.actions.push(Action::SetEqBand(0, db)),
                            1 => app.actions.push(Action::SetEqBand(1, db)),
                            2 => {
                                app.actions.push(Action::SetEqBand(2, db));
                                app.actions.push(Action::SetEqBand(3, db));
                            }
                            3 => app.actions.push(Action::SetEqBand(4, db)),
                            4 => {
                                app.actions.push(Action::SetEqBand(5, db));
                                app.actions.push(Action::SetEqBand(6, db));
                            }
                            _ => {
                                app.actions.push(Action::SetEqBand(7, db));
                                app.actions.push(Action::SetEqBand(8, db));
                                app.actions.push(Action::SetEqBand(9, db));
                            }
                        }
                    }
                    let hover_t = ui.ctx().animate_bool_responsive(
                        handle_id.with("hover"),
                        resp.hovered() || resp.dragged(),
                    );
                    let radius = 5.0 + 2.0 * hover_t;
                    painter.circle_filled(center, radius, palette.text);
                    if resp.hovered() || resp.dragged() {
                        let db = app.settings.eq_bands_db[PRIMARY_BAND[i]];
                        painter.text(
                            pos2(center.x, (center.y - 14.0).max(rect.top() + 8.0)),
                            egui::Align2::CENTER_BOTTOM,
                            format!("{db:+.1} dB"),
                            theme::semibold(11.0),
                            palette.text,
                        );
                    }
                }
            }

            ui.add_space(10.0);
            // Bottom-right inside card: "Redefinir" pill button
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::pill_button(ui, &palette, &gettext(locale, "Reset"), false).clicked() {
                    app.actions.push(Action::ApplyEqPreset(0));
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::{RowText, row_matches, section_matches};

    #[test]
    fn empty_filter_matches_everything() {
        assert!(row_matches(
            "",
            "Normalize volume",
            "Keep loud tracks level"
        ));
        assert!(section_matches(
            "",
            "Playback on this computer",
            &[RowText::new("Status", "Spotify")]
        ));
    }

    #[test]
    fn row_matches_title_or_description_case_insensitively() {
        assert!(row_matches(
            "volume",
            "Normalize volume",
            "Keep loud tracks level"
        ));
        assert!(row_matches(
            "VOLUME",
            "Normalize volume",
            "Keep loud tracks level"
        ));
        assert!(row_matches(
            "quiet",
            "Normalize volume",
            "Keep loud and quiet tracks level"
        ));
        assert!(!row_matches(
            "theme",
            "Normalize volume",
            "Keep loud tracks level"
        ));
    }

    #[test]
    fn section_matches_title_or_any_row() {
        let rows = [
            RowText::new("Normalize volume", "Keep loud and quiet tracks level"),
            RowText::new("Autoplay", "Keep playing similar songs"),
        ];
        assert!(section_matches(
            "playback",
            "Playback on this computer",
            &rows
        ));
        assert!(section_matches(
            "autoplay",
            "Playback on this computer",
            &rows
        ));
        assert!(section_matches("quiet", "Playback on this computer", &rows));
        assert!(!section_matches(
            "theme",
            "Playback on this computer",
            &rows
        ));
    }

    #[test]
    fn the_slider_fill_stops_under_its_handle() {
        let ctx = egui::Context::default();
        let palette = crate::theme::Palette::dark();
        crate::theme::install(&ctx);
        crate::theme::apply(&ctx, &palette);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let mut style = (**ui.style()).clone();
            super::slider_rail_style(&mut style, &palette);
            // egui paints the accent fill this far past the handle's centre,
            // and the handle's radius is at least this for a slider of the
            // settings' height.
            let overshoot = f32::from(style.visuals.widgets.inactive.corner_radius.nw);
            let handle = style.spacing.interact_size.y / 2.5;
            assert!(overshoot < handle, "{overshoot} >= {handle}");

            super::slider_value_style(&mut style, &palette);
            assert_eq!(
                style.visuals.widgets.inactive.corner_radius,
                egui::CornerRadius::same(14)
            );
        });
        output.textures_delta.clear();
    }

    #[test]
    fn a_setting_slider_draws_its_rail_and_value_box() {
        let ctx = egui::Context::default();
        let palette = crate::theme::Palette::dark();
        crate::theme::install(&ctx);
        crate::theme::apply(&ctx, &palette);
        let mut value = 42u32;
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let response =
                super::setting_slider(ui, &palette, &mut value, 2..=300, |s| s, |v| v.suffix(" s"));
            // The rail and the box sit side by side, so together they are
            // wider than the rail alone.
            assert!(response.rect.width() > ui.spacing().slider_width);
        });
        output.textures_delta.clear();
    }
}
