use async_trait::async_trait;
use clap::{
    command,
    error::{ContextKind, ErrorKind},
    Parser, Subcommand,
};
use miette::{miette, IntoDiagnostic, Result};
use owo_colors::OwoColorize;
use rustyline::{Config, DefaultEditor, EditMode};

use crate::history::get_history_path;

#[async_trait]
pub trait Repl {
    type Commands: Subcommand + Send;

    fn prompt(&self) -> &str;
    async fn respond(
        &mut self,
        command: Self::Commands,
    ) -> Result<Option<Response>>;

    async fn start(&mut self) -> Result<()> {
        loop {
            let line = read_line(self.prompt())?;
            if line.is_empty() {
                continue;
            }

            match self.handle_command(&line).await {
                Ok(Some(Response::Exit)) => break,
                Ok(None) => {}
                Err(e) => {
                    eprintln!("{e:?}");
                }
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, input: &str) -> Result<Option<Response>> {
        let input = input.trim();
        let args = shlex::split(input).ok_or_else(|| miette!("Invalid quoting."))?;
        let res = Cli::try_parse_from(args);

        match res {
            Ok(cli) => self.respond(cli.command).await,
            Err(e) => match e.kind() {
                ErrorKind::DisplayHelp
                | ErrorKind::DisplayVersion
                | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                    println!("{e}");
                    Ok(None)
                }
                ErrorKind::InvalidSubcommand => {
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
                ErrorKind::MissingRequiredArgument => {
                    let args = e.get(ContextKind::InvalidArg).unwrap();
                    let usage = e.get(ContextKind::Usage).unwrap();

                    let report = miette!(
                        help = format!("{usage}\nFor more information, try 'help'."),
                        "The following required arguments were not provided:\n\t{args}\n"
                    );

                    Err(report)
                }
                _ => Err(dbg!(e)).into_diagnostic(),
            },
        }
    }
}

pub enum Response {
    Exit,
}

#[derive(Debug, Parser)]
#[command(multicall = true)]
struct Cli<T: Subcommand> {
    #[command(subcommand)]
    command: T,
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
