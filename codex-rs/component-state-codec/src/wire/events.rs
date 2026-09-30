//! Trusted process transport DTOs. These deliberately bypass public rollout omissions.
//! External enum tags retain arbitrary-precision JSON values without Serde content buffering.

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::protocol::EventMsg")]
pub(crate) enum EventMsgWire {
    Error(
        #[serde(with = "super::events_nested::ErrorEventWire")]
        codex_protocol::protocol::ErrorEvent,
    ),
    Warning(codex_protocol::protocol::WarningEvent),
    AuthRecoveryStarted(codex_protocol::protocol::AuthRecoveryEvent),
    AuthRecoveryCompleted(codex_protocol::protocol::AuthRecoveryEvent),
    GuardianWarning(codex_protocol::protocol::WarningEvent),
    RealtimeConversationStarted(codex_protocol::protocol::RealtimeConversationStartedEvent),
    RealtimeConversationRealtime(codex_protocol::protocol::RealtimeConversationRealtimeEvent),
    RealtimeConversationClosed(codex_protocol::protocol::RealtimeConversationClosedEvent),
    RealtimeConversationSdp(codex_protocol::protocol::RealtimeConversationSdpEvent),
    ModelReroute(codex_protocol::protocol::ModelRerouteEvent),
    ModelVerification(codex_protocol::protocol::ModelVerificationEvent),
    TurnModerationMetadata(codex_protocol::protocol::TurnModerationMetadataEvent),
    SafetyBuffering(codex_protocol::protocol::SafetyBufferingEvent),
    ContextCompacted(codex_protocol::protocol::ContextCompactedEvent),
    ThreadRolledBack(codex_protocol::protocol::ThreadRolledBackEvent),
    TurnStarted(codex_protocol::protocol::TurnStartedEvent),
    ThreadSettingsApplied(
        #[serde(with = "super::events_nested::ThreadSettingsAppliedEventWire")]
        codex_protocol::protocol::ThreadSettingsAppliedEvent,
    ),
    TurnComplete(
        #[serde(with = "super::events_nested::TurnCompleteEventWire")]
        codex_protocol::protocol::TurnCompleteEvent,
    ),
    TokenCount(
        #[serde(with = "super::events_nested::TokenCountEventWire")]
        codex_protocol::protocol::TokenCountEvent,
    ),
    AgentMessage(codex_protocol::protocol::AgentMessageEvent),
    UserMessage(
        #[serde(with = "super::events_paths::UserMessageEventWire")]
        codex_protocol::protocol::UserMessageEvent,
    ),
    AgentReasoning(codex_protocol::protocol::AgentReasoningEvent),
    AgentReasoningRawContent(codex_protocol::protocol::AgentReasoningRawContentEvent),
    AgentReasoningSectionBreak(codex_protocol::protocol::AgentReasoningSectionBreakEvent),
    SessionConfigured(
        #[serde(with = "super::events_nested::SessionConfiguredEventWire")]
        codex_protocol::protocol::SessionConfiguredEvent,
    ),
    EnvironmentConnected(codex_protocol::protocol::EnvironmentConnectionEvent),
    EnvironmentDisconnected(codex_protocol::protocol::EnvironmentConnectionEvent),
    ThreadGoalUpdated(codex_protocol::protocol::ThreadGoalUpdatedEvent),
    ThreadQueueChanged(codex_protocol::protocol::ThreadQueueChangedEvent),
    McpStartupUpdate(codex_protocol::protocol::McpStartupUpdateEvent),
    McpStartupComplete(codex_protocol::protocol::McpStartupCompleteEvent),
    McpToolCallBegin(
        #[serde(with = "super::events_json::McpToolCallBeginEventWire")]
        codex_protocol::protocol::McpToolCallBeginEvent,
    ),
    McpToolCallEnd(
        #[serde(with = "super::events_json::McpToolCallEndEventWire")]
        codex_protocol::protocol::McpToolCallEndEvent,
    ),
    WebSearchBegin(codex_protocol::protocol::WebSearchBeginEvent),
    WebSearchEnd(codex_protocol::protocol::WebSearchEndEvent),
    ImageGenerationBegin(codex_protocol::protocol::ImageGenerationBeginEvent),
    ImageGenerationEnd(
        #[serde(with = "super::events_paths::ImageGenerationEndEventWire")]
        codex_protocol::protocol::ImageGenerationEndEvent,
    ),
    ExecCommandBegin(
        #[serde(with = "super::events_paths::ExecCommandBeginEventWire")]
        codex_protocol::protocol::ExecCommandBeginEvent,
    ),
    ExecCommandOutputDelta(codex_protocol::protocol::ExecCommandOutputDeltaEvent),
    TerminalInteraction(codex_protocol::protocol::TerminalInteractionEvent),
    ExecCommandEnd(
        #[serde(with = "super::events_paths::ExecCommandEndEventWire")]
        codex_protocol::protocol::ExecCommandEndEvent,
    ),
    ViewImageToolCall(codex_protocol::protocol::ViewImageToolCallEvent),
    ExecApprovalRequest(
        #[serde(with = "super::events_approvals::ExecApprovalRequestEventWire")]
        codex_protocol::approvals::ExecApprovalRequestEvent,
    ),
    RequestPermissions(
        #[serde(with = "super::events_approvals::RequestPermissionsEventWire")]
        codex_protocol::request_permissions::RequestPermissionsEvent,
    ),
    RequestUserInput(codex_protocol::request_user_input::RequestUserInputEvent),
    DynamicToolCallRequest(codex_protocol::dynamic_tools::DynamicToolCallRequest),
    DynamicToolCallResponse(codex_protocol::protocol::DynamicToolCallResponseEvent),
    ElicitationRequest(
        #[serde(with = "super::events_approvals::ElicitationRequestEventWire")]
        codex_protocol::approvals::ElicitationRequestEvent,
    ),
    ApplyPatchApprovalRequest(
        #[serde(with = "super::events_file_changes::ApplyPatchApprovalRequestEventWire")]
        codex_protocol::approvals::ApplyPatchApprovalRequestEvent,
    ),
    GuardianAssessment(
        #[serde(with = "super::events_approvals::GuardianAssessmentEventWire")]
        codex_protocol::approvals::GuardianAssessmentEvent,
    ),
    DeprecationNotice(codex_protocol::protocol::DeprecationNoticeEvent),
    StreamError(codex_protocol::protocol::StreamErrorEvent),
    PatchApplyBegin(
        #[serde(with = "super::events_file_changes::PatchApplyBeginEventWire")]
        codex_protocol::protocol::PatchApplyBeginEvent,
    ),
    PatchApplyUpdated(
        #[serde(with = "super::events_file_changes::PatchApplyUpdatedEventWire")]
        codex_protocol::protocol::PatchApplyUpdatedEvent,
    ),
    PatchApplyEnd(
        #[serde(with = "super::events_file_changes::PatchApplyEndEventWire")]
        codex_protocol::protocol::PatchApplyEndEvent,
    ),
    TurnDiff(codex_protocol::protocol::TurnDiffEvent),
    RealtimeConversationListVoicesResponse(
        codex_protocol::protocol::RealtimeConversationListVoicesResponseEvent,
    ),
    PlanUpdate(codex_protocol::plan_tool::UpdatePlanArgs),
    TurnAborted(
        #[serde(with = "super::events_nested::TurnAbortedEventWire")]
        codex_protocol::protocol::TurnAbortedEvent,
    ),
    ShutdownComplete,
    EnteredReviewMode(codex_protocol::protocol::EnteredReviewModeEvent),
    ExitedReviewMode(
        #[serde(with = "super::events_review::ExitedReviewModeEventWire")]
        codex_protocol::protocol::ExitedReviewModeEvent,
    ),
    RawResponseItem(
        #[serde(with = "super::events_nested::RawResponseItemEventWire")]
        codex_protocol::protocol::RawResponseItemEvent,
    ),
    RawResponseCompleted(
        #[serde(with = "super::events_nested::RawResponseCompletedEventWire")]
        codex_protocol::protocol::RawResponseCompletedEvent,
    ),
    ItemStarted(
        #[serde(with = "super::events_items::ItemStartedEventWire")]
        codex_protocol::protocol::ItemStartedEvent,
    ),
    ItemCompleted(
        #[serde(with = "super::events_items::ItemCompletedEventWire")]
        codex_protocol::protocol::ItemCompletedEvent,
    ),
    HookStarted(
        #[serde(with = "super::events_nested::HookStartedEventWire")]
        codex_protocol::protocol::HookStartedEvent,
    ),
    HookCompleted(
        #[serde(with = "super::events_nested::HookCompletedEventWire")]
        codex_protocol::protocol::HookCompletedEvent,
    ),
    AgentMessageContentDelta(codex_protocol::protocol::AgentMessageContentDeltaEvent),
    PlanDelta(codex_protocol::protocol::PlanDeltaEvent),
    ReasoningContentDelta(codex_protocol::protocol::ReasoningContentDeltaEvent),
    ReasoningRawContentDelta(codex_protocol::protocol::ReasoningRawContentDeltaEvent),
    CollabAgentSpawnBegin(
        #[serde(with = "super::events_nested::CollabAgentSpawnBeginEventWire")]
        codex_protocol::protocol::CollabAgentSpawnBeginEvent,
    ),
    CollabAgentSpawnEnd(
        #[serde(with = "super::events_nested::CollabAgentSpawnEndEventWire")]
        codex_protocol::protocol::CollabAgentSpawnEndEvent,
    ),
    CollabAgentInteractionBegin(codex_protocol::protocol::CollabAgentInteractionBeginEvent),
    CollabAgentInteractionEnd(codex_protocol::protocol::CollabAgentInteractionEndEvent),
    CollabWaitingBegin(codex_protocol::protocol::CollabWaitingBeginEvent),
    CollabWaitingEnd(codex_protocol::protocol::CollabWaitingEndEvent),
    CollabCloseBegin(codex_protocol::protocol::CollabCloseBeginEvent),
    CollabCloseEnd(codex_protocol::protocol::CollabCloseEndEvent),
    CollabResumeBegin(codex_protocol::protocol::CollabResumeBeginEvent),
    CollabResumeEnd(codex_protocol::protocol::CollabResumeEndEvent),
    SubAgentActivity(codex_protocol::protocol::SubAgentActivityEvent),
}
