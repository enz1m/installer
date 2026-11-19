// SPDX-License-Identifier: MIT
//! Graphical frontend for the native installer on enzimOS live media.
//! `--demo` simulates installation without writing to a disk.

mod development;
use development::dev_access_line;

mod i18n;
mod keyboard_nav;
mod log_transfer;
mod network;
mod system;
mod view;

use enzui_core::theme_file::integration::{ThemeEvent, ThemeIntegration};
use enzui_theme::EnzTheme;
use enzui_widgets::app_shell::{
    DesktopApplicationIdentity, ShellLayout, ShellTokens, ShellWindowEvent, ShellWindowState,
};
use enzui_widgets::title_bar_gesture::WindowCommand;
use iced::{Subscription, Task};

pub(crate) const LAYOUT: ShellLayout = ShellLayout::TitlebarContent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Step {
    Welcome,
    Region,
    Timezone,
    Network,
    Disk,
    Encryption,
    Account,
    Review,
    Installing,
    Finished,
}

mod backend_output;
mod install_flow;
mod probe;
mod secret;
use secret::Secret;
mod steps;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Outcome {
    Running,
    Done,
    Failed(String),
}

#[derive(Debug, Clone)]
pub(crate) enum Field {
    FullName,
    User,
    Host,
    Password,
    Confirm,
    Passphrase,
    PassphraseConfirm,
    ZoneSearch,
    KeyboardSearch,
    KeyboardTest,
}

#[derive(Clone)]
pub(crate) enum Message {
    Go(Step),
    Key(iced::keyboard::key::Named, bool, bool),
    KeyFocused(
        iced::keyboard::key::Named,
        bool,
        Option<iced::advanced::widget::Id>,
    ),
    TabFocused(
        Vec<iced::advanced::widget::Id>,
        Option<iced::advanced::widget::Id>,
        bool,
    ),
    Next,
    Back,
    Language(usize),
    Keyboard(usize),
    Zone(String),
    Disk(usize),
    RefreshDisks,
    DisksDiscovered(u64, Vec<system::Disk>),
    InstallDisksChecked(u64, Vec<system::Disk>),
    InspectDisk,
    DiskMode(bool),
    PartitionChoices((String, Result<Vec<system::PartitionChoice>, String>)),
    Partition(Option<usize>),
    Encrypt(bool),
    CheckMedia(bool),
    Edit(Field, String),
    Reveal(Field),
    Install,
    Progress(system::Progress),
    ToggleLog,
    ShowQr,
    CloseQr,
    QrTick,
    DevAccessTick,
    LoadingTick,
    NetworkRefresh,
    NetworkResult(u64, Result<network::State, String>),
    NetworkSelect(usize),
    NetworkPassword(String),
    NetworkReveal,
    NetworkConnect,
    DismissNetworkError,
    NetworkWired(String),
    NetworkRadio(bool),
    Theme(bool),
    ExitMenu,
    DismissExit,
    Reset,
    PowerOff,
    Modal(enzui_widgets::modal_host::ModalHostEvent),
    Restart,
    Quit,
    Window(WindowCommand),
    WindowState(ShellWindowEvent),
    SharedTheme(Box<ThemeEvent>),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Installer message (contents redacted)")
    }
}

pub(crate) struct Installer {
    localizer: enzui_core::localization::Localizer,
    localized_labels: std::collections::HashMap<&'static str, String>,
    pub(crate) demo: bool,
    pub(crate) dark: bool,
    pub(crate) exit_menu: bool,
    pub(crate) zone_countries: std::collections::HashMap<String, String>,
    pub(crate) step: Step,
    pub(crate) reached: Step,
    pub(crate) chrome: ShellWindowState,
    themes: ThemeIntegration,

    pub(crate) language: usize,
    pub(crate) keyboard: usize,
    pub(crate) keyboard_search: String,
    pub(crate) keyboard_test: String,
    pub(crate) keyboard_touched: bool,
    pub(crate) zone_touched: bool,
    pub(crate) zones: Vec<String>,
    pub(crate) zone: String,
    pub(crate) zone_search: String,

