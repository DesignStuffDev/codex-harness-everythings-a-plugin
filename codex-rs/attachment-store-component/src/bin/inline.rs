#[tokio::main]
async fn main() -> std::process::ExitCode {
    match codex_attachment_store_component::run_native_stdio().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(_) => {
            eprintln!("native attachment component protocol failed");
            std::process::ExitCode::FAILURE
        }
    }
}
