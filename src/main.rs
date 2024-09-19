mod app;
mod history;
mod repl;
mod session;
mod style;
mod termcraft;

use app::App;
use miette::Result;
use repl::Repl;

#[tokio::main]
async fn main() -> Result<()> {
    let mut rally = App::new().await?;
    rally.start().await?;
    rally.cleanup().await?;
    Ok(())
}
