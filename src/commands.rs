use crate::kb::{KB, Tuple, TupleID};
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
            bail!("Unknown command")
        };
        cmd(self, parts)
    }

    pub fn print_tuples_from_id(&mut self, ids: impl IntoIterator<Item = TupleID>) -> Result<()> {
        let ids: Vec<_> = ids.into_iter().collect();
        self.ui.messages.push(if ids.is_empty() {
            "No subj found.".to_string()
        } else {
            "Found:".to_string()
        });
        let ts = self.kb.retrieve_multiple_by_ids(ids)?;
        let mut strs: Vec<String> = ts.into_iter().map(|t| format!("  {}", t)).collect();
        strs.sort();
        self.ui.messages.extend(strs);
        Ok(())
    }
}

pub fn get_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(
        "help",
        "",
        "List commands",
        Box::new(|ctx, _args| {
            let entries = ctx.registry.list_commands();
            ctx.ui.messages.push(String::from("Commands:\n"));
            for (name, args, desc) in entries {
                let args_str = if args.is_empty() {
                    "".to_string()
                } else {
                    args
                };
                let cmd = format!("/{} {}", name, args_str);
                ctx.ui.messages.push(format!("  {:<30} - {}", cmd, desc));
            }
            Ok(())
        }),
    );

    registry.register(
        "clear",
        "",
        "Clear chat",
        Box::new(|ctx, _args| {
            ctx.ui.messages.clear();
            ctx.ui.messages.push("Cleared.".to_string());
            Ok(())
        }),
    );

    registry.register(
        "check",
        "<subj> <pred> <obj>",
        "Get full tuple",
        Box::new(|ctx, args| {
            if args.len() < 3 {
                return Err(UsageError.into());
            }
            let t = ctx.kb.retrieve_tuple(args[0], args[1], args[2])?;
            ctx.ui.messages.push(format!("Found: {}", t));
            Ok(())
        }),
    );

    registry.register(
        "find",
        "<entity>",
        "Get by subj/obj/pred",
        Box::new(|ctx, args| {
            if args.is_empty() {
                return Err(UsageError.into());
            }
            let mut ids = ctx.kb.retrieve_by_subject(args[0])?;
            let ids2 = ctx.kb.retrieve_by_predicate(args[0])?;
            let ids3 = ctx.kb.retrieve_by_object(args[0])?;
            ids = ids.union(&ids2).cloned().collect();
            ids = ids.union(&ids3).cloned().collect();
            ctx.print_tuples_from_id(ids)
        }),
    );

    registry.register(
        "subj",
        "<subj>",
        "Get by subject",
        Box::new(|ctx, args| {
            if args.is_empty() {
                return Err(UsageError.into());
            }
            let ids = ctx.kb.retrieve_by_subject(args[0])?;
            ctx.print_tuples_from_id(ids)
        }),
    );

    registry.register(
        "pred",
        "<pred>",
        "Get by predicate",
        Box::new(|ctx, args| {
            if args.is_empty() {
                return Err(UsageError.into());
            }
            let ids = ctx.kb.retrieve_by_predicate(args[0])?;
            ctx.print_tuples_from_id(ids)
        }),
    );

    registry.register(
        "obj",
        "<obj>",
        "Get by object",
        Box::new(|ctx, args| {
            if args.is_empty() {
                return Err(UsageError.into());
            }
            let ids = ctx.kb.retrieve_by_object(args[0])?;
            ctx.print_tuples_from_id(ids)
        }),
    );

    registry.register(
        "add",
        "<subj> [not] <pred> <obj>",
        "Add tuple",
        Box::new(|ctx, mut args| {
            let mut conf = 1.0;
            if args.len() > 1 && args[1] == "not" {
                conf = 0.0;
                args.remove(1);
            }
            if args.len() != 3 {
                return Err(UsageError.into());
            }
            let tuple = Tuple::new(args[0], args[1], args[2], conf);
            ctx.kb.store_tuple(&tuple).context("Add fail")?;
            ctx.ui.messages.push(format!("+ {}", tuple));
            Ok(())
        }),
    );

    registry
}
