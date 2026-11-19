use super::*;

#[test]
fn encryption_has_its_own_gate_and_back_preserves_choices() {
    let (mut app, _) = Installer::new(true);
    app.step = Step::Disk;
    let _ = app.update(Message::Disk(0));
    let _ = app.update(Message::DiskMode(false));
    assert!(app.blocker().is_none());
    let _ = app.update(Message::Next);
    assert_eq!(app.step, Step::Encryption);
    assert!(app.blocker().is_some());
    app.passphrase = "test phrase".into();
    app.passphrase_confirm = "different".into();
    assert!(app.blocker().is_some());
    app.passphrase_confirm = app.passphrase.clone();
    let _ = app.update(Message::Next);
    assert_eq!(app.step, Step::Account);
    let _ = app.update(Message::Back);
    assert_eq!(app.passphrase, "test phrase");
    let _ = app.update(Message::Encrypt(false));
    app.passphrase_confirm.clear();
    assert!(app.blocker().is_none());
}
