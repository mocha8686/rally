mod app;
mod history;
mod repl;
mod session;

use app::App;
use miette::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let rally = App::new();
    rally.start().await?;
    Ok(())
}
