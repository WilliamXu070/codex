use super::*;
use crate::app_event::TranscribeMenu;
use crate::bottom_pane::SettingsTextInputViewParams;

const TRANSCRIBE_USAGE: &str = "Usage: /transcribe [status|capture|record-wav [seconds] [path]|transcribe-file <path>|set-provider <provider>|set-key <api_key>|set-language <language>|help]";

#[derive(Default)]
struct TranscribeState {
    provider: String,
    language: String,
    api_key: String,
}

impl ChatWidget {
    pub(crate) fn open_transcribe_popup(&mut self, menu: TranscribeMenu) {
        let state = self.transcribe_state();
        let on_cancel = (menu != TranscribeMenu::Root).then_some({
            Box::new(|tx: &crate::app_event_sender::AppEventSender| {
                tx.send(AppEvent::OpenTranscribePopup {
                    menu: TranscribeMenu::Root,
                });
            }) as Box<dyn Fn(&crate::app_event_sender::AppEventSender) + Send + Sync>
        });

        let items = match menu {
            TranscribeMenu::Root => vec![
                self.transcribe_menu_item(
                    "Provider",
                    state.provider_label(),
                    TranscribeMenu::Provider,
                ),
                self.transcribe_menu_item(
                    "Language",
                    state.language_label(),
                    TranscribeMenu::Language,
                ),
                self.transcribe_menu_item("API Key", state.api_key_label(), TranscribeMenu::ApiKey),
            ],
            TranscribeMenu::Provider => ["openai", "gemini", "groq"]
                .into_iter()
                .map(|provider| {
                    self.transcribe_command_item(
                        provider,
                        "use this provider",
                        state.provider == provider,
                        &["set-provider", provider],
                    )
                })
                .collect(),
            TranscribeMenu::Language => ["auto", "en", "zh", "es", "fr"]
                .into_iter()
                .map(|language| {
                    self.transcribe_command_item(
                        language,
                        "use this language",
                        state.language == language,
                        &["set-language", language],
                    )
                })
                .collect(),
            TranscribeMenu::ApiKey => vec![self.transcribe_api_key_item()],
        };

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(
                match menu {
                    TranscribeMenu::Root => "Transcribe Settings",
                    TranscribeMenu::Provider => "Transcribe Provider",
                    TranscribeMenu::Language => "Transcribe Language",
                    TranscribeMenu::ApiKey => "Transcribe API Key",
                }
                .to_string(),
            ),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            on_cancel,
            ..Default::default()
        });
    }

    pub(super) fn handle_transcribe_command(&mut self, args: &str) {
        let args = args.trim();
        if args.is_empty() {
            self.open_transcribe_popup(TranscribeMenu::Root);
            return;
        }

        let parts = args.split_whitespace().collect::<Vec<_>>();
        let Some(command) = parts.first().copied() else {
            self.submit_transcribe_action(&["status"]);
            return;
        };
        let cmd = command.to_ascii_lowercase();

        match cmd.as_str() {
            "status" => {
                self.submit_transcribe_action(&["status"]);
            }
            "capture" => {
                self.submit_transcribe_action(&["capture"]);
            }
            "record-wav" => {
                self.submit_transcribe_action(&parts);
            }
            "transcribe-file" => {
                self.submit_transcribe_action(&parts);
            }
            "set-provider" | "provider" => {
                if parts.len() < 2 {
                    self.add_error_message(
                        "Usage: /transcribe set-provider <provider>".to_string(),
                    );
                    return;
                }
                let provider = parts[1];
                self.submit_transcribe_action(&["set-provider", provider]);
            }
            "set-key" | "set-api-key" => {
                if parts.len() < 2 {
                    self.add_error_message("Usage: /transcribe set-key <api_key>".to_string());
                    return;
                }
                let key = parts[1..].join(" ");
                if key.is_empty() {
                    self.add_error_message("Usage: /transcribe set-key <api_key>".to_string());
                    return;
                }
                self.submit_transcribe_action(&["set-key", &key]);
            }
            "set-language" => {
                if parts.len() < 2 {
                    self.add_error_message(
                        "Usage: /transcribe set-language <language>".to_string(),
                    );
                    return;
                }
                let language = parts[1];
                self.submit_transcribe_action(&["set-language", language]);
            }
            "help" => {
                self.add_info_message(TRANSCRIBE_USAGE.to_string(), None);
            }
            _ => {
                self.add_error_message(format!("Unrecognized subcommand. {TRANSCRIBE_USAGE}"));
            }
        }
    }

    fn transcribe_menu_item(
        &self,
        name: &str,
        description: String,
        menu: TranscribeMenu,
    ) -> SelectionItem {
        SelectionItem {
            name: name.to_string(),
            description: Some(description),
            actions: vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenTranscribePopup { menu })
            })],
            dismiss_on_select: true,
            ..Default::default()
        }
    }

    fn transcribe_command_item(
        &self,
        name: impl Into<String>,
        description: impl Into<String>,
        is_current: bool,
        args: &[&str],
    ) -> SelectionItem {
        let script = self.config.codex_home.join("commands/transcribe-command");
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        SelectionItem {
            name: name.into(),
            description: Some(description.into()),
            is_current,
            actions: vec![Box::new(move |tx| {
                let _ = std::process::Command::new(script.as_ref())
                    .args(&args)
                    .status();
                tx.send(AppEvent::OpenTranscribePopup {
                    menu: TranscribeMenu::Root,
                });
            })],
            dismiss_on_select: true,
            ..Default::default()
        }
    }

    fn transcribe_api_key_item(&self) -> SelectionItem {
        SelectionItem {
            name: "Set API key".to_string(),
            description: Some("type and save API key".to_string()),
            actions: vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenTranscribeApiKeyPrompt);
            })],
            dismiss_on_select: true,
            ..Default::default()
        }
    }

    pub(crate) fn open_transcribe_api_key_prompt(&mut self) {
        let script = self.config.codex_home.join("commands/transcribe-command");
        let tx = self.app_event_tx.clone();
        let cancel_tx = self.app_event_tx.clone();
        self.bottom_pane
            .show_settings_text_input(SettingsTextInputViewParams {
                title: "Transcribe API Key".to_string(),
                placeholder: "Paste API key".to_string(),
                initial_text: String::new(),
                secret: true,
                on_submit: Box::new(move |key: String| {
                    let _ = std::process::Command::new(script.as_ref())
                        .args(["set-key", key.trim()])
                        .status();
                    tx.send(AppEvent::OpenTranscribePopup {
                        menu: TranscribeMenu::Root,
                    });
                }),
                on_cancel: Some(Box::new(move || {
                    cancel_tx.send(AppEvent::OpenTranscribePopup {
                        menu: TranscribeMenu::ApiKey,
                    });
                })),
            });
    }

    fn submit_transcribe_action(&mut self, args: &[&str]) {
        let script = self.config.codex_home.join("commands/transcribe-command");
        let output = std::process::Command::new(script.as_ref())
            .args(args)
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|_| "transcribe command unavailable".to_string());
        if output.is_empty() {
            self.add_info_message("transcribe command produced no output".to_string(), None);
            return;
        }
        let has_error = output.contains("error:") || output.contains("Usage:");
        self.add_info_message(output.clone(), None);
        if has_error {
            return;
        }
    }

    fn transcribe_state(&self) -> TranscribeState {
        let mut state = TranscribeState {
            provider: "openai".to_string(),
            language: "en".to_string(),
            api_key: "unset".to_string(),
        };
        for line in self.run_transcribe_command(&["status"]).lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "provider" => state.provider = value.trim().to_string(),
                "language" => state.language = value.trim().to_string(),
                "api_key" => {
                    let value = value.trim();
                    state.api_key = if value.starts_with("set") {
                        "set".to_string()
                    } else {
                        "unset".to_string()
                    };
                }
                _ => {}
            }
        }
        state
    }

    fn run_transcribe_command(&self, args: &[&str]) -> String {
        std::process::Command::new(
            self.config
                .codex_home
                .join("commands/transcribe-command")
                .as_ref(),
        )
        .args(args)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default()
    }
}

impl TranscribeState {
    fn provider_label(&self) -> String {
        self.provider.clone()
    }

    fn language_label(&self) -> String {
        self.language.clone()
    }

    fn api_key_label(&self) -> String {
        self.api_key.clone()
    }
}
