// SPDX-License-Identifier: MIT
//! Disk discovery, regional choices, and installer execution.
//!
//! Sends the install answers over stdin and follows the live helper's progress.
//! Demo mode simulates the same screens without disk writes.

use crate::secret::Secret;
use std::path::PathBuf;
use std::time::Duration;
use zeroize::Zeroizing;

use iced::futures::{SinkExt, Stream};

/// Smallest disk the backend accepts.
pub(crate) const MIN_DISK_BYTES: u64 = 24 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Disk {
    pub(crate) path: String,
    pub(crate) model: String,
    pub(crate) bytes: u64,
    pub(crate) transport: String,
    pub(crate) removable: bool,
    /// Device serial (or WWN) when the hardware reports one; part of the identity
    /// re-checked before erasing so a hot-plugged device that took over the same
    /// /dev name is never erased in place of the reviewed one.
    pub(crate) serial: String,
}

impl Disk {
    pub(crate) fn same_device(&self, other: &Disk) -> bool {
        self.path == other.path
            && self.bytes == other.bytes
            && self.model == other.model
            && self.serial == other.serial
    }
    /// The model, or None (the UI then names it by size).
    pub(crate) fn model(&self) -> Option<&str> {
        (!self.model.is_empty()).then_some(self.model.as_str())
    }

    pub(crate) fn virtual_disk(&self) -> bool {
        matches!(self.transport.as_str(), "virtio" | "")
    }

    /// Connection kind; "Internal" is translated by the UI.
    pub(crate) fn kind(&self) -> &str {
        match self.transport.as_str() {
            "nvme" => "NVMe SSD",
            "sata" | "ata" => "SATA",
            "usb" => "USB",
            "virtio" | "" => "Internal",
            other => other,
        }
    }

    pub(crate) fn large_enough(&self) -> bool {
        self.bytes >= MIN_DISK_BYTES
    }
}

/// Decimal size, as disk vendors label them ("512 GB").
pub(crate) fn human_size(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < units.len() {
        value /= 1000.0;
        unit += 1;
    }
    if value >= 100.0 || unit == 0 {
        format!(
            "{} {}",
            crate::i18n::decimal(&format!("{value:.0}")),
            units[unit]
        )
    } else {
        format!(
            "{} {}",
            crate::i18n::decimal(&format!("{value:.1}")),
            units[unit]
        )
    }
}

/// Whole, writable disks, excluding the install medium (label ENZIMOS_INSTALL).
pub(crate) fn disks(demo: bool) -> Vec<Disk> {
    let found = if demo {
        Vec::new()
    } else {
        discover().unwrap_or_default()
    };
    if found.is_empty() && demo {
        return vec![
            Disk {
                path: "/dev/nvme0n1".into(),
                model: "Samsung SSD 980 PRO".into(),
                bytes: 1_000_204_886_016,
                transport: "nvme".into(),
                removable: false,
                serial: String::new(),
            },
            Disk {
                path: "/dev/sda".into(),
                model: "WDC WD20EZAZ".into(),
                bytes: 2_000_398_934_016,
                transport: "sata".into(),
                removable: false,
                serial: String::new(),
            },
        ];
    }
    found
}

