// SPDX-License-Identifier: MIT
//! Focused installer pages with a quiet identity panel and persistent navigation.

use enzui_widgets::app_shell::enz_app_shell;
use enzui_widgets::button::ButtonKind;
use enzui_widgets::icon::IconTone;
use enzui_widgets::modal_host::{enz_modal_host, presentation_scope_id};
use enzui_widgets::modal_surface::enz_modal_surface;
use enzui_widgets::notice_stack::{EnzNotice, enz_notice_stack};
use enzui_widgets::scroll_fade_edge::{ScrollFadeEdges, scroll_fade_edges};
use enzui_widgets::title_bar::enz_title_bar;
use enzui_widgets::toast::ToastTone;
use enzui_widgets::toast_host::{ToastOverlayPlacement, enz_toast_layer};
use enzui_widgets::{
    BannerTone, ClampSize, TextAlign, TextSize, TextTone, enz_banner, enz_button as native_button,
    enz_clamp, enz_divider, enz_icon, enz_preferences_group, enz_preferences_row, enz_progress_bar,
    enz_radio_button, enz_scroll_view, enz_search_field, enz_secure_field, enz_sidebar_row,
    enz_text, enz_text_field,
};
use iced::widget::{Space, column, container, responsive, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::system;
use crate::{Field, Installer, LAYOUT, Message, Outcome, Step};

const PAGE_WIDTH: f32 = 960.0;
/// The setup footer below the page divider: the step strip and the buttons.
const FOOTER_HEIGHT: f32 = 125.0;

mod forms;
use forms::{account, disk, encryption, region, timezone, welcome};

fn enz_button<'a>(
    label: impl Into<String>,
    message: Message,
) -> enzui_widgets::button::EnzButton<'a, Message> {
    let id = crate::keyboard_nav::choice_id(&message).unwrap_or_else(|| {
        iced::advanced::widget::Id::from(format!("installer-button-{message:?}"))
    });
    native_button(label, message).id(id)
}

pub(crate) fn view(app: &Installer) -> Element<'_, Message> {
    app.localizer.view(|| localized_view(app))
}

fn localized_view(app: &Installer) -> Element<'_, Message> {
    let shell = enz_app_shell(|| content(app))
        .layout(LAYOUT)
        .tokens(Installer::tokens())
        .on_window_gesture(Message::Window)
        .maximized(app.chrome.maximized())
        .fullscreen(app.chrome.fullscreen())
        .breakpoints(app.theme_tokens().breakpoints)
        .titlebar(|| titlebar(app))
        .into_element();
    // Notices float above the footer (step strip and buttons), clear of Continue.
    let shell = enz_toast_layer(
        shell,
        network_notice(app),
        ToastOverlayPlacement::default().bottom_inset(FOOTER_HEIGHT),
    );
    enz_modal_host(
        shell,
        if app.log_transfer.is_some() {
            Some(
                responsive(move |size| {
                    container(qr_dialog(app, size.height))
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                })
                .into(),
            )
        } else {
            app.exit_menu.then(|| exit_dialog(app))
        },
        Message::Modal,
    )
    .scope(presentation_scope_id("installer-exit"))
    .label(if app.log_transfer.is_some() {
        app.tr("Copy installation logs")
    } else {
        app.tr("Installation options")
    })
    .into_element()
}

/// A failed network request, as a dismissible notice in the bottom-right
/// corner of the network page. It stays until dismissed or the next request.
fn network_notice(app: &Installer) -> Option<Element<'_, Message>> {
    (app.step == Step::Network && !app.network_error.is_empty()).then(|| {
        enz_notice_stack(vec![
            EnzNotice::new(
                "installer-network-error",
                app.tr("Could not connect"),
                ToastTone::Warning,
            )
            .description(crate::i18n::translate(&app.localizer, &app.network_error))
            .on_dismiss(Message::DismissNetworkError),
        ])
        .into_element()
    })
}

