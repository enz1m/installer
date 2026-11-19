//! Installer selection, disk and account forms.

use super::*;
use iced::widget::column;

pub(super) fn welcome(app: &Installer, list_height: f32) -> Element<'_, Message> {
    let languages = system::LANGUAGES
        .iter()
        .enumerate()
        .map(|(i, (label, ..))| selected_row(*label, Message::Language(i), app.language == i))
        .collect();
    column![
        enz_text(app.tr("Used for the installer and the installed system")).tone(TextTone::Muted),
        list(languages, list_height, ""),
    ]
    .spacing(16.0)
    .into()
}

pub(super) fn region(app: &Installer, compact: bool, list_height: f32) -> Element<'_, Message> {
    let needle = app.keyboard_search.to_lowercase();
    let keyboards = system::KEYBOARDS
        .iter()
        .enumerate()
        .filter(|(_, (label, _, xkb))| {
            needle.is_empty() || label.to_lowercase().contains(&needle) || xkb.contains(&needle)
        })
        .map(|(i, (label, ..))| selected_row(*label, Message::Keyboard(i), app.keyboard == i))
        .collect();
    let choices = column![
        enz_search_field(&app.keyboard_search, |v| Message::Edit(
            Field::KeyboardSearch,
            v
        ))
        .id(iced::advanced::widget::Id::new("installer-keyboard-search"))
        .placeholder(crate::i18n::translate(&app.localizer, "Search layouts"))
        .on_submit(Message::Next)
        .into_element(),
        list(
            keyboards,
            if compact {
                list_height.min(180.0)
            } else {
                list_height - 10.0
            },
            app.tr("No layout matches.")
        )
    ]
    .spacing(12.0);
    let test = column![
        enz_icon("keyboard")
            .size(48.0)
            .tint_theme_tone(IconTone::Muted),
        enz_text(system::KEYBOARDS[app.keyboard].0).size(TextSize::Headline),
        enz_text_field(
            crate::i18n::translate(&app.localizer, "Test your keyboard"),
            crate::i18n::translate(&app.localizer, "Type here to try the layout"),
            &app.keyboard_test,
            |v| Message::Edit(Field::KeyboardTest, v)
        )
        .id(iced::advanced::widget::Id::new("installer-keyboard-test"))
        .on_submit(Message::Next)
        .into_element()
    ]
    .spacing(24.0);
    if compact {
        column![choices, test].spacing(20.0).into()
    } else {
        row![
            container(choices).width(Length::FillPortion(3)),
            container(test)
                .padding([32.0, 16.0])
                .width(Length::FillPortion(2))
        ]
        .spacing(36.0)
        .into()
    }
}

pub(super) fn timezone(app: &Installer, list_height: f32) -> Element<'_, Message> {
    let zones = app
        .zones
        .iter()
        .filter(|z| {
            system::zone_matches(
                z,
                app.zone_countries.get(*z).map_or("", String::as_str),
                &app.zone_search,
            )
        })
        .map(|zone| {
            let (city, region) = app.zone_parts(zone);
            let row = enz_sidebar_row(city, Message::Zone(zone.clone()))
                .id(crate::keyboard_nav::choice_id(&Message::Zone(zone.clone())).unwrap())
                .count(region)
                .selected(&app.zone == zone);
            if &app.zone == zone {
                row.trailing(enz_icon("circle-check").size(18.0).tint_theme_accent())
                    .into()
            } else {
                row.into()
            }
        })
        .collect();
    column![
        enz_search_field(&app.zone_search, |v| Message::Edit(Field::ZoneSearch, v))
            .id(iced::advanced::widget::Id::new("installer-zone-search"))
            .placeholder(crate::i18n::translate(
                &app.localizer,
                "Search cities or countries"
            ))
            .on_submit(Message::Next)
            .into_element(),
        list(zones, list_height - 10.0, app.tr("No time zone matches.")),
    ]
    .spacing(16.0)
    .into()
}

pub(super) fn list<'a>(
    rows: Vec<Element<'a, Message>>,
    height: f32,
    empty: &'a str,
) -> Element<'a, Message> {
    let inner: Element<'a, Message> = if rows.is_empty() {
        container(enz_text(empty).tone(TextTone::Muted))
            .padding(12.0)
            .into()
    } else {
        column(rows).spacing(2.0).into()
    };
    enz_scroll_view(container(inner).padding(6.0))
        .id("installer-choices".into())
        .height(Length::Fixed(height))
        .into()
}