fn discover() -> Option<Vec<Disk>> {
    let medium = crate::probe::run(crate::probe::request(
        "/usr/bin/blkid",
        &["-L", "ENZIMOS_INSTALL"],
    ))
    .ok();
    let medium_disk = medium.filter(|out| out.status.success()).and_then(|out| {
        let part = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        let parent = crate::probe::run(crate::probe::request(
            "/usr/bin/lsblk",
            &["-dno", "PKNAME", &part],
        ))
        .ok()?;
        let parent = String::from_utf8_lossy(&parent.stdout).trim().to_owned();
        Some(if parent.is_empty() {
            part
        } else {
            format!("/dev/{parent}")
        })
    });
    let out = crate::probe::run(crate::probe::request(
        "/usr/bin/lsblk",
        &[
            "-J",
            "-b",
            "-d",
            "-o",
            "PATH,SIZE,MODEL,TRAN,RM,RO,TYPE,SERIAL,WWN",
        ],
    ))
    .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let rows = json.get("blockdevices")?.as_array()?;
    let flag = |v: &serde_json::Value| {
        v.as_bool()
            .unwrap_or_else(|| v.as_str() == Some("1") || v.as_u64() == Some(1))
    };
    Some(
        rows.iter()
            .filter(|row| row["type"].as_str() == Some("disk") && !flag(&row["ro"]))
            .filter_map(|row| {
                let path = row["path"].as_str()?.to_owned();
                if medium_disk.as_deref() == Some(path.as_str())
                    || path.starts_with("/dev/zram")
                    || path.starts_with("/dev/loop")
                {
                    return None;
                }
                Some(Disk {
                    bytes: row["size"]
                        .as_u64()
                        .or_else(|| row["size"].as_str()?.parse().ok())?,
                    model: row["model"].as_str().unwrap_or("").trim().to_owned(),
                    transport: row["tran"].as_str().unwrap_or("").to_owned(),
                    removable: flag(&row["rm"]),
                    serial: row["serial"]
                        .as_str()
                        .or_else(|| row["wwn"].as_str())
                        .unwrap_or("")
                        .trim()
                        .to_owned(),
                    path,
                })
            })
            .collect(),
    )
}

/// (label, locale, suggested keyboard layout (xkb), suggested time zone) on the welcome page.
pub(crate) const LANGUAGES: &[(&str, &str, &str, &str)] = &[
    (
        "English (United States)",
        "en_US.UTF-8",
        "us",
        "America/New_York",
    ),
    (
        "English (United Kingdom)",
        "en_GB.UTF-8",
        "gb",
        "Europe/London",
    ),
    ("Deutsch", "de_DE.UTF-8", "de", "Europe/Berlin"),
    ("Español", "es_ES.UTF-8", "es", "Europe/Madrid"),
    ("Français", "fr_FR.UTF-8", "fr", "Europe/Paris"),
    ("Italiano", "it_IT.UTF-8", "it", "Europe/Rome"),
    ("Magyar", "hu_HU.UTF-8", "hu", "Europe/Budapest"),
    ("Nederlands", "nl_NL.UTF-8", "nl", "Europe/Amsterdam"),
    ("Polski", "pl_PL.UTF-8", "pl", "Europe/Warsaw"),
    (
        "Português (Brasil)",
        "pt_BR.UTF-8",
        "br",
        "America/Sao_Paulo",
    ),
    ("Svenska", "sv_SE.UTF-8", "se", "Europe/Stockholm"),
];

/// (label, console keymap, xkb layout). The console keymap also unlocks the disk at boot, so
/// the installer switches its own keyboard to the same layout while you type passwords.
pub(crate) const KEYBOARDS: &[(&str, &str, &str)] = &[
    ("English (US)", "us", "us"),
    ("English (UK)", "uk", "gb"),
    ("Deutsch", "de", "de"),
    ("Deutsch (Schweiz)", "de_CH-latin1", "ch"),
    ("Français", "fr", "fr"),
    ("Español", "es", "es"),
    ("Italiano", "it", "it"),
    ("Magyar", "hu", "hu"),
    ("Nederlands", "nl", "nl"),
    ("Polski", "pl", "pl"),
    ("Português (Brasil)", "br-abnt2", "br"),
    ("Svenska", "sv-latin1", "se"),
    ("Norsk", "no", "no"),
    ("Dansk", "dk", "dk"),
    ("Suomi", "fi", "fi"),
    ("Čeština", "cz", "cz"),
    ("Russian", "ru", "ru"),
    ("Ukrainian", "ua", "ua"),
    ("Türkçe", "trq", "tr"),
    ("Japanese", "jp106", "jp"),
];

