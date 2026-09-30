//! Typed timeline snapshots for an authenticated component connection.
//!
//! Local SQLite projections already omit some analytics fields. Other stores
//! may return them, so the native ThreadStore boundary must preserve them too.

use codex_app_server_protocol as api;
use codex_thread_store::TimelinePage;
use serde::Deserialize;
use serde::Serialize;

#[path = "timeline_items.rs"]
mod items;

#[derive(Serialize, Deserialize)]
#[serde(remote = "TimelinePage")]
struct TimelinePageWire {
    #[serde(with = "entries")]
    items: Vec<api::ThreadTimelineEntry>,
    next_cursor: Option<String>,
    active_realtime_session_at_page_start: Option<String>,
}

// External tags avoid Serde buffering arbitrary-precision JSON embedded in an
// item. These are component shapes, never app-server response shapes.
#[derive(Serialize, Deserialize)]
#[serde(remote = "api::ThreadTimelineEntry")]
enum TimelineEntryWire {
    Item {
        position: u64,
        turn_id: String,
        #[serde(with = "items::boxed")]
        item: Box<api::ThreadItem>,
    },
    Realtime {
        position: u64,
        item: api::ThreadRealtimeItem,
    },
    TurnStarted {
        position: u64,
        turn_id: String,
        started_at: Option<i64>,
    },
    TurnCompleted {
        position: u64,
        turn_id: String,
        status: api::TurnStatus,
        error: Option<api::TurnError>,
        started_at: Option<i64>,
        completed_at: Option<i64>,
        duration_ms: Option<i64>,
    },
}

pub(crate) fn serialize<S: serde::Serializer>(
    page: &TimelinePage,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    TimelinePageWire::serialize(page, serializer)
}

pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<TimelinePage, D::Error> {
    TimelinePageWire::deserialize(deserializer)
}

mod entries {
    use super::*;

    #[derive(Serialize)]
    #[serde(transparent)]
    struct Borrowed<'a>(#[serde(with = "TimelineEntryWire")] &'a api::ThreadTimelineEntry);

    #[derive(Deserialize)]
    #[serde(transparent)]
    struct Owned(#[serde(with = "TimelineEntryWire")] api::ThreadTimelineEntry);

    pub(super) fn serialize<S: serde::Serializer>(
        value: &[api::ThreadTimelineEntry],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(value.iter().map(Borrowed))
    }

    pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<api::ThreadTimelineEntry>, D::Error> {
        Vec::<Owned>::deserialize(deserializer)
            .map(|items| items.into_iter().map(|item| item.0).collect())
    }
}

#[cfg(test)]
#[path = "timeline_tests.rs"]
mod tests;
