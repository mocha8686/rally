use anyhow::{anyhow, Result};
use async_trait::async_trait;
use clap::{command, Parser, Subcommand};
use colored::Colorize;
use rustyline::DefaultEditor;

use crate::history::get_history_path;

#[async_trait]
pub trait Repl {
    type Commands: Subcommand;

    fn prompt(&self) -> &str;
    async fn respond(&self, command: Self::Commands) -> Result<Option<Response<Self::Commands>>>;
}

pub enum Response<C> {
    Switch(Box<dyn Repl<Commands = C>>),
    Exit,
}

#[derive(Debug, Parser)]
#[command(multicall = true)]
struct Cli<T: Subcommand> {
    #[command(subcommand)]
    command: T,
}

pub async fn start(repl: &impl Repl) -> Result<()> {
    loop {
        let line = read_line(repl.prompt())?;
        if line.is_empty() {
            continue;
        }

        match handle_command(repl, &line).await {
            Ok(Some(Response::Switch(repl))) => {
                todo!()
            }
            Ok(Some(Response::Exit)) => break,
            Ok(None) => {}
            Err(e) => {
                let res = e.downcast::<clap::error::Error>();
                if let Ok(e) = res {
                    match e.kind() {
                        clap::error::ErrorKind::DisplayHelp
                        | clap::error::ErrorKind::DisplayVersion => eprintln!("{e}"),
                        _ => eprintln!("{}", e.to_string().red()),
                    }
                } else {
                    eprintln!("{}", res.unwrap_err().to_string().red());
                }
            }
        }
    }

    Ok(())
}

pub async fn handle_command<C: Subcommand>(
    repl: &impl Repl<Commands = C>,
    input: &str,
) -> Result<Option<Response<C>>> {
    let args = shlex::split(input).ok_or(anyhow!("Invalid quoting."))?;
    let cli = Cli::try_parse_from(args)?;
    repl.respond(cli.command).await
}

pub fn read_line(prompt: &str) -> Result<String> {
    let history_res = get_history_path(prompt);

    let mut rl = DefaultEditor::new()?;

    if let Some(history_path) = &history_res {
        rl.load_history(history_path).ok();
    }

    let res = rl.readline(&format!("{}> ", prompt.blue()))?;

    if let Some(history_path) = &history_res {
        rl.add_history_entry(res.clone())?;
        rl.save_history(history_path)?;
    }

    Ok(res)
}
