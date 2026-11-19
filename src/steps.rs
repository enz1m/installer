//! Installer step order and navigation.

use super::*;

impl Step {
    /// Ordered setup steps.
    pub(crate) const LISTED: [Step; 9] = [
        Step::Welcome,
        Step::Region,
        Step::Timezone,
        Step::Network,
        Step::Disk,
        Step::Encryption,
        Step::Account,
        Step::Review,
        Step::Installing,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Step::Welcome => "Language",
            Step::Region => "Keyboard",
            Step::Timezone => "Time zone",
            Step::Network => "Network",
            Step::Encryption => "Encryption",
            Step::Disk => "Disk",
            Step::Account => "Account",
            Step::Review => "Review",
            Step::Installing | Step::Finished => "Install",
        }
    }

    pub(crate) fn icon(self) -> &'static str {
        match self {
            Step::Welcome => "sparkles",
            Step::Region => "keyboard",
            Step::Timezone => "map-pin",
            Step::Network => "wifi",
            Step::Encryption => "lock",
            Step::Disk => "hard-drive",
            Step::Account => "user",
            Step::Review => "list-checks",
            Step::Installing | Step::Finished => "download",
        }
    }

    pub(super) fn next(self) -> Step {
        match self {
            Step::Welcome => Step::Region,
            Step::Region => Step::Timezone,
            Step::Timezone => Step::Network,
            Step::Network => Step::Disk,
            Step::Disk => Step::Encryption,
            Step::Encryption => Step::Account,
            Step::Account => Step::Review,
            Step::Review => Step::Installing,
            Step::Installing | Step::Finished => Step::Finished,
        }
    }

    pub(super) fn back(self) -> Step {
        match self {
            Step::Region => Step::Welcome,
            Step::Timezone => Step::Region,
            Step::Disk => Step::Network,
            Step::Network => Step::Timezone,
            Step::Encryption => Step::Disk,
            Step::Account => Step::Encryption,
            Step::Review => Step::Account,
            other => other,
        }
    }
}
