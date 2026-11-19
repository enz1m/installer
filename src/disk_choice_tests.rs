use super::*;
#[test]
fn no_implicit_erase_and_late_probe_cannot_override_explicit_erase() {
    let (mut app, _) = Installer::new(true);
    app.step = Step::Disk;
    let _ = app.update(Message::Disk(0));
    assert!(app.blocker().is_some());
    let _ = app.update(Message::DiskMode(true));
    assert!(app.blocker().is_some());
    let disk = app.disks[app.disk.unwrap()].path.clone();
    let _ = app.update(Message::DiskMode(false));
    let _ = app.update(Message::PartitionChoices((
        disk,
        Ok(vec![system::PartitionChoice {
            id: "a".repeat(64),
            label: "Free space".into(),
            bytes: 32 << 30,
            shrink: false,
        }]),
    )));
    assert_eq!(app.disk_mode, Some(false));
    assert_eq!(app.partition_choice, None);
    assert!(app.blocker().is_none());
    let _ = app.update(Message::DiskMode(true));
    assert_eq!(app.partition_choice, Some(0));
    assert!(app.blocker().is_none());
    let _ = app.update(Message::RefreshDisks);
    assert!(app.blocker().is_some());

    // A visited review page cannot bypass a disk choice that was cleared.
    let _ = app.update(Message::Disk(0));
    app.reached = Step::Review;
    let _ = app.update(Message::Go(Step::Review));
    assert_eq!(app.step, Step::Disk);
    app.step = Step::Review;
    let _ = app.update(Message::Install);
    assert!(app.pending_install.is_none() && !app.installing());

    // An off-UI final probe cannot commit a plan after the user navigates back.
    let _ = app.update(Message::DiskMode(false));
    let _ = app.update(Message::Encrypt(false));
    let _ = app.update(Message::Edit(Field::FullName, "Jamie Example".into()));
    let _ = app.update(Message::Edit(Field::Password, "demo password".into()));
    let _ = app.update(Message::Edit(Field::Confirm, "demo password".into()));
    app.demo = false;
    assert!(app.blocker().is_none());
    let _ = app.update(Message::Install);
    let generation = app.discovery_generation;
    let current = app.disks.clone();
    assert!(app.disks_busy && app.pending_install.is_some());
    let _ = app.update(Message::Back);
    let step = app.step;
    let _ = app.update(Message::InstallDisksChecked(generation, current));
    assert_eq!(app.step, step);
    assert!(app.pending_install.is_none() && !app.installing());
}
#[test]
fn no_space_or_probe_failure_never_falls_back_to_erase() {
    let (mut app, _) = Installer::new(true);
    app.step = Step::Disk;
    let _ = app.update(Message::Disk(0));
    let disk = app.disks[app.disk.unwrap()].path.clone();
    let _ = app.update(Message::DiskMode(true));
    let _ = app.update(Message::PartitionChoices((disk.clone(), Ok(vec![]))));
    assert!(app.blocker().is_some());
    let _ = app.update(Message::PartitionChoices((disk, Err("Disk busy".into()))));
    assert!(app.blocker().is_some());
    assert_eq!(app.disk_mode, Some(true));
}