fn titlebar(app: &Installer) -> Element<'_, Message> {
    let toggle = enzui_widgets::switch::switch(app.dark, Message::Theme)
        .id(iced::advanced::widget::Id::new("installer-theme"))
        .label(app.tr("Dark appearance"))
        .colors(|theme, state| {
            let mut colors = enzui_widgets::switch::default_colors(theme, state);
            let palette = enzui_widgets::theme_lookup::palette(theme);
            colors.thumb = palette.surface_elevated;
            if !state.on {
                colors.track = if state.hovered {
                    palette.control_border_strong
                } else {
                    palette.control_border
                };
            }
            colors
        });
    let theme = row![
        enz_icon("sun").size(16.0).tint_theme_tone(IconTone::Muted),
        toggle,
        enz_icon("moon").size(16.0).tint_theme_tone(IconTone::Muted)
    ]
    .spacing(8.0)
    .align_y(Alignment::Center);
    let mut bar = enz_title_bar().push(theme);
    if app.demo {
        bar = bar.push(
            enzui_widgets::enz_badge(app.tr("Preview · no disks are changed"))
                .tone(enzui_widgets::BadgeTone::Warning)
                .into_element(),
        );
    }
    if let Some(line) = &app.dev_access {
        bar = bar.push(
            enzui_widgets::enz_badge(line.as_str())
                .tone(enzui_widgets::BadgeTone::Accent)
                .into_element(),
        );
    }
    bar.id("installer-window").into_element()
}

fn exit_dialog(app: &Installer) -> Element<'_, Message> {
    let busy = (app.step == Step::Installing && app.outcome == Outcome::Running) || app.disks_busy;
    let body = column![
        header(
            "power",
            app.tr("Installation options"),
            if busy {
                app.tr("Please wait for installation to finish before restarting or shutting down.")
            } else {
                app.tr("Start over, or turn off this computer.")
            }
        ),
        // A failed real install keeps its disk and mount state for diagnosis; only a reboot
        // gives the next attempt a fresh start.
        enz_button(app.tr("Restart installation"), Message::Reset)
            .icon("rotate-ccw")
            .kind(ButtonKind::Secondary)
            .enabled(
                !busy
                    && app.step != Step::Finished
                    && !(matches!(app.outcome, Outcome::Failed(_)) && !app.demo)
            )
            .into_element(),
        enz_button(app.tr("Reboot computer"), Message::Restart)
            .icon("refresh-cw")
            .kind(ButtonKind::Secondary)
            .enabled(!busy)
            .into_element(),
        enz_button(app.tr("Shut down"), Message::PowerOff)
            .icon("power")
            .kind(ButtonKind::Secondary)
            .enabled(!busy)
            .into_element(),
        enz_divider().into_element(),
        row![
            Space::new().width(Length::Fill),
            enz_button(app.tr("Cancel"), Message::DismissExit)
                .kind(ButtonKind::Accent)
                .into_element()
        ],
    ]
    .spacing(16.0);
    enz_modal_surface(body.into())
        .width(520.0)
        .padding(24.0)
        .into_element()
}

fn content(app: &Installer) -> Element<'_, Message> {
    responsive(move |size| content_sized(app, size)).into()
}

fn content_sized(app: &Installer, size: iced::Size) -> Element<'_, Message> {
    let compact = size.width < 900.0 || size.height < 650.0;
    // Lists run down to the footer divider: page chrome above them plus the footer below.
    let list_height = (size.height - 252.0).max(140.0);
    if app.step == Step::Finished {
        return finished(app, compact);
    }
    if app.step == Step::Installing {
        return enz_scroll_view(container(installing(app, compact)).padding(if compact {
            [16.0, 20.0]
        } else {
            [36.0, 52.0]
        }))
        .height(Length::Fill)
        .into();
    }
    let body: Element<'_, Message> = match app.step {
        Step::Welcome => welcome(app, list_height),
        Step::Region => region(app, compact, list_height),
        Step::Timezone => timezone(app, list_height),
        Step::Network => network(app),
        Step::Encryption => encryption(app),
        Step::Disk => disk(app),
        Step::Account => account(app),
        Step::Review => review(app, compact),
        Step::Installing => installing(app, compact),
        Step::Finished => finished(app, compact),
    };
    let body = if app.step < Step::Review {
        let title = match app.step {
            Step::Welcome => app.tr("Welcome to enzimOS"),
            Step::Disk => app.tr("Where to install"),
            Step::Account => app.tr("Your Account"),
            Step::Region => app.tr("Keyboard layout"),
            _ => app.tr(app.step.label()),
        };
        column![
            row![step_mark(app.step), enz_text(title).size(TextSize::Title1)]
                .spacing(18.0)
                .align_y(Alignment::Center),
            body
        ]
        .spacing(if compact { 16.0 } else { 28.0 })
        .into()
    } else {
        body
    };
    let page = container(
        enz_clamp(body, ClampSize::Default)
            .max_width(PAGE_WIDTH)
            .into_element(),
    )
    .padding(if compact { [14.0, 20.0] } else { [22.0, 32.0] });
    let mut layout = column![
        enz_scroll_view(page)
            .id("installer-page".into())
            .height(Length::Fill)
    ];
    if let Some(bar) = footer(app) {
        layout = layout.push(enz_divider().into_element()).push(bar);
    }
    layout.height(Length::Fill).into()
}

