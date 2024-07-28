use std::io::{stdin, stdout, Write};

use anyhow::{anyhow, Result};
use clap::{command, Parser, Subcommand};

pub trait Repl {
    type Commands: Subcommand;

    fn prompt(&self) -> &str;
    fn respond(&self, command: Self::Commands) -> Result<bool>;
}

#[derive(Debug, Parser)]
#[command(multicall = true)]
struct Cli<T: Subcommand> {
    #[command(subcommand)]
    command: T,
}

pub fn start(repl: &impl Repl) -> Result<()> {
    loop {
        let line = read_line(repl.prompt())?;
        if line.is_empty() {
            continue;
        }

        match handle_command(repl, &line) {
            Ok(should_quit) => {
                if should_quit {
                    break;
                }
            }
            Err(e) => eprintln!("{}", e),
        }
    }

    Ok(())
}

pub fn handle_command(repl: &impl Repl, input: &str) -> Result<bool> {
    let args = shlex::split(input).ok_or(anyhow!("Invalid quoting."))?;
    let cli = Cli::try_parse_from(args)?;
    repl.respond(cli.command)
}

pub fn read_line(prompt: &str) -> Result<String> {
    print!("{prompt}> ");
    stdout().flush()?;
    let mut buf = String::new();
    stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}
