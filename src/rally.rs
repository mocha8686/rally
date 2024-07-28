use anyhow::Result;
use clap::Subcommand;

use crate::repl::{start, Repl};

pub struct Rally;

impl Rally {
    pub fn new() -> Self {
        Rally
    }

    pub fn start(&self) -> Result<()> {
        start(self)
    }
}

impl Repl for Rally {
    type Commands = Commands;

    fn prompt(&self) -> &str {
        "rally"
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