    pub(crate) network: network::State,
    pub(crate) network_busy: bool,
    pub(crate) network_generation: u64,
    pub(crate) network_error: String,
    pub(crate) network_selection: Option<usize>,
    pub(crate) network_password: Secret,
    pub(crate) network_revealed: bool,
    pub(crate) disks: Vec<system::Disk>,
    pub(crate) disks_busy: bool,
    discovery_generation: u64,
    discovery_selection: Option<system::Disk>,
    pending_install: Option<system::Plan>,
    pub(crate) cleanup_incomplete: bool,
    pub(crate) disk: Option<usize>,
    pub(crate) partition_choices: Vec<system::PartitionChoice>,
    pub(crate) partition_choice: Option<usize>,
    pub(crate) partition_status: String,
    pub(crate) disk_mode: Option<bool>, // true: alongside, false: explicitly erase
    pub(crate) inspecting: bool,
    pub(crate) loading_started: std::time::Instant,
    pub(crate) loading_fraction: f32,
    pub(crate) stage_started: std::time::Instant,
    pub(crate) install_fraction: f32,
    pub(crate) encrypt: bool,
    pub(crate) check_media: bool,
    pub(crate) passphrase: Secret,
    pub(crate) passphrase_confirm: Secret,

    pub(crate) full_name: String,
    pub(crate) user: String,
    pub(crate) user_edited: bool,
    pub(crate) host: String,
    pub(crate) host_edited: bool,
    pub(crate) password: Secret,
    pub(crate) confirm: Secret,
    pub(crate) revealed: Vec<&'static str>,

    pub(crate) stage: usize,
    pub(crate) lines: Vec<String>,
    pub(crate) outcome: Outcome,
    pub(crate) show_log: bool,
    pub(crate) full_log: String,
    pub(crate) log_transfer: Option<log_transfer::Transfer>,
    /// Development media only: where to SSH in, shown in the title bar.
    pub(crate) dev_access: Option<String>,
}

impl Installer {
    fn new(demo: bool) -> (Self, Task<Message>) {
        let (mut themes, theme_boot) = ThemeIntegration::boot();
        if let Ok(mode) = std::env::var("ENZIMOS_INSTALLER_THEME") {
            themes.set_mode_override(Some(if mode == "dark" {
                enzui_theme::EnzThemeMode::Dark
            } else {
                enzui_theme::EnzThemeMode::Light
            }));
        }
        let (app, boot) = Self::with_themes(demo, themes);
        (
            app,
            Task::batch([
                boot,
                theme_boot.map(|event| Message::SharedTheme(Box::new(event))),
            ]),
        )
    }

