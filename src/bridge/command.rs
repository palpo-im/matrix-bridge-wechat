#[derive(Clone)]
pub struct CommandProcessor {
    command_prefix: String,
}

impl CommandProcessor {
    pub fn new(command_prefix: String) -> Self {
        Self { command_prefix }
    }

    pub fn command_prefix(&self) -> &str {
        &self.command_prefix
    }

    pub fn parse_command(&self, text: &str) -> Option<(String, Vec<String>)> {
        let text = text.trim();
        if !text.starts_with(&self.command_prefix) {
            return None;
        }

        let text = text[self.command_prefix.len()..].trim_start();
        let parts: Vec<&str> = text.split_whitespace().collect();

        if parts.is_empty() {
            return None;
        }

        let command = parts[0].to_lowercase();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        Some((command, args))
    }

    pub fn process(&self, command: &str, args: &[String]) -> CommandResult {
        match command {
            "help" | "h" | "?" => self.cmd_help(),
            "login" => CommandResult::Login,
            "logout" => CommandResult::Logout,
            "ping" => CommandResult::Success("Pong!".to_string()),
            "list" => self.cmd_list(args),
            "sync" => self.cmd_sync(args),
            "delete-portal" => CommandResult::DeletePortal,
            "delete-all-portals" => CommandResult::DeleteAllPortals,
            "double-puppet" | "dp" => CommandResult::DoublePuppet(args.get(0).cloned()),
            _ => CommandResult::Error(format!("Unknown command: {}", command)),
        }
    }

    fn cmd_help(&self) -> CommandResult {
        CommandResult::Success(
            r#"Available commands:
- help: Show this help message
- login: Login to WeChat via QR code
- logout: Logout from WeChat
- ping: Check connection status
- list contacts/groups: List contacts or groups
- sync contacts/groups/space: Sync data
- delete-portal: Delete current portal
- delete-all-portals: Delete all portals
- double-puppet <token>: Enable double puppeting with access token
"#
            .to_string(),
        )
    }

    fn cmd_list(&self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::Error("Usage: list contacts|groups".to_string());
        }
        match args[0].as_str() {
            "contacts" => CommandResult::ListContacts,
            "groups" => CommandResult::ListGroups,
            _ => CommandResult::Error("Usage: list contacts|groups".to_string()),
        }
    }

    fn cmd_sync(&self, args: &[String]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::Error("Usage: sync contacts|groups|space".to_string());
        }
        match args[0].as_str() {
            "contacts" => CommandResult::SyncContacts,
            "groups" => CommandResult::SyncGroups,
            "space" => CommandResult::SyncSpace,
            _ => CommandResult::Error("Usage: sync contacts|groups|space".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CommandResult {
    Success(String),
    Error(String),
    NeedsLogin,
    Login,
    Logout,
    ListContacts,
    ListGroups,
    SyncContacts,
    SyncGroups,
    SyncSpace,
    DeletePortal,
    DeleteAllPortals,
    DoublePuppet(Option<String>),
}
