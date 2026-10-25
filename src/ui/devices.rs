//! The Spotify Connect device picker.

use egui::{Align, CornerRadius, Layout, Rect, Sense, pos2, vec2};

use crate::api::models::Device;
use crate::app::App;
use crate::i18n::gettext;
use crate::model::Action;
use crate::theme::{self, Icon};

pub const BUTTON_RECT_ID: &str = "devices-button-rect";

fn receiver_is_listed(receiver: &crate::zeroconf::Receiver, devices: &[Device]) -> bool {
    devices
        .iter()
        .any(|device| match receiver.device_id.as_deref() {
            Some(id) => device.id.as_deref() == Some(id),
            None => device.name == receiver.name,
        })
}

fn name_devices(devices: &mut [Device], receivers: &[crate::zeroconf::Receiver]) {
    for device in devices {
        if device.id.is_some()
            && let Some(receiver) = receivers
                .iter()
                .find(|receiver| receiver.device_id == device.id)
        {
            device.name.clone_from(&receiver.name);
        }
    }
}

pub fn device_icon(kind: &str) -> Icon {
    match kind.to_ascii_lowercase().as_str() {
        "computer" => Icon::Laptop,
        "smartphone" => Icon::Smartphone,
        "tablet" => Icon::Tablet,
        "tv" => Icon::Tv,
        "game_console" => Icon::Gamepad,
        "automobile" => Icon::Car,
        "cast_video" | "cast_audio" | "castaudio" | "castvideo" => Icon::Cast,
        "smartwatch" => Icon::Watch,
        "avr" | "stb" | "audio_dongle" => Icon::Monitor,
        _ => Icon::Speaker,
    }
}

/// A row offering to authorize local playback on this computer.
fn enable_playback_row(app: &mut App, ui: &mut egui::Ui) {
    use egui::{Rect, Sense, Vec2, pos2, vec2};
    let palette = app.palette;
    let authorizing = matches!(
        app.local_playback,
        crate::backend::LocalPlayback::Authorizing
            | crate::backend::LocalPlayback::Connecting
            | crate::backend::LocalPlayback::WaitingForNetwork
    );
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
    if response.hovered() && !authorizing {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(6), palette.surface_hover);
    }
    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 24.0, rect.center().y), Vec2::splat(22.0));
    Icon::Laptop
        .image(palette.text, 22.0)
        .paint_at(ui, icon_rect);
    let painter = ui.painter().with_clip_rect(rect);
    painter.text(
        pos2(rect.left() + 48.0, rect.center().y - 9.0),
        egui::Align2::LEFT_CENTER,
        // Translators: {name} is the name this computer uses in Spotify Connect.
        gettext(app.locale, "{name} (this computer)").replace("{name}", &app.settings.device_name),
        theme::medium(14.0),
        palette.text,
    );
    painter.text(
        pos2(rect.left() + 48.0, rect.center().y + 10.0),
        egui::Align2::LEFT_CENTER,
        if authorizing {
            gettext(app.locale, "Setting up…")
        } else {
            gettext(app.locale, "Set up playback here")
        },
        theme::regular(12.0),
        palette.accent,
    );
    if authorizing {
        let mut spin = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_center_size(
                    pos2(rect.right() - 18.0, rect.center().y),
                    Vec2::splat(20.0),
                ))
                .layout(egui::Layout::centered_and_justified(
                    egui::Direction::LeftToRight,
                )),
        );
        theme::spinner(&mut spin, 16.0, palette.accent);
    }
    if response.clicked() && !authorizing {
        app.actions.push(Action::EnablePlayback);
    }
    ui.painter().hline(
        rect.x_range().shrink(6.0),
        rect.bottom(),
        egui::Stroke::new(1.0, palette.outline),
    );
    ui.add_space(4.0);
}

