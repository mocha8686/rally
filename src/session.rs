use anyhow::Result;
use url::Url;

pub mod ssh;

pub trait Session {
    async fn connect(url: Url) -> Result<Self>
    where
        Self: Sized;
    async fn start(&mut self) -> Result<()>;
}
