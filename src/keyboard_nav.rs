// SPDX-License-Identifier: MIT
//! Wizard keyboard handling, scoped to the control holding focus.
use crate::{Installer, Message, Step, system};
use iced::{
    Task,
    advanced::widget::{
        Id, Operation,
        operation::{Focusable, Outcome},
    },
    keyboard::key::Named,
};

pub(crate) fn choice_id(message: &Message) -> Option<Id> {
    let name = match message {
        Message::Language(i) => format!("installer-language-{i}"),
        Message::Keyboard(i) => format!("installer-keyboard-{i}"),
        Message::Zone(z) => format!("installer-zone-{z}"),
        Message::NetworkSelect(i) => format!("installer-network-{i}"),
        Message::Disk(i) => format!("installer-disk-{i}"),
        Message::DiskMode(v) => format!("installer-disk-mode-{v}"),
        Message::Partition(Some(i)) => format!("installer-partition-{i}"),
        _ => return None,
    };
    Some(Id::from(name))
}

struct FocusQuery {
    key: Named,
    captured: bool,
    focused: Option<Id>,
}
impl Operation<Message> for FocusQuery {
    fn focusable(&mut self, id: Option<&Id>, _: iced::Rectangle, state: &mut dyn Focusable) {
        if state.is_focused() {
            self.focused = id.cloned();
        }
    }
    fn traverse(&mut self, f: &mut dyn FnMut(&mut dyn Operation<Message>)) {
        f(self);
    }
    fn finish(&self) -> Outcome<Message> {
        Outcome::Some(Message::KeyFocused(
            self.key,
            self.captured,
            self.focused.clone(),
        ))
    }
}

pub(crate) fn key(app: &Installer, key: Named, shift: bool, captured: bool) -> Task<Message> {
    if app.exit_menu || app.log_transfer.is_some() {
        return Task::none();
    }
    if key == Named::Tab && !captured {
        return tab(app, shift);
    }
    if key == Named::Tab {
        return Task::none();
    }
    iced::advanced::widget::operate(FocusQuery {
        key,
        captured,
        focused: None,
    })
}

fn choices(app: &Installer) -> Vec<(Message, bool)> {
    match app.step {
        Step::Welcome => system::LANGUAGES
            .iter()
            .enumerate()
            .map(|(i, _)| (Message::Language(i), i == app.language))
            .collect(),
        Step::Region => {
            let needle = app.keyboard_search.to_lowercase();
            system::KEYBOARDS
                .iter()
                .enumerate()
                .filter(|(_, (label, _, xkb))| {
                    needle.is_empty()
                        || label.to_lowercase().contains(&needle)
                        || xkb.contains(&needle)
                })
                .map(|(i, _)| (Message::Keyboard(i), i == app.keyboard))
                .collect()
        }
        Step::Timezone => app
            .zones
            .iter()
            .filter(|z| {
                system::zone_matches(
                    z,
                    app.zone_countries.get(*z).map_or("", String::as_str),
                    &app.zone_search,
                )
            })
            .map(|z| (Message::Zone(z.clone()), *z == app.zone))
            .collect(),
        Step::Network if !app.network_busy => app
            .network
            .wifi
            .iter()
            .enumerate()
            .filter(|(_, ap)| ap.supported && !ap.active)
            .map(|(i, _)| (Message::NetworkSelect(i), app.network_selection == Some(i)))
            .collect(),
        Step::Disk => {
            let mut values: Vec<_> = app
                .disks
                .iter()
                .enumerate()
                .filter(|(_, d)| d.large_enough())
                .map(|(i, _)| (Message::Disk(i), app.disk == Some(i)))
                .collect();
            if app.disk.is_some() {
                values.extend([
                    (Message::DiskMode(true), app.disk_mode == Some(true)),
                    (Message::DiskMode(false), app.disk_mode == Some(false)),
                ]);
                if app.disk_mode == Some(true) {
                    values.extend(app.partition_choices.iter().enumerate().map(|(i, _)| {
                        (Message::Partition(Some(i)), app.partition_choice == Some(i))
                    }));
                }
            }
            values
        }
        _ => vec![],
    }
}

