use std::{io, sync::Arc};

use miette::{IntoDiagnostic, Result};
use tokio::{
    sync::{mpsc, Mutex, Notify},
    task::{self, JoinHandle},
};

pub type SharedInput = Arc<Mutex<Input>>;
pub type InputReceiver = mpsc::Receiver<String>;

pub struct Input {
    _thread: JoinHandle<Result<()>>,
    rx: InputReceiver,
    notify: Arc<Notify>,
}

impl Input {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(5);
        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();

        let thread = task::spawn(async move {
            let stdin = io::stdin();
            loop {
                let mut buf = String::new();

                if let Err(_) = stdin.read_line(&mut buf) {
                    continue;
                }

                if let Err(e) = tx.send(buf.clone()).await {
                    break Err(e).into_diagnostic();
                };

                notify.notified().await;
            }
        });

        Self {
            _thread: thread,
            rx,
            notify: notify_clone,
        }
    }

    pub fn rx(&mut self) -> &mut InputReceiver {
        &mut self.rx
    }

    pub fn notify(&self) -> Arc<Notify> {
        self.notify.clone()
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
