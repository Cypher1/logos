pub type CommandFn = Box<dyn Fn(&mut AppContext) -> Result<(), String> + Send + Sync>;

pub struct CommandRegistry {
    commands: std::collections::HashMap<String, CommandFn>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, cmd: CommandFn) {
        self.commands.insert(name.to_string(), cmd);
    }

    pub fn execute(&self, name: &str, context: &mut AppContext) -> Result<(), String> {
        if let Some(cmd) = self.commands.get(name) {
            cmd(context)
        } else {
            Err(format!("Unknown command: /{}", name))
        }
    }
}

/// Context passed to executed commands
pub struct AppContext<'a> {
    pub ui: &'a mut crate::ui::LogosUI<'a>,
    pub kb: &'a mut crate::kb::KB,
    pub system_state: &'a mut String,
}

pub fn get_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(
        "help",
        Box::new(|ctx| {
            ctx.ui
                .messages
                .push("Available commands: /help, /clear, /kb".to_string());
            Ok(())
        }),
    );

    registry.register(
        "clear",
        Box::new(|ctx| {
            ctx.ui.messages.clear();
            ctx.ui.messages.push("Chat cleared.".to_string());
            Ok(())
        }),
    );

    registry.register(
        "kb",
        Box::new(|ctx| {
            let status = format!("KB updated at {:?}", std::time::SystemTime::now());
            ctx.system_state.push_str(&format!("\n{}", status));
            ctx.ui
                .messages
                .push(format!("Knowledge Base Status: {}", status));
            Ok(())
        }),
    );

    registry
}
