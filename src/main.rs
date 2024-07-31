use anyhow::Result;
use rally::app::App;
mod app;
mod history;
mod repl;
mod session;

#[tokio::main]
async fn main() -> Result<()> {
    let rally = App::new();
    rally.start().await?;
    Ok(())
}
