use anyhow::Result;
use rally::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let rally = App::new();
    rally.start().await?;
    Ok(())
}
