//! Development-medium remote access status.

pub(super) fn dev_access_line() -> Option<String> {
    if !std::path::Path::new("/etc/enzimos-dev-access").exists() {
        return None;
    }
    let output = std::process::Command::new("ip")
        .args(["-4", "-o", "addr", "show", "scope", "global"])
        .output()
        .ok();
    let addresses = output
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| {
                    l.split_whitespace()
                        .skip_while(|w| *w != "inet")
                        .nth(1)
                        .and_then(|a| a.split('/').next())
                        .map(str::to_owned)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(if addresses.is_empty() {
        "Dev SSH: no network yet".into()
    } else {
        format!("Dev SSH: root@{}", addresses.join(", "))
    })
}
