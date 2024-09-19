pub mod scheme;
pub mod ssh;

use std::ops::{Deref, DerefMut};

use async_trait::async_trait;
use indexmap::IndexMap;
use miette::{bail, miette, Context, IntoDiagnostic, Result};
use scheme::Scheme;
use serde::{Deserialize, Serialize};
use tabled::{builder::Builder, Table};
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

    async fn close(&mut self) -> Result<()>;

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

#[derive(Serialize)]
pub struct StoredSession {
    #[serde(flatten)]
    connection_info: ConnectionInfo,

    #[serde(skip)]
    session: Box<dyn Session + Sync + Send>,
}

impl Deref for StoredSession {
    type Target = Box<dyn Session + Sync + Send>;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl DerefMut for StoredSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum DeserializedSession {
    Uninitialized(ConnectionInfo),

    #[serde(skip_deserializing)]
    Initialized(StoredSession),
}

impl DeserializedSession {
    pub fn unwrap(&mut self) -> &mut StoredSession {
        match self {
            Self::Uninitialized(_) => {
                panic!("called `MaybeUninitSession::unwrap()` on an `Uninitialized` session")
            }
            Self::Initialized(session) => session,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Sessions {
    sessions: IndexMap<String, DeserializedSession>,
}

impl Sessions {
    pub fn add(&mut self, session: StoredSession) -> &mut StoredSession {
        let id = self.sessions.len().to_string();
        self.sessions
            .insert(id.clone(), DeserializedSession::Initialized(session));
        let session = self.sessions.get_mut(&id).unwrap();
        session.unwrap()
    }

    pub async fn remove<K>(&mut self, id: &K) -> Result<()>
    where
        K: AsRef<str> + Send,
    {
        let id = id.as_ref();
        let session = self
            .sessions
            .shift_remove(id)
            .ok_or_else(|| miette!("No session found with ID `{}`.", id))?;
        if let DeserializedSession::Initialized(mut session) = session {
            session.close().await
        } else {
            Ok(())
        }
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

        let (id, session) = self
            .sessions
            .shift_remove_entry(id)
            .map(|(_, session)| (new_id, session))
            .ok_or_else(|| miette!("No session found with ID `{}`.", id))?;
        self.sessions.insert(id, session);
        Ok(())
    }

    pub async fn table(&mut self) -> Table {
        let mut builder = Builder::default();
        builder.push_record(["ID", "URL", "Status"]);
        for (id, session) in &mut self.sessions {
            let (url, status) = match session {
                DeserializedSession::Initialized(session) => {
                    let status = if session.is_connected().await {
                        "Connected"
                    } else {
                        "Disconnected"
                    };
                    (session.connection_info.url.to_string(), status)
                }
                DeserializedSession::Uninitialized(ConnectionInfo { url, .. }) => {
                    (url.to_string(), "Disconnected")
                }
            };

            builder.push_record([id.to_owned(), url, status.to_string()]);
        }
        builder.build()
    }
}

impl Deref for Sessions {
    type Target = IndexMap<String, DeserializedSession>;

    fn deref(&self) -> &Self::Target {
        &self.sessions
    }
}

impl DerefMut for Sessions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sessions
    }
}