pub(super) fn disk(app: &Installer) -> Element<'_, Message> {
    let rows: Vec<Element<'_, Message>> = if app.disks.is_empty() {
        vec![
            enz_preferences_row(app.tr("No disks found"))
                .subtitle(app.tr("Connect a disk of at least 20 GB, then refresh."))
                .icon("shield-alert")
                .into_element(),
        ]
    } else {
        app.disks
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let mut detail = app.disk_detail(d);
                if !d.large_enough() {
                    detail = format!("{detail} · {}", app.tr("too small"));
                } else if d.removable {
                    detail = format!("{detail} · {}", app.tr("removable"));
                }
                let row = enz_preferences_row(app.disk_title(d))
                    .subtitle(detail)
                    .icon(if d.transport == "usb" {
                        "usb"
                    } else {
                        "hard-drive"
                    })
                    .enabled(d.large_enough())
                    .control(
                        enz_radio_button(system::human_size(d.bytes), i, app.disk, Message::Disk)
                            .id(crate::keyboard_nav::choice_id(&Message::Disk(i)).unwrap())
                            .enabled(d.large_enough())
                            .into_element(),
                    );
                // The whole row picks the disk, not only the radio at its far
                // edge; the radio stays the keyboard and accessibility target.
                if d.large_enough() {
                    iced::widget::mouse_area(row.into_element())
                        .on_press(Message::Disk(i))
                        .interaction(iced::mouse::Interaction::Pointer)
                        .into()
                } else {
                    row.into_element()
                }
            })
            .collect()
    };
    let mut page = column![
        enz_preferences_group(column(rows))
            .title(app.tr("Disks"))
            .header_action(app.tr("Refresh"), Message::RefreshDisks)
            .into_element(),
    ]
    .spacing(20.0);
    if let Some(d) = app.disk.map(|i| &app.disks[i]) {
        let option = |alongside, icon, title: &str, detail: &str| -> Element<'_, Message> {
            let selected = app.disk_mode == Some(alongside);
            let mut top = row![
                enz_icon(icon).size(24.0).tint_theme_accent(),
                Space::new().width(Length::Fill)
            ]
            .height(26.0)
            .align_y(Alignment::Center);
            if selected {
                top = top.push(enz_icon("circle-check").size(20.0).tint_theme_accent());
            }
            enzui_widgets::focus_button::focus_button(
                column![
                    top,
                    enz_text(title).size(TextSize::Title3),
                    enz_text(detail).tone(TextTone::Muted),
                ]
                .spacing(12.0),
                Message::DiskMode(alongside),
            )
            .padding(20.0)
            .width(Length::Fill)
            .id(crate::keyboard_nav::choice_id(&Message::DiskMode(alongside)).unwrap())
            .style_focus(move |theme: &iced::Theme, status| {
                let p = enzui_widgets::theme_lookup::palette(theme);
                let hover = matches!(
                    status,
                    enzui_widgets::focus_button::FocusStatus::Hovered
                        | enzui_widgets::focus_button::FocusStatus::Pressed
                );
                iced::widget::button::Style {
                    background: Some(
                        (if selected {
                            p.surface_selected
                        } else if hover {
                            p.surface_hover
                        } else {
                            p.surface_elevated
                        })
                        .into(),
                    ),
                    text_color: p.text,
                    border: iced::Border {
                        color: if selected
                            || status == enzui_widgets::focus_button::FocusStatus::Focused
                        {
                            p.accent
                        } else {
                            p.border
                        },
                        width: 1.0,
                        radius: 12.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
        };
        page = page.push(
            row![
                container(option(
                    true,
                    "columns-2",
                    app.tr("Keep existing systems"),
                    app.tr("Install in available space and choose your system at startup.")
                ))
                .width(Length::Fill),
                container(option(
                    false,
                    "hard-drive",
                    app.tr("Use the entire disk"),
                    app.tr("Replace everything on this disk with a fresh enzimOS installation.")
                ))
                .width(Length::Fill),
            ]
            .spacing(16.0),
        );
        if app.disk_mode == Some(true) {
            if app.inspecting {
                page = page.push(enz_progress_bar(app.loading_fraction).into_element());
            } else if !app.partition_status.is_empty() {
                page = page.push(
                    enz_banner(app.tr("Space could not be prepared"))
                        .body(&app.partition_status)
                        .tone(BannerTone::Info)
                        .into_element(),
                );
                page = page.push(
                    enz_button(app.tr("Check again"), Message::InspectDisk)
                        .kind(ButtonKind::Secondary)
                        .into_element(),
                );
            }
            for (i, p) in app.partition_choices.iter().enumerate() {
                let description = if p.shrink {
                    crate::i18n::format(
                        &app.localizer,
                        "{} · {} for enzimOS",
                        &[
                            ("arg0", &(p.label)),
                            ("arg1", &(system::human_size(p.bytes))),
                        ],
                    )
                } else {
                    crate::i18n::format(
                        &app.localizer,
                        "Use {} of unallocated space",
                        &[("arg0", &(system::human_size(p.bytes)))],
                    )
                };
                page = page.push(selected_row(
                    description,
                    Message::Partition(Some(i)),
                    app.partition_choice == Some(i),
                ));
            }
            if let Some(i) = app.partition_choice {
                let p = &app.partition_choices[i];
                let used = (d.bytes.saturating_sub(p.bytes) as f32 / d.bytes.max(1) as f32)
                    .clamp(0.02, 0.98);
                page=page.push(column![
                    enz_text(app.tr("After installation")).size(TextSize::Callout),
                    row![container(Space::new().height(10.0)).width(Length::FillPortion((used*100.0) as u16))
                        .style(|_| container::Style::default().background(iced::Color::from_rgb8(142,151,170)).border(iced::Border::default().rounded(5.0))),
                        container(Space::new().height(10.0)).width(Length::FillPortion(((1.0-used)*100.0) as u16))
                        .style(|_| container::Style::default().background(iced::Color::from_rgb8(43,96,241)).border(iced::Border::default().rounded(5.0)))].spacing(4.0),
                    row![enz_text(app.tr("Existing partitions kept")).tone(TextTone::Muted), Space::new().width(Length::Fill),
                        enz_text(format!("enzimOS · {}",system::human_size(p.bytes))).tone(TextTone::Muted)],
                    enz_text(if p.shrink { app.tr("This resizes the selected filesystem. Back up its files before continuing.") }
                        else { app.tr("Existing partitions and files stay in place. No resizing is needed.") }).size(TextSize::Callout).tone(TextTone::Muted),
                ].spacing(10.0));
            }
        } else if app.disk_mode == Some(false) {
            page=page.push(enz_banner(app.tr("This disk will be erased"))
                .body(crate::i18n::format(&app.localizer, "All systems, partitions and files on {} ({}) will be removed. Other disks are kept.", &[("arg0", &(app.disk_title(d))), ("arg1", &(d.path))]))
                .tone(BannerTone::Warning).into_element());
        }
    }
    page.into()
}

pub(super) fn encryption(app: &Installer) -> Element<'_, Message> {
    let passphrase = app.revealed.contains(&"passphrase");
    let mut encryption = column![
        enz_preferences_row(app.tr("Encrypt enzimOS"))
            .subtitle(app.tr("Your files stay private if the computer is lost or stolen. You type the passphrase each time it starts."))
            .icon("lock")
            .control(enzui_widgets::switch::switch(app.encrypt, Message::Encrypt)
                .id(iced::advanced::widget::Id::new("installer-encryption"))
                .label(app.tr("Encrypt enzimOS")))
            .into_element()
    ]
    .spacing(14.0);
    if app.encrypt {
        let mismatch = (!app.passphrase_confirm.is_empty()
            && app.passphrase != app.passphrase_confirm)
            .then_some(app.tr("Does not match"));
        encryption = encryption.push(
            container(
                column![
                    secret(
                        crate::i18n::translate(&app.localizer, "Passphrase"),
                        crate::i18n::translate(&app.localizer, "Choose a passphrase"),
                        &app.passphrase,
                        passphrase,
                        || Field::Passphrase,
                        None
                    ),
                    secret(
                        crate::i18n::translate(&app.localizer, "Confirm passphrase"),
                        crate::i18n::translate(&app.localizer, "Type it again"),
                        &app.passphrase_confirm,
                        passphrase,
                        || Field::PassphraseConfirm,
                        mismatch
                    ),
                ]
                .spacing(16.0),
            )
            .padding([0.0, 12.0]),
        );
    }
    column![
        encryption,
        enz_text(if app.encrypt {
            app.tr("There is no way to recover a forgotten passphrase.")
        } else {
            app.tr("Without encryption, anyone with the disk can read your files.")
        })
        .tone(TextTone::Muted)
    ]
    .spacing(24.0)
    .into()
}

pub(super) fn secret<'a>(
    label: impl Into<std::borrow::Cow<'a, str>>,
    placeholder: impl Into<std::borrow::Cow<'a, str>>,
    value: &'a str,
    revealed: bool,
    field: fn() -> Field,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    let id = match field() {
        Field::Passphrase => "installer-passphrase",
        Field::PassphraseConfirm => "installer-passphrase-confirm",
        Field::Password => "installer-password",
        _ => "installer-password-confirm",
    };
    let input = enz_secure_field(
        value,
        move |v| Message::Edit(field(), v),
        revealed,
        Message::Reveal(field()),
    )
    .id(iced::advanced::widget::Id::new(id))
    .label(label)
    .placeholder(placeholder)
    .on_submit(Message::Next);
    match error {
        Some(e) => input.error(e),
        None => input,
    }
    .into_element()
}

