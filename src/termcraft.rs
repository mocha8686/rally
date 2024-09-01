use async_trait::async_trait;
use clap::Subcommand;
use miette::{IntoDiagnostic, Result};
use tokio::sync::mpsc;

use crate::repl::Repl;

type MessageSender = mpsc::Sender<Option<Box<[u8]>>>;

pub struct Termcraft {
    tx: MessageSender,
}

impl Termcraft {
    pub const fn new(tx: MessageSender) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl Repl for Termcraft {
    type Commands = Commands;

    fn prompt(&self) -> &str {
        ""
    }

    async fn respond(&mut self, command: Self::Commands) -> Result<bool> {
        match command {
            Commands::Bg => Ok(true),
            Commands::Echo { msg } => {
                let msg = (msg.join(" ") + "\n").into_bytes().into_boxed_slice();
                self.tx.send(Some(msg)).await.into_diagnostic()?;
                Ok(false)
            }
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Background the current session.
    #[command(alias = "exit")]
    Bg,

    /// Echo test!
    Echo {
        #[arg(trailing_var_arg = true)]
        msg: Vec<String>,
    },
}
