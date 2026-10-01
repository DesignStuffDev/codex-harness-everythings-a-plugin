//! Dedicated native-search worker; its cwd is fixed before process startup.

fn main() -> anyhow::Result<()> {
    // Separate from per-session native traversal/matcher worker allocations.
    // Leave blocking capacity for transport spooling while native joins run.
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(32)
        .enable_all()
        .build()?
        .block_on(codex_file_search_local_plugin::run_stdio())
}
