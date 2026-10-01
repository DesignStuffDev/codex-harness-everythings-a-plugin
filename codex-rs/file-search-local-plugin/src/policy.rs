//! Native allocation bounds and serialized wire bounds use different units.

use std::path::PathBuf;
use std::time::Duration;

use codex_file_search::NativeBackendLimits;
use codex_file_search_api::SearchError;
use codex_file_search_api::SearchErrorKind;
use codex_file_search_component::ServiceLimits;
use serde::Deserialize;
use serde_json::Value;

use crate::positive;

const DEFAULT_NATIVE_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
const MIN_NATIVE_SNAPSHOT_BYTES: usize = 64 * 1024;
const MAX_NATIVE_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct NativeConfig {
    native_snapshot_bytes: usize,
}

impl Default for NativeConfig {
    fn default() -> Self {
        Self {
            native_snapshot_bytes: DEFAULT_NATIVE_SNAPSHOT_BYTES,
        }
    }
}

pub(crate) fn native_limits(
    service: &ServiceLimits,
    config: Value,
) -> Result<NativeBackendLimits, SearchError> {
    service.validate()?;
    let config: NativeConfig = if config.is_null() {
        NativeConfig::default()
    } else {
        if !config.is_object() {
            return Err(SearchError::new(
                SearchErrorKind::InvalidInput,
                "native-search plugin configuration must be an object or null",
            ));
        }
        serde_json::from_value(config).map_err(|_| {
            SearchError::new(
                SearchErrorKind::InvalidInput,
                "native-search plugin configuration has an unsupported field or type",
            )
        })?
    };
    if !(MIN_NATIVE_SNAPSHOT_BYTES..=MAX_NATIVE_SNAPSHOT_BYTES)
        .contains(&config.native_snapshot_bytes)
    {
        return Err(SearchError::new(
            SearchErrorKind::UnsupportedOption,
            "native-search snapshot allocation limit must be between 64 KiB and 64 MiB",
        ));
    }

    // The wire cap includes at least one byte per root/exclude item plus all
    // encoded payload bytes. The native input charge also includes PathBuf or
    // String headers. This conservative checked expansion admits every input
    // within that wire cap without confusing serialization with heap layout.
    let input_multiplier = std::mem::size_of::<PathBuf>()
        .max(std::mem::size_of::<String>())
        .checked_add(1)
        .ok_or_else(input_overflow)?;
    let input_bytes = usize::try_from(service.max_roots_options_bytes)
        .ok()
        .and_then(|bytes| bytes.checked_mul(input_multiplier))
        .ok_or_else(input_overflow)?;
    Ok(NativeBackendLimits {
        max_sessions: positive(service.max_leases as usize)?,
        resources: service.resources.into_native()?,
        max_query_bytes: positive(service.max_query_utf8_bytes as usize)?,
        max_roots_options_bytes: positive(input_bytes)?,
        max_matches: positive(service.max_matches as usize)?,
        max_snapshot_bytes: positive(config.native_snapshot_bytes)?,
        max_poll_wait: Duration::from_millis(u64::from(service.max_poll_wait_ms)),
    })
}

fn input_overflow() -> SearchError {
    SearchError::new(
        SearchErrorKind::UnsupportedOption,
        "native-search input accounting is unsupported on this target",
    )
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
