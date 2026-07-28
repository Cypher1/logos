use anyhow::{Result, anyhow};

pub type CommandFn = Box<dyn Fn(&mut AppContext, Vec<&str>) -> Result<()> + Send + Sync>;

pub static COMMAND_PREFIX: &str = "/";

pub struct CommandRegistry {
    commands: std::collections::HashMap<String, (CommandFn, String)>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, desc: &str, cmd: CommandFn) {
        self.commands
            .insert(name.to_string(), (cmd, desc.to_string()));
    }

    pub fn get(&self, name: &str) -> Option<&CommandFn> {
        self.commands.get(name).map(|(cmd, _)| cmd)
    }

    pub fn list_commands(&self) -> Vec<(String, String)> {
        let mut entries: Vec<_> = self
            .commands
            .iter()
            .map(|(k, (_v, d))| (k.clone(), d.clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }
}

/// Context passed to executed commands
pub struct AppContext<'a, 'b> {
    pub registry: &'a CommandRegistry,
    pub ui: &'a mut crate::ui::LogosUI<'b>,
    pub kb: &'a mut crate::kb::KB,
}

impl<'a, 'b> AppContext<'a, 'b> {
    pub fn execute(&mut self, name: &str, parts: Vec<&str>) -> Result<()> {
        if let Some(cmd) = self.registry.get(name) {
            cmd(self, parts)
        } else {
            Err(anyhow!("Unknown command: /{}", name))
        }
    }
}

pub fn get_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(
        "help",
        "Show available commands and their descriptions",
        Box::new(|ctx, _args| {
            let entries = ctx.registry.list_commands();
            let mut help_msg = String::from("Available commands:\n");
            for (name, desc) in entries {
                // Columnar formatting: /command      - description
                help_msg.push_str(&format!("  /{:<15} - {}\n", name, desc));
            }

            ctx.ui.messages.push(help_msg);
            Ok(())
        }),
    );

    registry.register(
        "clear",
        "Clear the chat history",
        Box::new(|ctx, _args| {
            ctx.ui.messages.clear();
            ctx.ui.messages.push("Chat cleared.".to_string());
            Ok(())
        }),
    );

    registry.register(
        "kb",
        "Show Knowledge Base update status",
        Box::new(|ctx, _args| {
            let status = format!("KB updated at {:?}", std::time::SystemTime::now());
            ctx.ui.system_state.push_str(&format!("\n{}", status));
            ctx.ui
                .messages
                .push(format!("Knowledge Base Status: {}", status));
            Ok(())
        }),
    );

    registry.register(
        "retrieve-id",
        "Retrieve a tuple by its unique ID (Usage: /retrieve-id <ID>)",
        Box::new(|ctx, args| {
            if args.is_empty() {
                ctx.ui.messages.push("Usage: /retrieve-id <ID>".to_string());
                return Ok(());
            }
            let id = args[0]
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("Invalid ID format"))?;
            match ctx.kb.retrieve_by_id(id) {
                Ok(Some(tuple)) => {
                    ctx.ui.messages.push(format!("Tuple found: {:?}", tuple));
                    Ok(())
                }
                Ok(None) => {
                    ctx.ui
                        .messages
                        .push("No tuple found with that ID.".to_string());
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }),
    );

    registry.register(
        "retrieve-tuple",
        "Retrieve a tuple by subject, predicate, and object (Usage: /retrieve-tuple <subject> <predicate> <object>)",
        Box::new(|ctx, args| {
            if args.len() < 3 {
                ctx.ui
                    .messages
                    .push("Usage: /retrieve-tuple <subject> <predicate> <object>".to_string());
                return Ok(());
            }
            let tuple = ctx.kb.retrieve_tuple(args[0], args[1], args[2])?;
            ctx.ui.messages.push(format!("Tuple found: {:?}", tuple));
            Ok(())
        }),
    );

    registry.register(
        "retrieve-subject",
        "Retrieve all IDs for a specific subject (Usage: /retrieve-subject <subject>)",
        Box::new(|ctx, args| {
            if args.is_empty() {
                ctx.ui
                    .messages
                    .push("Usage: /retrieve-subject <subject>".to_string());
                return Ok(());
            }
            let ids = ctx.kb.retrieve_by_subject(args[0])?;
            let msg = if ids.is_empty() {
                "No tuples found for subject.".to_string()
            } else {
                format!("Found IDs: {:?}", ids)
            };
            ctx.ui.messages.push(msg);
            Ok(())
        }),
    );

    registry.register(
        "retrieve-predicate",
        "Retrieve all IDs for a specific predicate (Usage: /retrieve-predicate <predicate>)",
        Box::new(|ctx, args| {
            if args.is_empty() {
                ctx.ui
                    .messages
                    .push("Usage: /retrieve-predicate <predicate>".to_string());
                return Ok(());
            }
            let ids = ctx.kb.retrieve_by_predicate(args[0])?;
            let msg = if ids.is_empty() {
                "No tuples found for predicate.".to_string()
            } else {
                format!("Found IDs: {:?}", ids)
            };
            ctx.ui.messages.push(msg);
            Ok(())
        }),
    );

    registry.register(
        "retrieve-object",
        "Retrieve all IDs for a specific object (Usage: /retrieve-object <object>)",
        Box::new(|ctx, args| {
            if args.is_empty() {
                ctx.ui
                    .messages
                    .push("Usage: /retrieve-object <object>".to_string());
                return Ok(());
            }
            let ids = ctx.kb.retrieve_by_object(args[0])?;
            let msg = if ids.is_empty() {
                "No tuples found for object.".to_string()
            } else {
                format!("Found IDs: {:?}", ids)
            };
            ctx.ui.messages.push(msg);
            Ok(())
        }),
    );

    registry.register(
        "retrieve-multiple",
        "Retrieve multiple tuples by IDs (Usage: /retrieve-multiple <id1> <id2> ...)",
        Box::new(|ctx, args| {
            if args.is_empty() {
                ctx.ui
                    .messages
                    .push("Usage: /retrieve-multiple <id1> <id2> ...".to_string());
                return Ok(());
            }
            let parsed_ids: Vec<u64> = args
                .iter()
                .map(|s| s.parse::<u64>())
                .collect::<Result<Vec<_>, _>>()?;
            let tuples = ctx.kb.retrieve_multiple_by_ids(&parsed_ids[..])?;
            ctx.ui.messages.push(format!("Found tuples: {:?}", tuples));
            Ok(())
        }),
    );

    registry
}
