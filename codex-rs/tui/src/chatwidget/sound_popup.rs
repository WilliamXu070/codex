use super::*;
use crate::app_event::SoundMenu;

#[derive(Debug)]
struct SoundState {
    enabled: bool,
    volume: u32,
    completion: String,
    approval: String,
}

impl Default for SoundState {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 100,
            completion: "random".to_string(),
            approval: "default".to_string(),
        }
    }
}

impl SoundState {
    fn enabled_label(&self) -> String {
        if self.enabled { "On" } else { "Off" }.to_string()
    }

    fn volume_label(&self) -> String {
        format!("{}% {}", self.volume, sound_volume_bar(self.volume))
    }
}

impl ChatWidget {
    pub(crate) fn open_sound_popup(&mut self, menu: SoundMenu) {
        let state = self.sound_state();
        let on_cancel = (menu != SoundMenu::Root).then_some({
            Box::new(|tx: &crate::app_event_sender::AppEventSender| {
                tx.send(AppEvent::OpenSoundPopup {
                    menu: SoundMenu::Root,
                });
            }) as Box<dyn Fn(&crate::app_event_sender::AppEventSender) + Send + Sync>
        });

        let items = match menu {
            SoundMenu::Root => vec![
                self.sound_menu_item("Enabled", state.enabled_label(), SoundMenu::Enabled),
                self.sound_menu_item("Volume", state.volume_label(), SoundMenu::Volume),
                self.sound_menu_item("Completion", state.completion, SoundMenu::Completion),
                self.sound_menu_item("Approval", state.approval, SoundMenu::Approval),
                sound_exit_item(),
            ],
            SoundMenu::Enabled => vec![
                self.sound_command_item("On", "unmute", state.enabled, &["unmute"]),
                self.sound_command_item("Off", "mute", !state.enabled, &["mute"]),
            ],
            SoundMenu::Volume => [0, 10, 25, 50, 75, 100]
                .into_iter()
                .map(|value| {
                    self.sound_command_item(
                        format!("{value}% {}", sound_volume_bar(value)),
                        "set volume",
                        state.volume == value,
                        &["volume", &value.to_string()],
                    )
                })
                .collect(),
            SoundMenu::Completion => self.sound_file_items(
                "track",
                "random",
                &state.completion,
                "Random",
                "pick a random completion sound",
                "use this track",
            ),
            SoundMenu::Approval => self.sound_file_items(
                "approval",
                "default",
                &state.approval,
                "Default",
                "use the default approval sound",
                "use this approval sound",
            ),
        };

        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(
            match menu {
                SoundMenu::Root => "Sound Settings",
                SoundMenu::Enabled => "Sound Enabled",
                SoundMenu::Volume => "Sound Volume",
                SoundMenu::Completion => "Completion Sound",
                SoundMenu::Approval => "Approval Sound",
            }
            .to_string(),
            ),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            on_cancel,
            ..Default::default()
        });
    }

    fn sound_menu_item(&self, name: &str, description: String, menu: SoundMenu) -> SelectionItem {
        SelectionItem {
            name: name.to_string(),
            description: Some(description),
            actions: vec![Box::new(move |tx| {
                tx.send(AppEvent::OpenSoundPopup { menu })
            })],
            dismiss_on_select: true,
            ..Default::default()
        }
    }

    fn sound_command_item(
        &self,
        name: impl Into<String>,
        description: impl Into<String>,
        is_current: bool,
        args: &[&str],
    ) -> SelectionItem {
        let script = self.config.codex_home.join("commands/sound");
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        SelectionItem {
            name: name.into(),
            description: Some(description.into()),
            is_current,
            actions: vec![Box::new(move |tx| {
                let _ = std::process::Command::new(script.as_ref()).args(&args).status();
                tx.send(AppEvent::OpenSoundPopup {
                    menu: SoundMenu::Root,
                });
            })],
            dismiss_on_select: true,
            ..Default::default()
        }
    }

    fn sound_file_items(
        &self,
        command: &str,
        reset_arg: &str,
        current: &str,
        reset_name: &str,
        reset_description: &str,
        set_description: &str,
    ) -> Vec<SelectionItem> {
        let mut items = vec![self.sound_command_item(
            reset_name,
            reset_description,
            current == reset_arg,
            &[command, reset_arg],
        )];
        for track in self.sound_tracks() {
            let name = track.clone();
            items.push(self.sound_command_item(
                name.clone(),
                set_description,
                current == name,
                &[command, "set", &track],
            ));
        }
        items
    }

    fn sound_tracks(&self) -> Vec<String> {
        self.run_sound_command(&["track", "list"])
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect()
    }

    fn sound_state(&self) -> SoundState {
        let mut state = SoundState::default();
        for field in self.run_sound_command(&["status"]).split_whitespace() {
            let Some((key, value)) = field.split_once('=') else {
                continue;
            };
            match key {
                "enabled" => state.enabled = value != "0",
                "volume" => {
                    state.volume = value
                        .parse::<f32>()
                        .map(|v| (v * 100.0).round() as u32)
                        .unwrap_or(100);
                }
                "track" => state.completion = sound_label(value),
                "approval" => state.approval = sound_label(value),
                _ => {}
            }
        }
        state
    }

    fn run_sound_command(&self, args: &[&str]) -> String {
        std::process::Command::new(self.config.codex_home.join("commands/sound").as_ref())
            .args(args)
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_default()
    }
}

fn sound_exit_item() -> SelectionItem {
    SelectionItem {
        name: "Exit".to_string(),
        description: Some("close sound settings".to_string()),
        dismiss_on_select: true,
        ..Default::default()
    }
}

fn sound_label(value: &str) -> String {
    if value.is_empty() || value == "random" || value == "default" {
        value.to_string()
    } else {
        Path::new(value)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| value.to_string())
    }
}

fn sound_volume_bar(value: u32) -> String {
    let filled = (value / 10).min(10) as usize;
    format!("[{}{}]", "#".repeat(filled), "-".repeat(10 - filled))
}
