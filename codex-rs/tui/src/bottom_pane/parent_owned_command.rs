use crate::slash_command::SlashCommand;

pub(super) fn parent_owned_command_is_allowed(command: SlashCommand, args: &str) -> bool {
    if command == SlashCommand::Export {
        return true;
    }

    args.is_empty()
        && matches!(
            command,
            SlashCommand::Feedback
                | SlashCommand::New
                | SlashCommand::Clear
                | SlashCommand::Resume
                | SlashCommand::App
                | SlashCommand::Side
                | SlashCommand::Btw
                | SlashCommand::Agents
                | SlashCommand::MultiAgents
                | SlashCommand::Vim
                | SlashCommand::Keymap
                | SlashCommand::ElevateSandbox
                | SlashCommand::SandboxReadRoot
                | SlashCommand::Experimental
                | SlashCommand::Memories
                | SlashCommand::Quit
                | SlashCommand::Exit
                | SlashCommand::Logout
                | SlashCommand::Copy
                | SlashCommand::Diff
                | SlashCommand::Mention
                | SlashCommand::Skills
                | SlashCommand::Import
                | SlashCommand::Hooks
                | SlashCommand::Status
                | SlashCommand::Usage
                | SlashCommand::Ide
                | SlashCommand::DebugConfig
                | SlashCommand::Title
                | SlashCommand::Statusline
                | SlashCommand::Theme
                | SlashCommand::Pets
                | SlashCommand::Ps
                | SlashCommand::Stop
                | SlashCommand::MemoryDrop
                | SlashCommand::MemoryUpdate
                | SlashCommand::Mcp
                | SlashCommand::Apps
                | SlashCommand::Plugins
                | SlashCommand::Rollout
        )
}