pub(super) fn account(app: &Installer) -> Element<'_, Message> {
    let revealed = app.revealed.contains(&"password");
    let user_error = (!app.user.is_empty() && !system::valid_user(&app.user))
        .then_some(app.tr("Lowercase letters, digits, - and _"));
    let host_error = (!app.host.is_empty() && !system::valid_host(&app.host))
        .then_some(app.tr("Letters, digits and - only"));
    let user = enz_text_field(
        crate::i18n::translate(&app.localizer, "User name"),
        crate::i18n::translate(&app.localizer, "User name"),
        &app.user,
        |v| Message::Edit(Field::User, v),
    )
    .id(iced::advanced::widget::Id::new("installer-user"))
    .on_submit(Message::Next)
    .description(app.trf(
        "Your home folder is /home/{0}",
        &[if app.user.is_empty() {
            "…"
        } else {
            &app.user
        }],
    ));
    let host = enz_text_field(
        crate::i18n::translate(&app.localizer, "Computer name"),
        crate::i18n::translate(&app.localizer, "Computer name"),
        &app.host,
        |v| Message::Edit(Field::Host, v),
    )
    .id(iced::advanced::widget::Id::new("installer-host"))
    .on_submit(Message::Next)
    .description(app.tr("How this computer appears on your network"));
    let about = column![
        enz_text_field(
            crate::i18n::translate(&app.localizer, "Your name"),
            crate::i18n::translate(&app.localizer, "Your name"),
            &app.full_name,
            |v| Message::Edit(Field::FullName, v)
        )
        .id(iced::advanced::widget::Id::new("installer-name"))
        .on_submit(Message::Next)
        .into_element(),
        row![
            match user_error {
                Some(e) => user.error(e),
                None => user,
            }
            .into_element(),
            match host_error {
                Some(e) => host.error(e),
                None => host,
            }
            .into_element(),
        ]
        .spacing(16.0),
    ]
    .spacing(16.0);
    let mismatch = (!app.confirm.is_empty() && app.confirm != app.password)
        .then_some(app.tr("Does not match"));
    let mut password = column![
        row![
            secret(
                crate::i18n::translate(&app.localizer, "Password"),
                crate::i18n::translate(&app.localizer, "Choose a password"),
                &app.password,
                revealed,
                || Field::Password,
                None
            ),
            secret(
                crate::i18n::translate(&app.localizer, "Confirm password"),
                crate::i18n::translate(&app.localizer, "Type it again"),
                &app.confirm,
                revealed,
                || Field::Confirm,
                mismatch
            ),
        ]
        .spacing(16.0)
    ]
    .spacing(14.0);
    if !app.password.is_empty() {
        let (label, value) = system::strength(&app.password);
        let tone = match label {
            "Weak" => enzui_widgets::ProgressTone::Danger,
            "Fair" => enzui_widgets::ProgressTone::Warning,
            _ => enzui_widgets::ProgressTone::Success,
        };
        password = password.push(
            enz_progress_bar(value)
                .tone(tone)
                .label(app.tr("Strength"))
                .value_text(app.tr(label))
                .into_element(),
        );
    }
    column![
        enz_text(
            app.tr(
                "You sign in with this account. It can install apps and change system settings."
            )
        )
        .tone(TextTone::Muted),
        enz_text(app.tr("About you")).size(TextSize::Headline),
        about,
        enz_divider().into_element(),
        password,
    ]
    .spacing(20.0)
    .into()
}
