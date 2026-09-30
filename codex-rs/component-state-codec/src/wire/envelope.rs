use codex_history::CodexHarnessMetadata;
use codex_history::GuardianHistoryCheckpoint;
use codex_history::ResponseItemEnvelope;
use codex_protocol::models::ResponseItem;
use serde::Deserialize;
use serde::Serialize;

/// Metadata remains per envelope, preserving `None` independently of adjacent records.
#[derive(Serialize, Deserialize)]
#[serde(remote = "ResponseItemEnvelope", deny_unknown_fields)]
pub(crate) struct EnvelopeWire {
    #[serde(with = "super::response::TrustedResponseItem")]
    item: ResponseItem,
    metadata: Option<CodexHarnessMetadata>,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "GuardianHistoryCheckpoint")]
pub(crate) struct GuardianCheckpointWire(
    #[serde(with = "envelopes::vec")] Vec<ResponseItemEnvelope>,
);

remote_adapter!(
    envelopes,
    ResponseItemEnvelope,
    EnvelopeWire,
    "EnvelopeWire"
);
remote_adapter!(
    guardian_checkpoint,
    GuardianHistoryCheckpoint,
    GuardianCheckpointWire,
    "GuardianCheckpointWire"
);
