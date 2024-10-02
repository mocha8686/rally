use async_trait::async_trait;
use clap::{
    command,
    error::{ContextKind, ErrorKind},
    Parser, Subcommand,
};
use miette::{miette, IntoDiagnostic, Result};
use owo_colors::OwoColorize;
use tokio::io::{self, AsyncWriteExt};

use crate::input::{InputReceiver, SharedInput};

#[async_trait]
pub trait Repl {
    type Commands: Subcommand + Send;

    fn prompt(&self) -> &str;

    /// Respond to a command.
    ///
    /// Return Ok(true) to exit.
    async fn respond(&mut self, command: Self::Commands) -> Result<bool>;

    async fn start(&mut self, input: SharedInput) -> Result<()> {
        let notify = {
            let input = input.lock().await;
            input.notify()
        };

        loop {
            let line = {
                let mut input = input.lock().await;
                let mut rx = input.rx();
                read_line(self.prompt(), &mut rx).await?
            };

            if line.is_empty() {
                notify.notify_one();
                continue;
            }

            match self.handle_command(&line).await {
                Ok(true) => {
                    notify.notify_one();
                    break;
                },
                Ok(false) => {}
                Err(e) => {
                    eprintln!("{e:?}");
                }
            }
            notify.notify_one();
        }

        Ok(())
    }

    async fn handle_command(&mut self, input: &str) -> Result<bool> {
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
                    Ok(false)
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
                _ => Err(e).into_diagnostic(),
            },
        }
    }
}

#[derive(Debug, Parser)]
#[command(multicall = true)]
struct Cli<T: Subcommand> {
    #[command(subcommand)]
    command: T,
}

pub async fn read_line(prompt: &str, rx: &mut InputReceiver) -> Result<String> {
    // let history_res = get_history_path(prompt);
    //
    // let config = Config::builder().edit_mode(EditMode::Vi).build();
    // let mut rl = DefaultEditor::with_config(config).into_diagnostic()?;
    //
    // if let Some(history_path) = &history_res {
    //     rl.load_history(history_path).ok();
    // }

    let mut stdout = io::stdout();
    stdout.write_all(format!("{}> ", prompt.blue()).as_bytes()).await.into_diagnostic()?;
    stdout.flush().await.into_diagnostic()?;
    let res = rx
        .recv()
        .await
        .ok_or_else(|| miette!("Failed to read line."))?;

    // if let Some(history_path) = &history_res {
    //     rl.add_history_entry(res.clone()).into_diagnostic()?;
    //     rl.save_history(history_path).into_diagnostic()?;
    // }

    Ok(res)
}
