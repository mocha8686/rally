use std::sync::Arc;

use async_trait::async_trait;
use miette::{bail, miette, Context, IntoDiagnostic, Result};
use russh::{client, keys::key, Channel, ChannelMsg, Disconnect};
use tokio::{
    select,
    sync::mpsc,
    task::{self, JoinHandle},
};
use url::Url;

use crate::session::{scheme::Scheme, store::StoredSession, ConnectionInfo, In, Out, Session};

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
    thread: Option<task::JoinHandle<Result<()>>>,

    tx: Option<mpsc::Sender<In>>,
    rx: Option<mpsc::Receiver<Out>>,
}

#[async_trait]
impl Session for Ssh {
    async fn new(url: Url) -> Result<StoredSession> {
        let ssh = Self {
            url: url.clone(),
            thread: None,

            tx: None,
            rx: None,
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

    async fn connect(&mut self) -> Result<()> {
        let session = create_session(&self.url).await?;
        let mut channel = create_channel(&session).await?;
        let (tx_in, rx_in) = mpsc::channel(10);
        let (tx_out, rx_out) = mpsc::channel(10);

        self.tx.replace(tx_in);
        self.rx.replace(rx_out);

        let (tx, mut rx) = (tx_out, rx_in);

        let handle = task::spawn(async move {
            let mut is_new_command = false;

            loop {
                select! {
                    Some(input) = rx.recv() => match input {
                        In::Stdin(data) => {
                            channel.data(&data[..]).await.into_diagnostic()?;
                            is_new_command = true;
                        },
                        In::Close => break,
                    },
                    Some(msg) = channel.wait() => match msg {
                        ChannelMsg::Data { data } => {
                            if is_new_command {
                                if data.ends_with(b"\n") {
                                    is_new_command = false;
                                }
                            } else {
                                let data: Box<[u8]> = data.to_vec().into_boxed_slice();
                                tx.send(Out::Stdout(data)).await.into_diagnostic()?;
                            }
                        }
                        ChannelMsg::ExitStatus { .. } => {
                            break;
                        }
                        _ => {}
                    }
                }
            }

            channel.eof().await.into_diagnostic()?;
            session
                .disconnect(Disconnect::ByApplication, "", "English")
                .await
                .into_diagnostic()?;

            Ok(())
        });

        self.thread.replace(handle);

        Ok(())
    }

    fn tx(&self) -> Option<mpsc::Sender<In>> {
        self.tx.clone()
    }

    fn rx(&mut self) -> Option<&mut mpsc::Receiver<Out>> {
        self.rx.as_mut()
    }

    fn thread(&mut self) -> Option<&mut JoinHandle<Result<()>>> {
        self.thread.as_mut()
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
    let channel = session
        .channel_open_session()
        .await
        .into_diagnostic()
        .wrap_err("Failed to create ssh channel")?;
    let (w, h) = crossterm::terminal::size()
        .into_diagnostic()
        .wrap_err("Failed to create ssh channel")?;
    channel
        .request_pty(false, "xterm".into(), w.into(), h.into(), 0, 0, &[])
        .await
        .into_diagnostic()?;
    channel
        .request_shell(false)
        .await
        .into_diagnostic()
        .wrap_err("Failed to create ssh channel")?;
    Ok(channel)
}
