use async_trait::async_trait;
use indexmap::IndexMap;
use miette::{bail, miette, Context, IntoDiagnostic, Result};
use tabled::{builder::Builder, Table};
use tokio::sync::mpsc;
use url::Url;

use crate::{repl::Repl, termcraft::Termcraft};

pub mod ssh;

#[async_trait]
pub trait Session {
    async fn connect(url: Url) -> Result<Self>
    where
        Self: Sized;

    fn url(&self) -> &Url;

    async fn read(&mut self) -> Result<Option<Box<[u8]>>>;
    async fn is_connected(&mut self) -> bool;
    async fn reconnect(&mut self) -> Result<()>;

    async fn send_unchecked(&mut self, data: &[u8]) -> Result<()>;

    async fn close(&mut self) -> Result<()>;

    async fn start(&mut self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(10);
        let mut termcraft = Termcraft::new(tx);

        loop {
            let res = self.read().await?;
            if let Some(input) = res {
                if input.trim_ascii().starts_with(b"#") {
                    let input = String::from_utf8(input[1..].to_vec())
                        .into_diagnostic()
                        .wrap_err("Failed to parse command.")?;

                    if termcraft.handle_command(&input).await? {
                        break Ok(());
                    }

                    if let Some(data) = rx.recv().await.flatten() {
                        self.send_unchecked(&data).await?;
                    }
                } else {
                    self.send_unchecked(&input).await?;
                }
            } else {
                break Ok(());
            }
        }
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        if !self.is_connected().await {
            bail!("Cannot send data to a closed session.");
        }
        self.send_unchecked(data).await
    }
}

pub type StoredSession = Box<dyn Session + Sync + Send>;

#[derive(Default)]
pub struct Sessions {
    sessions: IndexMap<String, StoredSession>,
}

impl Sessions {
    pub fn add(&mut self, session: StoredSession) -> &mut StoredSession {
        let id = self.sessions.len().to_string();
        self.sessions.insert(id.clone(), session);
        self.sessions.get_mut(&id).unwrap()
    }

    pub async fn remove<K>(&mut self, id: &K) -> Result<()>
    where
        K: AsRef<str> + Send,
    {
        let id = id.as_ref();
        let mut session = self
            .sessions
            .shift_remove(id)
            .ok_or_else(|| miette!("No session found with ID `{}`.", id))?;
        session.close().await
    }

    pub fn rename<K, N>(&mut self, id: &K, new_id: &N) -> Result<()>
    where
        K: AsRef<str> + Send,
        N: ToString + Send,
    {
        let id = id.as_ref();
        let new_id = new_id.to_string();

        if self.sessions.contains_key(&new_id) {
            bail!("Session already exists with ID `{}`.", new_id);
        }

        let entry = self
            .sessions
            .shift_remove_entry(id)
            .map(|(_, session)| (new_id, session))
            .ok_or_else(|| miette!("No session found with ID `{}`.", id))?;
        self.sessions.insert(entry.0, entry.1);
        Ok(())
    }

    pub fn get<K>(&self, id: &K) -> Option<&StoredSession>
    where
        K: AsRef<str> + Send,
    {
        self.sessions.get(id.as_ref())
    }

    pub fn get_mut<K>(&mut self, id: &K) -> Option<&mut StoredSession>
    where
        K: AsRef<str> + Send,
    {
        self.sessions.get_mut(id.as_ref())
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub async fn table(&mut self) -> Table {
        let mut builder = Builder::default();
        builder.push_record(["ID", "URL", "Status"]);
        for (id, session) in &mut self.sessions {
            let status = if session.is_connected().await {
                "Connected".to_string()
            } else {
                "Disconnected".to_string()
            };
            builder.push_record([id.to_owned(), session.url().to_string(), status]);
        }
        builder.build()
    }
}