/// A receiver announced on the local network but not yet in the account.
/// Choosing it hands over the account, after which it behaves like any other
/// Spotify Connect device.
fn receiver_row(app: &mut App, ui: &mut egui::Ui, receiver: &crate::zeroconf::Receiver) {
    use egui::{Rect, Sense, Vec2, pos2, vec2};
    let palette = app.palette;
    let activating = app.activating_receiver.as_deref() == Some(receiver.name.as_str());
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 52.0), Sense::click());
    if response.hovered() && !activating {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(6), palette.surface_hover);
    }
    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 24.0, rect.center().y), Vec2::splat(22.0));
    let icon = if receiver.cast {
        Icon::Cast
    } else {
        Icon::Speaker
    };
    icon.image(palette.text, 22.0).paint_at(ui, icon_rect);
    let painter = ui.painter().with_clip_rect(rect);
    crate::bidi::paint_line(
        &painter,
        rect.left() + 48.0,
        rect.right() - 12.0,
        rect.center().y - 9.0,
        &receiver.name,
        theme::medium(14.0),
        palette.text,
    );
    painter.text(
        pos2(rect.left() + 48.0, rect.center().y + 10.0),
        egui::Align2::LEFT_CENTER,
        if activating {
            gettext(app.locale, "Connecting…")
        } else {
            gettext(app.locale, "On your network, click to connect")
        },
        theme::regular(12.0),
        palette.secondary,
    );
    if activating {
        let mut spin = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_center_size(
                    pos2(rect.right() - 18.0, rect.center().y),
                    Vec2::splat(20.0),
                ))
                .layout(egui::Layout::centered_and_justified(
                    egui::Direction::LeftToRight,
                )),
        );
        theme::spinner(&mut spin, 16.0, palette.accent);
    }
    if response.clicked() && !activating {
        app.actions
            .push(Action::ActivateReceiver(Box::new(receiver.clone())));
    }
}

pub fn popup(_app: &mut App, _ctx: &egui::Context) {}

#[allow(dead_code)]
pub fn side_panel(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let fit = super::yielding_panel(
        ui.ctx(),
        "devices-panel",
        theme::SIDE_PANEL_MIN_WIDTH..=420.0,
        app.settings.lyrics_width,
        ui.available_width() - super::topbar::least_width(ui.ctx()),
    );
    let panel = egui::Panel::right("devices-panel")
        .resizable(true)
        .default_size(app.settings.lyrics_width)
        .size_range(fit.range.clone())
        .show_separator_line(false)
        .frame(
            egui::Frame::new()
                .fill(palette.window)
                .inner_margin(egui::Margin {
                    left: 0,
                    right: theme::PANEL_GAP as i8,
                    top: 0,
                    bottom: 0,
                }),
        );
    let response = panel.show(ui, |ui| {
        let card_rect = ui.available_rect_before_wrap();
        egui::Frame::new()
            .fill(palette.panel)
            .corner_radius(CornerRadius::same(theme::RADIUS))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.set_min_height((card_rect.height() - 28.0).max(0.0));
                content(app, ui);
            });
    });
    let current_width = response.response.rect.width();
    if (app.settings.lyrics_width - current_width).abs() > 1.0
        && super::panel_width_chosen(ui.ctx(), "devices-panel", &fit)
    {
        app.settings.lyrics_width = current_width;
        app.actions.push(Action::SettingsChanged);
    }
}

