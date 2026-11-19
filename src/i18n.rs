//! Bundled Fluent messages and the application's explicit language boundary.
use enzui_core::localization::{Arguments, Catalog, CatalogBuilder, Locale, Localizer};
use std::sync::OnceLock;

fn catalog() -> Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            let mut builder = CatalogBuilder::new();
            builder
                .add_resource("en-US", include_str!("../assets/locales/en-US.ftl"))
                .expect("valid bundled translations");
            builder
                .add_resource("de", include_str!("../assets/locales/de.ftl"))
                .expect("valid bundled translations");
            builder
                .add_resource("es", include_str!("../assets/locales/es.ftl"))
                .expect("valid bundled translations");
            builder
                .add_resource("fr", include_str!("../assets/locales/fr.ftl"))
                .expect("valid bundled translations");
            builder
                .add_resource("it", include_str!("../assets/locales/it.ftl"))
                .expect("valid bundled translations");
            builder
                .add_resource("hu", include_str!("../assets/locales/hu.ftl"))
                .expect("valid bundled translations");
            builder
                .add_resource("nl", include_str!("../assets/locales/nl.ftl"))
                .expect("valid bundled translations");
            builder
                .add_resource("pl", include_str!("../assets/locales/pl.ftl"))
                .expect("valid bundled translations");
            builder
                .add_resource("pt", include_str!("../assets/locales/pt.ftl"))
                .expect("valid bundled translations");
            builder
                .add_resource("sv", include_str!("../assets/locales/sv.ftl"))
                .expect("valid bundled translations");
            builder.build()
        })
        .clone()
}

pub(crate) fn for_locale(locale: &str) -> Localizer {
    let preferred = Locale::from_posix(locale).ok().into_iter().collect();
    Localizer::new(catalog(), preferred, Locale::default())
}

pub(crate) fn translate(localizer: &Localizer, source: &str) -> String {
    match source_key(source) {
        Some(key) => localizer.text(key),
        None => source.to_owned(),
    }
}

pub(crate) fn format(
    localizer: &Localizer,
    source: &str,
    values: &[(&str, &dyn enzui_core::localization::MessageArgument)],
) -> String {
    let Some(key) = source_key(source) else {
        return source.to_owned();
    };
    let mut args = Arguments::new();
    for (name, value) in values {
        args.set(*name, value.message_value());
    }
    localizer.text_with(key, &args)
}

pub(crate) fn source_key(source: &str) -> Option<&'static str> {
    MESSAGES
        .iter()
        .find_map(|(en, key)| (*en == source).then_some(*key))
}

pub(crate) fn labels(localizer: &Localizer) -> std::collections::HashMap<&'static str, String> {
    MESSAGES
        .iter()
        .filter(|(source, _)| !source.contains('{'))
        .map(|(source, key)| (*source, localizer.text(key)))
        .collect()
}

pub(crate) fn decimal(digits: &str) -> String {
    digits
        .parse::<enzui_core::localization::Decimal>()
        .ok()
        .and_then(|value| {
            enzui_core::localization::current()
                .locale()
                .format_decimal(&value)
                .ok()
        })
        .unwrap_or_else(|| digits.to_owned())
}