/// Keep the current step visible; omitted neighbours get real edge fades.
pub(crate) fn step_window(current: usize, width: f32) -> (usize, usize) {
    let total = Step::LISTED.len();
    // Every chip fits without the edge fades; only a partial strip reserves room for them.
    let all = ((width - 16.0) / 106.0).floor() as usize >= total;
    let count = if all {
        total
    } else {
        (((width - 70.0) / 106.0).floor().max(3.0) as usize).min(total)
    };
    let start = current.saturating_sub(count / 2).min(total - count);
    (start, start + count)
}

fn step_strip(app: &Installer, width: f32) -> Element<'_, Message> {
    let current = (app.step as usize).min(Step::LISTED.len() - 1);
    let (start, end) = step_window(current, width);
    let chip = |i: usize| -> Element<'_, Message> {
        let done = i < current || app.step == Step::Finished;
        let icon = enz_icon(if done {
            "circle-check"
        } else if i == current {
            "circle-dot"
        } else {
            "circle"
        })
        .size(18.0);
        let icon = if done || i == current {
            icon.tint_theme_accent()
        } else {
            icon.tint_theme_tone(IconTone::Subtle)
        };
        container(
            column![
                icon,
                enz_text(app.tr(Step::LISTED[i].label()))
                    .size(TextSize::Caption)
                    .tone(if i == current {
                        TextTone::Default
                    } else {
                        TextTone::Muted
                    })
                    .align(TextAlign::Center)
            ]
            .spacing(6.0)
            .align_x(Alignment::Center),
        )
        .width(98.0)
        .center_x(98.0)
        .into()
    };
    let mut steps = row![].spacing(8.0).align_y(Alignment::Center);
    if start > 0 {
        steps = steps.push(container(chip(start - 1)).width(26.0).clip(true));
    }
    for i in start..end {
        steps = steps.push(chip(i));
    }
    if end < Step::LISTED.len() {
        steps = steps.push(container(chip(end)).width(26.0).clip(true));
    }
    container(
        scroll_fade_edges(
            steps,
            ScrollFadeEdges {
                left: start > 0,
                right: end < Step::LISTED.len(),
                ..ScrollFadeEdges::NONE
            },
        )
        .width(32.0)
        .color(app.theme_tokens().palette.background),
    )
    .center_x(Length::Fill)
    .into()
}

fn footer(app: &Installer) -> Option<Element<'_, Message>> {
    if app.log_transfer.is_some() {
        return None;
    }
    let (next, next_label) = match app.step {
        Step::Welcome => (Message::Next, app.tr("Get Started")),
        Step::Network if !app.network.connected() => (Message::Next, app.tr("Continue offline")),
        Step::Review => (
            Message::Install,
            if app.partition_choice.is_some() {
                app.tr("Install Alongside")
            } else {
                app.tr("Erase Disk & Install")
            },
        ),
        Step::Installing | Step::Finished => {
            return Some(
                container(responsive(move |size| step_strip(app, size.width)))
                    .height(64.0)
                    .padding([8.0, 20.0])
                    .into(),
            );
        }
        _ => (Message::Next, app.tr("Continue")),
    };
    let blocker = app.blocker();
    let mut bar = row![].spacing(12.0).align_y(Alignment::Center);
    if app.step != Step::Welcome {
        bar = bar.push(
            enz_button(app.tr("Back"), Message::Back)
                .kind(ButtonKind::Secondary)
                .into_element(),
        );
    }
    if let Some(label) = blocker {
        bar = bar.push(
            enz_text(app.tr(label))
                .size(TextSize::Callout)
                .tone(TextTone::Muted),
        );
    }
    bar = bar.push(Space::new().width(Length::Fill));
    bar = bar.push(
        enz_button(next_label, next)
            .kind(
                if app.step == Step::Review && app.partition_choice.is_none() {
                    ButtonKind::Destructive
                } else {
                    ButtonKind::Accent
                },
            )
            .enabled(blocker.is_none())
            .min_width(120.0)
            .into_element(),
    );
    Some(
        container(
            column![
                container(responsive(move |size| step_strip(app, size.width))).height(48.0),
                bar
            ]
            .spacing(12.0),
        )
        .padding([12.0, 24.0])
        .into(),
    )
}

