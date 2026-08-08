//! Keyboard input, external editor, and status-line dispatch for the TUI app.
//!
//! This module owns global key bindings that sit above ChatWidget, including transcript overlay
//! entry, Ctrl-L clear, external editor launch, and agent navigation shortcuts.

use super::*;
use crate::app_backtrack::SIDE_EDIT_PREVIOUS_UNAVAILABLE_MESSAGE;

const TRANSCRIBE_HOLD_THRESHOLD: Duration = Duration::from_millis(/*millis*/ 250);
const DEFAULT_TRANSCRIBE_UI_GAIN: f32 = 8.0;
const DEFAULT_TRANSCRIBE_MIN_RMS: f32 = 0.01;

fn transcribe_env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

fn read_transcribe_level(path: &Path) -> f32 {
    let Ok(text) = std::fs::read_to_string(path) else {
        return 0.0;
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|value| value.get("rms").and_then(serde_json::Value::as_f64))
        .map(|level| level.clamp(/*min*/ 0.0, /*max*/ 1.0) as f32)
        .unwrap_or(/*default*/ 0.0)
}

fn read_transcribe_max_rms(path: &Path) -> f32 {
    let Ok(text) = std::fs::read_to_string(path) else {
        return 0.0;
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|value| value.get("max_rms").and_then(serde_json::Value::as_f64))
        .map(|level| level.clamp(/*min*/ 0.0, /*max*/ 1.0) as f32)
        .unwrap_or(/*default*/ 0.0)
}

impl App {
    pub(super) fn route_key_chord_event(
        &mut self,
        tui: &mut tui::Tui,
        key_event: KeyEvent,
    ) -> Option<KeyEvent> {
        let contexts = self.active_keymap_contexts();
        let was_pending = self.key_chord_matcher.is_pending();
        match self.key_chord_matcher.advance(
            key_event,
            &self.keymap.chords,
            contexts,
            tokio::time::Instant::now(),
        ) {
            crate::keymap::KeyChordMatch::PassThrough => {
                if was_pending && !self.key_chord_matcher.is_pending() {
                    self.chat_widget.set_footer_hint_override(/*items*/ None);
                }
                Some(key_event)
            }
            crate::keymap::KeyChordMatch::Pending(prefix) => {
                if self.backtrack.primed {
                    self.reset_backtrack_state();
                }
                self.chat_widget.set_footer_hint_override(Some(vec![
                    (
                        format!("{} …", prefix.display_label()),
                        "waiting for next key".to_string(),
                    ),
                    ("esc".to_string(), "cancel".to_string()),
                ]));
                tui.frame_requester()
                    .schedule_frame_in(crate::keymap::KEY_CHORD_TIMEOUT);
                None
            }
            crate::keymap::KeyChordMatch::Completed(dispatch_event) => {
                self.chat_widget.set_footer_hint_override(/*items*/ None);
                Some(dispatch_event)
            }
            crate::keymap::KeyChordMatch::Cancelled => {
                self.chat_widget.set_footer_hint_override(/*items*/ None);
                None
            }
            crate::keymap::KeyChordMatch::Ignored => None,
        }
    }

    pub(super) fn expire_pending_key_chord(&mut self) {
        let contexts = self.active_keymap_contexts();
        if self
            .key_chord_matcher
            .expire(contexts, tokio::time::Instant::now())
        {
            self.chat_widget.set_footer_hint_override(/*items*/ None);
        }
    }

    pub(super) fn cancel_pending_key_chord(&mut self) {
        if self.key_chord_matcher.cancel() {
            self.chat_widget.set_footer_hint_override(/*items*/ None);
        }
    }

    fn active_keymap_contexts(&self) -> crate::keymap::KeymapContextSet {
        if self.overlay.is_some() {
            return crate::keymap::KeymapContextSet::new(crate::keymap::KeymapContext::Pager);
        }

        let contexts = self.chat_widget.keymap_contexts();
        if self.chat_widget.no_modal_or_popup_active() {
            contexts
                .with(crate::keymap::KeymapContext::Global)
                .with(crate::keymap::KeymapContext::Chat)
        } else {
            contexts
        }
    }

