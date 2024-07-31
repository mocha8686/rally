mod app;
mod history;
mod repl;
mod session;

use miette::Result;
use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let rally = App::new();
    rally.start().await?;
    Ok(())
}
