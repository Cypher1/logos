use crate::kb::{KB, Tuple};
use crate::ui::LogosUI;

use anyhow::{Context, Result, bail};
use thiserror::Error;

pub type CommandFn = Box<dyn Fn(&mut AppContext, Vec<&str>) -> Result<()> + Send + Sync>;

pub static COMMAND_PREFIX: &str = "/";

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Usage")]
    UsageError,
}
use CommandError::*;

pub struct CommandRegistry {
    commands: std::collections::HashMap<String, (String, String, CommandFn)>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, args: &str, desc: &str, cmd: CommandFn) {
        self.commands
            .insert(name.to_string(), (args.to_string(), desc.to_string(), cmd));
    }

    pub fn get(&self, name: &str) -> Option<&CommandFn> {
        self.commands.get(name).map(|(_, _, cmd)| cmd)
    }

    pub fn get_args(&self, name: &str) -> Option<&str> {
        self.commands
            .get(name)
            .map(|(args, _desc, _cmd)| args.as_str())
    }

    pub fn list_commands(&self) -> Vec<(String, String, String)> {
        let mut entries: Vec<_> = self
            .commands
            .iter()
            .map(|(k, (a, d, _cmd))| (k.clone(), a.clone(), d.clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }

    pub fn find_matches(&self, prefix: &str) -> Vec<String> {
        let mut matches = Vec::new();
        for name in self.commands.keys() {
            if name.starts_with(prefix) {
                matches.push(name.clone());
            }
        }
        matches
    }
}

/// Context passed to executed commands
pub struct AppContext<'a, 'b> {
    pub registry: &'a CommandRegistry,
    pub ui: &'a mut LogosUI<'b>,
    pub kb: &'a mut KB,
}

impl<'a, 'b> AppContext<'a, 'b> {
    pub fn get_args(&self, name: &str) -> &str {
        self.registry.get_args(name).unwrap_or("???")
    }

    pub fn execute(&mut self, name: &str, parts: Vec<&str>) -> Result<()> {
        let Some(cmd) = self.registry.get(name) else {
            bail!("Unknown command: /{}", name)
        };
        cmd(self, parts)
    }
}

pub fn get_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(
        "help",
        "",
        "Show available commands and their descriptions",
        Box::new(|ctx, _args| {
            let entries = ctx.registry.list_commands();
            let mut help_msg = String::from("Available commands:\n");
            for (name, args, desc) in entries {
                // Format: /command <args> - description
                let args_str = if args.is_empty() {
                    "".to_string()
                } else {
                    args
                };
                help_msg.push_str(&format!("  /{:<15} {} - {}\n", name, args_str, desc));
            }

            ctx.ui.messages.push(help_msg);
            Ok(())
        }),
    );

    registry.register(
        "clear",
        "",
        "Clear the chat history",
        Box::new(|ctx, _args| {
            ctx.ui.messages.clear();
            ctx.ui.messages.push("Chat cleared.".to_string());
            Ok(())
        }),
    );

    registry.register(
        "kb",
        "",
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
        "<ID>",
        "Retrieve a tuple by its unique ID",
        Box::new(|ctx, args| {
            if args.is_empty() {
                return Err(UsageError.into());
            }
            let id = args[0]
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("Invalid ID format"))?;
            match ctx.kb.retrieve_by_id(id)? {
                Some(tuple) => {
                    ctx.ui.messages.push(format!("Tuple found: {}", tuple));
                }
                None => {
                    ctx.ui
                        .messages
                        .push("No tuple found with that ID.".to_string());
                }
            }
            Ok(())
        }),
    );

    registry.register(
        "retrieve-tuple",
        "<subject> <predicate> <object>",
        "Retrieve a tuple by subject, predicate, and object",
        Box::new(|ctx, args| {
            if args.len() < 3 {
                return Err(UsageError.into());
            }
            let tuple = ctx.kb.retrieve_tuple(args[0], args[1], args[2])?;
            ctx.ui.messages.push(format!("Tuple found: {}", tuple));
            Ok(())
        }),
    );

    registry.register(
        "retrieve-subject",
        "<subject>",
        "Retrieve all IDs for a specific subject",
        Box::new(|ctx, args| {
            if args.is_empty() {
                return Err(UsageError.into());
            }
            let ids = ctx.kb.retrieve_by_subject(args[0])?;
            let msg = if ids.is_empty() {
                "No tuples found for subject.".to_string()
            } else {
                format!("Found IDs: {:#?}", ids)
            };
            ctx.ui.messages.push(msg);
            Ok(())
        }),
    );

    registry.register(
        "retrieve-predicate",
        "<predicate>",
        "Retrieve all IDs for a specific predicate",
        Box::new(|ctx, args| {
            if args.is_empty() {
                return Err(UsageError.into());
            }
            let ids = ctx.kb.retrieve_by_predicate(args[0])?;
            let msg = if ids.is_empty() {
                "No tuples found for predicate.".to_string()
            } else {
                format!("Found IDs: {:#?}", ids)
            };
            ctx.ui.messages.push(msg);
            Ok(())
        }),
    );

    registry.register(
        "retrieve-object",
        "<object>",
        "Retrieve all IDs for a specific object",
        Box::new(|ctx, args| {
            if args.is_empty() {
                return Err(UsageError.into());
            }
            let ids = ctx.kb.retrieve_by_object(args[0])?;
            let msg = if ids.is_empty() {
                "No tuples found for object.".to_string()
            } else {
                format!("Found IDs: {:#?}", ids)
            };
            ctx.ui.messages.push(msg);
            Ok(())
        }),
    );

    registry.register(
        "retrieve-multiple",
        "<id1> <id2> ...",
        "Retrieve multiple tuples by IDs",
        Box::new(|ctx, args| {
            if args.is_empty() {
                return Err(UsageError.into());
            }
            let parsed_ids: Vec<u64> = args
                .iter()
                .map(|s| s.parse::<u64>())
                .collect::<Result<Vec<_>, _>>()?;
            let tuples = ctx.kb.retrieve_multiple_by_ids(&parsed_ids[..])?;
            ctx.ui.messages.push(format!("Found tuples: {:#?}", tuples));
            Ok(())
        }),
    );

    registry.register(
        "add",
        "<subject> [not] <predicate> <object>",
        "Add a new tuple to the Knowledge Base",
        Box::new(|ctx, mut args| {
            let mut confidence = 1.0;
            if args.len() > 1 && args[1] == "not" {
                confidence = 0.0;
                args.remove(1);
            }
            if args.len() != 3 {
                return Err(UsageError.into());
            }
            let tuple = Tuple::new(args[0], args[1], args[2], confidence);
            ctx.kb.store_tuple(&tuple).context("Error adding tuple")?;
            ctx.ui.messages.push(format!("+ {}", tuple));
            Ok(())
        }),
    );

    registry
}
