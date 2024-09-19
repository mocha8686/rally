use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use miette::{bail, miette, IntoDiagnostic, Result};
use russh::{client, keys::key, Channel, ChannelMsg};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    select, time,
};
use url::Url;

use crate::session::{scheme::Scheme, store::StoredSession, ConnectionInfo, Session};

struct Client;

#[async_trait]
impl client::Handler for Client {
    type Error = russh::Error;

    // TODO: implement properly
    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct Ssh {
    url: Url,
    session: client::Handle<Client>,
    channel: Channel<client::Msg>,
    is_new_session: bool,
}

#[async_trait]
impl Session for Ssh {
    async fn connect(url: Url) -> Result<StoredSession> {
        let session = create_session(&url).await?;
        let channel = create_channel(&session).await?;

        let ssh = Self {
            url: url.clone(),
            session,
            channel,
            is_new_session: true,
        };
        let connection_info = ConnectionInfo {
            url,
            scheme: Scheme::Ssh,
        };

        Ok(StoredSession {
            connection_info,
            session: Box::new(ssh),
        })
    }

    async fn read(&mut self) -> Result<Option<Box<[u8]>>> {
        let mut stdin = BufReader::new(io::stdin());
        let mut stdout = BufWriter::new(io::stdout());
        let mut buf = vec![0; 1024];
        let mut is_new_command = !self.is_new_session;
        self.is_new_session = false;

        loop {
            select! {
                r = stdin.read(&mut buf) => {
                    break match r {
                        Ok(0) => {
                            self.close().await?;
                            Ok(None)
                        },
                        Ok(n) => {
                            let input = &buf[..n];
                            Ok(Some(input.into()))
                        },
                        Err(e) => Err(miette!(e)),
                    };
                },
                Some(msg) = self.channel.wait() => {
                    match msg {
                        ChannelMsg::Data { ref data } => {
                            if is_new_command {
                                if data.ends_with(b"\n") {
                                    is_new_command = false;
                                }
                            } else {
                                stdout.write_all(data).await.into_diagnostic()?;
                                stdout.flush().await.into_diagnostic()?;
                            }
                        }
                        ChannelMsg::ExitStatus { .. } => {
                            self.close().await?;
                            break Ok(None);
                        }
                        _ => {}
                    };
                }
            }
        }
    }

    async fn is_connected(&mut self) -> bool {
        self.session_is_connected() && self.channel_is_connected().await
    }

    async fn reconnect(&mut self) -> Result<()> {
        if !self.session_is_connected() {
            self.session = create_session(&self.url).await?;
        }

        if !self.channel_is_connected().await {
            self.channel = create_channel(&self.session).await?;
        }

        self.is_new_session = true;

        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.channel.data(data).await.into_diagnostic()
    }

    async fn close(&mut self) -> Result<()> {
        self.channel.eof().await.into_diagnostic()
    }
}

impl Ssh {
    fn session_is_connected(&self) -> bool {
        !self.session.is_closed()
    }

    async fn channel_is_connected(&mut self) -> bool {
        let timeout = time::timeout(Duration::from_millis(50), self.channel.wait()).await;
        !matches!(timeout, Ok(None))
    }
}

async fn create_session(url: &Url) -> Result<client::Handle<Client>> {
    let host = url.host_str().ok_or_else(|| miette!("No host provided."))?;
    let port = url.port().unwrap_or(22);

    let config = Arc::new(client::Config::default());

    let ssh = Client {};
    let mut session = client::connect(config, (host, port), ssh)
        .await
        .into_diagnostic()?;

    let auth_res = session
        .authenticate_password(url.username(), url.password().unwrap_or(""))
        .await
        .into_diagnostic()?;
    if !auth_res {
        bail!("Authentication (with password) failed.");
    }

    Ok(session)
}

async fn create_channel(session: &client::Handle<Client>) -> Result<Channel<client::Msg>> {
    let channel = session.channel_open_session().await.into_diagnostic()?;
    let (w, h) = crossterm::terminal::size().into_diagnostic()?;
    channel
        .request_pty(
            false,
            &std::env::var("TERM").unwrap_or_else(|_| "xterm".into()),
            w.into(),
            h.into(),
            0,
            0,
            &[],
        )
        .await
        .into_diagnostic()?;
    channel.request_shell(false).await.into_diagnostic()?;
    Ok(channel)
}