    fn with_themes(demo: bool, themes: ThemeIntegration) -> (Self, Task<Message>) {
        let zones = system::time_zones();
        let preferred = enzui_core::localization::preferred_locales();
        let language = preferred
            .first()
            .and_then(|p| {
                system::LANGUAGES.iter().position(|(_, locale, _, _)| {
                    enzui_core::localization::Locale::from_posix(locale).is_ok_and(|l| {
                        l.to_string().split('-').next() == p.to_string().split('-').next()
                    })
                })
            })
            .unwrap_or(0);
        let zone = std::fs::read_link("/etc/localtime")
            .ok()
            .and_then(|p| {
                p.to_str()
                    .and_then(|s| s.split_once("zoneinfo/"))
                    .map(|(_, z)| z.to_owned())
            })
            .filter(|z| zones.contains(z))
            .unwrap_or_else(|| {
                if zones.iter().any(|z| z == "Europe/London") {
                    "Europe/London".into()
                } else {
                    "UTC".into()
                }
            });
        let localizer = i18n::for_locale(system::LANGUAGES[language].1);
        let localized_labels = i18n::labels(&localizer);
        let disks = if demo {
            system::disks(true)
        } else {
            Vec::new()
        };
        let disk = (disks.len() == 1 && disks[0].large_enough()).then_some(0);
        (
            Self {
                demo,
                dark: themes.theme().mode() == enzui_theme::EnzThemeMode::Dark,
                themes,
                exit_menu: false,
                zone_countries: system::zone_countries(),
                step: Step::Welcome,
                reached: Step::Welcome,
                chrome: ShellWindowState::new(),
                language,
                localizer,
                localized_labels,
                keyboard: 0,
                keyboard_search: String::new(),
                keyboard_test: String::new(),
                keyboard_touched: false,
                zone_touched: false,
                zones,
                zone,
                zone_search: String::new(),
                network: network::State::default(),
                network_busy: false,
                network_generation: 0,
                network_error: String::new(),
                network_selection: None,
                network_password: Secret::default(),
                network_revealed: false,
                disks,
                disks_busy: !demo,
                discovery_generation: 0,
                discovery_selection: None,
                pending_install: None,
                cleanup_incomplete: false,
                disk,
                partition_choices: Vec::new(),
                partition_choice: None,
                partition_status: String::new(),
                disk_mode: None,
                inspecting: false,
                loading_started: std::time::Instant::now(),
                loading_fraction: 0.0,
                stage_started: std::time::Instant::now(),
                install_fraction: 0.0,
                encrypt: true,
                check_media: true,
                passphrase: Secret::default(),
                passphrase_confirm: Secret::default(),
                full_name: String::new(),
                user: String::new(),
                user_edited: false,
                host: String::new(),
                host_edited: false,
                password: Secret::default(),
                confirm: Secret::default(),
                revealed: Vec::new(),
                stage: 0,
                lines: Vec::new(),
                outcome: Outcome::Running,
                show_log: false,
                full_log: String::new(),
                log_transfer: None,
                dev_access: dev_access_line(),
            },
            Task::batch([
                iced::advanced::widget::operate(
                    iced::advanced::widget::operation::focusable::focus(
                        iced::advanced::widget::Id::from(format!("installer-language-{language}")),
                    ),
                ),
                if demo {
                    Task::none()
                } else {
                    Self::scan_disks(0, false)
                },
            ]),
        )
    }

    fn theme(&self) -> iced::Theme {
        self.theme_tokens().iced_theme()
    }

    fn title(&self) -> String {
        self.tr("Install enzimOS").to_owned()
    }

