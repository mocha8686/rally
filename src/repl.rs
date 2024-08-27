use async_trait::async_trait;
use clap::{command, error::ContextKind, Parser, Subcommand};
use miette::{miette, IntoDiagnostic, Result};
use owo_colors::OwoColorize;
use rustyline::{Config, DefaultEditor, EditMode};

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

pub async fn start<R>(repl: &R) -> Result<()>
where
    R: Repl,
{
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
                if let Some(e) = e.downcast_ref::<clap::Error>() {}
                eprintln!("{e:?}");
            }
        }
    }

    Ok(())
}

pub async fn handle_command<C, R>(repl: &R, input: &str) -> Result<Option<Response<C>>>
where
    C: Subcommand,
    R: Repl<Commands = C>
{
    let input = input.trim();
    let args = shlex::split(input).ok_or_else(|| miette!("Invalid quoting."))?;
    let res = Cli::try_parse_from(args);

    match res {
        Ok(cli) => repl.respond(cli.command).await,
        Err(e) => match e.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                println!("{e}");
                Ok(None)
            }
            clap::error::ErrorKind::InvalidSubcommand => {
                let invalid = e.get(ContextKind::InvalidSubcommand).unwrap();
                let suggested = e
                    .get(ContextKind::SuggestedSubcommand)
                    .map_or_else(String::new, |s| format!("A similar command exists: '{s}'"));

                let report = miette!(
                    help = format!("{suggested}\nFor more information, try 'help'."),
                    "Unrecognized command '{invalid}'."
                );

                Err(report)
            }
            _ => Err(e).into_diagnostic(),
        },
    }
}

pub fn read_line(prompt: &str) -> Result<String> {
    let history_res = get_history_path(prompt);

    let config = Config::builder().edit_mode(EditMode::Vi).build();
    let mut rl = DefaultEditor::with_config(config).into_diagnostic()?;

    if let Some(history_path) = &history_res {
        rl.load_history(history_path).ok();
    }

    let res = rl
        .readline(&format!("{}> ", prompt.blue()))
        .into_diagnostic()?;

    if let Some(history_path) = &history_res {
        rl.add_history_entry(res.clone()).into_diagnostic()?;
        rl.save_history(history_path).into_diagnostic()?;
    }

    Ok(res)
}
