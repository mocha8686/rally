use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use miette::{bail, miette, Result};
use serde::{Deserialize, Serialize};
use tabled::{builder::Builder, Table};

use super::{serde::DeserializedSession, ConnectionInfo, Session};

#[derive(Serialize)]
pub struct StoredSession {
    #[serde(flatten)]
    pub connection_info: ConnectionInfo,

    #[serde(skip)]
    pub session: Box<dyn Session + Sync + Send>,
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
        K: AsRef<str> + Send + Sync,
    {
        let id = id.as_ref();
        let session = self
            .sessions
            .shift_remove(id)
            .ok_or_else(|| miette!("No session found with ID `{}`.", id))?;
        if let DeserializedSession::Initialized(mut session) = session {
            session.disconnect().await;
        }

        Ok(())
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