    pub(super) async fn launch_external_editor(&mut self, tui: &mut tui::Tui) {
        let editor_cmd = match external_editor::resolve_editor_command() {
            Ok(cmd) => cmd,
            Err(external_editor::EditorError::MissingEditor) => {
                self.chat_widget
                    .add_to_history(history_cell::new_error_event(
                    "Cannot open external editor: set $VISUAL or $EDITOR before starting Codex."
                        .to_string(),
                ));
                self.reset_external_editor_state(tui);
                return;
            }
            Err(err) => {
                self.chat_widget
                    .add_to_history(history_cell::new_error_event(format!(
                        "Failed to open editor: {err}",
                    )));
                self.reset_external_editor_state(tui);
                return;
            }
        };

        let seed = self.chat_widget.composer_text_with_pending();
        let editor_result = tui
            .with_restored(|| async { external_editor::run_editor(&seed, &editor_cmd).await })
            .await;
        self.reset_external_editor_state(tui);

        match editor_result {
            Ok(new_text) => {
                // Trim trailing whitespace
                let cleaned = new_text.trim_end().to_string();
                self.chat_widget.apply_external_edit(cleaned);
            }
            Err(err) => {
                self.chat_widget
                    .add_to_history(history_cell::new_error_event(format!(
                        "Failed to open editor: {err}",
                    )));
            }
        }
        tui.frame_requester().schedule_frame();
    }

    pub(super) fn request_external_editor_launch(&mut self, tui: &mut tui::Tui) {
        self.chat_widget
            .set_external_editor_state(ExternalEditorState::Requested);
        self.chat_widget.set_footer_hint_override(Some(vec![(
            EXTERNAL_EDITOR_HINT.to_string(),
            String::new(),
        )]));
        tui.frame_requester().schedule_frame();
    }

    pub(super) fn reset_external_editor_state(&mut self, tui: &mut tui::Tui) {
        self.chat_widget
            .set_external_editor_state(ExternalEditorState::Closed);
        self.chat_widget.set_footer_hint_override(/*items*/ None);
        tui.frame_requester().schedule_frame();
    }

    pub(super) fn apply_raw_output_mode(
        &mut self,
        tui: &mut tui::Tui,
        enabled: bool,
        notify: bool,
    ) {
        if notify {
            self.chat_widget.set_raw_output_mode_and_notify(enabled);
        } else {
            self.chat_widget.set_raw_output_mode(enabled);
        }
        let terminal_width = tui.terminal.last_known_screen_size.into();
        if let Err(err) = self.reflow_transcript_now(tui, terminal_width) {
            tracing::warn!(error = %err, "failed to reflow transcript after raw output mode toggle");
            self.chat_widget
                .add_error_message(format!("Failed to redraw transcript: {err}"));
        }
        tui.frame_requester().schedule_frame();
    }

