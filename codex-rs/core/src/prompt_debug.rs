use std::sync::Arc;

use codex_exec_server::EnvironmentManager;
use codex_exec_server::ExecServerRuntimeOptions;
use codex_extension_api::ExtensionRegistry;
use codex_extension_api::UserInstructionsProvider;
use codex_login::AuthManager;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::SessionSource;
use codex_protocol::user_input::UserInput;
use codex_thread_store::ThreadStoreShutdownGuard;
use tokio_util::sync::CancellationToken;

use crate::component_persistence::persistence_from_config;
use crate::config::Config;
use crate::resolve_installation_id;
use crate::session::session::Session;
use crate::session::turn::build_prompt;
use crate::state_db_bridge::StateDbHandle;
use crate::thread_manager::StartThreadOptions;
use crate::thread_manager::ThreadManager;

/// Build the model-visible `input` list for a single debug turn.
#[doc(hidden)]
pub async fn build_prompt_input(
    mut config: Config,
    input: Vec<UserInput>,
    state_db: Option<StateDbHandle>,
    extensions: Arc<ExtensionRegistry<Config>>,
    user_instructions_provider: Arc<dyn UserInstructionsProvider>,
) -> CodexResult<Vec<ResponseItem>> {
    config.ephemeral = true;

    let auth_manager =
        AuthManager::shared_from_config(&config, /*enable_codex_api_key_env*/ false)
            .await
            .map_err(|err| CodexErr::Fatal(err.to_string()))?;

    let local_runtime_paths = ExecServerRuntimeOptions::from_optional_paths(
        config.codex_self_exe.clone(),
        config.codex_linux_sandbox_exe.clone(),
    )?;

    let installation_id = resolve_installation_id(&config.codex_home).await?;
    let environment_manager = Arc::new(
        EnvironmentManager::from_codex_home(
            config.codex_home.clone(),
            Some(local_runtime_paths),
            config.http_client_factory(),
        )
        .await
        .map_err(|err| CodexErr::Fatal(err.to_string()))?,
    );
    let persistence = persistence_from_config(&config, state_db.clone())
        .await
        .map_err(|err| CodexErr::Fatal(err.to_string()))?;
    let store_lifecycle = ThreadStoreShutdownGuard::new(Arc::clone(&persistence.thread_store));
    let thread_manager = ThreadManager::new_with_persistence(
        &config,
        Arc::clone(&auth_manager),
        crate::thread_manager::build_models_manager(&config, Arc::clone(&auth_manager)),
        crate::CodexAppsToolsCache::default(),
        SessionSource::Exec,
        environment_manager,
        extensions,
        user_instructions_provider,
        /*analytics_events_client*/ None,
        crate::thread_manager::passthrough_image_store(),
        persistence,
        crate::local_agent_graph_store_from_state_db(state_db.as_ref()),
        installation_id,
        /*attestation_provider*/ None,
        /*external_time_provider*/ None,
    )
    .with_default_attachment_store_components();
    let output = async {
        let thread = thread_manager
            .start_thread(StartThreadOptions::new(config))
            .await?;

        let output = build_prompt_input_from_session(&thread.thread.session, input).await;
        let shutdown = thread.thread.shutdown_and_wait().await;
        let _removed = thread_manager.remove_thread(&thread.thread_id).await;

        shutdown?;
        output
    }
    .await;
    let store_shutdown = store_lifecycle
        .shutdown()
        .await
        .map_err(|err| CodexErr::Fatal(err.to_string()));
    match (output, store_shutdown) {
        (Ok(output), Ok(())) => Ok(output),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(primary), Err(cleanup)) => Err(CodexErr::Fatal(format!(
            "{primary}; thread-store shutdown also failed: {cleanup}"
        ))),
    }
}

pub(crate) async fn build_prompt_input_from_session(
    sess: &Arc<Session>,
    input: Vec<UserInput>,
) -> CodexResult<Vec<ResponseItem>> {
    let turn_context = sess.new_default_turn().await;
    // Prompt debugging builds a standalone request without entering run_turn.
    let step_context = sess
        .capture_step_context(Arc::clone(&turn_context), &CancellationToken::new())
        .await?;
    sess.record_context_updates_and_set_reference_context_item(step_context.as_ref())
        .await?;

    if !input.is_empty() {
        let response_item = sess.response_item_from_user_input(input);
        sess.record_conversation_items(
            turn_context.as_ref(),
            &step_context.settings.model_info,
            std::slice::from_ref(&response_item),
        )
        .await;
    }

    let prompt_input = sess
        .clone_history()
        .await
        .for_prompt(&step_context.settings.model_info.input_modalities);
    let base_instructions = sess.get_base_instructions().await;
    let prompt = build_prompt(prompt_input, step_context.as_ref(), base_instructions);

    Ok(prompt.input)
}