#[rustfmt::skip]
const MESSAGES: &[(&str, &str)] = &[
    ("Sensitive installation inputs need cleanup", "app-sensitive-cleanup-title"),
    ("Installation succeeded. Restart or shut down to clear the live environment. Do not reinstall.", "app-sensitive-cleanup-body"),
    ("A few questions, then enzimOS installs itself. It takes about ten minutes and needs no internet connection.", "app-a-few-questions-then-enzimos-installs-itself-it-takes-about-ten--714e20"),
    ("About you", "app-about-you-428f4d"),
    ("Account", "app-account-7e1b0d"),
    ("Administrator · {0}", "app-administrator-0-9f1f28"),
    ("After installation", "app-after-installation-fd92ff"),
    ("All systems, partitions and files on {} ({}) will be removed. Other disks are kept.", "app-all-systems-partitions-and-files-on-will-be-removed-other-disks--f04089"),
    ("Back", "app-back-76900f"),
    ("Before you begin", "app-before-you-begin-d5bfdd"),
    ("Cancel", "app-cancel-19766e"),
    ("Cannot contact network service.", "app-cannot-contact-network-service-52f55d"),
    ("Check again", "app-check-again-fb7099"),
    ("Check the installation medium first", "app-check-the-installation-medium-first-e24ee5"),
    ("Check your choices. Nothing has been changed yet.", "app-check-your-choices-nothing-has-been-changed-yet-8440b8"),
    ("Choose a Disk", "app-choose-a-disk-ad7463"),
    ("Choose a computer name: letters, digits and -.", "app-choose-a-computer-name-letters-digits-and-6e9076"),
    ("Choose a passphrase", "app-choose-a-passphrase-0ef9a4"),
    ("Choose a password", "app-choose-a-password-efd02f"),
    ("Choose a password.", "app-choose-a-password-28fe8f"),
    ("Choose a user name: lowercase letters, digits, - or _.", "app-choose-a-user-name-lowercase-letters-digits-or-1357df"),
    ("Choose an encryption passphrase.", "app-choose-an-encryption-passphrase-4e2d7b"),
    ("Choose the disk to install on.", "app-choose-the-disk-to-install-on-18eaa6"),
    ("Close", "app-close-7d9eb7"),
    ("Computer name", "app-computer-name-3abf61"),
    ("Configuring your system", "app-configuring-your-system-4dc897"),
    ("Confirm passphrase", "app-confirm-passphrase-3cfaae"),
    ("Confirm password", "app-confirm-password-5ac265"),
    ("Connect", "app-connect-1a2303"),
    ("Connect a disk of at least 20 GB, then refresh.", "app-connect-a-disk-of-at-least-20-gb-then-refresh-2282a0"),
    ("Connect an Ethernet cable", "app-connect-an-ethernet-cable-d77426"),
    ("Connect now, or finish setup offline.", "app-connect-now-or-finish-setup-offline-2701c5"),
    ("Connected", "app-connected-229655"),
    ("Continue", "app-continue-31fbef"),
    ("Continue offline", "app-continue-offline-25fd98"),
    ("Coordinated Universal Time", "app-coordinated-universal-time-52936f"),
    ("Copy installation logs", "app-copy-installation-logs-e29fb8"),
    ("Copy logs with QR", "app-copy-logs-with-qr-acbc22"),
    ("Could not connect", "app-could-not-connect-8630b4"),
    ("Creating your account", "app-creating-your-account-25008b"),
    ("Dark appearance", "app-dark-appearance-ca3eef"),
    ("Dev SSH: no network yet", "app-dev-ssh-no-network-yet-eee852"),
    ("Development build", "app-development-build-63d499"),
    ("Disk", "app-disk-340e0c"),
    ("Disks", "app-disks-7d4865"),
    ("Does not match", "app-does-not-match-c9ba77"),
    ("Drivers, codecs and the default apps come from this medium.", "app-drivers-codecs-and-the-default-apps-come-from-this-medium-98e3ea"),
    ("Encrypt enzimOS", "app-encrypt-enzimos-098384"),
    ("Encryption", "app-encryption-82f4e7"),
    ("Enter your name.", "app-enter-your-name-a48022"),
    ("Erase Disk & Install", "app-erase-disk-install-7bae97"),
    ("Everything on the disk you choose is replaced. Other disks are not touched.", "app-everything-on-the-disk-you-choose-is-replaced-other-disks-are-no-6af1ee"),
    ("Everything on this disk will be erased", "app-everything-on-this-disk-will-be-erased-41e956"),
    ("Existing partitions and files stay in place. No resizing is needed.", "app-existing-partitions-and-files-stay-in-place-no-resizing-is-neede-3c301f"),
    ("Existing partitions kept", "app-existing-partitions-kept-7e8a4c"),
    ("Fair", "app-fair-f7b19a"),
    ("Finishing", "app-finishing-005220"),
    ("Finishing up", "app-finishing-up-13f4f8"),
    ("Get Started", "app-get-started-983f31"),
    ("Good", "app-good-c93932"),
    ("Hide Details", "app-hide-details-391596"),
    ("How this computer appears on your network", "app-how-this-computer-appears-on-your-network-285161"),
    ("Install", "app-install-569ca4"),
    ("Install Alongside", "app-install-alongside-d01ec4"),
    ("Install alongside the existing system", "app-install-alongside-the-existing-system-ef632f"),
    ("Install enzimOS", "app-install-enzimos-b40fea"),
    ("Install in available space and choose your system at startup.", "app-install-in-available-space-and-choose-your-system-at-startup-6d639b"),
    ("Installation Failed", "app-installation-failed-faa69b"),
    ("Installation options", "app-installation-options-7931d9"),
    ("Installation output", "app-installation-output-b36eb5"),
    ("Installing apps", "app-installing-apps-2c0586"),
    ("Installing enzimOS", "app-installing-enzimos-192268"),
    ("Installing the boot loader", "app-installing-the-boot-loader-c22fcc"),
    ("Installing the desktop", "app-installing-the-desktop-fc5af8"),
    ("Installing the system", "app-installing-the-system-935219"),
    ("Internal", "app-internal-2ea184"),
    ("Join network", "app-join-network-f4035c"),
    ("Keep existing systems", "app-keep-existing-systems-c57ab4"),
    ("Keyboard", "app-keyboard-231681"),
    ("Keyboard & Time", "app-keyboard-time-50f780"),
    ("Keyboard layout", "app-keyboard-layout-a6b693"),
    ("Language", "app-language-a4fe65"),
    ("Latest installation output", "app-latest-installation-output-f9ddd2"),
    ("Letters, digits and - only", "app-letters-digits-and-only-4e156f"),
    ("Light / dark appearance", "app-light-dark-appearance-dbbe71"),
    ("Lowercase letters, digits, - and _", "app-lowercase-letters-digits-and-08de52"),
    ("Network", "app-network-1744b9"),
    ("Network service is unavailable.", "app-network-service-is-unavailable-caf747"),
    ("Network service is unavailable. You can continue offline.", "app-network-service-is-unavailable-you-can-continue-offline-dae6bd"),
    ("Network service returned an invalid response.", "app-network-service-returned-an-invalid-response-8bb679"),
    ("Network service stopped unexpectedly.", "app-network-service-stopped-unexpectedly-d00331"),
    ("No disks found", "app-no-disks-found-f0a76a"),
    ("No layout matches.", "app-no-layout-matches-eb1112"),
    ("No nearby networks found. Try Refresh.", "app-no-nearby-networks-found-try-refresh-ed05ef"),
    ("No network adapter found", "app-no-network-adapter-found-8a4c16"),
    ("No output yet.", "app-no-output-yet-d2e93d"),
    ("No time zone matches.", "app-no-time-zone-matches-87caeb"),
    ("Off", "app-off-ca7981"),
    ("On", "app-on-130011"),
    ("One whole disk is used", "app-one-whole-disk-is-used-e8c686"),
    ("Open network", "app-open-network-f6572a"),
    ("Passphrase", "app-passphrase-e7611f"),
    ("Password", "app-password-e7cf3e"),
    ("Pick the keyboard layout you type with and where you are.", "app-pick-the-keyboard-layout-you-type-with-and-where-you-are-905793"),
    ("Please wait for installation to finish before restarting or shutting down.", "app-please-wait-for-installation-to-finish-before-restarting-or-shut-5626b1"),
    ("Preparing the disk", "app-preparing-the-disk-7682e6"),
    ("Preview mode", "app-preview-mode-481bc9"),
    ("Preview · no disks are changed", "app-preview-no-disks-are-changed-567a9b"),
    ("Private by default", "app-private-by-default-f52e06"),
    ("Quit Installer", "app-quit-installer-2ddf23"),
    ("Reads every package on the install drive before the disk is changed; adds a few minutes on slow USB drives. Each package is still signature-checked as it installs.", "app-reads-every-package-on-the-install-drive-before-the-disk-is-chan-07176b"),
    ("Ready to Install", "app-ready-to-install-939943"),
    ("Ready to use", "app-ready-to-use-996f87"),
    ("Reboot computer", "app-reboot-computer-62f0a8"),
    ("Reconnect", "app-reconnect-bf8a9e"),
    ("Refresh", "app-refresh-0e9161"),
    ("Region", "app-region-d3a008"),
    ("Remove the install medium, then restart and sign in as {0}.", "app-remove-the-install-medium-then-restart-and-sign-in-as-0-9f95a2"),
    ("Remove the install medium, then restart. Unlock the disk with your passphrase, then sign in as {0}.", "app-remove-the-install-medium-then-restart-unlock-the-disk-with-your-6a3edd"),
    ("Replace everything on this disk with a fresh enzimOS installation.", "app-replace-everything-on-this-disk-with-a-fresh-enzimos-installatio-96d7e7"),
    ("Requires enterprise or legacy settings", "app-requires-enterprise-or-legacy-settings-881477"),
    ("Restart Now", "app-restart-now-0fe0c9"),
    ("Restart installation", "app-restart-installation-b423b5"),
    ("Review", "app-review-aff076"),
    ("Scan to copy · updates every second", "app-scan-to-copy-updates-every-second-1d22d7"),
    ("Search cities", "app-search-cities-86800d"),
    ("Search cities or countries", "app-search-cities-or-countries-15335f"),
    ("Search layouts", "app-search-layouts-6cf461"),
    ("Setting up history snapshots", "app-setting-up-history-snapshots-57f1d1"),
    ("Setting up the desktop", "app-setting-up-the-desktop-9e6d4d"),
    ("Setting up trust and updates", "app-setting-up-trust-and-updates-de9a34"),
    ("Setup uses the packages on this installation drive. No updates are downloaded.", "app-setup-uses-the-packages-on-this-installation-drive-no-updates-ar-b1bdb3"),
    ("Show Details", "app-show-details-a525be"),
    ("Shut down", "app-shut-down-a41934"),
    ("Space could not be prepared", "app-space-could-not-be-prepared-16307c"),
    ("Start over, or turn off this computer.", "app-start-over-or-turn-off-this-computer-47d7df"),
    ("Storage", "app-storage-a69c4d"),
    ("Strength", "app-strength-63ee3a"),
    ("Strong", "app-strong-30a04f"),
    ("Test your keyboard", "app-test-your-keyboard-a7ce16"),
    ("That disk is too small; enzimOS needs at least 24 GiB.", "app-that-disk-is-too-small-enzimos-needs-at-least-24-gib-553bb3"),
    ("The administrator password also approves system changes. The root account stays locked.", "app-the-administrator-password-also-approves-system-changes-the-root-0c6706"),
    ("The disk", "app-the-disk-3b0050"),
    ("The full log is in {0}.", "app-the-full-log-is-in-0-a2116d"),
    ("The installer stopped", "app-the-installer-stopped-aa1e11"),
    ("The passphrase is too long or contains unsupported characters.", "app-the-passphrase-is-too-long-or-contains-unsupported-characters-6e4c7b"),
    ("The passphrases do not match.", "app-the-passphrases-do-not-match-4d3fc2"),
    ("The password is too long or contains unsupported characters.", "app-the-password-is-too-long-or-contains-unsupported-characters-961564"),
    ("The passwords do not match.", "app-the-passwords-do-not-match-f694aa"),
    ("There is no way to recover a forgotten passphrase.", "app-there-is-no-way-to-recover-a-forgotten-passphrase-f1e827"),
    ("This disk will be erased", "app-this-disk-will-be-erased-efa7cc"),
    ("This resizes the selected filesystem. Back up its files before continuing.", "app-this-resizes-the-selected-filesystem-back-up-its-files-before-co-66bf47"),
    ("This takes a few minutes. You can leave the computer on its own.", "app-this-takes-a-few-minutes-you-can-leave-the-computer-on-its-own-31e49a"),
    ("Time zone", "app-time-zone-b9fe14"),
    ("Type here to try the layout", "app-type-here-to-try-the-layout-817bde"),
    ("Type it again", "app-type-it-again-3b2acc"),
    ("Use the entire disk", "app-use-the-entire-disk-81b2c0"),
    ("Use {} of unallocated space", "app-use-of-unallocated-space-f31f25"),
    ("Used for the installer and the installed system", "app-used-for-the-installer-and-the-installed-system-9f3116"),
    ("User name", "app-user-name-b82e11"),
    ("Weak", "app-weak-8d6cea"),
    ("Welcome", "app-welcome-0e2226"),
    ("Welcome to enzimOS", "app-welcome-to-enzimos-5e2c9c"),
    ("When you continue, everything on this disk is permanently deleted and enzimOS is installed.", "app-when-you-continue-everything-on-this-disk-is-permanently-deleted-fe7690"),
    ("Where to install", "app-where-to-install-710728"),
    ("Wi-Fi", "app-wi-fi-337253"),
    ("Wi-Fi password", "app-wi-fi-password-0ea8c0"),
    ("Wired connection", "app-wired-connection-41c985"),
    ("Without encryption, anyone with the disk can read your files.", "app-without-encryption-anyone-with-the-disk-can-read-your-files-b4783f"),
    ("You can connect later from the desktop.", "app-you-can-connect-later-from-the-desktop-a62ed3"),
    ("You sign in with this account. It can install apps and change system settings.", "app-you-sign-in-with-this-account-it-can-install-apps-and-change-sys-34264b"),
    ("Your Account", "app-your-account-89e60b"),
    ("Your disk is encrypted and nothing is sent anywhere.", "app-your-disk-is-encrypted-and-nothing-is-sent-anywhere-f725bf"),
    ("Your files stay private if the computer is lost or stolen. You type the passphrase each time it starts.", "app-your-files-stay-private-if-the-computer-is-lost-or-stolen-you-ty-1280bd"),
    ("Your home folder is /home/{0}", "app-your-home-folder-is-home-0-b39a04"),
    ("Your name", "app-your-name-2c6b2e"),
    ("Your name cannot contain ':' or unsupported characters.", "app-your-name-cannot-contain-or-unsupported-characters-e6c6f1"),
    ("Your other disks were not changed.", "app-your-other-disks-were-not-changed-22eaa1"),
    ("enzimOS is installed", "app-enzimos-is-installed-9538cd"),
    ("enzimOS uses the whole disk. Windows and other systems on it are removed.", "app-enzimos-uses-the-whole-disk-windows-and-other-systems-on-it-are--bdcbee"),
    ("removable", "app-removable-30190a"),
    ("too small", "app-too-small-b4e1b3"),
    ("{0} ({1}). Back up anything you want to keep before you install.", "app-0-1-back-up-anything-you-want-to-keep-before-you-install-039b73"),
    ("{0} ({1}, {2}). Back up anything you want to keep before you install.", "app-0-1-2-back-up-anything-you-want-to-keep-before-you-install-94f661"),
    ("{0} Disk", "app-0-disk-5a97bb"),
    ("{0} Virtual Disk", "app-0-virtual-disk-ffe4d4"),
    ("{0} will be erased", "app-0-will-be-erased-e4acb2"),
    ("{} · {} for enzimOS", "app-for-enzimos-d99214"),
    ("{} · {}. Existing partitions stay in place; enzimOS gets its own boot and system partitions.", "app-existing-partitions-stay-in-place-enzimos-gets-its-own-boot-and--d2719b"),
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_spanish_messages_resolve_without_hidden_fallback() {
        let (mut app, _) = crate::Installer::new(true);
        app.keyboard_touched = true;
        app.zone_touched = true;
        app.user = "maria".to_owned();
        let choices = (app.keyboard, app.zone.clone(), app.disk);
        let spanish = crate::system::LANGUAGES
            .iter()
            .position(|(_, locale, _, _)| locale.starts_with("es_"))
            .unwrap();
        let _ = app.update(crate::Message::Language(spanish));
        assert_eq!(
            app.tr("Welcome to enzimOS"),
            "Te damos la bienvenida a enzimOS"
        );
        assert_eq!(app.user, "maria");
        assert_eq!((app.keyboard, app.zone.clone(), app.disk), choices);
        let _ = app.update(crate::Message::Language(usize::MAX));
        assert_eq!(app.language, spanish);
        let localizer = for_locale("es_MX.UTF-8");
        let mut arguments = Arguments::new();
        let resources = include_str!("../assets/locales/en-US.ftl");
        let mut rest = resources;
        while let Some((_, tail)) = rest.split_once('$') {
            let end = tail
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
                .unwrap_or(tail.len());
            if end > 0 {
                arguments.set(&tail[..end], 2.0);
            }
            rest = tail;
        }
        for (_, key) in MESSAGES {
            let message = localizer.resolve(key, Some(&arguments));
            assert!(message.errors.is_empty(), "{key}: {:?}", message.errors);
            assert_eq!(message.locale.as_deref(), Some("es"), "{key}");
            assert!(!message.value.is_empty(), "{key}");
        }
    }
}