    pub(super) async fn handle_key_event(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
        key_event: KeyEvent,
    ) {
        if self.transcribe_capture.is_some() && self.transcribe_stop_key_matches(key_event) {
            self.stop_transcribe_capture(tui);
            return;
        }
        if self.transcribe_arm.is_some() && self.transcribe_release_key_matches(key_event) {
            self.cancel_transcribe_arm();
            return;
        }

        // Some terminals, especially on macOS, encode Option+Left/Right as Option+b/f unless
        // enhanced keyboard reporting is available. We only treat those word-motion fallbacks as
        // agent-switch shortcuts when the composer is empty so we never steal the expected
        // editing behavior for moving across words inside a draft.
        let allow_agent_word_motion_fallback = !self.enhanced_keys_supported
            && self.chat_widget.composer_text_with_pending().is_empty();
        if self.overlay.is_none()
            && self.chat_widget.no_modal_or_popup_active()
            // Alt+Left/Right are also natural word-motion keys in the composer. Keep agent
            // fast-switch available only once the draft is empty so editing behavior wins whenever
            // there is text on screen.
            && self.chat_widget.composer_text_with_pending().is_empty()
            && previous_agent_shortcut_matches(key_event, allow_agent_word_motion_fallback)
        {
            if let Some(thread_id) = self
                .adjacent_thread_id_with_backfill(app_server, AgentNavigationDirection::Previous)
                .await
            {
                let _ = self
                    .select_agent_thread_and_discard_side(tui, app_server, thread_id)
                    .await;
            }
            return;
        }
        if self.overlay.is_none()
            && self.chat_widget.no_modal_or_popup_active()
            // Mirror the previous-agent rule above: empty drafts may use these keys for thread
            // switching, but non-empty drafts keep them for expected word-wise cursor motion.
            && self.chat_widget.composer_text_with_pending().is_empty()
            && next_agent_shortcut_matches(key_event, allow_agent_word_motion_fallback)
        {
            if let Some(thread_id) = self
                .adjacent_thread_id_with_backfill(app_server, AgentNavigationDirection::Next)
                .await
            {
                let _ = self
                    .select_agent_thread_and_discard_side(tui, app_server, thread_id)
                    .await;
            }
            return;
        }
        if side_return_shortcut_matches(key_event)
            && self.maybe_return_from_side(tui, app_server).await
        {
            return;
        }

        let app_keymap_shortcuts_available = self.app_keymap_shortcuts_available();

        let side_toggle_bindings = &self.keymap.app.toggle_side_conversation;
        if app_keymap_shortcuts_available
            && (side_toggle_bindings.is_pressed(key_event)
                || side_toggle_bindings.contains(&crate::key_hint::ctrl(KeyCode::Char('/')))
                    && crate::key_hint::ctrl(KeyCode::Char('7')).is_press(key_event))
        {
            if let Err(err) = self.toggle_side_conversation(tui, app_server).await {
                self.chat_widget
                    .add_error_message(format!("Failed to switch side conversation: {err}"));
            }
            return;
        }

        if app_keymap_shortcuts_available && self.keymap.app.toggle_vim_mode.is_pressed(key_event) {
            self.chat_widget.toggle_vim_mode_and_notify();
            return;
        }

        if app_keymap_shortcuts_available
            && self.keymap.app.toggle_fast_mode.is_pressed(key_event)
            && self.chat_widget.can_toggle_fast_mode_from_keybinding()
        {
            self.chat_widget.toggle_fast_mode_from_ui();
            return;
        }

        if app_keymap_shortcuts_available && self.keymap.app.toggle_raw_output.is_pressed(key_event)
        {
            let enabled = !self.chat_widget.raw_output_mode();
            self.apply_raw_output_mode(tui, enabled, /*notify*/ false);
            return;
        }

        if app_keymap_shortcuts_available && self.keymap.app.open_transcript.is_pressed(key_event) {
            self.scrollback_has_older_history = self
                .chat_widget
                .thread_id()
                .is_some_and(|thread_id| app_server.has_older_history(thread_id));
            self.open_transcript_overlay(tui);
            return;
        }

        if app_keymap_shortcuts_available && self.transcribe_start_key_matches(key_event) {
            self.request_transcribe_capture();
            return;
        }

        if app_keymap_shortcuts_available && self.terminal_transcribe_fallback_matches(key_event) {
            self.start_transcribe_capture(tui);
            return;
        }

        if app_keymap_shortcuts_available
            && self.keymap.app.open_external_editor.is_pressed(key_event)
        {
            // Only launch the external editor if there is no overlay and the bottom pane is not in use.
            // Note that it can be launched while a task is running to enable editing while the previous turn is ongoing.
            if self.overlay.is_none()
                && self.chat_widget.can_launch_external_editor()
                && self.chat_widget.external_editor_state() == ExternalEditorState::Closed
            {
                self.request_external_editor_launch(tui);
            }
            return;
        }

        if matches!(key_event.code, KeyCode::Esc)
            && matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
        {
            // Esc primes/advances backtracking only in normal (not working) mode
            // with the composer focused and empty. In any other state, forward
            // Esc so the active UI (e.g. status indicator, modals, popups)
            // handles it.
            if self.should_handle_backtrack_esc(key_event) {
                self.handle_backtrack_esc_key(tui);
            } else if self.should_reject_side_backtrack_esc(key_event) {
                self.reject_side_backtrack_esc();
            } else {
                self.chat_widget.handle_key_event(key_event);
            }
            return;
        }

        match key_event {
            _ if app_keymap_shortcuts_available
                && self.keymap.app.clear_terminal.is_pressed(key_event) =>
            {
                if !self.chat_widget.can_run_ctrl_l_clear_now() {
                    return;
                }
                if let Err(err) = self.clear_terminal_ui(tui, /*redraw_header*/ false) {
                    tracing::warn!(error = %err, "failed to clear terminal UI");
                    self.chat_widget
                        .add_error_message(format!("Failed to clear terminal UI: {err}"));
                } else {
                    self.reset_app_ui_state_after_clear();
                    self.queue_clear_ui_header(tui);
                    tui.frame_requester().schedule_frame();
                }
            }
            // Enter confirms backtrack when primed + count > 0. Otherwise pass to widget.
            KeyEvent {
                code: KeyCode::Enter,
                kind: KeyEventKind::Press,
                ..
            } if self.backtrack.primed
                && self.backtrack.nth_user_message != usize::MAX
                && self.chat_widget.composer_is_empty() =>
            {
                if let Some(selection) = self.confirm_backtrack_from_main() {
                    self.apply_backtrack_selection(selection);
                    tui.frame_requester().schedule_frame();
                }
            }
            KeyEvent {
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                ..
            } => {
                // Any non-Esc key press should cancel a primed backtrack.
                // This avoids stale "Esc-primed" state after the user starts typing
                // (even if they later backspace to empty).
                if key_event.code != KeyCode::Esc && self.backtrack.primed {
                    self.reset_backtrack_state();
                }
                self.chat_widget.handle_key_event(key_event);
            }
            _ => {
                self.chat_widget.handle_key_event(key_event);
            }
        };
    }