pub(crate) fn focused(
    app: &mut Installer,
    key: Named,
    captured: bool,
    focus: Option<Id>,
) -> Task<Message> {
    if app.exit_menu || app.log_transfer.is_some() || app.installing() {
        return Task::none();
    }
    let mut values = choices(app);
    let at = focus.as_ref().and_then(|id| {
        values
            .iter()
            .position(|(m, _)| choice_id(m).as_ref() == Some(id))
    });
    if key == Named::Enter {
        // A focused button/field owns Enter. Choice rows already selected their
        // value; they can now continue. Never submit twice or bypass blockers.
        // Installation requires explicitly activating its button. A delayed
        // Enter from the previous page must leave the review open.
        if app.step < Step::Review && (at.is_some() || !captured) {
            return app.update(Message::Next);
        }
        return Task::none();
    }
    let from_search = matches!(key, Named::ArrowUp | Named::ArrowDown)
        && focus.as_ref().is_some_and(|id| {
            id == &Id::new("installer-keyboard-search") || id == &Id::new("installer-zone-search")
        });
    if focus.is_some() && at.is_none() && !from_search {
        return Task::none();
    } // text cursor / other control
    if let Some(i) = at {
        let group = std::mem::discriminant(&values[i].0);
        values.retain(|(m, _)| std::mem::discriminant(m) == group);
    } else if app.step == Step::Disk {
        return Task::none();
    }
    if values.is_empty() {
        return Task::none();
    }
    let current = focus
        .as_ref()
        .and_then(|id| {
            values
                .iter()
                .position(|(m, _)| choice_id(m).as_ref() == Some(id))
        })
        .or_else(|| values.iter().position(|(_, selected)| *selected));
    let backwards = matches!(key, Named::ArrowUp | Named::ArrowLeft);
    let index = match current {
        Some(i) if backwards => i.saturating_sub(1),
        Some(i) => (i + 1).min(values.len() - 1),
        None if backwards => values.len() - 1,
        None => 0,
    };
    let message = values[index].0.clone();
    let id = choice_id(&message).unwrap();
    Task::batch([app.update(message), focus_visible(id)])
}

// Choice lists are one Tab stop; arrows move within the list. This keeps a
// country list from requiring hundreds of Tab presses to reach Continue.
fn tab(app: &Installer, backwards: bool) -> Task<Message> {
    let values = choices(app);
    let mut stops = vec![];
    for (message, _) in &values {
        let group = std::mem::discriminant(message);
        if stops.iter().any(|(g, _)| *g == group) {
            continue;
        }
        let chosen = values
            .iter()
            .find(|(m, selected)| std::mem::discriminant(m) == group && *selected)
            .unwrap_or_else(|| {
                values
                    .iter()
                    .find(|(m, _)| std::mem::discriminant(m) == group)
                    .unwrap()
            });
        stops.push((group, choice_id(&chosen.0).unwrap()));
    }
    let all = values
        .iter()
        .filter_map(|(m, _)| choice_id(m))
        .collect::<Vec<_>>();
    let skip = all
        .into_iter()
        .filter(|id| !stops.iter().any(|(_, stop)| id == stop))
        .collect();
    iced::advanced::widget::operate(TabQuery {
        backwards,
        skip,
        eligible: vec![],
        current: None,
    })
}
struct TabQuery {
    backwards: bool,
    skip: Vec<Id>,
    eligible: Vec<Id>,
    current: Option<Id>,
}
impl Operation<Message> for TabQuery {
    fn focusable(&mut self, id: Option<&Id>, _: iced::Rectangle, state: &mut dyn Focusable) {
        if state.is_focused() {
            self.current = id.cloned();
        }
        if let Some(id) = id
            && !self.skip.contains(id)
            && !self.eligible.contains(id)
        {
            self.eligible.push(id.clone());
        }
    }
    fn traverse(&mut self, f: &mut dyn FnMut(&mut dyn Operation<Message>)) {
        f(self);
    }
    fn finish(&self) -> Outcome<Message> {
        Outcome::Some(Message::TabFocused(
            self.eligible.clone(),
            self.current.clone(),
            self.backwards,
        ))
    }
}
pub(crate) fn tab_focused(ids: Vec<Id>, focus: Option<Id>, backwards: bool) -> Task<Message> {
    let current = focus
        .as_ref()
        .and_then(|id| ids.iter().position(|i| i == id));
    let indices: Vec<_> = (0..ids.len()).collect();
    match tab_target(&indices, current, backwards) {
        Some(target) => focus_visible(ids[target].clone()),
        None => Task::none(),
    }
}
fn tab_target(eligible: &[usize], current: Option<usize>, backwards: bool) -> Option<usize> {
    if backwards {
        current
            .and_then(|at| eligible.iter().rev().find(|i| **i < at).copied())
            .or_else(|| eligible.last().copied())
    } else {
        current
            .and_then(|at| eligible.iter().find(|i| **i > at).copied())
            .or_else(|| eligible.first().copied())
    }
}