    pub(crate) fn locale(&self) -> &'static str {
        system::LANGUAGES[self.language].1
    }

    pub(crate) fn tr(&self, key: &'static str) -> &str {
        self.localized_labels.get(key).map_or(key, String::as_str)
    }

    pub(crate) fn trf(&self, key: &'static str, args: &[&str]) -> String {
        {
            let names: Vec<String> = (0..args.len()).map(|i| format!("arg{i}")).collect();
            let values: Vec<(&str, &dyn enzui_core::localization::MessageArgument)> = names
                .iter()
                .zip(args)
                .map(|(n, v)| {
                    (
                        n.as_str(),
                        v as &dyn enzui_core::localization::MessageArgument,
                    )
                })
                .collect();
            i18n::format(&self.localizer, key, &values)
        }
    }

    pub(crate) fn disk_title(&self, disk: &system::Disk) -> String {
        match disk.model() {
            Some(model) => model.to_owned(),
            None if disk.virtual_disk() => {
                self.trf("{0} Virtual Disk", &[&system::human_size(disk.bytes)])
            }
            None => self.trf("{0} Disk", &[&system::human_size(disk.bytes)]),
        }
    }

    pub(crate) fn disk_detail(&self, disk: &system::Disk) -> String {
        let kind = if disk.kind() == "Internal" {
            self.tr("Internal")
        } else {
            disk.kind()
        };
        format!("{kind} · {}", disk.path)
    }

    pub(crate) fn zone_parts(&self, zone: &str) -> (String, String) {
        let (city, region) = system::zone_parts(zone);
        (
            city,
            region.unwrap_or_else(|| self.tr("Coordinated Universal Time").to_owned()),
        )
    }

    /// Why the current step cannot continue yet (None: it can).
    pub(crate) fn blocker(&self) -> Option<&'static str> {
        if self.step == Step::Review {
            return self.blocker_before(Step::Review);
        }
        self.step_blocker(self.step)
    }

    fn blocker_before(&self, target: Step) -> Option<&'static str> {
        Step::LISTED
            .into_iter()
            .take_while(|step| *step < target)
            .find_map(|step| self.step_blocker(step))
    }

    fn step_blocker(&self, step: Step) -> Option<&'static str> {
        match step {
            Step::Network if self.network_busy => Some("Please wait for the connection check."),
            Step::Disk => match self.disk.map(|i| &self.disks[i]) {
                None => Some("Choose the disk to install on."),
                Some(d) if !d.large_enough() => {
                    Some("That disk is too small; enzimOS needs at least 24 GiB.")
                }
                _ if self.disk_mode.is_none() => Some("Choose how to use this disk."),
                _ if self.disk_mode == Some(true) && self.partition_choice.is_none() => {
                    Some("Choose space for enzimOS, or use another disk.")
                }
                _ => None,
            },
            Step::Encryption => match () {
                () if self.encrypt && self.passphrase.is_empty() => {
                    Some("Choose an encryption passphrase.")
                }
                () if self.encrypt && self.passphrase != self.passphrase_confirm => {
                    Some("The passphrases do not match.")
                }
                () if self.encrypt
                    && (self.passphrase.len() > 1000
                        || self.passphrase.chars().any(char::is_control)) =>
                {
                    Some("The passphrase is too long or contains unsupported characters.")
                }
                () => None,
            },
            Step::Account => {
                if self.full_name.trim().is_empty() {
                    Some("Enter your name.")
                } else if self.full_name.contains(':')
                    || self.full_name.len() > 256
                    || self.full_name.chars().any(char::is_control)
                {
                    Some("Your name cannot contain ':' or unsupported characters.")
                } else if !system::valid_user(&self.user) {
                    Some("Choose a user name: lowercase letters, digits, - or _.")
                } else if !system::valid_host(&self.host) {
                    Some("Choose a computer name: letters, digits and -.")
                } else if self.password.is_empty() {
                    Some("Choose a password.")
                } else if self.password != self.confirm {
                    Some("The passwords do not match.")
                } else if self.password.len() >= 512 || self.password.chars().any(char::is_control)
                {
                    Some("The password is too long or contains unsupported characters.")
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn plan(&self) -> system::Plan {
        system::Plan {
            disk: self
                .disk
                .map(|i| self.disks[i].path.clone())
                .unwrap_or_default(),
            partition_plan: self
                .partition_choice
                .map(|i| self.partition_choices[i].id.clone())
                .unwrap_or_default(),
            encrypt: self.encrypt,
            passphrase: self.passphrase.clone(),
            hostname: self.host.clone(),
            user: self.user.clone(),
            full_name: self.full_name.trim().to_owned(),
            password: self.password.clone(),
            timezone: self.zone.clone(),
            locale: system::LANGUAGES[self.language].1.to_owned(),
            keymap: system::KEYBOARDS[self.keyboard].1.to_owned(),
            xkb: system::KEYBOARDS[self.keyboard].2.to_owned(),
            skip_media_check: !self.check_media,
        }
    }

    pub(crate) fn installing(&self) -> bool {
        matches!(self.step, Step::Installing | Step::Finished)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        self.localizer
            .clone()
            .scope(|| self.localized_update(message))
    }

    fn localized_update(&mut self, message: Message) -> Task<Message> {
        if self.pending_install.is_some()
            && matches!(
                &message,
                Message::Go(_)
                    | Message::Next
                    | Message::Back
                    | Message::Disk(_)
                    | Message::Edit(_, _)
                    | Message::Encrypt(_)
                    | Message::CheckMedia(_)
            )
        {
            self.pending_install = None;
            self.discovery_generation += 1;
            self.disks_busy = false;
        }
        match message {
            Message::Key(key, shift, captured) => {
                return keyboard_nav::key(self, key, shift, captured);
            }
            Message::TabFocused(ids, focus, backwards) => {
                return keyboard_nav::tab_focused(ids, focus, backwards);
            }
            Message::KeyFocused(key, captured, focus) => {
                return keyboard_nav::focused(self, key, captured, focus);
            }
            Message::Go(step) => {
                if !self.installing()
                    && step <= self.reached
                    && step < Step::Installing
                    && (step <= self.step || self.blocker_before(step).is_none())
                {
                    self.step = step;
                    return keyboard_nav::page_focus(self);
                }
            }
            Message::Next => {
                if self.blocker().is_none() && self.step < Step::Review {
                    self.step = self.step.next();
                    self.reached = self.reached.max(self.step);
                    if self.step == Step::Network {
                        return Task::batch([
                            self.update(Message::NetworkRefresh),
                            keyboard_nav::page_focus(self),
                        ]);
                    }
                    return keyboard_nav::page_focus(self);
                }
            }
            Message::Back => {
                self.step = self.step.back();
                return keyboard_nav::page_focus(self);
            }
            Message::Language(i) => {
                if i >= system::LANGUAGES.len() {
                    return Task::none();
                }
                self.language = i;
                self.localizer.select(vec![
                    enzui_core::localization::Locale::from_posix(self.locale())
                        .expect("valid supported language"),
                ]);
                self.localized_labels = i18n::labels(&self.localizer);
                // Suggest the language's keyboard and time zone until the user picks their own.
                let (_, _, xkb, zone) = system::LANGUAGES[i];
                if !self.keyboard_touched
                    && let Some(k) = system::KEYBOARDS.iter().position(|k| k.2 == xkb)
                {
                    self.keyboard = k;
                    if !self.demo {
                        system::apply_keyboard(system::KEYBOARDS[k].2);
                    }
                }
                if !self.zone_touched && self.zones.iter().any(|z| z == zone) {
                    self.zone = zone.to_owned();
                }
            }
            Message::Keyboard(i) => {
                self.keyboard = i;
                self.keyboard_touched = true;
                if !self.demo {
                    system::apply_keyboard(system::KEYBOARDS[i].2);
                }
            }
            Message::Zone(zone) => {
                self.zone = zone;
                self.zone_touched = true;
            }
            Message::Disk(i) => {
                if self.disks_busy || i >= self.disks.len() {
                    return Task::none();
                }
                self.disk = Some(i);
                self.partition_choice = None;
                self.partition_choices.clear();
                self.partition_status.clear();
                self.disk_mode = None;
                self.inspecting = false;
            }
            Message::DiskMode(alongside) => {
                self.disk_mode = Some(alongside);
                self.partition_choice = if alongside {
                    self.partition_choices.iter().position(|p| !p.shrink)
                } else {
                    None
                };
                if alongside && self.partition_choices.is_empty() && !self.inspecting {
                    return self.update(Message::InspectDisk);
                }
            }
            Message::Partition(choice) => {
                if self.disk_mode == Some(true) {
                    self.partition_choice = choice.filter(|i| *i < self.partition_choices.len());
                }
            }
            Message::InspectDisk => {
                if self.inspecting || self.disks_busy {
                    return Task::none();
                }
                if let Some(i) = self.disk {
                    self.partition_choice = None;
                    self.partition_choices.clear();
                    self.partition_status = "Looking for available space…".into();
                    self.inspecting = true;
                    self.loading_started = std::time::Instant::now();
                    self.loading_fraction = 0.0;
                    return Task::perform(
                        system::partition_choices(self.disks[i].path.clone(), self.demo),
                        Message::PartitionChoices,
                    );
                }
            }
            Message::PartitionChoices((disk, result)) => {
                if self.disk.is_some_and(|i| self.disks[i].path == disk) {
                    self.inspecting = false;
                    match result {
                        Ok(choices) => {
                            self.partition_status = if choices.is_empty() {
                                "No suitable space was found. Free space in the other OS first; encrypted and hibernated volumes cannot be resized here.".into()
                            } else {
                                String::new()
                            };
                            self.partition_choice = if self.disk_mode == Some(true) {
                                choices.iter().position(|p| !p.shrink)
                            } else {
                                None
                            };
                            self.partition_choices = choices;
                        }
                        Err(error) => self.partition_status = error,
                    }
                }
            }
            Message::RefreshDisks => {
                if self.disks_busy {
                    return Task::none();
                }
                self.partition_choice = None;
                self.partition_choices.clear();
                self.partition_status.clear();
                self.disk_mode = None;
                self.inspecting = false;
                self.pending_install = None;
                self.discovery_selection = self.disk.and_then(|i| self.disks.get(i)).cloned();
                self.discovery_generation += 1;
                if self.demo {
                    self.disks = system::disks(true);
                    self.disk = None;
                } else {
                    self.disks_busy = true;
                    self.disks.clear();
                    self.disk = None;
                    return Self::scan_disks(self.discovery_generation, false);
                }
            }
            Message::DisksDiscovered(generation, disks) => {
                if generation == self.discovery_generation {
                    self.disks_busy = false;
                    self.disk = if let Some(selected) = self.discovery_selection.take() {
                        disks.iter().position(|disk| disk.same_device(&selected))
                    } else {
                        (generation == 0 && disks.len() == 1 && disks[0].large_enough())
                            .then_some(0)
                    };
                    self.disks = disks;
                }
            }
            Message::InstallDisksChecked(generation, current) => {
                if generation != self.discovery_generation {
                    return Task::none();
                }
                self.disks_busy = false;
                let Some(plan) = self.pending_install.take() else {
                    return Task::none();
                };
                if self.step != Step::Review || self.blocker().is_some() || self.plan() != plan {
                    return Task::none();
                }
                let selected = self.disk.and_then(|i| self.disks.get(i));
                if !selected.is_some_and(|s| current.iter().any(|d| d.same_device(s))) {
                    self.disks = current;
                    self.disk = None;
                    self.disk_mode = None;
                    self.partition_choice = None;
                    self.partition_choices.clear();
                    self.step = Step::Disk;
                } else {
                    return self.begin_install();
                }
            }
            Message::Encrypt(on) => self.encrypt = on,
            Message::CheckMedia(on) => self.check_media = on,
            Message::Edit(field, value) => match field {
                Field::FullName => {
                    self.full_name = value;
                    if !self.user_edited {
                        self.user = system::derive_user(&self.full_name);
                    }
                    if !self.host_edited {
                        self.host = if self.user.is_empty() {
                            String::new()
                        } else {
                            format!("{}-pc", self.user.trim_start_matches('_'))
                        };
                    }
                }
                Field::User => {
                    self.user = value.to_lowercase();
                    self.user_edited = true;
                }
                Field::Host => {
                    self.host = value;
                    self.host_edited = true;
                }
                Field::Password => self.password = value.into(),
                Field::Confirm => self.confirm = value.into(),
                Field::Passphrase => self.passphrase = value.into(),
                Field::PassphraseConfirm => self.passphrase_confirm = value.into(),
                Field::ZoneSearch => self.zone_search = value,
                Field::KeyboardSearch => self.keyboard_search = value,
                Field::KeyboardTest => self.keyboard_test = value,
            },
            Message::Reveal(field) => {
                let key = match field {
                    Field::Passphrase | Field::PassphraseConfirm => "passphrase",
                    _ => "password",
                };
                if let Some(at) = self.revealed.iter().position(|k| *k == key) {
                    self.revealed.remove(at);
                } else {
                    self.revealed.push(key);
                }
            }
            Message::Install => {
                if self.step != Step::Review || self.disks_busy || self.blocker().is_some() {
                    return Task::none();
                }
                if self.demo {
                    return self.begin_install();
                }
                self.discovery_generation += 1;
                self.disks_busy = true;
                self.pending_install = Some(self.plan());
                return Self::scan_disks(self.discovery_generation, true);
            }
            Message::Progress(progress) => match progress {
                system::Progress::Stage(stage) => {
                    if stage > self.stage {
                        self.stage = stage;
                        self.stage_started = std::time::Instant::now();
                    }
                }
                system::Progress::Line(line) => return self.push_line(line),
                system::Progress::Done => self.finish_install(false),
                system::Progress::CleanupIncomplete => self.finish_install(true),
                system::Progress::Failed(error) => {
                    self.clear_secrets();
                    self.outcome = Outcome::Failed(error);
                    self.show_log = true;
                }
            },
            Message::Theme(dark) => {
                self.dark = dark;
                self.themes.set_mode_override(Some(if dark {
                    enzui_theme::EnzThemeMode::Dark
                } else {
                    enzui_theme::EnzThemeMode::Light
                }));
            }
            Message::SharedTheme(event) => {
                let task = self.themes.update(*event);
                self.dark = self.themes.theme().mode() == enzui_theme::EnzThemeMode::Dark;
                return task.map(|event| Message::SharedTheme(Box::new(event)));
            }
            Message::ExitMenu | Message::Quit => {
                self.exit_menu = true;
                return iced::advanced::widget::operate(
                    enzui_widgets::modal_host::initial_focus_operation(
                        enzui_widgets::modal_host::presentation_scope_id("installer-exit"),
                    ),
                );
            }
            Message::DismissExit => self.exit_menu = false,
            Message::Modal(event) => {
                if matches!(
                    event,
                    enzui_widgets::modal_host::ModalHostEvent::Dismissed(_)
                ) {
                    self.exit_menu = false;
                    self.log_transfer = None;
                }
                if let Some(op) = enzui_widgets::modal_host::scoped_focus_operation(
                    event,
                    enzui_widgets::modal_host::presentation_scope_id("installer-exit"),
                ) {
                    return iced::advanced::widget::operate(op);
                }
            }
            Message::Reset => return self.reset(),
            Message::PowerOff => {
                if !(self.step == Step::Installing && self.outcome == Outcome::Running) {
                    system::poweroff(self.demo);
                    if self.demo {
                        return iced::exit();
                    }
                }
            }
            Message::ShowQr => {
                self.log_transfer = Some(log_transfer::Transfer::new(
                    &self.full_log,
                    &[&self.password, &self.passphrase],
                ));
            }
            Message::CloseQr => self.log_transfer = None,
            Message::LoadingTick => {
                self.loading_fraction =
                    (1.0 - (-self.loading_started.elapsed().as_secs_f32() / 3.0).exp()).min(0.95);
                if self.step == Step::Installing && matches!(self.outcome, Outcome::Running) {
                    let weights = system::STAGE_SECONDS;
                    let stage = self.stage.min(system::STAGES.len() - 1);
                    let before: f32 = weights[..stage].iter().sum();
                    let total: f32 = weights.iter().sum();
                    let within = (1.0
                        - (-self.stage_started.elapsed().as_secs_f32() / weights[stage]).exp())
                    .min(0.95);
                    self.install_fraction = self
                        .install_fraction
                        .max((before + within * weights[stage]) / total)
                        .min(0.99);
                }
            }
            Message::DevAccessTick => self.dev_access = dev_access_line(),
            Message::QrTick => {
                if let Some(transfer) = &mut self.log_transfer {
                    transfer.refresh(&self.full_log, &[&self.password, &self.passphrase]);
                }
            }
            Message::NetworkRefresh => {
                if !self.network_busy {
                    return self.network_request(serde_json::json!({"action":"status"}));
                }
            }
            Message::NetworkResult(generation, result) => {
                if generation != self.network_generation {
                    return Task::none();
                }
                self.network_busy = false;
                self.network_password.clear();
                match result {
                    Ok(state) => {
                        self.network = state;
                        self.network_selection = None;
                        self.network_error.clear();
                    }
                    Err(e) => self.network_error = e,
                }
            }
            Message::NetworkSelect(i) => {
                if !self.network_busy {
                    self.network_selection = Some(i);
                    self.network_password.clear();
                    self.network_error.clear();
                    // Typing goes straight into the password that opened under the network.
                    return iced::advanced::widget::operate(
                        iced::advanced::widget::operation::focusable::focus(
                            iced::advanced::widget::Id::new("installer-network-password"),
                        ),
                    );
                }
            }
            Message::NetworkReveal => self.network_revealed = !self.network_revealed,
            Message::DismissNetworkError => self.network_error.clear(),
            Message::NetworkPassword(value) => self.network_password = value.into(),
            Message::NetworkWired(device) => {
                if !self.network_busy {
                    return self
                        .network_request(serde_json::json!({"action":"wired", "device":device}));
                }
            }
            Message::NetworkRadio(on) => {
                if !self.network_busy {
                    return self.network_request(serde_json::json!({"action":"radio", "on":on}));
                }
            }
            Message::NetworkConnect => {
                if !self.network_busy
                    && let Some(ap) = self
                        .network_selection
                        .and_then(|i| self.network.wifi.get(i))
                {
                    let request = serde_json::json!({"action":"wifi", "bssid":ap.bssid, "ssid":ap.ssid, "device":ap.device, "password":self.network_password.as_str()});
                    return self.network_request(request);
                }
            }
            Message::ToggleLog => {
                self.show_log = !self.show_log;
                if self.show_log {
                    return iced::widget::operation::snap_to_end(
                        "install-terminal",
                        iced::widget::operation::Animation::Instant,
                    );
                }
            }
            Message::Restart => {
                if self.step == Step::Installing && self.outcome == Outcome::Running {
                    return Task::none();
                }
                system::restart(self.demo);
                if self.demo {
                    return iced::exit();
                }
            }
            Message::Window(command) => {
                // The window cannot be closed mid-installation.
                if matches!(command, WindowCommand::Close) {
                    return self.update(Message::ExitMenu);
                }
                return self.chrome.run_command(command);
            }
            Message::WindowState(event) => {
                return self.chrome.update(event).map(Message::WindowState);
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let window = iced::event::listen_with(|event, status, id| match event {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Named(key),
                modifiers,
                repeat,
                ..
            }) if !modifiers.control()
                && !modifiers.alt()
                && !modifiers.logo()
                && matches!(
                    key,
                    iced::keyboard::key::Named::Tab
                        | iced::keyboard::key::Named::Enter
                        | iced::keyboard::key::Named::ArrowUp
                        | iced::keyboard::key::Named::ArrowDown
                        | iced::keyboard::key::Named::ArrowLeft
                        | iced::keyboard::key::Named::ArrowRight
                )
                && !(repeat && key == iced::keyboard::key::Named::Enter) =>
            {
                Some(Message::Key(
                    key,
                    modifiers.shift(),
                    status == iced::event::Status::Captured,
                ))
            }
            iced::Event::Window(iced::window::Event::CloseRequested) => {
                Some(Message::Window(WindowCommand::Close))
            }
            iced::Event::Window(event) => {
                ShellWindowEvent::from_window_event(&event, id).map(Message::WindowState)
            }
            _ => None,
        });
        let mut subscriptions = vec![
            window,
            self.themes
                .subscription()
                .map(|event| Message::SharedTheme(Box::new(event))),
        ];
        if self.dev_access.is_some() {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_secs(5))
                    .map(|_| Message::DevAccessTick),
            );
        }
        if self.log_transfer.is_some() {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::QrTick),
            );
        }
        if self.inspecting
            || self.network_busy
            || (self.step == Step::Installing && matches!(self.outcome, Outcome::Running))
        {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_millis(100))
                    .map(|_| Message::LoadingTick),
            );
        }
        Subscription::batch(subscriptions)
    }
}

