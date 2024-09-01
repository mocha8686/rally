use async_trait::async_trait;
use indexmap::IndexMap;
use miette::{bail, miette, Result};
use tabled::{builder::Builder, Table};
use url::Url;

pub mod ssh;

#[async_trait]
pub trait Session {
    async fn connect(url: Url) -> Result<Self>
    where
        Self: Sized;

    fn url(&self) -> &Url;

    async fn read_loop(&mut self) -> Result<()>;
    async fn is_connected(&mut self) -> bool;
    async fn reconnect(&mut self) -> Result<()>;

    async fn send_data(&mut self, data: &[u8]) -> Result<()>;

    async fn start(&mut self) -> Result<()> {
        if !self.is_connected().await {
            self.reconnect().await?;
        }
        self.read_loop().await
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        if !self.is_connected().await {
            bail!("Cannot send data to a closed session.");
        }
        self.send_data(data).await
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

    pub fn remove(&mut self, id: impl AsRef<str>) -> Result<()> {
        let id = id.as_ref();
        self.sessions
            .shift_remove(id)
            .map(|_| ())
            .ok_or_else(|| miette!("No session found with ID `{}`.", id))
    }

    pub fn rename(&mut self, id: impl AsRef<str>, new_id: &impl ToString) -> Result<()> {
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

    pub fn get(&self, id: impl AsRef<str>) -> Option<&StoredSession> {
        self.sessions.get(id.as_ref())
    }

    pub fn get_mut(&mut self, id: impl AsRef<str>) -> Option<&mut StoredSession> {
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