pub(crate) fn page_focus(app: &Installer) -> Task<Message> {
    let id = match app.step {
        Step::Welcome => choice_id(&Message::Language(app.language)),
        Step::Region => choice_id(&Message::Keyboard(app.keyboard)),
        Step::Timezone => Some(Id::new("installer-zone-search")),
        Step::Disk => app.disk.and_then(|i| choice_id(&Message::Disk(i))),
        Step::Encryption if app.encrypt => Some(Id::new("installer-passphrase")),
        Step::Account => Some(Id::new("installer-name")),
        _ => None,
    };
    iced::widget::operation::scroll_to(
        "installer-page",
        iced::widget::scrollable::AbsoluteOffset {
            x: None,
            y: Some(0.0),
        },
        iced::widget::operation::Animation::Instant,
    )
    .chain(match id {
        Some(id) => focus_visible(id),
        None => {
            iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::unfocus())
        }
    })
}

// Framework operation reveals through all actual enclosing viewports.
fn focus_visible(id: Id) -> Task<Message> {
    iced::advanced::widget::operate(enzui_core::focus::focus_and_reveal_id(id))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_form_enter_stays_on_page() {
        let (mut app, _) = Installer::new(true);
        app.step = Step::Encryption;
        let _ = focused(&mut app, Named::Enter, false, None);
        assert_eq!(app.step, Step::Encryption);
    }
    #[test]
    fn enter_does_not_steal_a_control_or_modal() {
        let (mut app, _) = Installer::new(true);
        let _ = focused(
            &mut app,
            Named::Enter,
            true,
            Some(Id::new("installer-theme")),
        );
        assert_eq!(app.step, Step::Welcome);
        app.exit_menu = true;
        let _ = focused(&mut app, Named::Enter, false, None);
        assert_eq!(app.step, Step::Welcome);

        app.exit_menu = false;
        let _ = app.update(Message::Disk(0));
        let _ = app.update(Message::DiskMode(false));
        let _ = app.update(Message::Encrypt(false));
        let _ = app.update(Message::Edit(
            crate::Field::FullName,
            "Jamie Example".into(),
        ));
        let _ = app.update(Message::Edit(crate::Field::Password, "sample pass".into()));
        let _ = app.update(Message::Edit(crate::Field::Confirm, "sample pass".into()));
        app.step = Step::Review;
        assert!(app.blocker().is_none());
        let _ = focused(&mut app, Named::Enter, false, None);
        assert_eq!(app.step, Step::Review);
    }
    #[test]
    fn text_cursor_arrows_do_not_change_choice() {
        let (mut app, _) = Installer::new(true);
        app.step = Step::Timezone;
        let before = app.zone.clone();
        let _ = focused(
            &mut app,
            Named::ArrowDown,
            true,
            Some(Id::new("timezone-search")),
        );
        assert_eq!(app.zone, before);
    }
}