fn main() -> iced::Result {
    let demo = std::env::args().any(|a| a == "--demo");
    if !demo && !std::path::Path::new(system::UI_INSTALL_HELPER).is_file() {
        eprintln!("enzimos-installer: run it from the install medium, or use --demo");
        std::process::exit(2);
    }
    let identity = DesktopApplicationIdentity::new("org.enzimos.Installer")
        .expect("BUG: static application id");
    let mut settings = identity.window_settings(LAYOUT);
    settings.size = iced::Size::new(1080.0, 740.0);
    // Small: a compositor honours the minimum even past the screen edge (old 1366x768 panels at 2x).
    settings.min_size = Some(iced::Size::new(480.0, 320.0));
    enzui_widgets::app_setup::run_with_traversal(
        iced::application(move || Installer::new(demo), Installer::update, view::view)
            .title(Installer::title)
            .theme(Installer::theme)
            .subscription(Installer::subscription)
            .window(settings)
            .exit_on_close_request(enzui_core::close_guard::EXIT_ON_CLOSE_REQUEST),
        enzui_widgets::app_setup::TraversalBinding::Disabled,
    )
}

#[cfg(test)]
mod flow_tests;

#[cfg(test)]
mod polish_tests;

#[cfg(test)]
mod disk_choice_tests;