pub fn content(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let locale = app.locale;
    ui.horizontal(|ui| {
        theme::text(
            ui,
            gettext(locale, "Connect").as_ref(),
            theme::bold(20.0),
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
                app.actions.push(Action::ToggleDevicesPopup);
            }
            if app.devices_loading {
                theme::spinner(ui, 15.0, palette.accent);
            }
        });
    });
    ui.add_space(18.0);

    let local_id = app.local_device_id.clone();
                let mut devices: Vec<Device> = app.devices.clone();
                if app.local_ready
                    && let Some(local_id) = &local_id
                    && !devices
                        .iter()
                        .any(|device| device.id.as_deref() == Some(local_id.as_str()))
                {
                    devices.insert(
                        0,
                        Device {
                            id: Some(local_id.clone()),
                            name: app.settings.device_name.clone(),
                            is_active: app.local.is_active(),
                            is_restricted: false,
                            volume_percent: Some(crate::app::volume_to_percent(app.local.volume)),
                            supports_volume: Some(true),
                            kind: "computer".into(),
                        },
                    );
                }
                let active_id = match app.target() {
                    crate::app::Target::Local => local_id.clone(),
                    crate::app::Target::Remote(id) => id,
                };
                for receiver in &app.receivers {
                    let listed = devices
                        .iter()
                        .any(|device| device.id.is_some() && device.id == receiver.device_id);
                    if !listed
                        && let Some(row) =
                            crate::sonos::device_row(receiver, receiver.device_id == active_id)
                    {
                        devices.push(row);
                    }
                }
                devices.sort_by_key(|device| device.id != active_id);
                name_devices(&mut devices, &app.receivers);
                let mut seen = std::collections::HashSet::new();
                devices
                    .retain(|device| device.id.as_ref().is_none_or(|id| seen.insert(id.clone())));

                let waiting: Vec<crate::zeroconf::Receiver> = app
                    .receivers
                    .iter()
                    .filter(|receiver| !receiver_is_listed(receiver, &devices))
                    .cloned()
                    .collect();

                ui.spacing_mut().scroll.bar_width = 0.0;
                ui.spacing_mut().scroll.floating_width = 0.0;
                egui::ScrollArea::vertical()
                    .id_salt("connect-device-list")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if !app.local_ready {
                            enable_playback_row(app, ui);
                        }

                        let active_device = devices
                            .iter()
                            .find(|d| d.id.is_some() && d.id == active_id)
                            .cloned();
                        let other_devices: Vec<Device> = devices
                            .into_iter()
                            .filter(|d| !(d.id.is_some() && d.id == active_id))
                            .collect();

                        // Active Device Card (#18241c green-tinted card with Jam CTA)
                        if let Some(device) = active_device {
                            let is_local = device.id.is_some() && device.id == local_id;
                            let card_bg = if palette.dark {
                                egui::Color32::from_rgb(0x18, 0x24, 0x1c)
                            } else {
                                super::blend(palette.surface, palette.accent, 0.14)
                            };
                            egui::Frame::new()
                                .fill(card_bg)
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(egui::Margin::symmetric(14, 14))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        theme::icon(
                                            ui,
                                            device_icon(&device.kind),
                                            22.0,
                                            palette.accent,
                                        );
                                        ui.add_space(6.0);
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 2.0;
                                            theme::text(
                                                ui,
                                                &device.name,
                                                theme::bold(15.0),
                                                palette.accent,
                                            );
                                            let sub = if is_local {
                                                gettext(locale, "This computer")
                                            } else {
                                                gettext(locale, "Listening on this device")
                                            };
                                            theme::text(
                                                ui,
                                                sub,
                                                theme::regular(12.5),
                                                palette.secondary,
                                            );
                                        });
                                    });
                                    ui.add_space(12.0);
                                    ui.horizontal(|ui| {
                                        let avail = ui.available_width();
                                        let jam_label = gettext(locale, "Start a Jam");
                                        let jam_galley = ui.painter().layout_no_wrap(
                                            jam_label.to_string(),
                                            theme::bold(12.5),
                                            palette.text,
                                        );
                                        let btn_w = jam_galley.size().x + 42.0;
                                        let text_w = (avail - btn_w - 8.0).max(60.0);
                                        ui.allocate_ui_with_layout(
                                            vec2(text_w, 34.0),
                                            Layout::left_to_right(Align::Center),
                                            |ui| {
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(
                                                            gettext(
                                                                locale,
                                                                "Listen together from anywhere",
                                                            )
                                                            .as_ref(),
                                                        )
                                                        .font(theme::regular(12.0))
                                                        .color(palette.secondary),
                                                    )
                                                    .wrap(),
                                                );
                                            },
                                        );
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                let (btn_rect, btn_resp) = ui.allocate_exact_size(
                                                    vec2(btn_w, 32.0),
                                                    Sense::click(),
                                                );
                                                let jam_active = app.jam_active;
                                                let btn_fill = if jam_active {
                                                    palette.accent.gamma_multiply(0.2)
                                                } else if btn_resp.hovered() {
                                                    palette.surface_hover
                                                } else {
                                                    palette.surface
                                                };
                                                ui.painter().rect(
                                                    btn_rect,
                                                    CornerRadius::same(16),
                                                    btn_fill,
                                                    egui::Stroke::new(
                                                        1.0,
                                                        if jam_active {
                                                            palette.accent
                                                        } else {
                                                            palette.outline
                                                        },
                                                    ),
                                                    egui::StrokeKind::Inside,
                                                );
                                                theme::paint_icon(
                                                    ui,
                                                    Icon::Radio,
                                                    Rect::from_center_size(
                                                        pos2(
                                                            btn_rect.left() + 16.0,
                                                            btn_rect.center().y,
                                                        ),
                                                        vec2(14.0, 14.0),
                                                    ),
                                                    14.0,
                                                    if jam_active {
                                                        palette.accent
                                                    } else {
                                                        palette.text
                                                    },
                                                );
                                                ui.painter().galley(
                                                    pos2(
                                                        btn_rect.left() + 28.0,
                                                        btn_rect.center().y
                                                            - jam_galley.size().y / 2.0,
                                                    ),
                                                    jam_galley,
                                                    if jam_active {
                                                        palette.accent
                                                    } else {
                                                        palette.text
                                                    },
                                                );
                                                if btn_resp.clicked() {
                                                    app.actions.push(Action::ToggleJam);
                                                }
                                            },
                                        );
                                    });
                                });
                            ui.add_space(18.0);
                        }

                        if !other_devices.is_empty() || !waiting.is_empty() {
                            theme::text(
                                ui,
                                gettext(locale, "On other networks").as_ref(),
                                theme::bold(14.0),
                                palette.text,
                            );
                            ui.add_space(8.0);
                        } else if app.local_ready {
                            ui.add_space(4.0);
                            theme::text(
                                ui,
                                gettext(locale, "No other devices found").as_ref(),
                                theme::regular(13.0),
                                palette.secondary,
                            );
                        }

                        for device in &other_devices {
                            let (rect, response) = ui.allocate_exact_size(
                                vec2(ui.available_width(), 46.0),
                                Sense::click(),
                            );
                            if response.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::same(6),
                                    palette.surface_hover,
                                );
                            }
                            let icon_rect = Rect::from_center_size(
                                pos2(rect.left() + 20.0, rect.center().y),
                                egui::Vec2::splat(20.0),
                            );
                            device_icon(&device.kind)
                                .image(palette.text, 20.0)
                                .paint_at(ui, icon_rect);
                            let painter = ui.painter().with_clip_rect(rect);
                            crate::bidi::paint_line(
                                &painter,
                                rect.left() + 44.0,
                                rect.right() - 12.0,
                                rect.center().y,
                                &device.name,
                                theme::medium(14.0),
                                palette.text,
                            );
                            if response.clicked()
                                && let Some(id) = &device.id
                            {
                                app.actions.push(Action::Transfer(id.clone()));
                            }
                        }
                        for receiver in &waiting {
                            receiver_row(app, ui, receiver);
                        }

                        ui.add_space(20.0);
                        let (link_rect, link_resp) = ui.allocate_exact_size(
                            vec2(ui.available_width(), 36.0),
                            Sense::click(),
                        );
                        let link_color = if link_resp.hovered() {
                            palette.text
                        } else {
                            palette.secondary
                        };
                        ui.painter().text(
                            pos2(link_rect.left() + 4.0, link_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            gettext(locale, "Don't see your device?"),
                            theme::medium(13.5),
                            link_color,
                        );
                        theme::paint_icon(
                            ui,
                            Icon::ExternalLink,
                            Rect::from_center_size(
                                pos2(link_rect.right() - 14.0, link_rect.center().y),
                                vec2(16.0, 16.0),
                            ),
                            16.0,
                            link_color,
                        );
                        if link_resp.clicked() {
                            app.actions.push(Action::RefreshDevices);
                        }
                    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_receivers_identity_matches_even_when_its_advertised_name_differs() {
        let receiver = crate::zeroconf::Receiver {
            name: "SpotifyConnect".into(),
            device_id: Some("living-room".into()),
            address: "127.0.0.1".parse().unwrap(),
            port: 80,
            path: "/".into(),
            cast: false,
        };
        let mut device = Device {
            id: Some("living-room".into()),
            name: "Living Room".into(),
            ..Default::default()
        };
        assert!(receiver_is_listed(&receiver, &[device.clone()]));
        let mut named = [Device {
            name: "unnamed".into(),
            ..device.clone()
        }];
        name_devices(&mut named, std::slice::from_ref(&receiver));
        assert_eq!(named[0].name, receiver.name);
        assert_eq!(
            named[0].id, device.id,
            "renaming preserves the transfer target"
        );
        device.id = Some("other-room".into());
        device.name = receiver.name.clone();
        assert!(
            !receiver_is_listed(&receiver, &[device]),
            "two identities may have the same name"
        );
    }
}
