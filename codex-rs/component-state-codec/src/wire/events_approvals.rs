//! Trusted process transport DTOs. These deliberately bypass public rollout omissions.
//! External enum tags retain arbitrary-precision JSON values without Serde content buffering.

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::approvals::GuardianAssessmentAction")]
pub(crate) enum GuardianAssessmentActionWire {
    Command {
        source: codex_protocol::approvals::GuardianCommandSource,
        command: String,
        cwd: codex_utils_path_uri::LegacyAppPathString,
    },
    Execve {
        source: codex_protocol::approvals::GuardianCommandSource,
        program: String,
        argv: Vec<String>,
        #[serde(with = "super::paths::absolute")]
        cwd: codex_utils_absolute_path::AbsolutePathBuf,
    },
    WriteStdin {
        approval_id: String,
        process_id: String,
        stdin: String,
        cwd: codex_utils_path_uri::PathUri,
    },
    ApplyPatch {
        cwd: codex_utils_path_uri::LegacyAppPathString,
        files: Vec<codex_utils_path_uri::LegacyAppPathString>,
    },
    NetworkAccess {
        target: String,
        host: String,
        protocol: codex_protocol::approvals::NetworkApprovalProtocol,
        port: u16,
    },
    McpToolCall {
        server: String,
        tool_name: String,
        connector_id: Option<String>,
        connector_name: Option<String>,
        tool_title: Option<String>,
    },
    RequestPermissions {
        reason: Option<String>,
        #[serde(with = "super::events_permissions::RequestPermissionProfileWire")]
        permissions: codex_protocol::request_permissions::RequestPermissionProfile,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::approvals::GuardianAssessmentEvent")]
pub(crate) struct GuardianAssessmentEventWire {
    review_reason: Option<codex_protocol::approvals::GuardianReviewReason>,
    #[serde(with = "super::events_scalars::optional_model_context")]
    model_context: Option<codex_protocol::items::ModelInvocationContext>,
    id: String,
    target_item_id: Option<String>,
    plugin_id: Option<String>,
    script_path: Option<String>,
    turn_id: String,
    started_at_ms: i64,
    completed_at_ms: Option<i64>,
    status: codex_protocol::approvals::GuardianAssessmentStatus,
    risk_level: Option<codex_protocol::approvals::GuardianRiskLevel>,
    user_authorization: Option<codex_protocol::approvals::GuardianUserAuthorization>,
    rationale: Option<String>,
    decision_source: Option<codex_protocol::approvals::GuardianAssessmentDecisionSource>,
    #[serde(with = "GuardianAssessmentActionWire")]
    action: codex_protocol::approvals::GuardianAssessmentAction,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::approvals::ExecApprovalRequestEvent")]
pub(crate) struct ExecApprovalRequestEventWire {
    #[serde(with = "super::events_scalars::optional_model_context")]
    model_context: Option<codex_protocol::items::ModelInvocationContext>,
    kind: codex_protocol::approvals::ExecApprovalKind,
    call_id: String,
    plugin_id: Option<String>,
    script_path: Option<String>,
    approval_id: Option<String>,
    turn_id: String,
    environment_id: Option<String>,
    started_at_ms: i64,
    command: Vec<String>,
    cwd: codex_utils_path_uri::LegacyAppPathString,
    reason: Option<String>,
    network_approval_context: Option<codex_protocol::approvals::NetworkApprovalContext>,
    proposed_execpolicy_amendment: Option<codex_protocol::approvals::ExecPolicyAmendment>,
    proposed_network_policy_amendments:
        Option<Vec<codex_protocol::approvals::NetworkPolicyAmendment>>,
    #[serde(with = "super::events_permissions::optional_additional")]
    additional_permissions: Option<codex_protocol::models::AdditionalPermissionProfile>,
    available_decisions: Option<Vec<codex_protocol::protocol::ReviewDecision>>,
    #[serde(with = "super::events_paths::parsed_command::vec")]
    parsed_cmd: Vec<codex_protocol::parse_command::ParsedCommand>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::request_permissions::RequestPermissionsEvent")]
pub(crate) struct RequestPermissionsEventWire {
    call_id: String,
    turn_id: String,
    environment_id: Option<String>,
    started_at_ms: i64,
    reason: Option<String>,
    #[serde(with = "super::events_permissions::RequestPermissionProfileWire")]
    permissions: codex_protocol::request_permissions::RequestPermissionProfile,
    cwd: Option<codex_utils_path_uri::LegacyAppPathString>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::approvals::ElicitationRequest")]
pub(crate) enum ElicitationRequestWire {
    UserVerification {
        #[serde(with = "super::events_json::optional_value")]
        meta: Option<serde_json::Value>,
        title: String,
        description: String,
        challenge: String,
    },
    Form {
        #[serde(with = "super::events_json::optional_value")]
        meta: Option<serde_json::Value>,
        message: String,
        requested_schema: serde_json::Value,
    },
    OpenAiForm {
        #[serde(with = "super::events_json::optional_value")]
        meta: Option<serde_json::Value>,
        message: String,
        requested_schema: serde_json::Value,
    },
    OpenAiElicitationForm {
        #[serde(with = "super::events_json::optional_value")]
        meta: Option<serde_json::Value>,
        message: String,
        requested_schema: serde_json::Value,
    },
    Url {
        #[serde(with = "super::events_json::optional_value")]
        meta: Option<serde_json::Value>,
        message: String,
        url: String,
        elicitation_id: String,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(remote = "codex_protocol::approvals::ElicitationRequestEvent")]
pub(crate) struct ElicitationRequestEventWire {
    turn_id: Option<String>,
    server_name: String,
    id: codex_protocol::mcp::RequestId,
    #[serde(with = "ElicitationRequestWire")]
    request: codex_protocol::approvals::ElicitationRequest,
}
