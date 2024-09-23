pub mod impls;
pub mod scheme;
pub mod store;
pub mod serde;

use async_trait::async_trait;
use miette::{Context, IntoDiagnostic, Result};
use scheme::Scheme;
use ::serde::{Deserialize, Serialize};
use store::StoredSession;
use tokio::sync::mpsc;
use url::Url;

use crate::{repl::Repl, termcraft::Termcraft};

#[async_trait]
pub trait Session {
    async fn connect(url: Url) -> Result<StoredSession>
    where
        Self: Sized;

    async fn read(&mut self) -> Result<Option<Box<[u8]>>>;
    async fn is_connected(&mut self) -> bool;
    async fn reconnect(&mut self) -> Result<()>;

    async fn send(&mut self, data: &[u8]) -> Result<()>;

    async fn close(&mut self);

    async fn start(&mut self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(10);
        let mut termcraft = Termcraft::new(tx);

        loop {
            let res = self.read().await?;
            let Some(input) = res else {
                break Ok(());
            };

            if input.trim_ascii().starts_with(b"#") {
                let input = String::from_utf8(input[1..].to_vec())
                    .into_diagnostic()
                    .wrap_err("Failed to parse command.")?;

                if termcraft.handle_command(&input).await? {
                    break Ok(());
                }

                if let Some(data) = rx.recv().await.flatten() {
                    self.send(&data).await?;
                }
            } else {
                self.send(&input).await?;
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConnectionInfo {
    pub url: Url,
    pub scheme: Scheme,
}
