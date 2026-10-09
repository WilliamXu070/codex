use super::*;

#[tokio::test]
async fn custom_settings_commands_open_local_popups_without_submitting_a_turn() {
    for (command, title) in [("/sound", "Sound Settings")] {
        let (mut chat, _rx, mut op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
        chat.bottom_pane
            .set_composer_text(command.to_string(), Vec::new(), Vec::new());
        chat.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        chat.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let popup = render_bottom_popup(&chat, /*width*/ 80);
        assert!(popup.contains(title), "{command}: {popup}");
        assert_matches!(op_rx.try_recv(), Err(TryRecvError::Empty));
    }
}
