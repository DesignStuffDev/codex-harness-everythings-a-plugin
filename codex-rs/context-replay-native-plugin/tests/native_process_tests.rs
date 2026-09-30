//! Staged actual-package/process coverage; requires coordinated contract activation.

use anyhow::Context;
use anyhow::Result;
use codex_component_host::ComponentBinding;
use codex_component_host::ComponentCatalog;
use codex_context_engine::ContextManager;
use codex_context_replay::RECONSTRUCT_METHOD;
use codex_context_replay::REPLAY_COMPONENT_KIND;
use codex_context_replay::REPLAY_COMPONENT_NAME;
use codex_context_replay::ReplayInput;
use codex_context_replay::ReviewPolicy;
use codex_context_replay::RolloutReconstruction;
use codex_context_replay::reconstruct;
use codex_context_replay_component::ProcessReplay;
use codex_history::ResponseItemEnvelope;
use codex_history::RolloutItem;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::InternalChatMessageMetadataPassthrough;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::ThreadHistoryMode;
use codex_protocol::protocol::TruncationPolicy;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;

struct Installed {
    _root: TempDir,
    home: std::path::PathBuf,
    catalog: ComponentCatalog,
    binding: ComponentBinding,
}

impl Installed {
    fn new() -> Result<Self> {
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        let package = root.path().join("external-package");
        std::fs::create_dir(&package)?;
        let source = codex_utils_cargo_bin::cargo_bin("codex-context-replay-native-plugin")?;
        let executable = source.file_name().context("worker executable filename")?;
        codex_utils_cargo_bin::copy_executable(&source, &package.join(executable))?;
        std::fs::write(
            package.join("codex-component.json"),
            serde_json::to_vec(&json!({
                "api_version": 1,
                "id": "test.native-context-replay",
                "version": "0.1.0",
                "entrypoint": executable.to_string_lossy(),
                "components": [{"kind": "context_replay", "name": "default", "contract_version": 1}]
            }))?,
        )?;
        let id = codex_component_host::install(&home, &package)?;
        codex_component_host::select(
            &home,
            REPLAY_COMPONENT_KIND,
            REPLAY_COMPONENT_NAME,
            Some(&id),
        )?;
        std::fs::remove_dir_all(package)?;
        let catalog = ComponentCatalog::load(&home)?;
        let binding = catalog
            .selected(REPLAY_COMPONENT_KIND, REPLAY_COMPONENT_NAME)
            .context("installed selection")?;
        Ok(Self {
            _root: root,
            home,
            catalog,
            binding,
        })
    }
}

fn message(id: &str, role: &str, text: String) -> ResponseItemEnvelope {
    ResponseItemEnvelope::new(ResponseItem::Message {
        id: Some(codex_protocol::ResponseItemId::with_suffix("msg", id)),
        role: role.to_owned(),
        content: vec![ContentItem::InputText { text }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    })
}

fn input(label: &str) -> ReplayInput {
    ReplayInput {
        items: vec![RolloutItem::ResponseItem(message(
            "user-1",
            "user",
            label.to_owned(),
        ))],
        history_mode: ThreadHistoryMode::Legacy,
        truncation_policy: TruncationPolicy::Tokens(4_000),
        review_policy: ReviewPolicy {
            independent_review: true,
            retain_inherited_user_messages: true,
        },
    }
}

#[tokio::test]
async fn installed_worker_preserves_internal_state_and_large_chunked_history() -> Result<()> {
    let installed = Installed::new()?;
    let service = ProcessReplay::from_catalog(&installed.catalog)
        .await?
        .context("selected replay")?;
    let mut request = input("retain this original authorization");
    request.items.extend([
        RolloutItem::ResponseItem(ResponseItemEnvelope::new(ResponseItem::Reasoning {
            id: Some(codex_protocol::ResponseItemId::with_suffix("rs", "reasoning-1")),
            summary: Vec::new(),
            content: Some(vec![ReasoningItemContent::Text {
                text: "internal text retained by the trusted codec".to_owned(),
            }]),
            encrypted_content: None,
            internal_chat_message_metadata_passthrough: Some(
                InternalChatMessageMetadataPassthrough {
                    cell_id: Some("native-cell".to_owned()),
                    tool_calls_complete: Some(false),
                    ..Default::default()
                },
            ),
        })),
        RolloutItem::ResponseItem(ResponseItemEnvelope::new(ResponseItem::FunctionCallOutput {
            id: Some(codex_protocol::ResponseItemId::with_suffix("fc", "output-1")),
            call_id: None,
            name: Some("external_event".to_owned()),
            namespace: None,
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::Text("native result".to_owned()),
                success: Some(false),
            },
            internal_chat_message_metadata_passthrough: None,
        })),
    ]);
    for index in 0..1_200 {
        request.items.push(RolloutItem::ResponseItem(message(
            &format!("assistant-{index}"),
            "assistant",
            "a".repeat(4_096),
        )));
    }
    assert!(serde_json::to_vec(&request)?.len() > 4 * 1024 * 1024);
    let expected = reconstruct::<ContextManager>(request.clone());
    let actual = service.reconstruct(request).await?;
    assert_eq!(actual, expected);
    service.close().await?;
    Ok(())
}

#[tokio::test]
async fn invalid_wire_input_does_not_poison_the_next_native_reconstruction() -> Result<()> {
    let installed = Installed::new()?;
    let session = installed.binding.connect().await?;
    let error = session
        .call(RECONSTRUCT_METHOD, json!({"private_history": "must not appear in error"}))
        .await
        .expect_err("invalid input must fail");
    let diagnostic = error.to_string();
    assert!(diagnostic.contains("invalid context replay input"));
    assert!(!diagnostic.contains("must not appear in error"));
    let request = input("next request survives");
    let actual: RolloutReconstruction = serde_json::from_value(
        session.call(RECONSTRUCT_METHOD, serde_json::to_value(&request)?).await?,
    )?;
    assert_eq!(actual, reconstruct::<ContextManager>(request));
    session.close().await?;
    Ok(())
}

#[tokio::test]
async fn independent_sessions_and_existing_selection_survive_future_deselection() -> Result<()> {
    let installed = Installed::new()?;
    let first = ProcessReplay::connect(installed.binding.clone()).await?;
    let second = ProcessReplay::connect(installed.binding.clone()).await?;
    codex_component_host::select(
        &installed.home,
        REPLAY_COMPONENT_KIND,
        REPLAY_COMPONENT_NAME,
        /*plugin_id*/ None,
    )?;
    assert!(ProcessReplay::from_catalog(&ComponentCatalog::load(&installed.home)?)
        .await?
        .is_none());
    let one = input("first session authorization");
    let two = input("second session authorization");
    let (first_result, second_result) = tokio::join!(
        first.reconstruct(one.clone()),
        second.reconstruct(two.clone()),
    );
    assert_eq!(first_result?, reconstruct::<ContextManager>(one));
    assert_eq!(second_result?, reconstruct::<ContextManager>(two));
    first.close().await?;
    second.close().await?;
    Ok(())
}
