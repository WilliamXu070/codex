use super::*;
use crate::app_event::SoundMenu;

#[tokio::test]
async fn sound_command_opens_settings_and_routes_enabled_submenu() {
    let (mut chat, mut events, _ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.dispatch_command(SlashCommand::Sound);
    assert_snapshot!(
        "sound_command_settings",
        render_bottom_popup(&chat, /*width*/ 80)
    );

    chat.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_matches!(
        events.try_recv(),
        Ok(AppEvent::OpenSoundPopup {
            menu: SoundMenu::Enabled
        })
    );
    chat.open_sound_popup(SoundMenu::Enabled);
    assert_snapshot!(
        "sound_command_enabled",
        render_bottom_popup(&chat, /*width*/ 80)
    );
}