fn header<'a>(icon: &str, title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    row![
        container(enz_icon(icon).size(30.0).tint_theme_accent()).padding(12.0),
        column![
            enz_text(title).size(TextSize::Title1),
            enz_text(subtitle)
                .size(TextSize::Body)
                .tone(TextTone::Muted)
        ]
        .spacing(4.0),
    ]
    .spacing(12.0)
    .align_y(Alignment::Center)
    .into()
}

fn selected_row<'a>(
    label: impl Into<String>,
    message: Message,
    selected: bool,
) -> Element<'a, Message> {
    let id = crate::keyboard_nav::choice_id(&message);
    let mut row = enz_sidebar_row(label, message).selected(selected);
    if let Some(id) = id {
        row = row.id(id);
    }
    if selected {
        row.trailing(enz_icon("circle-check").size(18.0).tint_theme_accent())
            .into()
    } else {
        row.trailing(Space::new().width(18.0)).into()
    }
}
fn review(app: &Installer, compact: bool) -> Element<'_, Message> {
    let plan = app.plan();
    let disk = app.disk.map(|i| &app.disks[i]);
    let (city, region) = app.zone_parts(&plan.timezone);
    let rows = column![
        enz_preferences_row(app.tr("Language"))
            .icon("globe")
            .action(
                system::LANGUAGES[app.language].0,
                Message::Go(Step::Welcome)
            )
            .into_element(),
        enz_preferences_row(app.tr("Keyboard"))
            .icon("keyboard")
            .action(system::KEYBOARDS[app.keyboard].0, Message::Go(Step::Region))
            .into_element(),
        enz_preferences_row(app.tr("Time zone"))
            .icon("clock")
            .action(format!("{city}, {region}"), Message::Go(Step::Timezone))
            .into_element(),
    ];
    let storage = column![
        enz_preferences_row(app.tr("Disk"))
            .icon("hard-drive")
            .subtitle(disk.map(|d| app.disk_detail(d)).unwrap_or_default())
            // A disk without a model is already named by its size.
            .action(
                disk.map(|d| if d.model().is_some() {
                    format!("{} · {}", app.disk_title(d), system::human_size(d.bytes))
                } else {
                    app.disk_title(d)
                })
                .unwrap_or_default(),
                Message::Go(Step::Disk)
            )
            .into_element(),
        enz_preferences_row(app.tr("Encryption"))
            .icon("lock")
            .action(
                if plan.encrypt {
                    app.tr("On")
                } else {
                    app.tr("Off")
                },
                Message::Go(Step::Encryption)
            )
            .into_element(),
    ];
    let account = column![
        enz_preferences_row(app.tr("Account"))
            .icon("user")
            .subtitle(app.trf("Administrator · {0}", &[&plan.user]))
            .action(plan.full_name.clone(), Message::Go(Step::Account))
            .into_element(),
        enz_preferences_row(app.tr("Computer name"))
            .icon("monitor")
            .action(plan.hostname.clone(), Message::Go(Step::Account))
            .into_element(),
    ];
    column![
        header("list-checks", app.tr("Ready to Install"), app.tr("Check your choices. Nothing has been changed yet.")),
        enz_preferences_group(rows).title(app.tr("Region")).into_element(),
        if compact {
            Element::from(column![enz_preferences_group(storage).title(app.tr("Storage")).into_element(),
                enz_preferences_group(account).title(app.tr("Account")).into_element()].spacing(16.0))
        } else {
            Element::from(row![container(enz_preferences_group(storage).title(app.tr("Storage")).into_element()).width(Length::Fill),
                container(enz_preferences_group(account).title(app.tr("Account")).into_element()).width(Length::Fill)].spacing(24.0))
        },
        enz_preferences_row(app.tr("Check the installation medium first"))
            .subtitle(app.tr("Reads every package on the install drive before the disk is changed; adds a few minutes on slow USB drives. Each package is still signature-checked as it installs."))
            .subtitle_wrap(true)
            .icon("shield-check")
            .control(enzui_widgets::switch::switch(app.check_media, Message::CheckMedia)
                .id(iced::advanced::widget::Id::new("installer-check-media"))
                .label(app.tr("Check the installation medium first")))
            .into_element(),
        enz_banner(if app.partition_choice.is_some() { app.tr("Install alongside the existing system").into() } else { app.trf("{0} will be erased", &[&disk.map_or_else(|| app.tr("The disk").into(), |d| app.disk_title(d))]) })
            .body(if let Some(i)=app.partition_choice { crate::i18n::format(&app.localizer, "{} · {}. Existing partitions stay in place; enzimOS gets its own boot and system partitions.", &[("arg0", &(app.partition_choices[i].label)), ("arg1", &(system::human_size(app.partition_choices[i].bytes)))]) } else { app.tr("When you continue, everything on this disk is permanently deleted and enzimOS is installed.").into() })
            .tone(if let Some(i)=app.partition_choice { if app.partition_choices[i].shrink {BannerTone::Warning} else {BannerTone::Info} } else {BannerTone::Danger})
            .into_element(),
    ]
    .spacing(20.0)
    .into()
}

