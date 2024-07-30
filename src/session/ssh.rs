use std::{sync::Arc, time::Duration};

use super::Session;

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use russh::{client, keys::key, Channel, ChannelMsg};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    select, time,
};
use url::Url;

struct Client;

#[async_trait]
impl client::Handler for Client {
    type Error = anyhow::Error;

    // TODO: implement properly
    async fn check_server_key(&mut self, _server_public_key: &key::PublicKey) -> Result<bool> {
        Ok(true)
    }
}

pub struct Ssh {
    url: Url,
    session: client::Handle<Client>,
    channel: Channel<client::Msg>,
}

impl Session for Ssh {
    async fn connect(url: Url) -> Result<Self> {
        let session = create_session(&url).await?;
        let channel = create_channel(&session).await?;
        Ok(Self { url, session, channel })
    }

    async fn start(&mut self) -> Result<()> {
        if self.session.is_closed() {
            self.session = create_session(&self.url).await?;
        }

        if let Ok(None) = time::timeout(Duration::from_millis(50), self.channel.wait()).await {
            self.channel = create_channel(&self.session).await?;
        }

        let mut stdin = BufReader::new(io::stdin());
        let mut stdout = BufWriter::new(io::stdout());
        let mut buf = vec![0; 1024];
        let mut stdin_closed = false;

        loop {
            select! {
                r = stdin.read(&mut buf), if !stdin_closed => {
                    match r {
                        Ok(0) => {
                            stdin_closed = true;
                            self.channel.eof().await?;
                        },
                        Ok(n) => self.channel.data(&buf[..n]).await?,
                        Err(e) => return Err(e.into()),
                    };
                },
                Some(msg) = self.channel.wait() => {
                    match msg {
                        ChannelMsg::Data { ref data } => {
                            stdout.write_all(data).await?;
                            stdout.flush().await?;
                        }
                        // ChannelMsg::ExitStatus { exit_status } => {
                            // code = exit_status;
                        ChannelMsg::ExitStatus { .. } => {
                            if !stdin_closed {
                                self.channel.eof().await?;
                            }
                            break;
                        }
                        _ => {}
                    };
                }
            }
        }

        Ok(())
    }
}

async fn create_session(url: &Url) -> Result<client::Handle<Client>> {
    let host = url.host_str().ok_or(anyhow!("No host provided."))?;
    let port = url.port().unwrap_or(22);

    let config = Arc::new(client::Config {
        ..Default::default()
    });

    let ssh = Client {};
    let mut session = client::connect(config, (host, port), ssh).await?;

    let auth_res = session
        .authenticate_password(url.username(), url.password().unwrap_or(""))
        .await?;
    if !auth_res {
        bail!("Authentication (with password) failed.");
    }

    Ok(session)
}

async fn create_channel(session: &client::Handle<Client>) -> Result<Channel<client::Msg>> {
    let channel = session.channel_open_session().await?;
    let (w, h) = crossterm::terminal::size()?;
    channel
        .request_pty(
            false,
            &std::env::var("TERM").unwrap_or("xterm".into()),
            w.into(),
            h.into(),
            0,
            0,
            &[],
        )
        .await?;
    channel.request_shell(false).await?;
    Ok(channel)
}
