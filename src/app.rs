use async_trait::async_trait;
use clap::{command, Args, Subcommand};
use crossterm::{cursor, terminal, ExecutableCommand, QueueableCommand};
use miette::{bail, miette, Context, IntoDiagnostic, Result};
use tokio::{
    fs::File,
    io::{self, AsyncReadExt, AsyncWriteExt},
};
use url::Url;

use crate::{
    repl::Repl,
    session::{
        impls::ssh::Ssh,
        scheme::Scheme,
        serde::DeserializedSession,
        store::{Sessions, StoredSession},
        Session,
    },
    style::Style,
};

#[derive(Default)]
pub struct App {
    sessions: Sessions,
}

impl App {
    pub async fn new() -> Result<Self> {
        let res = match File::open("rally.toml").await {
            Ok(mut file) => {
                let mut data = String::new();
                let sessions = file
                    .read_to_string(&mut data)
                    .await
                    .into_diagnostic()
                    .and_then(|_| toml::from_str(&data).into_diagnostic())
                    .wrap_err("Failed to load sessions")?;

                Self { sessions }
            }
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => Self::default(),
                _ => bail!(e),
            },
        };

        Ok(res)
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
            Commands::Connect { url } => self.handle_connect(url).await?,
            Commands::Exit => {
                return Ok(true);
            }
            Commands::Clear => {
                let (_, lines) = cursor::position().into_diagnostic()?;
                std::io::stdout()
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
    pub async fn cleanup(mut self) -> Result<()> {
        let serialized = toml::to_string(&self.sessions)
            .into_diagnostic()
            .wrap_err("Error while saving sessions")?;

        let mut file = File::create("rally.toml")
            .await
            .into_diagnostic()
            .wrap_err("Failed to save sessions")?;

        file.write_all(serialized.as_bytes())
            .await
            .into_diagnostic()
            .wrap_err("Failed to save sessions")?;

        for (_, session) in self.sessions.iter_mut() {
            if let DeserializedSession::Initialized(ref mut session) = session {
                session.close().await?;
            }
        }

        Ok(())
    }

    async fn handle_connect(&mut self, url: Url) -> Result<()> {
        let scheme: Scheme = url.scheme().parse()?;
        let session = self.create_session(url, scheme, None).await?;
        session.start().await
    }

    async fn create_session(
        &mut self,
        url: Url,
        scheme: Scheme,
        key: Option<String>,
    ) -> Result<&mut StoredSession> {
        let session = match scheme {
            Scheme::Ssh => Ssh::connect(url).await?,
        };

        let session = if let Some(key) = key {
            self.sessions
                .insert(key.clone(), DeserializedSession::Initialized(session));
            self.sessions.get_mut(&key).unwrap().unwrap()
        } else {
            self.sessions.add(session)
        };

        Ok(session)
    }

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

                let session = match session {
                    DeserializedSession::Uninitialized(connection_info) => {
                        let connection_info = connection_info.clone();
                        self.create_session(connection_info.url, connection_info.scheme, Some(id))
                            .await?
                    }
                    DeserializedSession::Initialized(session) => {
                        if session.is_connected().await {
                            session.send(b"\n").await?;
                        } else {
                            session.reconnect().await?;
                        }
                        session
                    }
                };

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
    #[command(aliases = ["conn", "c"])]
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
    #[command(aliases = ["ses", "s"])]
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
    #[command(aliases = ["fg", "connect", "conn", "c", "o"])]
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
