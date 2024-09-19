use serde::{Deserialize, Serialize};

use super::{store::StoredSession, ConnectionInfo};

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
