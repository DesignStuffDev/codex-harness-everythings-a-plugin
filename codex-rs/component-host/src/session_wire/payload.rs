//! Private spool ownership; serialization/parser task behavior is unchanged.
use anyhow::Context;
use anyhow::Result;
use serde_json::Value;
use tempfile::NamedTempFile;

/// Private temporary storage avoids a logical JSON-size cap on native histories.
pub(crate) struct Payload {
    pub(super) file: NamedTempFile,
    pub(super) bytes: u64,
}

impl Payload {
    pub(crate) async fn from_value(value: Value) -> Result<Self> {
        tokio::task::spawn_blocking(move || {
            let mut file = NamedTempFile::new()?;
            serde_json::to_writer(&mut file, &value)?;
            let bytes = file.as_file().metadata()?.len();
            Ok(Self { file, bytes })
        })
        .await
        .context("serialize session payload task")?
    }

    pub(crate) async fn into_value(self) -> Result<Value> {
        tokio::task::spawn_blocking(move || {
            serde_json::from_reader(self.file.reopen()?).context("decode session payload")
        })
        .await
        .context("deserialize session payload task")?
    }
}
