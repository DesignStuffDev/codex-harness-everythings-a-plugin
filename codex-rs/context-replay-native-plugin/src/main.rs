#[tokio::main]
async fn main() -> Result<(), codex_context_replay_native_plugin::ReplayWorkerError> {
    codex_context_replay_native_plugin::run_stdio().await
}