    pub(super) fn should_handle_backtrack_esc(&self, key_event: KeyEvent) -> bool {
        !self.chat_widget.side_conversation_active()
            && self.chat_widget.is_normal_backtrack_mode()
            && self.chat_widget.composer_is_empty()
            && !self.chat_widget.should_handle_vim_insert_escape(key_event)
    }

    pub(super) fn should_reject_side_backtrack_esc(&self, key_event: KeyEvent) -> bool {
        self.chat_widget.side_conversation_active()
            && self.chat_widget.is_normal_backtrack_mode()
            && self.chat_widget.composer_is_empty()
            && !self.chat_widget.should_handle_vim_insert_escape(key_event)
    }

    pub(super) fn reject_side_backtrack_esc(&mut self) {
        self.reset_backtrack_state();
        self.chat_widget
            .add_error_message(SIDE_EDIT_PREVIOUS_UNAVAILABLE_MESSAGE.to_string());
    }

    fn app_keymap_shortcuts_available(&self) -> bool {
        self.overlay.is_none() && self.chat_widget.no_modal_or_popup_active()
    }

    fn transcribe_stop_key_matches(&self, key_event: KeyEvent) -> bool {
        self.transcribe_release_key_matches(key_event)
            || self.terminal_transcribe_toggle_key_matches(key_event)
    }

    fn transcribe_start_key_matches(&self, key_event: KeyEvent) -> bool {
        key_event.kind == KeyEventKind::Press && self.keymap.app.transcribe.is_pressed(key_event)
    }

    fn transcribe_release_key_matches(&self, key_event: KeyEvent) -> bool {
        if key_event.kind != KeyEventKind::Release {
            return false;
        }
        let press_event = KeyEvent {
            kind: KeyEventKind::Press,
            ..key_event
        };
        self.keymap.app.transcribe.is_pressed(press_event)
    }

