use super::*;
impl Installer {
    pub(super) fn reset(&mut self) -> Task<Message> {
        if !(self.step == Step::Installing && self.outcome == Outcome::Running)
            && !(matches!(self.outcome, Outcome::Failed(_)) && !self.demo)
        {
            let dark = self.dark;
            let chrome = self.chrome.clone();
            let placeholder = ThemeIntegration::unmanaged_compiled_theme(self.theme_tokens()).0;
            let themes = std::mem::replace(&mut self.themes, placeholder);
            let generation = self.discovery_generation + 1;
            let network_generation = self.network_generation + 1;
            let (fresh, _) = Self::with_themes(self.demo, themes);
            *self = fresh;
            self.dark = dark;
            self.chrome = chrome;
            self.discovery_generation = generation;
            self.network_generation = network_generation;
            if !self.demo {
                system::apply_keyboard(system::KEYBOARDS[self.keyboard].2);
            }
            return Task::batch([
                keyboard_nav::page_focus(self),
                if self.demo {
                    Task::none()
                } else {
                    Self::scan_disks(generation, false)
                },
            ]);
        }

        Task::none()
    }

    pub(crate) fn theme_tokens(&self) -> EnzTheme {
        self.themes.theme().clone()
    }

    pub(crate) fn tokens() -> ShellTokens {
        ShellTokens::default()
    }

    pub(super) fn scan_disks(generation: u64, installing: bool) -> Task<Message> {
        Task::perform(
            async {
                tokio::task::spawn_blocking(|| system::disks(false))
                    .await
                    .unwrap_or_default()
            },
            move |disks| {
                if installing {
                    Message::InstallDisksChecked(generation, disks)
                } else {
                    Message::DisksDiscovered(generation, disks)
                }
            },
        )
    }
    pub(super) fn clear_secrets(&mut self) {
        self.password.clear();
        self.confirm.clear();
        self.passphrase.clear();
        self.passphrase_confirm.clear();
        self.network_password.clear();
        self.revealed.clear();
    }
    pub(super) fn finish_install(&mut self, cleanup: bool) {
        self.clear_secrets();
        self.cleanup_incomplete = cleanup;
        self.stage = system::STAGES.len();
        self.outcome = Outcome::Done;
        self.step = Step::Finished;
        self.reached = Step::Finished;
    }
    pub(super) fn begin_install(&mut self) -> Task<Message> {
        self.step = Step::Installing;
        self.reached = Step::Installing;
        self.outcome = Outcome::Running;
        self.stage = 0;
        self.stage_started = std::time::Instant::now();
        self.install_fraction = 0.0;
        self.lines.clear();
        self.full_log.clear();
        self.log_transfer = None;
        let disk = self
            .disk
            .map(|i| self.disk_title(&self.disks[i]))
            .unwrap_or_default();
        let _ = self.push_line(if self.check_media {
                        format!("Installing on {disk}: checking the installation medium first, then partitions and file systems")
                    } else {
                        format!("Installing on {disk}: partitions and file systems")
                    });
        if self.encrypt {
            let _ = self.push_line("Setting up disk encryption; this can take a minute".into());
        }
        Task::run(system::install(self.plan(), self.demo), Message::Progress)
    }
    pub(super) fn push_line(&mut self, line: String) -> Task<Message> {
        let raw = zeroize::Zeroizing::new(line);
        let line = backend_output::bounded(
            backend_output::redact(
                &raw,
                &[&self.password, &self.passphrase, &self.network_password],
            ),
            false,
        );
        self.full_log.push_str(&line);
        self.full_log.push('\n');
        const MAX_LOG: usize = 2 * 1024 * 1024;
        const OMITTED: &str = "[older installation output omitted]\n";
        if self.full_log.len() > MAX_LOG {
            let mut start = self.full_log.len() - MAX_LOG + OMITTED.len();
            while !self.full_log.is_char_boundary(start) {
                start += 1;
            }
            self.full_log = format!("{OMITTED}{}", &self.full_log[start..]);
        }
        self.lines.push(line);
        if self.lines.len() > 400 {
            self.lines.drain(..100);
        }
        if self.show_log {
            return iced::widget::operation::snap_to_end(
                "install-terminal",
                iced::widget::operation::Animation::Instant,
            );
        }
        Task::none()
    }

    pub(super) fn network_request(&mut self, request: serde_json::Value) -> Task<Message> {
        if self.network_busy {
            return Task::none();
        }
        self.network_busy = true;
        self.loading_started = std::time::Instant::now();
        self.loading_fraction = 0.0;
        self.network_error.clear();
        self.network_generation += 1;
        let generation = self.network_generation;
        Task::perform(network::request(self.demo, request), move |result| {
            Message::NetworkResult(generation, result)
        })
    }
}