fn installing(app: &Installer, compact: bool) -> Element<'_, Message> {
    let failed = matches!(app.outcome, Outcome::Failed(_));
    let inset = if compact { 8.0 } else { 52.0 };
    let progress = enz_progress_bar(app.install_fraction)
        .tone(if failed {
            enzui_widgets::ProgressTone::Danger
        } else {
            enzui_widgets::ProgressTone::Accent
        })
        .into_element();
    let mut page = column![header(
        "download",
        if failed {
            app.tr("Installation Failed")
        } else {
            app.tr("Installing enzimOS")
        },
        if failed {
            app.tr("Your other disks were not changed.")
        } else {
            app.tr("This takes a few minutes. You can leave the computer on its own.")
        }
    )]
    .spacing(if compact { 14.0 } else { 20.0 });
    let stages = system::STAGES
        .iter()
        .enumerate()
        .map(|(i, (label, _))| {
            let icon = if i < app.stage {
                "circle-check"
            } else if i == app.stage && failed {
                "circle-x"
            } else if i == app.stage {
                "circle-dot"
            } else {
                "circle"
            };
            let marker = enz_icon(icon).size(18.0);
            let marker = if i < app.stage {
                marker.tint_theme_tone(IconTone::Success)
            } else if i == app.stage && failed {
                marker.tint_theme_tone(IconTone::Danger)
            } else if i == app.stage {
                marker.tint_theme_accent()
            } else {
                marker.tint_theme_tone(IconTone::Subtle)
            };
            row![
                container(marker).width(22.0).center_x(22.0),
                enz_text(app.tr(label)).tone(if i <= app.stage {
                    TextTone::Default
                } else {
                    TextTone::Subtle
                })
            ]
            .spacing(12.0)
            .align_y(Alignment::Center)
            .into()
        })
        .collect::<Vec<Element<'_, Message>>>();
    page = page.push(
        container(column(stages).spacing(if compact { 4.0 } else { 9.0 })).padding([0.0, inset]),
    );
    page = page.push(
        container(progress)
            .padding([0.0, inset])
            .width(Length::Fill),
    );
    if let Outcome::Failed(error) = &app.outcome {
        page = page.push(
            enz_banner(app.tr("The installer stopped"))
                .body(format!(
                    "{error}\n{}\nTo try again, restart the computer from the installation drive.",
                    app.trf(
                        "The full log is in {0}.",
                        &[&system::log_path().display().to_string()]
                    )
                ))
                .tone(BannerTone::Danger)
                .into_element(),
        );
        page = page.push(row![
            Space::new().width(Length::Fill),
            enz_button(app.tr("Quit Installer"), Message::Quit)
                .kind(ButtonKind::Secondary)
                .into_element()
        ]);
    }
    if !app.show_log {
        let toggle = enz_button(app.tr("Show Details"), Message::ToggleLog)
            .kind(ButtonKind::Plain)
            .icon("chevron-down")
            .into_element();
        page = page.push(row![Space::new().width(Length::Fill), toggle]);
        return page.into();
    }
    let ink = iced::Color::from_rgb8(196, 206, 214);
    let last: Vec<Element<'_, Message>> = app
        .lines
        .iter()
        .rev()
        .take(400)
        .rev()
        .map(|line| {
            text(line.as_str())
                .size(11.5)
                .font(iced::Font::MONOSPACE)
                .color(ink)
                .into()
        })
        .collect();
    let live = !failed && app.outcome == Outcome::Running;
    let bar = container(
        row![
            text("●").size(10.0).color(if failed {
                iced::Color::from_rgb8(232, 98, 92)
            } else if live {
                iced::Color::from_rgb8(88, 190, 138)
            } else {
                iced::Color::from_rgb8(120, 130, 140)
            }),
            text(app.tr("Installation output"))
                .size(12.0)
                .color(iced::Color::from_rgb8(214, 222, 228)),
            Space::new().width(Length::Fill),
            terminal_button("qr-code", app.tr("Copy logs with QR"), Message::ShowQr),
            terminal_button("chevron-up", app.tr("Hide Details"), Message::ToggleLog),
        ]
        .spacing(10.0)
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 14.0,
    })
    .style(|_| {
        container::Style::default()
            .background(iced::Color::from_rgb8(36, 41, 48))
            .border(iced::Border {
                radius: iced::border::Radius::default().top(10.0),
                ..Default::default()
            })
    });
    let body = container(
        scrollable(container(column(last).spacing(2.0)).padding([10.0, 14.0]))
            .width(Length::Fill)
            .id("install-terminal")
            .style(|theme, status| {
                // A quiet scroller that belongs on the dark panel.
                let mut style = iced::widget::scrollable::default(theme, status);
                for rail in [&mut style.vertical_rail, &mut style.horizontal_rail] {
                    rail.background = None;
                    rail.border = iced::Border::default();
                    rail.scroller.background = iced::Color::from_rgba8(255, 255, 255, 0.22).into();
                    rail.scroller.border = iced::Border::default().rounded(3.0);
                }
                style
            })
            .height(if compact { 150.0 } else { 200.0 }),
    )
    .style(|_| {
        container::Style::default()
            .background(iced::Color::from_rgb8(22, 26, 31))
            .border(iced::Border {
                radius: iced::border::Radius::default().bottom(10.0),
                ..Default::default()
            })
    });
    page = page.push(
        container(column![bar, body])
            .width(Length::Fill)
            .padding([0.0, inset - 8.0_f32.min(inset)]),
    );
    page.into()
}