/// Switch the running compositor's keyboard layout (the installer's own session). Only
/// under Hyprland; other sessions (the preview on a desktop) keep their layout.
pub(crate) fn apply_keyboard(xkb: &'static str) {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        return;
    }
    std::thread::spawn(move || {
        let run = |args: &[&str]| {
            crate::probe::run(crate::probe::request("/usr/bin/hyprctl", args))
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
                .unwrap_or_default()
        };
        // User and computer names must be ASCII: a non-Latin layout alone left no
        // way to type them. Pair it with US and an Alt+Shift switch.
        let (layout, options) = if matches!(xkb, "ru" | "ua") {
            (format!("us,{xkb}"), "grp:alt_shift_toggle")
        } else {
            (xkb.to_owned(), "")
        };
        let _ = run(&["keyword", "input:kb_options", options]);
        let reply = run(&["keyword", "input:kb_layout", &layout]);
        if reply != "ok" {
            let lua = format!(
                "hl.config({{ input = {{ kb_layout = \"{layout}\", kb_options = \"{options}\" }} }})"
            );
            let second = run(&["dispatch", &lua]);
            eprintln!(
                "enzimos-installer: keyboard {xkb}: keyword -> {reply:?}, dispatch -> {second:?}"
            );
        }
    });
}

/// Canonical time zones from the system tz database, sorted; a short fallback list.
pub(crate) fn time_zones() -> Vec<String> {
    let mut zones: Vec<String> = std::fs::read_to_string("/usr/share/zoneinfo/zone.tab")
        .map(|text| {
            text.lines()
                .filter(|line| !line.starts_with('#'))
                .filter_map(|line| line.split('\t').nth(2).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    if zones.is_empty() {
        zones = [
            "America/New_York",
            "America/Los_Angeles",
            "America/Sao_Paulo",
            "Europe/London",
            "Europe/Dublin",
            "Europe/Berlin",
            "Europe/Budapest",
            "Europe/Paris",
            "Asia/Tokyo",
            "Asia/Kolkata",
            "Australia/Sydney",
        ]
        .map(str::to_owned)
        .to_vec();
    }
    zones.push("UTC".to_owned());
    zones.sort();
    zones.dedup();
    zones
}

/// Country names are search metadata; the picker continues to display cities.
pub(crate) fn parse_zone_countries(
    zones: &str,
    countries: &str,
) -> std::collections::HashMap<String, String> {
    let countries: std::collections::HashMap<_, _> = countries
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .collect();
    zones
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let fields: Vec<_> = l.split('\t').collect();
            if fields.len() < 3 {
                return None;
            }
            let names = fields[0]
                .split(',')
                .filter_map(|c| countries.get(c).copied())
                .collect::<Vec<_>>()
                .join(" · ");
            Some((fields[2].to_owned(), names))
        })
        .collect()
}

pub(crate) fn zone_countries() -> std::collections::HashMap<String, String> {
    parse_zone_countries(
        &std::fs::read_to_string("/usr/share/zoneinfo/zone.tab").unwrap_or_default(),
        &std::fs::read_to_string("/usr/share/zoneinfo/iso3166.tab").unwrap_or_default(),
    )
}

pub(crate) fn zone_matches(zone: &str, country: &str, query: &str) -> bool {
    let haystack = format!("{} {}", zone.replace('_', " "), country).to_lowercase();
    query
        .to_lowercase()
        .replace('_', " ")
        .split_whitespace()
        .all(|word| haystack.contains(word))
}

/// "Europe/Budapest" -> ("Budapest", "Europe").
pub(crate) fn zone_parts(zone: &str) -> (String, Option<String>) {
    match zone.rsplit_once('/') {
        Some((region, city)) => (
            city.replace('_', " "),
            Some(region.replace('_', " ").replace('/', " · ")),
        ),
        None => (zone.to_owned(), None),
    }
}

/// Validation mirrors the backend's own checks, so the UI refuses before the backend would.
pub(crate) fn valid_user(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c == '_')
        && name.len() <= 32
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        && !matches!(name, "root" | "greeter" | "nobody")
}

