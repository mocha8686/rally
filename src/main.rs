use anyhow::Result;
use rally::rally::Rally;

fn main() -> Result<()> {
    let rally = Rally::new();
    rally.start()?;
    Ok(())
}