/// A light, chromeless button for the dark output panel's header.
fn terminal_button<'a>(icon: &'a str, label: &'a str, message: Message) -> Element<'a, Message> {
    let ink = iced::Color::from_rgb8(214, 222, 228);
    iced::widget::button(
        row![
            enz_icon(icon).size(14.0).tint(ink),
            text(label).size(12.0).color(ink)
        ]
        .spacing(6.0)
        .align_y(Alignment::Center),
    )
    .padding([5.0, 9.0])
    .style(move |_, status| iced::widget::button::Style {
        background: matches!(
            status,
            iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
        )
        .then(|| iced::Color::from_rgba8(255, 255, 255, 0.10).into()),
        text_color: ink,
        border: iced::Border::default().rounded(6.0),
        ..Default::default()
    })
    .on_press(message)
    .into()
}

fn finished(app: &Installer, compact: bool) -> Element<'_, Message> {
    let mut page = column![
        container(
            row![
                enzimos_mark(52.0),
                enz_text(app.tr("enzimOS is installed")).size(TextSize::Title1)
            ]
            .spacing(24.0)
            .align_y(Alignment::Center)
        )
        .padding(if compact { [24.0, 20.0] } else { [48.0, 52.0] }),
    ];
    if app.cleanup_incomplete {
        page = page.push(enz_banner(app.tr("Sensitive installation inputs need cleanup"))
            .body(app.tr("Installation succeeded. Restart or shut down to clear the live environment. Do not reinstall.").to_owned())
            .tone(BannerTone::Warning).into_element());
    }
    page.push(Space::new().height(Length::Fill))
        .push(
            container(
                row![
                    enz_button(app.tr("Shut down"), Message::PowerOff)
                        .icon("power")
                        .kind(ButtonKind::Secondary)
                        .into_element(),
                    Space::new().width(Length::Fill),
                    enz_button(app.tr("Restart Now"), Message::Restart)
                        .icon("refresh-cw")
                        .kind(ButtonKind::Accent)
                        .into_element()
                ]
                .spacing(16.0)
                .align_y(Alignment::Center),
            )
            .padding([24.0, 32.0]),
        )
        .height(Length::Fill)
        .into()
}

