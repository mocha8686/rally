pub mod impls;
pub mod scheme;
pub mod serde;
pub mod store;

use ::serde::{Deserialize, Serialize};
use async_trait::async_trait;
use miette::{miette, IntoDiagnostic, Result};
use scheme::Scheme;
use store::StoredSession;
use tokio::{io::AsyncWriteExt, select, sync::mpsc, task::JoinHandle};
use url::Url;

use crate::{input::SharedInput, repl::Repl, termcraft::Termcraft};

#[async_trait]
pub trait Session {
    async fn new(url: Url) -> Result<StoredSession>
    where
        Self: Sized;

    async fn connect(&mut self) -> Result<()>;

    fn tx(&self) -> Option<mpsc::Sender<In>>;
    fn rx(&mut self) -> Option<&mut mpsc::Receiver<Out>>;
    fn thread(&mut self) -> Option<&mut JoinHandle<Result<()>>>;

    async fn is_connected(&mut self) -> bool {
        self.thread()
            .map(|thread| !thread.is_finished())
            .unwrap_or(false)
    }

    async fn send(&mut self, data: Box<[u8]>) -> Result<()> {
        let tx = self
            .tx()
            .ok_or_else(|| miette!("Channel to session thread is closed."))?;
        tx.send(In::Stdin(data)).await.into_diagnostic()?;
        Ok(())
    }

    async fn send_bytes(&mut self, data: &[u8]) -> Result<()> {
        self.send(data[..].into()).await
    }

    async fn disconnect(&mut self) {
        let Some(tx) = self.tx() else { return };
        tx.send(In::Close).await.ok();
        let Some(thread) = self.thread() else { return };

        if let Err(e) = thread.await {
            println!("{e}");
        };
    }

    async fn start(&mut self, input: SharedInput) -> Result<()> {
        let (termcraft_tx, mut termcraft_rx) = mpsc::channel(1);
        let mut termcraft = Termcraft::new(termcraft_tx);

        let tx = self
            .tx()
            .ok_or_else(|| miette!("Channel does not exist."))?;
        let rx = self
            .rx()
            .ok_or_else(|| miette!("Channel does not exist."))?;

        let mut input = input.lock().await;
        let notify = input.notify();
        let input = input.rx();
        notify.notify_waiters();

        let res = loop {
            select! {
                Some(data) = input.recv() => {
                    notify.notify_one();
                    let data = data.trim_start();

                    if data.starts_with("#") {
                        if termcraft.handle_command(&data).await? {
                            break Ok(());
                        }

                        if let Some(data) = termcraft_rx.recv().await.flatten() {
                            tx.send(In::Stdin(data)).await.into_diagnostic()?;
                        }

                        tx.send(In::Stdin(b"\n"[..].into()))
                            .await
                            .into_diagnostic()?;
                    } else {
                        let data: Box<[u8]> = data.bytes().collect();
                        tx.send(In::Stdin(data)).await.into_diagnostic()?;
                    }
                }
                Some(input) = rx.recv() => match input {
                    Out::Stdout(data) => {
                        let mut stdout = tokio::io::stdout();
                        stdout.write_all(&data).await.into_diagnostic()?;
                        stdout.flush().await.into_diagnostic()?;
                    },
                }
            }
        };

        tx.send(In::Close).await.into_diagnostic()?;
        res
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConnectionInfo {
    pub url: Url,
    pub scheme: Scheme,
}

#[derive(Debug, Clone)]
pub enum In {
    Stdin(Box<[u8]>),
    Close,
}

#[derive(Debug, Clone)]
pub enum Out {
    Stdout(Box<[u8]>),
}