pub(crate) fn valid_host(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && name.len() <= 63
        && name.ends_with(|c: char| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// A user name derived from a full name ("Anna Kovács" -> "anna").
pub(crate) fn derive_user(full_name: &str) -> String {
    let first = full_name.split_whitespace().next().unwrap_or("");
    let mut out: String = first
        .chars()
        .filter_map(|c| {
            let c = fold(c).to_ascii_lowercase();
            (c.is_ascii_lowercase() || c.is_ascii_digit()).then_some(c)
        })
        .take(32)
        .collect();
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn fold(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'Á' | 'Ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'É' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'Í' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ő' | 'Ó' | 'Ö' | 'Ő' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'ű' | 'Ú' | 'Ü' | 'Ű' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        'ß' => 's',
        other => other,
    }
}

/// Rough password guidance (the backend enforces nothing beyond non-empty).
pub(crate) fn strength(password: &str) -> (&'static str, f32) {
    let classes = [
        password.chars().any(|c| c.is_ascii_lowercase()),
        password.chars().any(|c| c.is_ascii_uppercase()),
        password.chars().any(|c| c.is_ascii_digit()),
        password.chars().any(|c| !c.is_ascii_alphanumeric()),
    ]
    .iter()
    .filter(|x| **x)
    .count();
    let score = password.chars().count() as f32 / 16.0 + classes as f32 * 0.12;
    match score {
        s if s < 0.5 => ("Weak", 0.25),
        s if s < 0.85 => ("Fair", 0.5),
        s if s < 1.2 => ("Good", 0.75),
        _ => ("Strong", 1.0),
    }
}

/// Everything the user chose.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Plan {
    pub(crate) partition_plan: String,
    pub(crate) disk: String,
    pub(crate) encrypt: bool,
    pub(crate) passphrase: Secret,
    pub(crate) hostname: String,
    pub(crate) user: String,
    pub(crate) full_name: String,
    pub(crate) password: Secret,
    pub(crate) timezone: String,
    pub(crate) locale: String,
    pub(crate) keymap: String,
    pub(crate) xkb: String,
    /// Skip reading the whole medium before installing (pacman still checks signatures).
    pub(crate) skip_media_check: bool,
}

impl Plan {
    /// The backend's answers format: one KEY=VALUE per line, no quoting.
    pub(crate) fn answers(&self) -> Result<Zeroizing<String>, String> {
        // The same limits the backend enforces, so a refusal never appears only
        // after Install is pressed (where the only way out is starting over).
        let clean = |label: &str, value: &str| {
            if value.chars().any(char::is_control) {
                Err(format!(
                    "{label} contains a control character (for example a tab or line break)"
                ))
            } else {
                Ok(())
            }
        };
        for (label, value) in [
            ("Full name", self.full_name.as_str()),
            ("Password", self.password.as_str()),
            ("Passphrase", self.passphrase.as_str()),
        ] {
            clean(label, value)?;
        }
        if self.full_name.contains(':') || self.full_name.len() > 256 {
            return Err("Full name cannot contain ':' or exceed 256 bytes".into());
        }
        if self.password.len() >= 512 {
            return Err("Password is too long".into());
        }
        if self.encrypt && self.passphrase.len() > 1000 {
            return Err("Encryption passphrase is too long".into());
        }
        if !valid_host(&self.hostname) {
            return Err("Invalid computer name".into());
        }
        if !self.partition_plan.is_empty()
            && (self.partition_plan.len() != 64
                || !self.partition_plan.chars().all(|c| c.is_ascii_hexdigit()))
        {
            return Err("Invalid partition plan".into());
        }
        let lines = vec![
            format!(
                "ENZIMOS_PARTITION_MODE={}",
                if self.partition_plan.is_empty() {
                    "erase"
                } else {
                    "alongside"
                }
            ),
            format!("ENZIMOS_PARTITION_PLAN={}", self.partition_plan),
            format!("ENZIMOS_DISK={}", self.disk),
            "ENZIMOS_ERASE_CONFIRM=yes".to_owned(),
            format!("ENZIMOS_HOSTNAME={}", self.hostname),
            format!("ENZIMOS_USER={}", self.user),
            format!("ENZIMOS_FULLNAME={}", self.full_name),
            format!(
                "ENZIMOS_ENCRYPT={}",
                if self.encrypt { "yes" } else { "no" }
            ),
            format!(
                "ENZIMOS_VERIFY_MEDIA={}",
                if self.skip_media_check { "no" } else { "yes" }
            ),
            format!("ENZIMOS_TIMEZONE={}", self.timezone),
            format!("ENZIMOS_LOCALE={}", self.locale),
            format!("ENZIMOS_KEYMAP={}", self.keymap),
            format!("ENZIMOS_XKB_LAYOUT={}", self.xkb),
            "ENZIMOS_FINISH=reboot".to_owned(),
        ];
        let capacity = lines.iter().map(|line| line.len() + 1).sum::<usize>()
            + self.password.len()
            + self.passphrase.len()
            + 64;
        let mut body = Zeroizing::new(String::with_capacity(capacity));
        for line in lines {
            body.push_str(&line);
            body.push('\n');
        }
        body.push_str("ENZIMOS_PASSWORD=");
        body.push_str(&self.password);
        body.push('\n');
        if self.encrypt {
            body.push_str("ENZIMOS_PASSPHRASE=");
            body.push_str(&self.passphrase);
            body.push('\n');
        }
        Ok(body)
    }
}

/// The installation's visible stages, matched against the backend's `==>` lines. The
/// first two come from the installer itself; the rest are phase markers the package
/// worker prints, which the installer forwards from the worker's journal.
pub(crate) const STAGES: &[(&str, &str)] = &[
    ("Preparing the disk", "creating the enzimOS layout"),
    (
        "Installing the system",
        "installing the offline package set",
    ),
    ("Setting up the desktop", "setting up the desktop"),
    ("Installing apps", "installing apps"),
    ("Finishing up", "installing the boot loader"),
];

/// Expected seconds per visible stage: paces the progress bar within a stage.
pub(crate) const STAGE_SECONDS: [f32; 5] = [30.0, 120.0, 70.0, 60.0, 60.0];

#[derive(Debug, Clone)]
pub(crate) enum Progress {
    Stage(usize),
    Line(String),
    Done,
    CleanupIncomplete,
    Failed(String),
}

pub(crate) fn stage_of(line: &str) -> Option<usize> {
    let step = line.strip_prefix("==> ")?;
    STAGES
        .iter()
        .position(|(_, needle)| !needle.is_empty() && step.contains(needle))
}

/// Runs the installation (or its simulation) and reports progress.
pub(crate) fn install(plan: Plan, demo: bool) -> impl Stream<Item = Progress> {
    iced::stream::channel(
        64,
        async move |mut out: iced::futures::channel::mpsc::Sender<Progress>| {
            let result = if demo {
                simulate(&mut out).await.map(|()| false)
            } else {
                run_backend(&plan, &mut out).await
            };
            let _ = out
                .send(match result {
                    Ok(false) => Progress::Done,
                    Ok(true) => Progress::CleanupIncomplete,
                    Err(error) => Progress::Failed(error),
                })
                .await;
        },
    )
}

async fn simulate(out: &mut iced::futures::channel::mpsc::Sender<Progress>) -> Result<(), String> {
    let detail: &[&[&str]] = &[
        &[
            "wipefs: all signatures removed",
            "GPT: BIOS boot, 1 GiB EFI system, root",
            "LUKS2: formatting the root partition",
            "Btrfs: creating @, @home, @snapshots, @swap",
        ],
        &[
            "verifying 455 signed packages",
            "installing linux, linux-lts, systemd, mesa…",
            "installing NetworkManager, PipeWire, BlueZ, fwupd…",
            "running package hooks",
        ],
        &[
            "importing the enzimOS signing keys",
            "three-master Arch keyring quorum: verified",
        ],
        &[
            "locale, keyboard and time zone",
            "firewall zones, NetworkManager, chrony (NTS)",
        ],
        &["administrator account created; root login locked"],
        &["system and home history configured"],
        &[
            "mkinitcpio: regular and LTS kernels",
            "rEFInd (UEFI) and Limine (BIOS)",
        ],
        &["greetd with the enzUI greeter", "enzUI shell on Hyprland"],
        &["Flathub: installing default apps offline"],
        &[
            "scheduling the recovery image for the first boot",
            "syncing disks",
        ],
    ];
    for (index, lines) in detail.iter().enumerate() {
        let _ = out
            .send(Progress::Stage([0, 1, 1, 2, 2, 2, 2, 2, 3, 4][index]))
            .await;
        for line in *lines {
            tokio::time::sleep(Duration::from_millis(1400)).await;
            let _ = out.send(Progress::Line((*line).to_owned())).await;
        }
        tokio::time::sleep(Duration::from_millis(900)).await;
    }
    Ok(())
}

/// The live medium's root helper (the UI's user may run exactly this through sudo).
pub(crate) const UI_INSTALL_HELPER: &str = "/usr/lib/enzimos-installer/ui-install.sh";

fn is_root() -> bool {
    std::fs::metadata("/proc/self").is_ok_and(|m| std::os::unix::fs::MetadataExt::uid(&m) == 0)
}

async fn run_backend(
    plan: &Plan,
    out: &mut iced::futures::channel::mpsc::Sender<Progress>,
) -> Result<bool, String> {
    use tokio::io::AsyncWriteExt as _;
    let answers = plan.answers()?;
    let mut command = if is_root() {
        tokio::process::Command::new(UI_INSTALL_HELPER)
    } else {
        let mut sudo = tokio::process::Command::new("/usr/bin/sudo");
        sudo.args(["-n", UI_INSTALL_HELPER]);
        sudo
    };
    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot start the installer: {error}"))?;
    let mut stdin = child.stdin.take().ok_or("installer input unavailable")?;
    stdin
        .write_all(answers.as_bytes())
        .await
        .map_err(|error| format!("cannot pass the answers: {error}"))?;
    drop(stdin);
    drop(answers);
    let stdout = child.stdout.take().ok_or("installer output unavailable")?;
    let stderr = child.stderr.take().ok_or("installer output unavailable")?;
    let mut lines = crate::backend_output::Lines::new(stdout);
    let mut errors = crate::backend_output::Lines::new(stderr);
    let secrets = [plan.password.as_str(), plan.passphrase.as_str()];
    let mut cleanup_incomplete = false;
    let mut last = String::new();
    let mut last_error = String::new();
    let mut passed = false;
    // Read both streams to their end: the backend prints its failure reason on
    // stderr just before exiting, and stopping at stdout's end lost it about
    // half of the time (and spun on an already-closed stderr).
    let (mut out_open, mut err_open) = (true, true);
    let mut exit_status = None;
    let mut grace = Box::pin(tokio::time::sleep(Duration::from_secs(86400 * 365)));
    while out_open || err_open {
        tokio::select! {
            // A daemon started by a package hook may inherit the pipes and keep
            // them open; stop reading shortly after the backend itself exits.
            status = child.wait(), if exit_status.is_none() => {
                exit_status = Some(status.map_err(|e| e.to_string())?);
                grace.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(5));
            },
            () = &mut grace => break,
            line = lines.next_line(&secrets), if out_open => match line {
                Ok(Some(line)) => {
                    passed |= line.passed;
                    cleanup_incomplete |= line.cleanup_incomplete;
                    if let Some(stage) = line.stage { let _ = out.send(Progress::Stage(stage)).await; }
                    if !line.text.trim().is_empty() { last = line.text.clone(); let _ = out.send(Progress::Line(line.text)).await; }
                }
                _ => out_open = false,
            },
            line = errors.next_line(&secrets), if err_open => match line {
                Ok(Some(line)) => if !line.text.trim().is_empty() { last_error = line.text.clone(); let _ = out.send(Progress::Line(line.text)).await; },
                _ => err_open = false,
            },
        }
    }
    if !last_error.is_empty() {
        last = last_error;
    }
    let status = match exit_status {
        Some(status) => status,
        None => child.wait().await.map_err(|e| e.to_string())?,
    };
    if status.success() && (passed || cleanup_incomplete) {
        Ok(cleanup_incomplete)
    } else {
        Err(if last.is_empty() {
            format!("installer exited with {status}")
        } else {
            last
        })
    }
}