fn qr_dialog(app: &Installer, height: f32) -> Element<'_, Message> {
    let transfer = app.log_transfer.as_ref().expect("QR modal state");
    let body = column![
        row![
            enz_text(app.tr("Latest installation output")).size(TextSize::Title2),
            Space::new().width(Length::Fill),
            enz_button(app.tr("Close"), Message::CloseQr)
                .kind(ButtonKind::Secondary)
                .into_element()
        ]
        .align_y(Alignment::Center),
        container(
            iced::widget::qr_code(&transfer.code)
                .total_size((height - 180.0).clamp(180.0, 360.0))
                .style(|_| iced::widget::qr_code::Style {
                    cell: iced::Color::BLACK,
                    background: iced::Color::WHITE,
                })
        )
        .padding(16.0)
        .style(|_| container::Style::default().background(iced::Color::WHITE))
        .center_x(Length::Fill),
        enz_text(app.tr("Scan to copy · updates every second"))
            .size(TextSize::Caption)
            .tone(TextTone::Muted),
    ]
    .spacing(16.0);
    enz_modal_surface(body.into())
        .width(540.0)
        .padding(20.0)
        .into_element()
}

fn network(app: &Installer) -> Element<'_, Message> {
    let mut body = column![
        row![
            enz_text(app.tr("Connect now, or finish setup offline.")).tone(TextTone::Muted),
            Space::new().width(Length::Fill),
            enz_button(app.tr("Refresh"), Message::NetworkRefresh)
                .icon("refresh-cw")
                .kind(ButtonKind::Secondary)
                .enabled(!app.network_busy)
                .into_element()
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(18.0);
    for device in &app.network.devices {
        if device.kind == "ethernet" {
            let connected = device.state == "connected";
            body = body.push(
                enz_preferences_group(
                    enz_preferences_row(app.tr("Wired connection"))
                        .icon("network")
                        .subtitle(if connected {
                            app.tr("Connected")
                        } else {
                            app.tr("Connect an Ethernet cable")
                        })
                        .action(
                            if connected {
                                app.tr("Reconnect")
                            } else {
                                app.tr("Connect")
                            },
                            Message::NetworkWired(device.device.clone()),
                        )
                        .enabled(!app.network_busy)
                        .into_element(),
                )
                .into_element(),
            );
        }
    }
    if app.network.devices.iter().any(|d| d.kind == "wifi") {
        body = body.push(
            enz_preferences_row(app.tr("Wi-Fi"))
                .icon("wifi")
                .control(
                    enzui_widgets::switch::switch(app.network.radio, Message::NetworkRadio)
                        .id(iced::advanced::widget::Id::new("installer-wifi"))
                        .label(app.tr("Wi-Fi")),
                )
                .enabled(!app.network_busy)
                .into_element(),
        );
        let mut networks = column![].spacing(2.0);
        for (i, ap) in app.network.wifi.iter().enumerate() {
            let selected = app.network_selection == Some(i);
            let detail = if ap.active {
                app.tr("Connected").to_owned()
            } else if !ap.supported {
                app.tr("Requires enterprise or legacy settings").into()
            } else {
                format!(
                    "{} · {}% signal",
                    if ap.security == "--" || ap.security.is_empty() {
                        app.tr("Open network")
                    } else {
                        &ap.security
                    },
                    ap.signal
                )
            };
            let mut row = enz_preferences_row(&ap.ssid)
                .icon(if ap.active { "circle-check" } else { "wifi" })
                .subtitle(detail)
                .id(crate::keyboard_nav::choice_id(&Message::NetworkSelect(i)).unwrap());
            // The whole row joins; the chevron is the row's own affordance.
            if !ap.active {
                row = row
                    .action("", Message::NetworkSelect(i))
                    .enabled(!app.network_busy && ap.supported);
            }
            networks = networks.push(row.into_element());
            if selected && !ap.active {
                // The password opens right under the network it belongs to.
                let secure = !ap.security.is_empty() && ap.security != "--";
                // One line: the password, then a compact accent action beside it.
                let connect = enz_button(
                    if secure {
                        app.tr("Connect")
                    } else {
                        app.tr("Join network")
                    },
                    Message::NetworkConnect,
                )
                .icon("arrow-right")
                .kind(ButtonKind::Accent)
                .enabled(!app.network_busy && (!secure || !app.network_password.is_empty()))
                .into_element();
                let join: Element<'_, Message> = if secure {
                    row![
                        container(
                            enz_secure_field(
                                &app.network_password,
                                Message::NetworkPassword,
                                app.network_revealed,
                                Message::NetworkReveal
                            )
                            .id(iced::advanced::widget::Id::new(
                                "installer-network-password"
                            ))
                            .placeholder(crate::i18n::translate(&app.localizer, "Wi-Fi password"))
                            .on_submit(Message::NetworkConnect)
                            .into_element()
                        )
                        .width(Length::Fill),
                        connect
                    ]
                    .spacing(10.0)
                    .align_y(Alignment::Center)
                    .into()
                } else {
                    row![Space::new().width(Length::Fill), connect].into()
                };
                networks = networks.push(container(join).padding(iced::Padding {
                    top: 2.0,
                    right: 12.0,
                    bottom: 12.0,
                    left: 52.0,
                }));
            }
        }
        if !app.network.wifi.is_empty() {
            body = body.push(enz_preferences_group(networks).into_element());
        } else if app.network.radio && !app.network_busy {
            body = body.push(
                enz_text(app.tr("No nearby networks found. Try Refresh.")).tone(TextTone::Muted),
            );
        }
    }
    if app.network_busy {
        body = body.push(enz_progress_bar(app.loading_fraction).into_element());
    } else if app.network.devices.is_empty() && app.network_error.is_empty() {
        body = body.push(
            enz_banner(app.tr("No network adapter found"))
                .body(app.tr("You can connect later from the desktop."))
                .tone(BannerTone::Info)
                .into_element(),
        );
    }
    body.push(
        enz_text(
            app.tr(
                "Setup uses the packages on this installation drive. No updates are downloaded.",
            ),
        )
        .size(TextSize::Callout)
        .tone(TextTone::Muted),
    )
    .into()
}

/// The small enzimOS mark (enzimos-icons `brand/`), monochrome.
const ENZIMOS_MARK: &[u8] = include_bytes!("../assets/enzimos-mark.svg");

/// The enzimOS mark in the brand's two tones: ink #1C1C1C on light, paper #F1F0EE on dark.
fn enzimos_mark<'a>(size: f32) -> iced::Element<'a, Message> {
    enzui_widgets::icon::enz_icon_from_handle(iced::widget::svg::Handle::from_memory(ENZIMOS_MARK))
        .size(size)
        .tint_resolved(|theme| {
            if enzui_widgets::theme_lookup::palette(theme).is_dark() {
                iced::Color::from_rgb8(0xF1, 0xF0, 0xEE)
            } else {
                iced::Color::from_rgb8(0x1C, 0x1C, 0x1C)
            }
        })
        .into()
}

/// The step's header glyph: the enzimOS mark on Welcome, where the installer
/// introduces the system, and the step's own icon in the accent elsewhere.
fn step_mark<'a>(step: Step) -> iced::Element<'a, Message> {
    if step == Step::Welcome {
        enzimos_mark(40.0)
    } else {
        enz_icon(step.icon()).size(36.0).tint_theme_accent().into()
    }
}
