use super::*;
#[test]
fn exit_menu_preserves_edits_and_reset_clears_secrets() {
    let (mut app, _) = Installer::new(true);
    app.password = "test secret".into();
    let _ = app.update(Message::Theme(true));
    let window = iced::window::Id::unique();
    let _ = app.update(Message::WindowState(ShellWindowEvent::Opened(window)));
    let _ = app.update(Message::WindowState(ShellWindowEvent::MaximizedChanged {
        window,
        maximized: true,
    }));
    let _ = app.update(Message::ExitMenu);
    assert!(app.exit_menu);
    let _ = app.update(Message::DismissExit);
    assert_eq!(app.password, "test secret");
    let _ = app.network_request(serde_json::json!({"action":"status"}));
    let generation = app.network_generation;
    let _ = app.update(Message::Reset);
    let _ = app.network_request(serde_json::json!({"action":"status"}));
    app.network_password = "new-network-secret".into();
    let _ = app.update(Message::NetworkResult(
        generation,
        Ok(network::State::default()),
    ));
    assert!(app.network_busy && app.network_password == "new-network-secret");
    let current = app.network_generation;
    let _ = app.update(Message::NetworkResult(
        current,
        Ok(network::State::default()),
    ));
    assert!(!app.network_busy && app.network_password.is_empty());
    let _ = app.network_request(serde_json::json!({"action":"status"}));
    let failed = app.network_generation;
    let _ = app.update(Message::NetworkResult(failed, Err("offline".into())));
    assert_eq!(app.network_error, "offline");
    let _ = app.update(Message::DismissNetworkError);
    assert!(app.network_error.is_empty());
    assert_eq!(app.step, Step::Welcome);
    assert_eq!(app.chrome.window_id(), Some(window));
    assert!(app.chrome.maximized());
    assert!(app.password.is_empty() && app.passphrase.is_empty());
    assert!(app.dark && app.encrypt);
    app.password = "canary-password".into();
    app.passphrase = "canary-passphrase".into();
    let plan = app.plan();
    assert!(!format!("{plan:?}").contains("canary"));
    assert!(
        !format!(
            "{:?}",
            Message::Edit(Field::Password, "canary-password".into())
        )
        .contains("canary")
    );
    let _ = app.push_line("canary-password canary-passphrase".into());
    assert!(!app.full_log.contains("canary") && !app.lines[0].contains("canary"));
    for _ in 0..200 {
        let _ = app.push_line("x".repeat(16 * 1024));
    }
    assert!(app.full_log.len() <= 2 * 1024 * 1024);
    assert!(
        app.full_log
            .starts_with("[older installation output omitted]")
    );
    let _ = app.update(Message::Progress(system::Progress::CleanupIncomplete));
    assert_eq!(app.outcome, Outcome::Done);
    assert!(app.cleanup_incomplete && app.password.is_empty() && app.passphrase.is_empty());
}
#[test]
fn active_installation_cannot_be_reset() {
    let (mut app, _) = Installer::new(true);
    app.step = Step::Installing;
    let _ = app.update(Message::Reset);
    assert_eq!(app.step, Step::Installing);
    let _ = app.update(Message::Window(WindowCommand::Close));
    assert!(app.exit_menu);
    assert_eq!(app.step, Step::Installing);
    assert!(matches!(app.outcome, Outcome::Running));
    app.outcome = Outcome::Failed("test failure".into());
    let _ = app.update(Message::Reset);
    assert_eq!(app.step, Step::Welcome);
}
