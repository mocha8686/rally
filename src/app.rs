use std::io;

use async_trait::async_trait;
use clap::{command, Args, Subcommand};
use crossterm::{cursor, terminal, ExecutableCommand, QueueableCommand};
use miette::{bail, miette, IntoDiagnostic, Result};
use url::Url;

use crate::{
    repl::Repl,
    session::{ssh::Ssh, Session, Sessions},
    style::Style,
};

#[derive(Default)]
pub struct App {
    sessions: Sessions,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Repl for App {
    type Commands = Commands;

    fn prompt(&self) -> &str {
        env!("CARGO_PKG_NAME")
    }

    async fn respond(&mut self, command: Self::Commands) -> Result<bool> {
        match command {
            Commands::Connect { url } => match url.scheme() {
                "ssh" => {
                    let session = Ssh::connect(url).await?;
                    let session = self.sessions.add(Box::new(session));
                    session.start().await?;
                }
                _ => bail!("Scheme {} is not supported.", url.scheme()),
            },
            Commands::Exit => {
                return Ok(true);
            }
            Commands::Clear => {
                let (_, lines) = cursor::position().into_diagnostic()?;
                io::stdout()
                    .queue(terminal::ScrollUp(lines))
                    .and_then(|s| s.execute(cursor::MoveTo(0, 0)))
                    .into_diagnostic()?;
            }
            Commands::Sessions(SessionsArgs { command }) => {
                self.handle_session_command(command).await?;
            }
        }
        Ok(false)
    }
}

impl App {
    async fn handle_session_command(&mut self, command: SessionsCommands) -> Result<()> {
        match command {
            SessionsCommands::List => {
                let out = if self.sessions.is_empty() {
                    "No sessions found.".to_string()
                } else {
                    self.sessions.table().await.style().to_string()
                };
                println!("{out}");
            }
            SessionsCommands::Open { id } => {
                let session = self
                    .sessions
                    .get_mut(&id)
                    .ok_or_else(|| miette!("No session found with ID `{}`.", id))?;
                if session.is_connected().await {
                    session.send(b"\n").await?;
                } else {
                    session.reconnect().await?;
                }
                session.start().await?;
            }
            SessionsCommands::Rename { id, new_id } => self.sessions.rename(&id, &new_id)?,
            SessionsCommands::Remove { id } => self.sessions.remove(&id).await?,
        }

        Ok(())
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Connect to a new remote.
    Connect {
        /// URL to connect to (proto://user:pass@host:port)
        url: Url,
    },

    /// Exit the application.
    #[command(aliases = ["quit", "q"])]
    Exit,

    /// Clear the screen.
    Clear,

    /// Manage sessions.
    Sessions(SessionsArgs),
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct SessionsArgs {
    #[command(subcommand)]
    command: SessionsCommands,
}

#[derive(Debug, Subcommand)]
enum SessionsCommands {
    /// List open and stored sessions.
    #[command(alias = "ls")]
    List,

    /// Open a session.
    #[command(aliases = ["fg", "connect"])]
    Open {
        /// Session ID.
        id: String,
    },

    /// Rename a session.
    #[command(alias = "mv")]
    Rename {
        /// Current session ID.
        id: String,

        /// ID to rename the session to.
        new_id: String,
    },

    /// Remove a session.
    #[command(alias = "rm")]
    Remove {
        /// Session ID.
        id: String,
    },
}
