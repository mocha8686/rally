use anyhow::Result;
use rally::app::App;

fn main() -> Result<()> {
    let rally = App::new();
    rally.start()?;
    Ok(())
}
