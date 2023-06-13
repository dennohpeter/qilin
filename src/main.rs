use tokio;
use qilin_core::runner;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // for debugging
    // env::set_var("RUST_LOG", "trace");

    runner().await?;
    

    Ok(())
}