    fn request_transcribe_capture(&mut self) {
        let arm_id = self.arm_transcribe_capture();
        let tx = self.app_event_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(TRANSCRIBE_HOLD_THRESHOLD).await;
            tx.send(AppEvent::TranscribeHoldElapsed { arm_id });
        });
    }

    fn arm_transcribe_capture(&mut self) -> u64 {
        if let Some(arm) = &self.transcribe_arm {
            return arm.id;
        }

        let id = self.transcribe_next_arm_id;
        self.transcribe_next_arm_id = self.transcribe_next_arm_id.saturating_add(/*rhs*/ 1);
        self.transcribe_arm = Some(TranscribeArmState { id });
        id
    }

    fn cancel_transcribe_arm(&mut self) {
        self.transcribe_arm = None;
    }

    pub(super) fn start_armed_transcribe_capture(&mut self, tui: &mut tui::Tui, arm_id: u64) {
        let Some(arm) = self.transcribe_arm.take() else {
            return;
        };
        if arm.id == arm_id {
            self.start_transcribe_capture(tui);
        }
    }

    fn start_transcribe_capture(&mut self, tui: &mut tui::Tui) {
        if self.transcribe_capture.is_some() {
            return;
        }

        let marker_id = self.chat_widget.start_transcribe_marker();
        let script = self.config.codex_home.join("commands/transcribe-command");
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(/*default*/ 0);
        let wav_path = std::env::temp_dir().join(format!(
            "codex-transcribe-{}-{stamp}.wav",
            std::process::id()
        ));
        let level_path = std::env::temp_dir().join(format!(
            "codex-transcribe-{}-{stamp}.level.json",
            std::process::id()
        ));

        let child = std::process::Command::new(script.as_ref())
            .arg("record-wav-live")
            .arg(&wav_path)
            .arg(&level_path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match child {
            Ok(child) => {
                let (spinner_stop_tx, mut spinner_stop_rx) = tokio::sync::oneshot::channel();
                self.transcribe_capture = Some(TranscribeCaptureState {
                    child,
                    wav_path,
                    level_path: level_path.clone(),
                    started_at: Instant::now(),
                    marker_id,
                    waveform_samples: std::iter::repeat_n(
                        /*element*/ 0.0,
                        super::TRANSCRIBE_WAVEFORM_SAMPLES,
                    )
                    .collect(),
                    spinner_stop_tx,
                });
                let tx = self.app_event_tx.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_millis(/*millis*/ 120)) => {},
                            _ = &mut spinner_stop_rx => break,
                        }
                        let amplitude = read_transcribe_level(&level_path)
                            * transcribe_env_f32(
                                "CODEX_TRANSCRIBE_UI_GAIN",
                                DEFAULT_TRANSCRIBE_UI_GAIN,
                            );
                        tx.send(AppEvent::TranscribeMarkerTick {
                            marker_id,
                            amplitude,
                        });
                    }
                });
                self.chat_widget
                    .show_transcribe_status("Listening".to_string());
            }
            Err(err) => {
                self.chat_widget.replace_transcribe_marker(
                    marker_id,
                    &format!("transcribe failed to start: {err}"),
                );
            }
        }
        tui.frame_requester().schedule_frame();
    }

    fn stop_transcribe_capture(&mut self, tui: &mut tui::Tui) {
        let Some(mut capture) = self.transcribe_capture.take() else {
            return;
        };
        let _ = capture.spinner_stop_tx.send(());

        if capture.started_at.elapsed() < Duration::from_millis(/*millis*/ 250) {
            std::thread::sleep(
                Duration::from_millis(/*millis*/ 250) - capture.started_at.elapsed(),
            );
        }

        #[cfg(unix)]
        unsafe {
            libc::kill(capture.child.id() as i32, libc::SIGINT);
        }
        #[cfg(not(unix))]
        let _ = capture.child.kill();
        let _ = capture.child.wait();

        self.chat_widget
            .show_transcribe_status("Transcribing".to_string());
        tui.frame_requester().schedule_frame();

        let script = self.config.codex_home.join("commands/transcribe-command");
        let wav_path = capture.wav_path;
        let level_path = capture.level_path;
        let marker_id = capture.marker_id;
        let cleanup_path = wav_path.clone();
        let level_cleanup_path = level_path.clone();
        let tx = self.app_event_tx.clone();
        tokio::spawn(async move {
            let min_rms =
                transcribe_env_f32("CODEX_TRANSCRIBE_MIN_RMS", DEFAULT_TRANSCRIBE_MIN_RMS);
            let max_rms = read_transcribe_max_rms(&level_path);
            if max_rms < min_rms {
                let _ = std::fs::remove_file(&cleanup_path);
                let _ = std::fs::remove_file(&level_cleanup_path);
                tx.send(AppEvent::TranscribeCaptureFinished {
                    marker_id,
                    result: Ok(String::new()),
                });
                return;
            }

            let result = tokio::task::spawn_blocking(move || {
                std::process::Command::new(script.as_ref())
                    .arg("transcribe-file")
                    .arg(&wav_path)
                    .output()
                    .map_err(|err| format!("failed to transcribe: {err}"))
            })
            .await
            .map_err(|err| format!("transcribe task failed: {err}"))
            .and_then(|output| output);

            let result = match result {
                Ok(output) if output.status.success() => {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if text.is_empty() {
                        Err("transcribe returned empty text".to_string())
                    } else {
                        Ok(text)
                    }
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    Err(if stderr.is_empty() { stdout } else { stderr })
                }
                Err(err) => Err(err),
            };
            let _ = std::fs::remove_file(&cleanup_path);
            let _ = std::fs::remove_file(&level_cleanup_path);
            tx.send(AppEvent::TranscribeCaptureFinished { marker_id, result });
        });
    }

    fn terminal_transcribe_fallback_matches(&self, key_event: KeyEvent) -> bool {
        self.terminal_transcribe_toggle_key_matches(key_event)
    }

    fn terminal_transcribe_toggle_key_matches(&self, key_event: KeyEvent) -> bool {
        key_event.kind == KeyEventKind::Press
            && matches!(
                key_event,
                KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::CONTROL,
                    ..
                }
            )
    }

    pub(super) fn refresh_status_line(&mut self) {
        self.chat_widget.refresh_status_line();
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::make_test_app;
    use crossterm::event::KeyCode;
    use crossterm::event::KeyEvent;
    use crossterm::event::KeyEventKind;
    use crossterm::event::KeyModifiers;

    #[tokio::test]
    async fn app_keymap_shortcuts_are_disabled_while_keymap_view_is_active() {
        let mut app = make_test_app().await;
        assert!(app.app_keymap_shortcuts_available());

        let keymap = app.keymap.clone();
        app.chat_widget.open_keymap_debug(&keymap);

        assert!(!app.app_keymap_shortcuts_available());
    }

    #[tokio::test]
    async fn terminal_ctrl_shift_d_fallback_matches_collapsed_ctrl_d_with_draft_text() {
        let mut app = make_test_app().await;
        let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);

        assert!(app.terminal_transcribe_fallback_matches(ctrl_d));

        app.chat_widget.apply_external_edit("draft".to_string());
        assert!(app.terminal_transcribe_fallback_matches(ctrl_d));
    }

    #[tokio::test]
    async fn transcribe_shortcut_stops_on_release_not_second_press() {
        let app = make_test_app().await;
        let modifiers = KeyModifiers::CONTROL | KeyModifiers::SHIFT;
        let press = KeyEvent::new_with_kind(KeyCode::Char('D'), modifiers, KeyEventKind::Press);
        let repeat = KeyEvent::new_with_kind(KeyCode::Char('D'), modifiers, KeyEventKind::Repeat);
        let release = KeyEvent::new_with_kind(KeyCode::Char('D'), modifiers, KeyEventKind::Release);

        assert!(!app.transcribe_stop_key_matches(press));
        assert!(!app.transcribe_stop_key_matches(repeat));
        assert!(app.transcribe_stop_key_matches(release));
    }

    #[tokio::test]
    async fn transcribe_shortcut_tap_only_arms_then_cancels_without_listening() {
        let mut app = make_test_app().await;
        let modifiers = KeyModifiers::CONTROL | KeyModifiers::SHIFT;
        let press = KeyEvent::new_with_kind(KeyCode::Char('D'), modifiers, KeyEventKind::Press);
        let release = KeyEvent::new_with_kind(KeyCode::Char('D'), modifiers, KeyEventKind::Release);

        assert!(app.transcribe_start_key_matches(press));
        let arm_id = app.arm_transcribe_capture();
        assert_eq!(Some(arm_id), app.transcribe_arm.as_ref().map(|arm| arm.id));
        assert!(app.transcribe_capture.is_none());

        assert!(app.transcribe_release_key_matches(release));
        app.cancel_transcribe_arm();
        assert!(app.transcribe_arm.is_none());
        assert!(app.transcribe_capture.is_none());
    }
}
