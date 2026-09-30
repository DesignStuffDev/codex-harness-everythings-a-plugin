//! Independently packaged native model catalog implementation.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    codex_model_catalog_native_plugin::run_stdio().await
}

