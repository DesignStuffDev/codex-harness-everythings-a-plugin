//! Native LocalThreadStore component executable, independent of codex-core.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    codex_thread_store_local_plugin::run_stdio().await
}
