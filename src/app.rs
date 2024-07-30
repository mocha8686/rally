use anyhow::{ bail, Result};
use async_trait::async_trait;
use clap::Subcommand;
use url::Url;

use crate::{repl::{start, Repl, Response}, session::{ssh::Ssh, Session}};

pub struct App;

impl App {
    pub fn new() -> Self {
        App
    }

    pub async fn start(&self) -> Result<()> {
        start(self).await
    }
}

#[async_trait]
impl Repl for App {
    type Commands = Commands;

    fn prompt(&self) -> &str {
        env!("CARGO_PKG_NAME")
    }

    async fn respond(&self, command: Self::Commands) -> Result<Option<Response<Self::Commands>>> {
        match command {
            Commands::Connect { url } => match url.scheme() {
                "ssh" => {
                    let mut session = Ssh::connect(url).await?;
                    session.start().await?;
                }
                _ => bail!("Scheme {} is not supported.", url.scheme()),
            },
            Commands::Exit => {
                return Ok(Some(Response::Exit));
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Connect to a new remote.
    Connect { url: Url },

    /// Exit the application.
    #[command(aliases = ["quit", "q"])]
    Exit,
}