/// Restart after a successful installation (the live medium's session is root).
pub(crate) fn restart(demo: bool) {
    power(demo, "reboot");
}

pub(crate) fn poweroff(demo: bool) {
    power(demo, "poweroff");
}

fn power(demo: bool, action: &str) {
    if !demo {
        let mut command = if is_root() {
            std::process::Command::new("/usr/bin/systemctl")
        } else {
            std::process::Command::new("/usr/bin/sudo")
        };
        if !is_root() {
            command.args(["-n", "/usr/bin/systemctl"]);
        }
        let _ = command.arg(action).status();
    }
}

pub(crate) fn log_path() -> PathBuf {
    PathBuf::from("/run/enzimos-install.log")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_refuse_line_breaks() {
        let mut plan = Plan {
            disk: "/dev/vda".into(),
            hostname: "enzimos".into(),
            password: "a\nENZIMOS_DISK=/dev/sdb".into(),
            ..Plan::default()
        };
        assert!(plan.answers().is_err());
        plan.password = "secret".into();
        let text = plan.answers().unwrap();
        assert!(text.contains("ENZIMOS_ENCRYPT=no\n") && !text.contains("PASSPHRASE"));
        assert!(text.contains("ENZIMOS_VERIFY_MEDIA=yes\n"));
    }
}

