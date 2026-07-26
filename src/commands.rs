use anyhow::{Result, anyhow};

pub type CommandFn = Box<dyn Fn(&mut AppContext, Vec<&str>) -> Result<()> + Send + Sync>;

pub static COMMAND_PREFIX: &str = "/";

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

    pub fn execute(&self, name: &str, parts: Vec<&str>, context: &mut AppContext) -> Result<()> {
        if let Some(cmd) = self.commands.get(name) {
            cmd(context, parts)
        } else {
            Err(anyhow!("Unknown command: /{}", name))
        }
    }
}

/// Context passed to executed commands
pub struct AppContext<'a, 'b> {
    pub ui: &'a mut crate::ui::LogosUI<'b>,
    #[allow(unused)]
    pub kb: &'a mut crate::kb::KB,
}

pub fn get_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(
        "help",
        Box::new(|ctx, _args| {
            ctx.ui
                .messages
                .push("Available commands: /help, /clear, /kb".to_string());
            Ok(())
        }),
    );

    registry.register(
        "clear",
        Box::new(|ctx, _args| {
            ctx.ui.messages.clear();
            ctx.ui.messages.push("Chat cleared.".to_string());
            Ok(())
        }),
    );

    registry.register(
        "kb",
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
        Box::new(|ctx, args| {
            if args.len() < 1 {
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
        Box::new(|ctx, args| {
            if args.len() < 1 {
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
        Box::new(|ctx, args| {
            if args.len() < 1 {
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
        Box::new(|ctx, args| {
            if args.len() < 1 {
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
