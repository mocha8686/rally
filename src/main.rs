mod app;
mod history;
mod input;
mod repl;
mod session;
mod style;
mod termcraft;

use app::App;
use miette::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut rally = App::new().await?;
    rally.start_app().await?;
    rally.cleanup().await?;
    Ok(())
}
