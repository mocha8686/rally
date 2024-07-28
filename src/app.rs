use anyhow::Result;
use clap::Subcommand;

use crate::repl::{start, Repl};

pub struct App;

impl App {
    pub fn new() -> Self {
        App
    }

    pub fn start(&self) -> Result<()> {
        start(self)
    }
}

impl Repl for App {
    type Commands = Commands;

    fn prompt(&self) -> &str {
        env!("CARGO_PKG_NAME")
    }

    fn respond(&self, command: Self::Commands) -> Result<bool> {
        match command {
            Commands::Test => {
                println!("Hello, world!");
            }
            Commands::Exit => {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Test,
    Exit,
}
