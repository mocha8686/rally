use std::io::{stdin, stdout, Write};

use anyhow::{anyhow, Result};
use clap::{command, Parser, Subcommand};

fn main() -> Result<()> {
    loop {
        let line = read_line()?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match respond(line) {
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

fn respond(line: &str) -> Result<bool> {
    let args = shlex::split(line).ok_or(anyhow!("Invalid quoting."))?;
    let cli = Cli::try_parse_from(args)?;
    match cli.command {
        Commands::Test => {
            println!("Hello, world!");
        }
        Commands::Exit => {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Debug, Parser)]
#[command(multicall = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Test,
    Exit,
}

fn read_line() -> Result<String> {
    print!("rally> ");
    stdout().flush()?;
    let mut buf = String::new();
    stdin().read_line(&mut buf)?;
    Ok(buf)
}
