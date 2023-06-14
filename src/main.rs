use anyhow::Result;
use qilin_core::runner;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    // for debugging
    // env::set_var("RUST_LOG", "trace");

    runner().await?;

    Ok(())
}
