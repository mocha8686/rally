mod app;
mod history;
mod repl;
mod session;
mod style;

use app::App;
use miette::Result;
use repl::Repl;

#[tokio::main]
async fn main() -> Result<()> {
    let mut rally = App::new();
    rally.start().await?;
    Ok(())
}