#[cfg(test)]
mod country_search_tests {
    use super::*;
    #[test]
    fn country_search_retains_city_and_handles_shared_zones() {
        let index = parse_zone_countries(
            "# comment\nIE\t+5320-00615\tEurope/Dublin\nDE,DK\t+5230+01322\tEurope/Berlin\n",
            "IE\tIreland\nDE\tGermany\nDK\tDenmark\n",
        );
        assert!(zone_matches(
            "Europe/Dublin",
            &index["Europe/Dublin"],
            "Ireland"
        ));
        assert!(zone_matches(
            "Europe/Dublin",
            &index["Europe/Dublin"],
            " dublin IRELAND "
        ));
        assert!(!zone_matches(
            "Europe/Dublin",
            &index["Europe/Dublin"],
            "Germany"
        ));
        assert!(zone_matches(
            "Europe/Berlin",
            &index["Europe/Berlin"],
            "Denmark"
        ));
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PartitionChoice {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) bytes: u64,
    pub(crate) shrink: bool,
}
pub(crate) async fn partition_choices(
    disk: String,
    demo: bool,
) -> (String, Result<Vec<PartitionChoice>, String>) {
    if demo {
        return (
            disk,
            Ok(vec![PartitionChoice {
                id: "a".repeat(64),
                label: "Use unallocated space".into(),
                bytes: 64 * 1024 * 1024 * 1024,
                shrink: false,
            }]),
        );
    }
    let result = async {
        let out = crate::probe::run_async(crate::probe::request(
            "/usr/bin/sudo",
            &["-n", "/usr/lib/enzimos-installer/ui-partition.sh", &disk],
        ))
        .await?;
        let v: serde_json::Value = serde_json::from_slice(&out.stdout)
            .map_err(|_| "Cannot inspect this disk.".to_owned())?;
        if !out.status.success() {
            return Err(v["error"]
                .as_str()
                .unwrap_or("Disk inspection failed.")
                .to_owned());
        }
        if v["uefi"] != true {
            return Err(
                "Restart the installer in UEFI mode to install alongside another system.".into(),
            );
        }
        Ok(v["plans"]
            .as_array()
            .ok_or("Invalid partition response")?
            .iter()
            .filter_map(|p| {
                Some(PartitionChoice {
                    id: p["id"].as_str()?.to_owned(),
                    label: p["label"].as_str()?.to_owned(),
                    bytes: p["size"].as_u64()? * p["sector"].as_u64()?,
                    shrink: !p["shrink"].is_null(),
                })
            })
            .collect())
    }
    .await;
    (disk, result)
}
